mod error;

use std::{
    collections::HashMap,
    io::{BufReader, prelude::*},
    net::TcpStream,
    sync::{
        Arc, Mutex,
        mpsc::{self, Sender},
    },
    thread,
};

pub use error::InternalError;
use error::OperationError;
use log::Log;
use redis_cmd::{Command, pub_sub::*};
use redis_resp::{Array, BulkString, Integer, RespDataType, SimpleError};
use uuid::Uuid;

use crate::server::node::cluster::{ClusterAction, message::payload::PublishPayload};

type SubsRegister = HashMap<BulkString, HashMap<Uuid, Sender<Vec<u8>>>>;

#[derive(Debug)]
pub struct PubSubBroker {
    reg: Arc<Mutex<SubsRegister>>,
    logger_tx: Sender<Log>,
}

#[derive(Debug)]
pub struct PubSubEnvelope {
    pub client: TcpStream,
    pub cmd: PubSubCommand,
    pub cluster_tx: Sender<ClusterAction>,
}

#[derive(Debug)]
struct Subscriber {
    id: Uuid,
    conn: TcpStream,
    tx: Sender<Vec<u8>>,
}

impl PubSubBroker {
    pub fn start(logger_tx: Sender<Log>) -> (Sender<PubSubEnvelope>, Sender<PublishPayload>) {
        let (tx, rx) = mpsc::channel();

        let mut broker = Self {
            reg: Arc::new(Mutex::new(HashMap::new())),
            logger_tx: logger_tx.clone(),
        };

        let (publish_tx, publish_rx) = mpsc::channel::<PublishPayload>();

        let reg = broker.reg.clone();
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
                let sub = self.keep_alive(envel.client, self.logger_tx.clone())?;
                Self::subscribe(&sub, &self.reg, channels, &self.logger_tx)?;
            }
            PubSubCommand::Publish(Publish { channel, message }) => {
                let reply = self.publish(&channel, message, envel.cluster_tx)?;
                envel.client.write_all(&reply)?;
            }
            PubSubCommand::Unsubscribe(Unsubscribe { channels }) => {
                Self::fake_unsubscribe(envel.client, channels)?;
            }
            PubSubCommand::PubSubChannels(PubSubChannels { pattern }) => {
                let reply = self.channels(&pattern)?;
                envel.client.write_all(&reply)?;
            }
            PubSubCommand::PubSubNumSub(PubSubNumSub { channels }) => {
                let reply = self.numsub(channels)?;
                envel.client.write_all(&reply)?;
            }
        };

        Ok(())
    }

    fn keep_alive(
        &mut self,
        client: TcpStream,
        logger_tx: Sender<Log>,
    ) -> Result<Subscriber, InternalError> {
        let (sub_tx, sub_rx) = mpsc::channel();

        let sub = Subscriber {
            id: Uuid::new_v4(),
            conn: client,
            tx: sub_tx,
        };

        let mut sub_publish_stream = sub.conn.try_clone()?;
        let sub_commands_stream = sub.conn.try_clone()?;

        let reg = Arc::clone(&self.reg);
        let logger_tx_clone = logger_tx.clone();

        thread::spawn(move || {
            for message in sub_rx {
                if let Err(err) = sub_publish_stream.write_all(&message) {
                    //aqui habria q envia un paquete publish a cada uno de los otros noodos
                    //con el mismo mensjae
                    logger_tx_clone
                        .send(log::warn!(
                            "error mandando mensaje a cliente {}: {err}",
                            sub.id
                        ))
                        .unwrap();
                    break;
                }
            }

            logger_tx_clone
                .send(log::info!("cerrando la conexión con cliente {}", sub.id))
                .unwrap();

            Self::prune_sub(sub.id, &reg, &logger_tx_clone).unwrap();
        });

        let reg = Arc::clone(&self.reg);
        let sub_clone = Subscriber {
            id: sub.id,
            conn: sub_commands_stream,
            tx: sub.tx.clone(),
        };
        let logger_tx_clone = logger_tx.clone();

        thread::spawn(move || {
            if let Err(err) = Self::listen_incoming_commands(&sub_clone, &reg, &logger_tx_clone) {
                logger_tx_clone
                    .send(log::warn!(
                        "error escuchando stream del cliente {}: {err}",
                        sub.id
                    ))
                    .unwrap();
            }

            logger_tx_clone
                .send(log::info!("cerrando la conexión con cliente {}", sub.id))
                .unwrap();

            Self::prune_sub(sub.id, &reg, &logger_tx_clone).unwrap();
        });

        logger_tx.send(log::info!(
            "manteniendo viva conexión de cliente {}",
            sub.id
        ))?;

        Ok(sub)
    }

    fn listen_incoming_commands(
        sub: &Subscriber,
        reg: &Arc<Mutex<SubsRegister>>,
        logger_tx: &Sender<Log>,
    ) -> Result<(), InternalError> {
        let commands_stream = sub.conn.try_clone()?;
        let mut reader = BufReader::new(commands_stream);

        loop {
            match reader.fill_buf() {
                Ok(bytes) if !bytes.is_empty() => {
                    let cmd = Command::try_from(bytes);

                    let length = bytes.len();
                    reader.consume(length);

                    match cmd {
                        Ok(cmd) => Self::handle_incoming_command(sub, reg, cmd, logger_tx)?,
                        Err(err) => {
                            logger_tx.send(log::info!(
                                "comando enviado por cliente {} es invalido",
                                sub.id,
                            ))?;

                            sub.tx.send(SimpleError::from(err).into())?;
                            continue;
                        }
                    }
                }
                Ok(_) => return Ok(()),
                Err(err) => return Err(err.into()),
            };
        }
    }

    fn handle_incoming_command(
        sub: &Subscriber,
        reg: &Arc<Mutex<SubsRegister>>,
        cmd: Command,
        logger_tx: &Sender<Log>,
    ) -> Result<(), InternalError> {
        match cmd {
            Command::PubSub(PubSubCommand::Subscribe(Subscribe { channels })) => {
                Self::subscribe(sub, reg, channels, logger_tx)
            }
            Command::PubSub(PubSubCommand::Unsubscribe(Unsubscribe { channels })) => {
                Self::unsubscribe(sub, reg, channels, logger_tx)
            }
            _ => {
                sub.tx
                    .send(SimpleError::from(OperationError::NotAPubSubCommand).into())?;

                Ok(())
            }
        }
    }

    // https://redis.io/docs/latest/commands/subscribe
    fn subscribe(
        client: &Subscriber,
        state: &Arc<Mutex<SubsRegister>>,
        channels: Vec<BulkString>,
        logger_tx: &Sender<Log>,
    ) -> Result<(), InternalError> {
        let mut state = state.lock()?;

        for chan_name in channels {
            let chan_subs = state.entry(chan_name.clone()).or_default();
            chan_subs.insert(client.id, client.tx.clone());

            logger_tx.send(log::info!(
                "cliente {} suscrito al channel {chan_name}",
                client.id
            ))?;

            logger_tx.send(log::debug!(
                "channel {chan_name} tiene {} suscriptores",
                chan_subs.len()
            ))?;

            let n_client_subs = Self::count_client_subs(client.id, &state);

            let reply = Array::from(vec![
                BulkString::from("subscribe").into(),
                chan_name.into(),
                Integer::from(n_client_subs as i64).into(),
            ]);

            client.tx.send(reply.into())?;
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
        let state = self.reg.lock()?;
        let mut n_chan_subs = 0;

        if let Some(chan_subs) = state.get(chan_name) {
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
        let state = reg.lock()?;
        let mut n_chan_subs = 0;

        if let Some(chan_subs) = state.get(chan_name) {
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
        client: &Subscriber,
        state: &Arc<Mutex<SubsRegister>>,
        mut channels: Vec<BulkString>,
        logger_tx: &Sender<Log>,
    ) -> Result<(), InternalError> {
        let mut state = state.lock()?;

        if channels.is_empty() {
            channels = state
                .iter()
                .filter_map(|(chan_name, chan_subs)| {
                    chan_subs.contains_key(&client.id).then_some(chan_name)
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

            return Ok(client.tx.send(reply.into())?);
        }

        for chan_name in channels {
            if let Some(chan_subs) = state.get_mut(&chan_name) {
                if chan_subs.remove(&client.id).is_some() {
                    logger_tx.send(log::info!(
                        "desuscrito cliente {} del channel {chan_name}",
                        client.id
                    ))?;

                    logger_tx.send(log::debug!(
                        "channel {chan_name} tiene {} suscriptores",
                        chan_subs.len()
                    ))?;
                }

                if chan_subs.is_empty() {
                    state.remove(&chan_name);
                }
            }

            let n_client_subs = Self::count_client_subs(client.id, &state);

            let reply = Array::from(vec![
                BulkString::from("unsubscribe").into(),
                chan_name.clone().into(),
                Integer::from(n_client_subs as i64).into(),
            ]);

            client.tx.send(reply.into())?;
        }

        Ok(())
    }

    // https://redis.io/docs/latest/commands/pubsub-channels
    fn channels(&self, pattern: &Option<BulkString>) -> Result<Vec<u8>, InternalError> {
        let state = self.reg.lock()?;

        let filtered_chans: Vec<_> = state
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
        let state = self.reg.lock()?;

        let mut chans_subs = Vec::new();

        for chan_name in channels {
            let n_chan_subs = match state.get(&chan_name) {
                Some(chan_subs) => chan_subs.len() as i64,
                _ => 0,
            };

            chans_subs.push(RespDataType::BulkString(chan_name));
            chans_subs.push(RespDataType::Integer(n_chan_subs.into()));
        }

        Ok(Array::from(chans_subs).into())
    }

    fn fake_unsubscribe(
        mut client_conn: TcpStream,
        channels: Vec<BulkString>,
    ) -> Result<(), InternalError> {
        if channels.is_empty() {
            let reply = Array::from(vec![
                BulkString::from("unsubscribe").into(),
                RespDataType::Null,
                Integer::from(0).into(),
            ]);

            return Ok(client_conn.write_all(&Vec::from(reply))?);
        }

        for chan_name in channels {
            let reply = Array::from(vec![
                BulkString::from("unsubscribe").into(),
                chan_name.into(),
                Integer::from(0).into(),
            ]);

            client_conn.write_all(&Vec::from(reply))?;
        }

        Ok(())
    }

    fn count_client_subs(client_id: Uuid, state: &SubsRegister) -> usize {
        state
            .values()
            .filter(|chan_subs| chan_subs.contains_key(&client_id))
            .count()
    }

    fn prune_sub(
        client_id: Uuid,
        state: &Arc<Mutex<SubsRegister>>,
        logger_tx: &Sender<Log>,
    ) -> Result<(), InternalError> {
        let mut state = state.lock()?;

        for (_, chan_subs) in state.iter_mut() {
            chan_subs.remove(&client_id);
        }

        logger_tx.send(log::info!(
            "eliminadas todas las subscripciones de cliente {client_id}"
        ))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn se_eliminan_channels_de_cliente_desconectado() {
        let client_id = Uuid::new_v4();
        let client_tx = mpsc::channel().0;

        let state: SubsRegister = HashMap::from([
            (
                BulkString::from("chan_1"),
                HashMap::from([
                    (Uuid::new_v4(), mpsc::channel().0),
                    (client_id, client_tx.clone()),
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
                    (client_id, client_tx.clone()),
                ]),
            ),
        ]);

        let before = Arc::new(Mutex::new(state));

        PubSubBroker::prune_sub(client_id, &before, &mpsc::channel().0).unwrap();

        let after = before.lock().unwrap();

        let chan_1_subs = after.get(&BulkString::from("chan_1")).unwrap();
        let chan_2_subs = after.get(&BulkString::from("chan_2")).unwrap();
        let chan_3_subs = after.get(&BulkString::from("chan_3")).unwrap();

        assert_eq!(chan_1_subs.len(), 1);
        assert!(!chan_1_subs.contains_key(&client_id));

        assert_eq!(chan_2_subs.len(), 1);

        assert_eq!(chan_3_subs.len(), 1);
        assert!(!chan_1_subs.contains_key(&client_id));
    }
}
