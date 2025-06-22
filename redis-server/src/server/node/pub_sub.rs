mod error;

use std::{
    collections::HashMap,
    io::{
        self, BufReader,
        prelude::{BufRead, *},
    },
    net::TcpStream,
    sync::{
        Arc, Mutex,
        mpsc::{self, Receiver, Sender},
    },
    thread,
    time::Duration,
};

pub use error::InternalError;
use error::OperationError;
use log::Log;
use redis_cmd::{Command, pub_sub::*};
use redis_resp::{Array, BulkString, Integer, RespDataType, SimpleError};
use rustls::{ServerConnection, StreamOwned};
use uuid::Uuid;

use crate::server::node::cluster::{ClusterAction, message::payload::PublishPayload};

type SubsRegister = HashMap<BulkString, HashMap<Uuid, Sender<Vec<u8>>>>;

#[derive(Debug)]
pub struct PubSubBroker {
    subs_reg: Arc<Mutex<SubsRegister>>,
    logger_tx: Sender<Log>,
}

#[derive(Debug)]
pub enum PubSubAction {
    ClientCommand {
        stream: StreamOwned<ServerConnection, TcpStream>,
        cmd: PubSubCommand,
        reply_tx: Sender<Vec<u8>>,
    },
}

#[derive(Debug)]
pub struct PubSubEnvelope {
    pub stream: StreamOwned<ServerConnection, TcpStream>,
    pub cmd: PubSubCommand,
    pub cluster_tx: Sender<ClusterAction>,
}

#[derive(Debug)]
struct Subscriber {
    id: Uuid,
    reply_tx: Sender<Vec<u8>>,
}

impl PubSubBroker {
    pub fn start(logger_tx: Sender<Log>) -> (Sender<PubSubEnvelope>, Sender<PublishPayload>) {
        let (tx, rx) = mpsc::channel();

        let mut broker = Self {
            subs_reg: Arc::new(Mutex::new(HashMap::new())),
            logger_tx: logger_tx.clone(),
        };

        let (publish_tx, publish_rx) = mpsc::channel::<PublishPayload>();

        let reg = broker.subs_reg.clone();
        let logger_tx_clone = logger_tx.clone();

        thread::spawn(move || {
            for msg in publish_rx {
                PubSubBroker::publish_from_cluster(
                    &msg.channel,
                    msg.message,
                    reg.clone(),
                    logger_tx_clone.clone(),
                )
                .unwrap_or_else(|err| {
                    logger_tx_clone.send(log::error!("{err}")).unwrap();
                    Vec::new()
                });
            }
        });

        thread::spawn(move || {
            while let Ok(envel) = rx.recv() {
                if let Err(err) = broker.process(envel) {
                    logger_tx.send(log::error!("{err}")).unwrap();
                    break;
                }
            }
        });

        (tx, publish_tx)
    }

    pub fn process(&mut self, mut envel: PubSubEnvelope) -> Result<(), InternalError> {
        match envel.cmd {
            PubSubCommand::Subscribe(Subscribe { channels }) => {
                let sub = self.keep_alive(envel.stream, self.logger_tx.clone())?;
                Self::subscribe(
                    &self.subs_reg,
                    sub.id,
                    sub.reply_tx,
                    channels,
                    &self.logger_tx,
                )?;
            }
            PubSubCommand::Publish(Publish { channel, message }) => {
                let reply = self.publish(&channel, message, envel.cluster_tx)?;
                envel.stream.write_all(&reply)?;
            }
            PubSubCommand::Unsubscribe(Unsubscribe { channels }) => {
                Self::fake_unsubscribe(envel.stream, channels)?;
            }
            PubSubCommand::PubSubChannels(PubSubChannels { pattern }) => {
                let reply = self.channels(&pattern)?;
                envel.stream.write_all(&reply)?;
            }
            PubSubCommand::PubSubNumSub(PubSubNumSub { channels }) => {
                let reply = self.numsub(channels)?;
                envel.stream.write_all(&reply)?;
            }
        };

        Ok(())
    }

    fn keep_alive(
        &mut self,
        stream: StreamOwned<ServerConnection, TcpStream>,
        logger_tx: Sender<Log>,
    ) -> Result<Subscriber, InternalError> {
        let (reply_tx, reply_rx) = mpsc::channel();

        let sub = Subscriber {
            id: Uuid::new_v4(),
            reply_tx: reply_tx.clone(),
        };

        let subs_reg = Arc::clone(&self.subs_reg);

        let logger_tx_clone = logger_tx.clone();

        thread::spawn(move || {
            Self::start_pub_sub_state(
                subs_reg,
                sub.id,
                stream,
                reply_tx,
                reply_rx,
                logger_tx_clone,
            )
            .unwrap();
        });

        logger_tx.send(log::info!(
            "manteniendo viva conexión de cliente {}",
            sub.id
        ))?;

        Ok(sub)
    }

    fn start_pub_sub_state(
        subs_reg: Arc<Mutex<SubsRegister>>,
        sub_id: Uuid,
        mut stream: StreamOwned<ServerConnection, TcpStream>,
        sub_reply_tx: Sender<Vec<u8>>,
        sub_reply_rx: Receiver<Vec<u8>>,
        logger_tx: Sender<Log>,
    ) -> Result<(), InternalError> {
        stream.sock.set_nonblocking(true).unwrap();

        loop {
            if let Ok(message) = sub_reply_rx.try_recv() {
                if let Err(err) = stream.write_all(&message) {
                    logger_tx
                        .send(log::warn!(
                            "error mandando mensaje a cliente {sub_id}: {err}"
                        ))
                        .unwrap();
                }
            }

            let _ = stream.conn.complete_io(&mut stream.sock);

            match stream.fill_buf() {
                Ok(bytes) if !bytes.is_empty() => {
                    let cmd = Command::try_from(bytes);

                    let length = bytes.len();
                    stream.consume(length);

                    match cmd {
                        Ok(cmd) => {
                            Self::handle_incoming_command(
                                &subs_reg,
                                sub_id,
                                sub_reply_tx.clone(),
                                cmd,
                                &logger_tx,
                            )?;
                        }
                        Err(err) => {
                            logger_tx.send(log::info!(
                                "comando enviado por cliente {sub_id} es invalido",
                            ))?;

                            sub_reply_tx.send(SimpleError::from(err).into())?;
                            continue;
                        }
                    }
                }
                Ok(_) => break,
                Err(err) => match err.kind() {
                    io::ErrorKind::WouldBlock => continue,
                    io::ErrorKind::ConnectionReset => break,
                    _ => {
                        break logger_tx
                            .send(log::warn!(
                                "error escuchando stream del cliente {sub_id}: {err}",
                            ))
                            .unwrap();
                    }
                },
            }
        }

        logger_tx
            .send(log::warn!("stream del cliente desconectado {sub_id}"))
            .unwrap();

        Self::prune_sub(&subs_reg, sub_id, &logger_tx).unwrap();

        Ok(())
    }

    fn handle_incoming_command(
        subs_reg: &Arc<Mutex<SubsRegister>>,
        sub_id: Uuid,
        sub_reply_tx: Sender<Vec<u8>>,
        cmd: Command,
        logger_tx: &Sender<Log>,
    ) -> Result<(), InternalError> {
        match cmd {
            Command::PubSub(PubSubCommand::Subscribe(Subscribe { channels })) => {
                Self::subscribe(subs_reg, sub_id, sub_reply_tx, channels, logger_tx)
            }
            Command::PubSub(PubSubCommand::Unsubscribe(Unsubscribe { channels })) => {
                Self::unsubscribe(subs_reg, sub_id, sub_reply_tx, channels, logger_tx)
            }
            _ => {
                sub_reply_tx.send(SimpleError::from(OperationError::NotAPubSubCommand).into())?;

                Ok(())
            }
        }
    }

    // https://redis.io/docs/latest/commands/subscribe
    fn subscribe(
        subs_reg: &Arc<Mutex<SubsRegister>>,
        sub_id: Uuid,
        sub_reply_tx: Sender<Vec<u8>>,
        channels: Vec<BulkString>,
        logger_tx: &Sender<Log>,
    ) -> Result<(), InternalError> {
        let mut state = subs_reg.lock()?;

        for chan_name in channels {
            let chan_subs = state.entry(chan_name.clone()).or_default();
            chan_subs.insert(sub_id, sub_reply_tx.clone());

            logger_tx.send(log::info!(
                "cliente {sub_id} suscrito al channel {chan_name}"
            ))?;

            logger_tx.send(log::debug!(
                "channel {chan_name} tiene {} suscriptores",
                chan_subs.len()
            ))?;

            let n_client_subs = Self::count_client_subs(sub_id, &state);

            let reply = Array::from(vec![
                BulkString::from("subscribe").into(),
                chan_name.into(),
                Integer::from(n_client_subs as i64).into(),
            ]);

            sub_reply_tx.send(reply.into())?;
        }

        Ok(())
    }

    // https://redis.io/docs/latest/commands/publish
    fn publish(
        &self,
        chan_name: &BulkString,
        msg: BulkString,
        cluster_tx: Sender<ClusterAction>,
    ) -> Result<Vec<u8>, InternalError> {
        let subs_reg = self.subs_reg.lock()?;
        let mut n_chan_subs = 0;

        if let Some(chan_subs) = subs_reg.get(chan_name) {
            n_chan_subs = chan_subs.len();

            for client_tx in chan_subs.values() {
                let reply = Array::from(vec![
                    BulkString::from("message").into(),
                    chan_name.clone().into(),
                    msg.clone().into(),
                ]);

                client_tx.send(reply.into())?;
            }
        }

        cluster_tx
            .send(ClusterAction::BroadcastPublish {
                channel: chan_name.clone(),
                message: msg.clone(),
            })
            .unwrap();

        self.logger_tx.send(log::info!(
            "publicados {} bytes al channel {chan_name}",
            msg.len()
        ))?;

        Ok(Integer::from(n_chan_subs as i64).into())
    }

    // https://redis.io/docs/latest/commands/publish
    fn publish_from_cluster(
        chan_name: &BulkString,
        msg: BulkString,
        reg: Arc<Mutex<SubsRegister>>,
        logger_tx: Sender<Log>,
    ) -> Result<Vec<u8>, InternalError> {
        let subs_reg = reg.lock()?;
        let mut n_chan_subs = 0;

        if let Some(chan_subs) = subs_reg.get(chan_name) {
            n_chan_subs = chan_subs.len();

            for client_tx in chan_subs.values() {
                let reply = Array::from(vec![
                    BulkString::from("message").into(),
                    chan_name.clone().into(),
                    msg.clone().into(),
                ]);

                client_tx.send(reply.into())?;
            }
        }

        logger_tx.send(log::info!(
            "Republicados {} bytes al channel {chan_name} que vienen de otro nodo",
            msg.len()
        ))?;

        Ok(Integer::from(n_chan_subs as i64).into())
    }

    // https://redis.io/docs/latest/commands/unsubscribe
    fn unsubscribe(
        subs_reg: &Arc<Mutex<SubsRegister>>,
        sub_id: Uuid,
        sub_reply_tx: Sender<Vec<u8>>,
        mut channels: Vec<BulkString>,
        logger_tx: &Sender<Log>,
    ) -> Result<(), InternalError> {
        let mut subs_reg = subs_reg.lock()?;

        if channels.is_empty() {
            channels = subs_reg
                .iter()
                .filter_map(|(chan_name, chan_subs)| {
                    chan_subs.contains_key(&sub_id).then_some(chan_name)
                })
                .cloned()
                .collect()
        };

        if channels.is_empty() {
            let reply = Array::from(vec![
                BulkString::from("unsubscribe").into(),
                RespDataType::Null,
                Integer::from(0).into(),
            ]);

            return Ok(sub_reply_tx.send(reply.into())?);
        }

        for chan_name in channels {
            if let Some(chan_subs) = subs_reg.get_mut(&chan_name) {
                if chan_subs.remove(&sub_id).is_some() {
                    logger_tx.send(log::info!(
                        "desuscrito cliente {sub_id} del channel {chan_name}",
                    ))?;

                    logger_tx.send(log::debug!(
                        "channel {chan_name} tiene {} suscriptores",
                        chan_subs.len()
                    ))?;
                }

                if chan_subs.is_empty() {
                    subs_reg.remove(&chan_name);
                }
            }

            let n_client_subs = Self::count_client_subs(sub_id, &subs_reg);

            let reply = Array::from(vec![
                BulkString::from("unsubscribe").into(),
                chan_name.clone().into(),
                Integer::from(n_client_subs as i64).into(),
            ]);

            sub_reply_tx.send(reply.into())?;
        }

        Ok(())
    }

    // https://redis.io/docs/latest/commands/pubsub-channels
    fn channels(&self, pattern: &Option<BulkString>) -> Result<Vec<u8>, InternalError> {
        let subs_reg = self.subs_reg.lock()?;

        let filtered_chans: Vec<_> = subs_reg
            .iter()
            .filter(|(_, chan_subs)| !chan_subs.is_empty())
            .filter_map(|(chan_name, _)| match pattern {
                Some(p) if chan_name.contains(p) => Some(chan_name),
                None => Some(chan_name),
                _ => None,
            })
            .cloned()
            .map(RespDataType::BulkString)
            .collect();

        Ok(Array::from(filtered_chans).into())
    }

    // https://redis.io/docs/latest/commands/pubsub-numsub
    fn numsub(&self, channels: Vec<BulkString>) -> Result<Vec<u8>, InternalError> {
        let subs_reg = self.subs_reg.lock()?;

        let mut chans_subs = Vec::new();

        for chan_name in channels {
            let n_chan_subs = match subs_reg.get(&chan_name) {
                Some(chan_subs) => chan_subs.len() as i64,
                _ => 0,
            };

            chans_subs.push(RespDataType::BulkString(chan_name));
            chans_subs.push(RespDataType::Integer(n_chan_subs.into()));
        }

        Ok(Array::from(chans_subs).into())
    }

    fn fake_unsubscribe(
        mut stream: StreamOwned<ServerConnection, TcpStream>,
        channels: Vec<BulkString>,
    ) -> Result<(), InternalError> {
        if channels.is_empty() {
            let reply = Array::from(vec![
                BulkString::from("unsubscribe").into(),
                RespDataType::Null,
                Integer::from(0).into(),
            ]);

            return Ok(stream.write_all(&Vec::from(reply))?);
        }

        for chan_name in channels {
            let reply = Array::from(vec![
                BulkString::from("unsubscribe").into(),
                chan_name.into(),
                Integer::from(0).into(),
            ]);

            stream.write_all(&Vec::from(reply))?;
        }

        Ok(())
    }

    fn count_client_subs(sub_id: Uuid, subs_reg: &SubsRegister) -> usize {
        subs_reg
            .values()
            .filter(|chan_subs| chan_subs.contains_key(&sub_id))
            .count()
    }

    fn prune_sub(
        subs_reg: &Arc<Mutex<SubsRegister>>,
        sub_id: Uuid,
        logger_tx: &Sender<Log>,
    ) -> Result<(), InternalError> {
        let mut state = subs_reg.lock()?;

        for (_, chan_subs) in state.iter_mut() {
            chan_subs.remove(&sub_id);
        }

        logger_tx.send(log::info!(
            "eliminadas todas las subscripciones de cliente {sub_id}"
        ))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn se_eliminan_channels_de_cliente_desconectado() {
        let sub_id = Uuid::new_v4();
        let sub_reply_tx = mpsc::channel().0;

        let state: SubsRegister = HashMap::from([
            (
                BulkString::from("chan_1"),
                HashMap::from([
                    (Uuid::new_v4(), mpsc::channel().0),
                    (sub_id, sub_reply_tx.clone()),
                ]),
            ),
            (
                BulkString::from("chan_2"),
                HashMap::from([(Uuid::new_v4(), mpsc::channel().0)]),
            ),
            (
                BulkString::from("chan_3"),
                HashMap::from([
                    (Uuid::new_v4(), mpsc::channel().0),
                    (sub_id, sub_reply_tx.clone()),
                ]),
            ),
        ]);

        let before = Arc::new(Mutex::new(state));

        PubSubBroker::prune_sub(&before, sub_id, &mpsc::channel().0).unwrap();

        let after = before.lock().unwrap();

        let chan_1_subs = after.get(&BulkString::from("chan_1")).unwrap();
        let chan_2_subs = after.get(&BulkString::from("chan_2")).unwrap();
        let chan_3_subs = after.get(&BulkString::from("chan_3")).unwrap();

        assert_eq!(chan_1_subs.len(), 1);
        assert!(!chan_1_subs.contains_key(&sub_id));

        assert_eq!(chan_2_subs.len(), 1);

        assert_eq!(chan_3_subs.len(), 1);
        assert!(!chan_1_subs.contains_key(&sub_id));
    }
}
