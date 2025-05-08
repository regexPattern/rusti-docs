use std::{
    collections::HashMap,
    io::{BufReader, prelude::*},
    net::TcpStream,
    sync::mpsc::{self, Sender},
    sync::{Arc, Mutex},
    thread,
};

use commands::{Command, pub_sub::*};
use log::LogMsg;
use resp::{BulkString, RespDataType, SimpleString};
use uuid::Uuid;

type State = HashMap<BulkString, HashMap<Uuid, Sender<BulkString>>>;

#[derive(Debug)]
pub struct Broker {
    state: Arc<Mutex<State>>,
    logger_tx: Sender<LogMsg>,
}

#[derive(Debug)]
pub struct Envelope {
    pub client_conn: TcpStream,
    pub cmd: PubSubCommand,
}

impl Broker {
    pub fn new(logger_tx: Sender<LogMsg>) -> Self {
        Self {
            state: Arc::new(Mutex::new(HashMap::new())),
            logger_tx,
        }
    }

    pub fn process(&mut self, mut envel: Envelope) {
        match envel.cmd {
            PubSubCommand::Subscribe(Subscribe { channels }) => {
                self.subscribe(envel.client_conn, channels);
            }
            PubSubCommand::Publish(Publish { channel, message }) => {
                let reply = self.publish(channel, message).unwrap();
                let bytes = Vec::from(reply);
                envel.client_conn.write_all(bytes.as_slice()).unwrap();
            }
            PubSubCommand::Unsubscribe(Unsubscribe { channels }) => todo!(),
            PubSubCommand::PubSubChannels(PubSubChannels { pattern }) => todo!(),
            PubSubCommand::PubSubNumSub(PubSubNumSub { channels }) => todo!(),
        }
    }

    // https://redis.io/docs/latest/commands/subscribe#resp2resp3-reply
    fn subscribe(&mut self, client_conn: TcpStream, channels: Vec<BulkString>) {
        let client_id = Uuid::new_v4();
        let (client_tx, client_rx) = mpsc::channel();

        let mut state = self.state.lock().unwrap();

        for channel in channels {
            self.logger_tx
                .send(log::info!(
                    "suscribiendo cliente {client_id} a channel {channel}"
                ))
                .unwrap();

            let chan_subs = state.entry(channel).or_default();
            chan_subs.insert(client_id, client_tx.clone());
        }

        let subs = Arc::clone(&self.state);
        let logger_tx = self.logger_tx.clone();

        //OJO FALTA RTA DEL SERVER AL CLINETE Q POSTA SE SUBSCRIBIO
        // When successful, this command doesn't return anything.
        // Instead, for each channel, one message with the first element
        // being the string subscribe is pushed as a confirmation
        // that the command succeeded.

        thread::spawn(move || {
            let read_stream = client_conn.try_clone().unwrap();
            let mut write_stream = client_conn;

            thread::spawn(move || {
                for message in client_rx {
                    if let Err(e) = writeln!(write_stream, "{}", String::from(message)) {
                        println!("Error escribiendo al cliente {}: {}", client_id, e);
                        break;
                    }
                }
            });

            thread::spawn(move || {
                let mut reader = BufReader::new(read_stream);

                loop {
                    let buffer = match reader.fill_buf() {
                        Ok(buf) if !buf.is_empty() => buf,
                        Ok(_) => continue, // Si no hay datos, sigue esperando
                        Err(e) => {
                            println!("Error leyendo del cliente {}: {:?}", client_id, e);
                            break;
                        }
                    };
                    match commands::Command::try_from(buffer) {
                        Ok(cmd) => {
                            println!("CLIENTE ENVIÓ: {:?}", cmd);
                            Broker::process_pubsub_cmd_anidado(
                                Arc::clone(&subs),
                                logger_tx.clone(),
                                cmd,
                                client_id,
                                client_tx.clone(),
                            );
                            let len = buffer.len();
                            reader.consume(len);
                        }
                        Err(e) => {
                            println!("Error parseando RESP del cliente {}: {:?}", client_id, e);
                            break;
                        }
                    }
                }
            });
        });
    }

    fn process_pubsub_cmd_anidado(
        subs: Arc<Mutex<State>>,
        logger_tx: Sender<LogMsg>,
        cmd: commands::Command,
        client_id: Uuid,
        client_tx: Sender<BulkString>,
    ) {
        if let Command::PubSub(PubSubCommand::Subscribe(Subscribe { channels })) = cmd {
            let mut state = subs.lock().unwrap();
            for channel in channels {
                logger_tx
                    .send(log::info!(
                        "suscribiendo cliente {client_id} a channel {channel}"
                    ))
                    .unwrap();
                let chan_subs = state.entry(channel).or_default();
                chan_subs.insert(client_id, client_tx.clone());
            }

            //OJO FALTA RTA DEL SERVER AL CLINETE Q POSTA SE SUBSCRIBIO
            // When successful, this command doesn't return anything.
            // Instead, for each channel, one message with the first element
            // being the string subscribe is pushed as a confirmation
            // that the command succeeded.
        }
    }

    fn add_subscriptions(
        &mut self,
        client_id: Uuid,
        channels: Vec<BulkString>,
        client_tx: Sender<BulkString>,
    ) {
        let mut state = self.state.lock().unwrap();
        for channel in channels {
            self.logger_tx
                .send(log::info!(
                    "suscribiendo cliente {client_id} a channel {channel}"
                ))
                .unwrap();
            let chan_subs = state.entry(channel).or_default();
            chan_subs.insert(client_id, client_tx.clone());
        }
    }

    // https://redis.io/docs/latest/commands/publish#resp2resp3-reply
    fn publish(&mut self, channel: BulkString, msg: BulkString) -> Result<RespDataType, ()> {
        let state = self.state.lock().unwrap();

        if let Some(chan_subs) = state.get(&channel) {
            for client_tx in chan_subs.values() {
                client_tx.send(msg.clone()).unwrap();
            }
        }

        Ok(SimpleString::from("OK").into())
    }

    // https://redis.io/docs/latest/commands/unsubscribe#resp2resp3-reply
    fn unsubscribe(subs: Arc<Mutex<State>>, client_id: Uuid) -> Result<RespDataType, ()> {
        let mut state = subs.lock().unwrap();

        for (chan_name, chan_subs) in state.iter_mut() {
            if chan_subs.remove(&client_id).is_some() {
                println!(
                    "emergy remove client {} from channel {:?}",
                    client_id, chan_name
                );
            }
        }
        //eliminamos canales vacíos
        state.retain(|_, clients| !clients.is_empty());

        Ok(SimpleString::from("OK").into())
    }
}
