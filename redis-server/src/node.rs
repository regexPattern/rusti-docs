mod error;
mod pub_sub;

use std::io::Write;
use std::{
    io::{BufRead, BufReader},
    net::TcpStream,
    sync::mpsc::{self, Sender},
    thread,
};

use commands::Command;
use error::Error;
use log::LogMsg;
use pub_sub::Broker;

use crate::storage::{self, Shard};

#[derive(Debug)]
pub struct Node {
    broker_tx: Sender<pub_sub::Envelope>,
    storage_tx: Sender<storage::Envelope>,
    _logger_tx: Sender<LogMsg>,
}

impl Node {
    pub fn new(logger_tx: Sender<LogMsg>) -> Self {
        let (broker_tx, broker_rx) = mpsc::channel::<pub_sub::Envelope>();
        let (storage_tx, storage_rx) = mpsc::channel::<storage::Envelope>();

        let node = Self {
            broker_tx,
            storage_tx,
            _logger_tx: logger_tx.clone(),
        };

        // pub-sub-actor/broker
        thread::spawn(move || {
            let mut broker = Broker::new(logger_tx);
            while let Ok(envel) = broker_rx.recv() {
                broker.process(envel);
            }
        });

        // storage-actor
        thread::spawn(move || {
            let mut shard = Shard::new();
            while let Ok(envel) = storage_rx.recv() {
                shard.process(envel);
            }
        });

        node
    }

    pub fn handle_conn(&self, conn: TcpStream) -> Result<(), Error> {
        // let mut buffer = [0u8; 1024];
        // let n = stream.read(&mut buffer).unwrap();
        // let bytes = &buffer[..n];

        let mut reader = BufReader::new(conn.try_clone().unwrap());
        let bytes = reader.fill_buf().unwrap();

        let cmd = Command::try_from(bytes).unwrap();

        let length = bytes.len();
        reader.consume(length);

        self.execute_command(conn, cmd).unwrap();

        Ok(())
    }

    fn execute_command(&self, mut client_conn: TcpStream, cmd: Command) -> Result<(), ()> {
        let (reply_tx, reply_rx) = mpsc::channel();

        match cmd {
            Command::Storage(cmd) => {
                self.storage_tx
                    .send(storage::Envelope { cmd, reply_tx })
                    .unwrap();

                while let Ok(reply) = reply_rx.recv() {
                    let reply = match reply {
                        Ok(reply) => reply,
                        Err(_hash) => todo!("TRAER VALOR DEL NODO QUE TIENE EL HASH"),
                    };

                    // TEMP: solo hacemos esto para imprimir los valores crudos en el cliente
                    // mientras no tenemos forma de tomar los tipos de resp e imprimirlos.
                    let reply_repr = String::from_utf8(reply)
                        .unwrap()
                        .replace("\r", "\\r")
                        .replace("\n", "\\n");

                    client_conn.write_all(reply_repr.as_bytes()).unwrap();
                }
            }
            Command::PubSub(cmd) => {
                self.broker_tx
                    .send(pub_sub::Envelope { client_conn, cmd })
                    .unwrap();
            }
            Command::Cluster(_command) => todo!(),
        };

        Ok(())
    }
}
