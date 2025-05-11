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

use commands::{Command, pub_sub::*};
use error::Error;
use log::LogMsg;
use resp::{Array, BulkString, Integer, RespDataType, SimpleError};
use uuid::Uuid;

type State = HashMap<BulkString, HashMap<Uuid, Sender<Vec<u8>>>>;

#[derive(Debug)]
pub struct PubSubBroker {
    state: Arc<Mutex<State>>,
    logger_tx: Sender<LogMsg>,
}

#[derive(Debug)]
pub struct PubSubEnvelope {
    pub client_conn: TcpStream,
    pub cmd: PubSubCommand,
}

#[derive(Debug)]
enum PubSubActionKind {
    Subscribe,
    Unsubscribe,
}

#[derive(Debug)]
struct Client {
    id: Uuid,
    conn: TcpStream,
    tx: Sender<Vec<u8>>,
}

impl PubSubBroker {
    pub fn new(logger_tx: Sender<LogMsg>) -> Self {
        Self {
            state: Arc::new(Mutex::new(HashMap::new())),
            logger_tx,
        }
    }

    pub fn process(&mut self, mut envel: PubSubEnvelope) -> Result<(), Error> {
        match envel.cmd {
            PubSubCommand::Subscribe(Subscribe { channels }) => {
                let client = self.keep_alive(envel.client_conn, self.logger_tx.clone())?;
                Self::subscribe(&client, &self.state, channels, &self.logger_tx)?;
            }
            PubSubCommand::Publish(Publish { channel, message }) => {
                let reply = self.publish(&channel, message)?;
                envel.client_conn.write_all(&reply)?;
            }
            PubSubCommand::Unsubscribe(Unsubscribe { channels }) => {
                Self::fake_unsubscribe(envel.client_conn, channels)?;
            }
            PubSubCommand::PubSubChannels(PubSubChannels { pattern }) => todo!(),
            PubSubCommand::PubSubNumSub(PubSubNumSub { channels }) => todo!(),
        };

        Ok(())
    }

    fn keep_alive(
        &mut self,
        client_conn: TcpStream,
        logger_tx: Sender<LogMsg>,
    ) -> Result<Client, Error> {
        let (client_tx, client_rx) = mpsc::channel();

        let client = Client {
            id: Uuid::new_v4(),
            conn: client_conn,
            tx: client_tx,
        };

        let mut client_publish_stream = client.conn.try_clone()?;
        let client_commands_stream = client.conn.try_clone()?;

        let state = Arc::clone(&self.state);
        let logger_tx_clone = logger_tx.clone();

        thread::spawn(move || {
            for message in client_rx {
                if let Err(err) = client_publish_stream.write_all(&message) {
                    logger_tx_clone
                        .send(log::warn!(
                            "error mandando mensaje a cliente {}: {err}",
                            client.id
                        ))
                        .unwrap();
                    Self::prune_client(client.id, &state, &logger_tx_clone).unwrap();
                    break;
                }
            }
        });

        let state = Arc::clone(&self.state);
        let client_clone = Client {
            id: client.id,
            conn: client_commands_stream,
            tx: client.tx.clone(),
        };
        let logger_tx_clone = logger_tx.clone();

        thread::spawn(move || {
            if let Err(err) =
                Self::listen_incoming_commands(&client_clone, &state, &logger_tx_clone)
            {
                logger_tx_clone
                    .send(log::warn!(
                        "error escuchando stream del cliente {}: {err}",
                        client.id
                    ))
                    .unwrap();
                Self::prune_client(client.id, &state, &logger_tx_clone).unwrap();
            }
        });

        logger_tx.send(log::info!(
            "manteniendo viva conexión de cliente {}",
            client.id
        ))?;

        Ok(client)
    }

    fn listen_incoming_commands(
        client: &Client,
        state: &Arc<Mutex<State>>,
        logger_tx: &Sender<LogMsg>,
    ) -> Result<(), Error> {
        let commands_stream = client.conn.try_clone()?;
        let mut reader = BufReader::new(commands_stream);

        loop {
            match reader.fill_buf() {
                Ok(bytes) if !bytes.is_empty() => {
                    let cmd = Command::try_from(bytes);

                    let length = bytes.len();
                    reader.consume(length);

                    match cmd {
                        Ok(cmd) => Self::handle_incoming_command(client, state, cmd, logger_tx)?,
                        Err(err) => {
                            logger_tx.send(log::info!(
                                "comando enviado por el cliente {} es invalido",
                                client.id,
                            ))?;

                            client.tx.send(SimpleError::from(err).into())?;
                            continue;
                        }
                    }
                }
                Ok(_) => {
                    return Err(Error::ClientDisconnect);
                }
                Err(err) => {
                    logger_tx.send(log::error!(
                        "error al leer bytes mandados por el cliente {}: {err}",
                        client.id
                    ))?;

                    return Err(err.into());
                }
            };
        }
    }

    fn handle_incoming_command(
        client: &Client,
        state: &Arc<Mutex<State>>,
        cmd: Command,
        logger_tx: &Sender<LogMsg>,
    ) -> Result<(), Error> {
        match cmd {
            Command::PubSub(PubSubCommand::Subscribe(Subscribe { channels })) => {
                Self::subscribe(client, state, channels, logger_tx)
            }
            Command::PubSub(PubSubCommand::Unsubscribe(Unsubscribe { channels })) => {
                Self::unsubscribe(client, state, channels, logger_tx)
            }
            _ => {
                client.tx.send(SimpleError::from("ERR solo se permiten los comandos `SUBSCRIBE` / `UNSUBSCRIBE` / `QUIT` en subscriber state").into()).unwrap();
                Ok(())
            }
        }
    }

    // https://redis.io/docs/latest/commands/subscribe
    fn subscribe(
        client: &Client,
        state: &Arc<Mutex<State>>,
        channels: Vec<BulkString>,
        logger_tx: &Sender<LogMsg>,
    ) -> Result<(), Error> {
        let mut state = state.lock()?;

        for chan_name in channels {
            let chan_subs = state.entry(chan_name.clone()).or_default();
            chan_subs.insert(client.id, client.tx.clone());

            logger_tx.send(log::info!(
                "cliente {} suscrito al channel {chan_name}",
                client.id
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
    fn publish(&self, chan_name: &BulkString, msg: BulkString) -> Result<Vec<u8>, Error> {
        let state = self.state.lock()?;
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

        self.logger_tx.send(log::info!(
            "publicados {} bytes al channel {chan_name}",
            msg.len()
        ))?;

        Ok(Integer::from(n_chan_subs as i64).into())
    }

    // https://redis.io/docs/latest/commands/unsubscribe
    fn unsubscribe(
        client: &Client,
        state: &Arc<Mutex<State>>,
        mut channels: Vec<BulkString>,
        logger_tx: &Sender<LogMsg>,
    ) -> Result<(), Error> {
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

    fn fake_unsubscribe(
        mut client_conn: TcpStream,
        channels: Vec<BulkString>,
    ) -> Result<(), Error> {
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

    fn count_client_subs(client_id: Uuid, state: &State) -> usize {
        state
            .values()
            .filter(|chan_subs| chan_subs.contains_key(&client_id))
            .count()
    }

    fn prune_client(
        client_id: Uuid,
        state: &Arc<Mutex<State>>,
        logger_tx: &Sender<LogMsg>,
    ) -> Result<(), Error> {
        let mut state = state.lock()?;

        for (chan_name, chan_subs) in state.iter_mut() {
            chan_subs.remove(&client_id);
        }

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

        let state: State = HashMap::from([
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

        PubSubBroker::prune_client(client_id, &before, &mpsc::channel().0).unwrap();

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
