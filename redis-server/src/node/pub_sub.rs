mod error;

use std::{
    collections::HashMap,
    io::{BufReader, prelude::*},
    net::TcpStream,
    sync::{
        Arc, Mutex, MutexGuard,
        mpsc::{self, Sender},
    },
    thread,
};

use error::Error;
use log::LogMsg;
use redis_cmd::{Command, pub_sub::*};
use redis_resp::{Array, BulkString, Integer, SimpleError};
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
                let (mut client_conn, client_id, client_tx) =
                    self.keep_alive(envel.client_conn, self.logger_tx.clone())?;

                Self::subscribe(
                    &mut client_conn,
                    client_id,
                    client_tx,
                    &self.state,
                    channels,
                    &self.logger_tx,
                )?;
            }
            PubSubCommand::Publish(Publish { channel, message }) => {
                let reply = self.publish(&channel, message)?;
                envel.client_conn.write_all(&reply)?;
            }
            PubSubCommand::Unsubscribe(Unsubscribe { channels }) => {
                Self::unsubscribe(
                    &mut envel.client_conn,
                    None,
                    None,
                    &self.state,
                    channels,
                    &self.logger_tx,
                )?;
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
    ) -> Result<(TcpStream, Uuid, Sender<Vec<u8>>), Error> {
        let client_id = Uuid::new_v4();
        let (client_tx, client_rx) = mpsc::channel::<Vec<u8>>();

        let mut publisher_stream = client_conn.try_clone()?;
        let listener_stream = client_conn.try_clone()?;

        let state = Arc::clone(&self.state);
        let client_tx_publisher = client_tx.clone();
        let logger_tx_publisher = logger_tx.clone();

        thread::spawn(move || {
            for message in client_rx {
                if let Err(err) = publisher_stream.write_all(&message) {
                    logger_tx_publisher
                        .send(log::error!(
                            "error mandando mensaje a cliente {client_id}: {err}"
                        ))
                        .unwrap();

                    Self::unsubscribe(
                        &mut publisher_stream,
                        Some(client_id),
                        Some(client_tx_publisher),
                        &state,
                        Vec::new(),
                        &logger_tx_publisher,
                    )
                    .unwrap();

                    break;
                }
            }
        });

        let state = Arc::clone(&self.state);
        let client_tx_listener = client_tx.clone();
        let logger_tx_listener = logger_tx.clone();

        thread::spawn(move || {
            if let Err(err) = Self::listen_client_conn(
                listener_stream,
                client_id,
                client_tx_listener,
                &state,
                &logger_tx_listener,
            ) {
                logger_tx_listener
                    .send(log::error!(
                        "error escuchando stream del cliente {client_id}: {err}"
                    ))
                    .unwrap();
            }
        });

        logger_tx.send(log::info!(
            "manteniendo viva conexión de cliente {}",
            client_id
        ))?;

        Ok((client_conn, client_id, client_tx))
    }

    fn listen_client_conn(
        client_conn: TcpStream,
        client_id: Uuid,
        client_tx: Sender<Vec<u8>>,

        state: &Arc<Mutex<State>>,
        logger_tx: &Sender<LogMsg>,
    ) -> Result<(), Error> {
        let read_stream = client_conn.try_clone()?;
        let mut reply_stream = client_conn;

        let mut reader = BufReader::new(read_stream);

        loop {
            match reader.fill_buf() {
                Ok(bytes) if !bytes.is_empty() => {
                    let cmd = match Command::try_from(bytes) {
                        Ok(cmd) => cmd,
                        Err(err) => {
                            logger_tx.send(log::info!(
                                "comando enviado por el cliente {} es invalido",
                                client_id,
                            ))?;

                            client_tx.send(SimpleError::from(err).into())?;
                            continue;
                        }
                    };

                    match cmd {
                        Command::PubSub(PubSubCommand::Subscribe(Subscribe { channels })) => {
                            Self::subscribe(
                                &mut reply_stream,
                                client_id,
                                client_tx.clone(),
                                state,
                                channels,
                                logger_tx,
                            )?;
                        }
                        Command::PubSub(PubSubCommand::Unsubscribe(Unsubscribe { channels })) => {
                            Self::unsubscribe(
                                &mut reply_stream,
                                Some(client_id),
                                Some(client_tx.clone()),
                                state,
                                channels,
                                logger_tx,
                            )?;
                        }
                        _ => {
                            client_tx.send(SimpleError::from("ERR solo se permiten los comandos `SUBSCRIBE` / `UNSUBSCRIBE` / `QUIT` en subscriber state").into())?;
                        }
                    };

                    let length = bytes.len();
                    reader.consume(length);
                }
                Ok(_) => {
                    logger_tx.send(log::info!("cliente {client_id} desconectado"))?;
                    return Err(Error::ClientDisconnect);
                }
                Err(err) => {
                    logger_tx.send(log::error!(
                        "error al leer bytes mandados por el cliente {}: {err}",
                        client_id
                    ))?;

                    return Err(err.into());
                }
            };
        }
    }

    /// https://redis.io/docs/latest/commands/subscribe
    fn subscribe(
        client_conn: &mut TcpStream,
        client_id: Uuid,
        client_tx: Sender<Vec<u8>>,
        state: &Arc<Mutex<State>>,
        channels: Vec<BulkString>,
        logger_tx: &Sender<LogMsg>,
    ) -> Result<(), Error> {
        let mut state = state.lock()?;

        for chan_name in channels {
            logger_tx.send(log::info!(
                "cliente {client_id} suscrito al canal {chan_name}",
            ))?;

            let chan_subs = state.entry(chan_name.clone()).or_default();
            chan_subs.insert(client_id, client_tx.clone());

            Self::action_reply(
                Some(client_id),
                client_conn,
                &state,
                chan_name,
                PubSubActionKind::Subscribe,
            )?;
        }

        Ok(())
    }

    /// https://redis.io/docs/latest/commands/publish
    fn publish(&self, chan_name: &BulkString, msg: BulkString) -> Result<Vec<u8>, Error> {
        let state = self.state.lock()?;

        let mut n_chan_subs = 0;

        if let Some(chan_subs) = state.get(chan_name) {
            n_chan_subs = chan_subs.len();

            for client_tx in chan_subs.values() {
                self.logger_tx.send(log::info!(
                    "publicados {} bytes al canal {chan_name}",
                    msg.len()
                ))?;

                let reply = Array::from(vec![
                    BulkString::from("message").into(),
                    chan_name.clone().into(),
                    msg.clone().into(),
                ]);

                client_tx.send(reply.into())?;
            }
        }

        Ok(Integer::from(n_chan_subs as i64).into())
    }

    /// https://redis.io/docs/latest/commands/unsubscribe
    fn unsubscribe(
        client_conn: &mut TcpStream,
        client_id: Option<Uuid>,
        client_tx: Option<Sender<Vec<u8>>>,
        state: &Arc<Mutex<State>>,
        channels: Vec<BulkString>,
        logger_tx: &Sender<LogMsg>,
    ) -> Result<(), Error> {
        let mut state = state.lock()?;

        for chan_name in channels {
            if let Some(chan_subs) = state.get_mut(&chan_name) {
                if let Some(client_id) = client_id {
                    if chan_subs.remove(&client_id).is_some() {
                        logger_tx.send(log::info!(
                            "cliente {client_id} desuscrito del canal {chan_name}",
                        ))?;
                    }
                }
            }

            Self::action_reply(
                client_id,
                client_conn,
                &state,
                chan_name,
                PubSubActionKind::Unsubscribe,
            )?;
        }

        Ok(())
    }

    /// Construye y manda un mensaje que notifica al cliente ante una suscripción o desuscripción de un canal.
    /// https://redis.io/docs/latest/develop/interact/pubsub#wire-protocol-example
    fn action_reply(
        client_id: Option<Uuid>,
        client_conn: &mut TcpStream,
        state: &MutexGuard<'_, State>,
        chan_name: BulkString,
        action_kind: PubSubActionKind,
    ) -> Result<(), Error> {
        let n_client_subs = if let Some(client_id) = client_id {
            state
                .values()
                .filter(|chan_subs| chan_subs.contains_key(&client_id))
                .count()
        } else {
            0
        };

        let reply_kind = match action_kind {
            PubSubActionKind::Subscribe => "subscribe",
            PubSubActionKind::Unsubscribe => "unsubscribe",
        };

        let msg = Array::from(vec![
            BulkString::from(reply_kind).into(),
            chan_name.into(),
            Integer::from(n_client_subs as i64).into(),
        ]);

        Ok(client_conn.write_all(&Vec::from(msg))?)
    }
}
