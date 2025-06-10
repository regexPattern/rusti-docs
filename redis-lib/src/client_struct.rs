// mod error;
// mod pub_sub;

use std::io::Write;
use std::{
    io::{BufRead, BufReader},
    net::TcpStream,
    sync::mpsc::{self, Receiver, Sender},
    thread,
};

use crate::error::Error;
use redis_cmd::Command;
// use crate::client_pub_sub_broker::PubSubBroker;
use log::Log;
use redis_cmd::pub_sub::Publish;
use redis_cmd::pub_sub::{PubSubCommand, Subscribe, Unsubscribe};
use redis_resp::{RespDataType, SimpleError};

// use crate::client_pub_sub_broker::PubSubEnvelope;

#[derive(Debug)]
pub struct Client {
    //STREAM
    logger_tx: Sender<Log>,
}

impl Client {
    pub fn new(logger_tx: Sender<Log>) -> Self {
        //CREAR CON EL STREAM YA!!!!
        //TOTAL SERA SIEMPRE  EL MISMO NO???

        Self {
            //STREAM
            logger_tx: logger_tx.clone(),
        }
    }

    pub fn execute_non_pub_sub_mode_command(
        //INCLUYE PUBLISH!!
        &self,
        mut client_conn: TcpStream,
        cmd: Command,
    ) -> Result<RespDataType, Error> {
        match cmd {
            Command::Storage(cmd) => {
                let cmd_bytes: Vec<u8> = <Vec<u8>>::from(Command::Storage(cmd));
                client_conn.write_all(&cmd_bytes).unwrap();
            }
            Command::PubSub(cmd) => {
                let cmd_bytes: Vec<u8> =
                    <Vec<u8>>::from(Command::PubSub(PubSubCommand::Publish(Publish {
                        channel: match cmd {
                            PubSubCommand::Publish(Publish { ref channel, .. }) => channel.clone(),
                            _ => return Err(Error::InvalidCommand("Expected Publish".into())),
                        },
                        message: match cmd {
                            PubSubCommand::Publish(Publish { ref message, .. }) => message.clone(),
                            _ => return Err(Error::InvalidCommand("Expected Publish".into())),
                        },
                    })));
                match cmd {
                    PubSubCommand::Publish(Publish {
                        channel: _,
                        message: _,
                    }) => {
                        //SI ESTO PUEDE LLEGAR XQ NO ESTA EN MODO PUB SUB
                        // let cmd_bytes: Vec<u8> = <Vec<u8>>::from(Command::PubSub(cmd.clone()));
                        client_conn.write_all(&cmd_bytes).unwrap();

                        //esperar rta con buffer devolverla
                        //no vale lo de abajooo
                        // return Ok(RespDataType::BulkString(("Ok").into()));
                    }
                    PubSubCommand::Subscribe(_) => {
                        //ESTE NO DEBERIA LLEGAR NUNCA!!
                        return Ok(RespDataType::Null);
                    }
                    PubSubCommand::Unsubscribe(_) => {
                        //ESTE NO DEBERIA LLEGAR NUNCA!!
                        return Ok(RespDataType::Null);
                    }
                    PubSubCommand::PubSubChannels(_) | PubSubCommand::PubSubNumSub { .. } => {
                        //ESTE NO DEBERIA LLEGAR NUNCA!!
                        return Ok(RespDataType::Null);
                    }
                }
            }
            Command::Cluster(_command) => todo!(),
        };

        let mut reader = BufReader::new(client_conn.try_clone().unwrap());
        let mut buf = Vec::new();

        let available = reader.fill_buf().unwrap();
        if available.is_empty() {
            // break; // EOF
        }
        buf.extend_from_slice(available);

        let resp = RespDataType::try_from(buf.as_slice()).unwrap();

        Ok(resp)
    }

    //en realidad creo q es como execute subscribe  por que es lo unico que va hacer posta..
    pub fn execute_pubsub_mode_cmd(
        //NO INLCUYE PUBLISH!
        //TAMPOCO ME DEBERIA LLEGAR UN UNSUBSCRIBE AQUI..
        &mut self,
        cmd: PubSubCommand,
        mut client_conn: TcpStream,
        pub_sub_reply_tx: Sender<RespDataType>,
    ) -> Result<Option<Sender<redis_cmd::Command>>, Error> {
        match cmd {
            PubSubCommand::Subscribe(subscribe) => {
                // keep_alive devuelve el pub_sub_cmd_tx -> VER si esta ok mandar cmds por ahi...

                //REVISAR ESTE TRY_CLONE TODO
                let pub_sub_cmd_tx = self.keep_alive(
                    client_conn.try_clone()?,
                    self.logger_tx.clone(),
                    pub_sub_reply_tx,
                )?;

                let channels = subscribe.channels.clone();
                let cmd_bytes: Vec<u8> =
                    <Vec<u8>>::from(Command::PubSub(PubSubCommand::Subscribe(Subscribe {
                        channels,
                    })));
                //Mando el comando al server...
                // el server me va responder por el pub_sub_reply_tx...
                client_conn.write_all(&cmd_bytes)?;

                // Devuelve el sender para que la GUI pueda enviar comandos pubsub
                Ok(Some(pub_sub_cmd_tx))
            }
            PubSubCommand::Publish(Publish {
                channel: _,
                message: _,
            }) => {
                //ESTE NO DEBERIA LLEGAR NUNCA!!
                Ok(None)
            }
            PubSubCommand::Unsubscribe(_unsubscribe) => {
                //SI LLEGA ESTO NO ESTOY SUBSCRIPTO NADA ACA...
                Ok(None)
            }
            PubSubCommand::PubSubChannels(_) | PubSubCommand::PubSubNumSub { .. } => {
                // Implementa según sea necesario
                Ok(None)
            }
        }
    }

    fn keep_alive(
        &mut self,
        server_conn: TcpStream,
        logger_tx: Sender<Log>,
        pub_sub_reply_tx: Sender<RespDataType>,
    ) -> Result<Sender<redis_cmd::Command>, Error> {
        let write_server_stream = server_conn.try_clone()?;
        let read_server_reply_stream = server_conn.try_clone()?;

        let logger_gui_cmd_tx = logger_tx.clone();

        //CHANNEL PARA MANDAR COMANDOS A LA GUI DEVUELVO EL cmd_pub_sub_tx..
        let (cmd_pub_sub_tx, cmd_pub_sub_rx) = mpsc::channel::<redis_cmd::Command>();

        let pub_sub_reply_gui_thread_tx = pub_sub_reply_tx.clone();
        thread::spawn(move || {
            //EN ESTE THREAD  ESUCHO COMANDOS DE LA GUI Y LOS REENVIO al server !
            //ENTRO EN MODO PUB SUB...
            //entre en modo subscribe..
            Self::listen_incoming_commands(
                cmd_pub_sub_rx,
                &pub_sub_reply_gui_thread_tx,
                &logger_gui_cmd_tx,
                write_server_stream,
            )
            .unwrap_or_else(|err| {
                logger_gui_cmd_tx
                    .send(log::error!("Error in listen_incoming_commands: {err}"))
                    .ok();
            });
        });

        let logger_stream_tx = logger_tx.clone();
        let pub_sub_reply_stream_thread_tx = pub_sub_reply_tx.clone();

        thread::spawn(move || {
            // EN ESTE THREAD ESCUCHO LA RESPUESTA DEL SERVER A LOS COMANDOS QUE LE MANDE
            // + LOS MSJS PUBlISH QUE ME LLEGUEN y SE LOS DEVUELVO A LA GUI
            // POR pub_sub_reply_tx

            let mut reader = BufReader::new(read_server_reply_stream);

            loop {
                match reader.fill_buf() {
                    Ok(bytes) if !bytes.is_empty() => {
                        pub_sub_reply_stream_thread_tx
                            .send(RespDataType::try_from(bytes).unwrap())
                            .unwrap();
                        let len = bytes.len();
                        reader.consume(len);
                    }
                    Ok(_) => {
                        logger_stream_tx.send(log::info!("server -servercon- desconectado"))?;
                        return Err::<(), Error>(Error::ClientDisconnect);
                    }
                    Err(err) => {
                        logger_stream_tx.send(log::error!(
                            "error al leer bytes mandados por el SERVER {err}",
                        ))?;

                        return Err(err.into());
                    }
                };
            }
        });

        logger_tx.send(log::info!("manteniendo viva conexión de servidor ",))?;

        // Devolvemos el canal para que la GUI pueda enviar comandos
        Ok(cmd_pub_sub_tx)
    }

    fn listen_incoming_commands(
        cmd_pub_sub_rx: Receiver<redis_cmd::Command>,
        pub_sub_reply_tx: &Sender<RespDataType>, //reply a la GUI
        _logger_tx: &Sender<Log>,
        mut write_server_stream: TcpStream,
    ) -> Result<(), Error> {
        //ver q tipo devolver en este chanelll!!
        //ESTO SE EJECUTA UNA SOLA VEZ ! DSps entro al loop de abaJo
        //ACA SOLO MANDO COMANDOS AL SERVER, RECIBO RESPUESTAS POR EL OTRO THREAD...

        for cmd in cmd_pub_sub_rx {
            match cmd {
                Command::PubSub(PubSubCommand::Subscribe(Subscribe { channels })) => {
                    let cmd_bytes: Vec<u8> =
                        <Vec<u8>>::from(Command::PubSub(PubSubCommand::Subscribe(Subscribe {
                            channels,
                        })));
                    write_server_stream.write_all(&cmd_bytes)?;
                }
                Command::PubSub(PubSubCommand::Unsubscribe(Unsubscribe { channels })) => {
                    let cmd_bytes: Vec<u8> =
                        <Vec<u8>>::from(Command::PubSub(PubSubCommand::Unsubscribe(Unsubscribe {
                            channels,
                        })));
                    write_server_stream.write_all(&cmd_bytes)?;
                }
                _ => {
                    //FALTA EL LOOGER
                    pub_sub_reply_tx
                        .send(SimpleError::from("ERR solo se permiten los comandos `SUBSCRIBE` / `UNSUBSCRIBE` / `QUIT` en subscriber state").into())
                        .unwrap();
                }
            }
        }
        Ok(())
    }
}
