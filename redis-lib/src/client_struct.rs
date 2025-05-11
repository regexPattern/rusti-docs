// mod error;
// mod pub_sub;

use std::io::Write;
use std::{
    io::{BufRead, BufReader},
    net::TcpStream,
    sync::mpsc::{self, Sender},
    thread,
};

use redis_cmd::Command;
// use error::Error;
use crate::client_pub_sub_broker::PubSubBroker;
use log::LogMsg;
use redis_resp::SimpleError;

use crate::client_pub_sub_broker::PubSubEnvelope;

#[derive(Debug)]
pub struct Client {
    //STREAM
    //client_storage_tx //sera siempre  el mismo !!!
    broker_tx: Sender<PubSubEnvelope>,
    logger_tx: Sender<LogMsg>,
}

impl Client {
    pub fn new(logger_tx: Sender<LogMsg>) -> Self {
        //CREAR CON EL STREAM YA!!!!
        //TOTAL SERA SIEMPRE  EL MISMO NO???

        let (broker_tx, broker_rx) = mpsc::channel::<PubSubEnvelope>();

        let client = Self {
            //STREAM
            broker_tx,
            logger_tx: logger_tx.clone(),
        };

        thread::spawn(move || {
            let mut broker = PubSubBroker::new(logger_tx);
            while let Ok(envel) = broker_rx.recv() {
                broker.process(envel).unwrap();
            }
        });

        client
    }

    fn execute_command(&self, mut server_conn: TcpStream, cmd: Command) -> Result<(), ()> {
        //no hace falta pasar por pata,etro el stream  xq lo podemos pasar al crear el clinete..

        //la server_conn puede estar guardada como atributo  por que es siempre la misma
        // tambien puede tener guarddo  el channel  de rta  para comandos storgae
        //client_storage_tx

        // tambien puede tener guarddo  el channel  de rta  para subscribe xq es siempre  igual..

        match cmd {
            Command::Storage(cmd) => {
                //envio comando
                let cmd_bytes: Vec<u8> = <Vec<u8>>::from(Command::Storage(cmd));
                server_conn.write_all(&cmd_bytes).unwrap();

                //recibo respuesta

                //escribo rta por client_storage_tx
            }
            Command::PubSub(cmd) => {
                self.broker_tx
                    .send(PubSubEnvelope { server_conn, cmd })
                    .unwrap();
            }
            Command::Cluster(_command) => todo!(),
        };

        Ok(())
    }
}
