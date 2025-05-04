mod error;
mod pub_sub;

use crc16::{State, XMODEM};
use std::collections::{HashMap, HashSet};
use std::{
    io::{BufRead, BufReader},
    net::TcpStream,
    sync::mpsc::{self, Sender},
    thread,
};

use crate::command::Keyed;
use command::Command;
use error::Error;
use pub_sub::Broker; //esto es la mejor manera de hacerlo??

use crate::{
    command,
    storage::{self, Database},
};

#[derive(Debug)]
pub struct Node {
    storage_tx: Sender<storage::Envelope>,
    broker_tx: Sender<pub_sub::Envelope>,
    logger_tx: Sender<String>,
    slots: HashSet<u16>,            // los slots que maneja este nodo
    slot_map: HashMap<u16, String>, // hash_slot -> nodo_id
}

impl Node {
    pub fn new(logger_tx: Sender<String>) -> Self {
        let (storage_tx, storage_rx) = mpsc::channel();
        let (broker_tx, broker_rx) = mpsc::channel();

        let logger_tx_actor = logger_tx.clone();

        // storage-actor
        thread::spawn(move || {
            let mut db = Database::new(logger_tx_actor);
            while let Ok(envel) = storage_rx.recv() {
                db.process(envel).unwrap();
                // envel.stream.write_all(b"OK\r\n").unwrap();
            }
        });

        let logger_tx_actor = logger_tx.clone();

        // pub-sub-actor/broker
        thread::spawn(move || {
            let mut broker = Broker::new(logger_tx_actor);
            while let Ok(envel) = broker_rx.recv() {
                broker.handle_request(envel);
            }
        });

        Self {
            storage_tx,
            broker_tx,
            logger_tx,
            slots: HashSet::new(),
            slot_map: HashMap::new(),
        }
    }

    pub fn handle_conn(&self, conn: TcpStream, logs_tx: Sender<String>) -> Result<(), Error> {
        // let mut buffer = [0u8; 1024];
        // let n = stream.read(&mut buffer).unwrap();
        // let bytes = &buffer[..n];

        let mut reader = BufReader::new(conn.try_clone().unwrap());
        let bytes = reader.fill_buf().unwrap();

        // TODO: si nos vamos por la ruta de una thread por cliente, es este caso no deberiamos
        // retornar de la thread/funcion handle_conn (realmente esta funcion deberia irse a su
        // propia thread). En casos de error de serializacion de comandos por ejemplo, deberiamos
        // no cerrar la funcion para no cerrar el stream, solo hacemos otro loop de lectura de
        // comandos.

        let cmd = Command::try_from(bytes).unwrap();

        let length = bytes.len();
        reader.consume(length);

        self.execute_command(conn, cmd).unwrap();

        Ok(())
    }

    pub fn handle_conn_keep_alive(
        &self,
        conn: TcpStream,
        logs_tx: Sender<String>,
    ) -> Result<(), Error> {
        Ok(())
    }

    fn execute_command(&self, conn: TcpStream, cmd: Command) -> Result<(), ()> {
        match cmd {
            Command::Storage(cmd) => {
                //REVISARN TAL VEZ NO ES LA MEJOR IDEEA HACERLO ACA
                // modularizar eso seguro...

                let key = cmd.key(); //***** VER COMAND.RS

                ////VER TRAIT KEYED
                let slot = self.hash_slot(key);

                if self.slots.contains(&slot) {
                    //si le corresponde a este nodo

                    self.storage_tx
                        .send(storage::Envelope {
                            stream: conn,
                            command: cmd,
                        })
                        .unwrap();
                } else {
                    //no le corresponde a este nodo
                    let destino = self.slot_map[&slot].clone();
                    // self.forward_to(&nodo_destino, cmd);
                }

                // let envel = storage::Envelope {
                //     stream: conn,
                //     command: cmd,
                // };

                //self.storage_tx.send(envel).unwrap();
            }
            Command::PubSub(cmd) => match cmd {
                command::PubSubCommand::Subscribe { channel } => {
                    self.broker_tx
                        .send(pub_sub::Envelope::Subscribe {
                            stream: conn,
                            channel,
                        })
                        .unwrap();
                }
                command::PubSubCommand::Publish { channel, message } => {
                    self.broker_tx
                        .send(pub_sub::Envelope::Publish { channel, message })
                        .unwrap();
                }
                command::PubSubCommand::UnSubscribe { channel } => {}
            },
        };

        Ok(())
    }

    fn hash_slot(&self, key: &str) -> u16 {
        let real_key = self.extract_hash_tag(key);
        let hash = State::<XMODEM>::calculate(real_key.as_bytes());
        hash % 16384
    }
    // ES POSIBLE Q ESTO SEA MUY UTIL
    // (O QUE NO SRIVA DIRECTAMENTE JAJA)
    // si tengo por ejemplo las siguientes claves:
    // "doc1:{42}:titulo"
    // "doc1:{42}:contenido"
    // Si usamos todo el string como clave,
    // cada una probablemente caigna en slots diferentes.

    // Pero si usamos solo lo que está entre {}
    // (es decir, 42), entonces todas las claves caen en el mismo slot.

    // Asi Redis (y nosostros tambien) aseguramos
    // que datos relacionados
    // (por ejemplo, de un mismo documento)
    // estén en el mismo hash_slot.
    fn extract_hash_tag(&self, key: &str) -> String {
        if let Some(start) = key.find('{') {
            if let Some(end) = key[start + 1..].find('}') {
                return key[start + 1..start + 1 + end].to_string();
            }
        }
        key.to_string()
        // si no hay {}, se usa toda la clave
    }
}
