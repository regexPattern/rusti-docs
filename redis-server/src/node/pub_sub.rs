use std::{
    collections::HashMap,
    io::{BufReader, prelude::*},
    net::TcpStream,
    sync::mpsc::{self, Sender},
    sync::{Arc, Mutex},
    thread,
};

use uuid::Uuid;

type Subs = HashMap<String, HashMap<Uuid, Sender<String>>>;

#[derive(Debug)]
pub struct Broker {
    state: Arc<Mutex<Subs>>,
    logger_tx: Sender<String>,
}

#[derive(Debug)]
pub enum Envelope {
    Subscribe { stream: TcpStream, channel: String },
    Publish { channel: String, message: String },
    UnSubscribe { channel: String, client_id: Uuid },
}

impl Broker {
    pub fn new(logger_tx: Sender<String>) -> Self {
        Self {
            state: Arc::new(Mutex::new(HashMap::new())),
            logger_tx,
        }
    }

    pub fn handle_request(&mut self, envel: Envelope) {
        match envel {
            Envelope::Subscribe { stream, channel } => self.handle_subscribe(stream, channel),
            Envelope::Publish { channel, message } => self.handle_publish(channel, message),
            Envelope::UnSubscribe { channel, client_id } => {
                // self.handle_unsubscribe(channel, client_id);
            }
        }
    }

    fn handle_subscribe(&mut self, stream: TcpStream, channel: String) {
        let (client_tx, client_rx) = mpsc::channel();

        let client_id = Uuid::new_v4();

        let mut map = self.state.lock().unwrap();
        let subs = map.entry(channel.clone()).or_default();
        subs.insert(client_id, client_tx);

        let subs_clone = self.state.clone();

        thread::spawn(move || {
            println!("SUSCRIBIENDO CLIENTE: {}", client_id);

            let read_stream = stream.try_clone().unwrap();
            let mut write_stream = stream;

            // let (err_tx, err_rx) = mpsc::channel();
            // let err_tx_clone = err_tx.clone();

            thread::spawn(move || {
                for message in client_rx {
                    if let Err(e) = writeln!(write_stream, "{}", message) {
                        println!("Error escribiendo al cliente {}: {}", client_id, e);
                        // Broker::handle_emergency_unsubscribe(subs_clone, client_id_clone);
                        break;
                    }
                }
            });

            thread::spawn(move || {
                let reader = BufReader::new(read_stream);
                // let bytes = reader.fill_buf().unwrap();
                // let length = bytes.len();
                // reader.consume(length);

                for line in reader.lines() {
                    match line {
                        Ok(comando) => {
                            println!("CLIENTE ENVIÓ: {}", comando);
                        }
                        Err(e) => {
                            println!("Cliente desconectado o error de lectura: {}", e);
                            Broker::handle_emergency_unsubscribe(subs_clone, client_id);
                            break;
                        }
                    }
                }
            })
        });
    }

    fn handle_publish(&mut self, channel: String, message: String) {
        dbg!(&self.state);

        let map_guard = self.state.lock().unwrap();
        if let Some(channel_subs) = map_guard.get(&channel) {
            for (_client_id, client_tx) in channel_subs {
                println!("Enviando mensaje a cliente: {}", _client_id);
                client_tx.send(message.clone()).unwrap();
            }
        }
    }

    fn handle_emergency_unsubscribe(subs: Arc<Mutex<Subs>>, client_id: Uuid) {
        let mut subs_guard = subs.lock().unwrap();
        for (chan, clients) in subs_guard.iter_mut() {
            if clients.remove(&client_id).is_some() {
                println!("emergy remove client {} from channel {}", client_id, chan);
            }
        }
        //eliminamos canales vacíos
        subs_guard.retain(|_, clients| !clients.is_empty());
    }
}
