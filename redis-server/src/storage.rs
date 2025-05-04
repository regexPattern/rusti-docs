use std::{collections::HashMap, net::TcpStream, sync::mpsc::Sender};

use crate::command::StorageCommand;

#[derive(Debug)]
// enum Role {
//     Maestro,
//     Replica { nodo_maestro: String },
// }

pub struct Database {
    // role: Role,
    data: HashMap<String, String>,
    logger_tx: Sender<String>,
}

#[derive(Debug)]
pub struct Envelope {
    pub stream: TcpStream,
    pub command: StorageCommand,
}

impl Database {
    pub fn new(logger_tx: Sender<String>) -> Self {
        Self {
            data: HashMap::new(),
            logger_tx,
        }
    }

    pub fn process(&mut self, envel: Envelope) -> Result<(), ()> {
        match envel.command {
            StorageCommand::Get { key } => {
                self.logger_tx
                    .send(format!("OP PROCESSING: GET {key}"))
                    .unwrap();
                let value = self.data.get(&key).unwrap();
                self.logger_tx
                    .send(format!("obtenido valor: {}", value))
                    .unwrap();
            }
            StorageCommand::Set { key, value } => {
                self.logger_tx.send("manejando SET".to_string()).unwrap();
                self.data.insert(key.clone(), value);
                self.logger_tx
                    .send(format!("escrito valor: {}", key))
                    .unwrap();
            }
        }

        Ok(())
    }
}
