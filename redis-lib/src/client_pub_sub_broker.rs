// mod error;

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

use crate::error::Error;
use commands::{Command, pub_sub::*};
use log::LogMsg;
use resp::{Array, BulkString, Integer, SimpleError};

// use uuid::Uuid;

//este state sera un estado  de sus subscripciones
//COMO EL ES EL UNICO  Q LAS MODIFICA NO NECESITA UN MUTTEX !!!!!
type State = HashMap<BulkString, Sender<Vec<u8>>>;
//listado de canales a los q esta subscripto
// es mas simple todvia q eese hashmap
//no deberi hacer falat el mutx pero temo por el caso borde en que el cliente
//hace subscrbe para ver un docu  e inmediatamente un publish (dde el otro thread)
//parta decir me conecte. podemos esperar la rta del server para hacer el publish(entrega final..)

#[derive(Debug)]
pub struct PubSubBroker {
    state: Arc<Mutex<State>>,
    logger_tx: Sender<LogMsg>,
}

#[derive(Debug)]
pub struct PubSubEnvelope {
    pub server_conn: TcpStream,
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
                    self.keep_alive(envel.server_conn, self.logger_tx.clone())?;

                Self::subscribe(
                    &mut client_conn,
                    // client_id,
                    client_tx,
                    &self.state,
                    channels,
                    &self.logger_tx,
                )?;
            }
            PubSubCommand::Publish(Publish { channel, message }) => {
                let reply = self.publish(&channel, message)?;
                envel.server_conn.write_all(&reply)?;
            }
            PubSubCommand::Unsubscribe(Unsubscribe { channels }) => {
                Self::unsubscribe(
                    &mut envel.server_conn,
                    // None,
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
        server_conn: TcpStream,
        logger_tx: Sender<LogMsg>,
        //  reply_pub_sub_tx // POR ESTE LE MANDO REPLIES A LA GUI
    ) -> Result<(TcpStream, Sender<Vec<u8>>), Error> {
        // CUANDO ME MANDAN SUBSCRIBE CREO UM CHANNEL ESPECIAL PARA QUE EL CLIENYE ME PUEDA
        //  ENVIARRRR COMANDOS  Y LE PASO  EL cmd_pub_sub_tx
        //( SALVO Q SEA POSIBLE CREARLO ALLA Y PASARLKO CON OWNERSAHIP PARA ACA PERO ES HORRIBLE)

        // let (cmd_pub_sub_tx, cmd_pub_sub_rx) = mpsc::channel::<PubSubEnvelope>(  );

        let (client_tx, client_rx) = mpsc::channel::<Vec<u8>>();

        let mut publisher_stream = server_conn.try_clone()?;
        let listener_stream = server_conn.try_clone()?;

        let state = Arc::clone(&self.state);
        let client_tx_publisher = client_tx.clone();
        let logger_tx_publisher = logger_tx.clone();

        //thread de lectura de publishes de channels del servidor...
        thread::spawn(move || {
            // THREAD  PARA  ESUCHAR PUBLISHES  DE LOS CHANNELS  A LOS Q ME SUBSCRIBI

            // ESTE  ESUCHA EN UN CLIENT_SERVER_RX Q SE CREA AQI Y VEV AQUI

            //reply_pub_sub_tx POR ESTE LE MANDO LOS MSJS que llegan por publish A LA GUIA
            // este me lo paso por parametro...

            for message in client_rx {
                if let Err(err) = publisher_stream.write_all(&message) {
                    // en vez de pubslih stream sera un
                    //reply_pub_sub_tx.send()
                    //mensajes publsih que llegan..

                    logger_tx_publisher
                        .send(log::error!(
                            "error mandando mensaje al a la GUI {client_id}: {err}"
                        ))
                        .unwrap();

                    Self::unsubscribe(
                        &mut publisher_stream,
                        // Some(client_id),
                        Some(client_tx_publisher),
                        &state,
                        Vec::new(),
                        &logger_tx_publisher,
                    )
                    .unwrap();

                    //  IMPORTANTE
                    //acabo de darme cuenta q borre todas las fucniones que hacian reply
                    /// esas funciones
                    // ESCRIBEMN  A LA INTERFAZ  LA RTA  DEL SERVIDOR
                    // POR ACAAAAAA
                    // reply_pub_sub_tx
                    break;
                }
            }
        });

        let state = Arc::clone(&self.state);
        let client_tx_listener = client_tx.clone();
        let logger_tx_listener = logger_tx.clone();

        thread::spawn(move || {
            // EN ESTE THREAD ESCUCHO LOS COMANDOS POR EL CHANNEL QUE CREE ANTES
            //cmd_pub_sub_rx
            // al cliente le devuelvo el senderrrr xa q l tenga yu me peuda enviuars comamndps!!
            // cmd_pub_sub_tx

            if let Err(err) = Self::listen_server_conn(
                listener_stream,
                // client_id,
                client_tx_listener,
                //cmd_pub_sub_rx_listener, //listener para comandossss es IGUALLLL
                &state,
                &logger_tx_listener,
            ) {
                logger_tx_listener
                    .send(log::error!("error escuchando stream del SERVIDOR {err}"))
                    .unwrap();
            }
        });

        logger_tx.send(log::info!("manteniendo viva conexión de servidor ",))?;

        Ok((server_conn, client_tx))

        //CLAAAAAAAAARO ES TAN PARECIDO  QUE EN VEZ DE DEVOLVER EL STREAM  ESTOY DEVOLVIEDNO
        // EL CHANNEL POR EL QUE LE HABLO A LA GUI
        //ok(cmd_pub_sub_tx)
    }

    fn listen_server_conn(
        server_conn: TcpStream,
        // client_id: Uuid,
        client_tx: Sender<Vec<u8>>,

        state: &Arc<Mutex<State>>,
        logger_tx: &Sender<LogMsg>,
    ) -> Result<(), Error> {
        // EN ESTE THREAD ESCUCHO LOS COMANDOS POR EL CHANNEL QUE CREE ANTES
        //cmd_pub_sub_rx
        // al cliente le devuelvo el senderrrr xa q l tenga yu me peuda enviuars comamndps!!
        // cmd_pub_sub_tx

        let read_stream = server_conn.try_clone()?;
        let mut reply_stream = server_conn;

        let mut reader = BufReader::new(read_stream);

        loop {
            match reader.fill_buf() {
                Ok(bytes) if !bytes.is_empty() => {
                    //NO LEEREMOS UN COMANDO SI NO UNA REPLY....
                }
                Ok(_) => {
                    logger_tx.send(log::info!("server -servercon- desconectado"))?;
                    return Err(Error::ClientDisconnect);
                }
                Err(err) => {
                    logger_tx.send(log::error!(
                        "error al leer bytes mandados por el SERVER {err}",
                    ))?;

                    return Err(err.into());
                }
            };
        }
    }

    /// https://redis.io/docs/latest/commands/subscribe
    fn subscribe(
        server_conn: &mut TcpStream,
        // client_id: Uuid,
        client_tx: Sender<Vec<u8>>,
        state: &Arc<Mutex<State>>,
        channels: Vec<BulkString>,
        logger_tx: &Sender<LogMsg>,
    ) -> Result<(), Error> {
        let mut state = state.lock()?;

        for chan_name in channels {
            logger_tx.send(log::info!("cliente suscrito al canal {chan_name}",))?;

            let chan_subs = state.entry(chan_name.clone()).or_default();
            chan_subs.insert(client_tx.clone());
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
        server_conn: &mut TcpStream,
        // client_id: Option<Uuid>,
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
        }

        Ok(())
    }
}
