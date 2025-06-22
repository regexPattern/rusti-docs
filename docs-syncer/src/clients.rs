use std::{io::Write, net::TcpStream, sync::mpsc::Sender, thread::{self, JoinHandle}, time::Duration};

use redis_cmd::{
    Command,
    pub_sub::{PubSubCommand, Publish},
};

use crate::{DocsSyncer, DocsSyncerAction, error::Error};

/// Intervalo de publicación de listado de clientes conectados por documento.
const CLIENTS_LIST_INTERVAL: Duration = Duration::from_millis(1000);

impl DocsSyncer {
    pub fn start_connected_clients_publisher(
        actions_tx: Sender<DocsSyncerAction>,
    ) -> JoinHandle<Result<(), Error>> {
        thread::spawn(move || {
            loop {
                thread::sleep(CLIENTS_LIST_INTERVAL);
                actions_tx.send(DocsSyncerAction::PublishConnectedClients)?;
            }
        })
    }

    pub fn connect_client(
        &mut self,
        client_id: String,
        doc_id: String,
        doc_kind: String,
        doc_basename: String,
    ) -> Result<(), Error> {
        if self
            .connected_clients
            .entry(doc_id.clone())
            .or_default()
            .insert(client_id.clone())
        {
            print!(
                "{}",
                log::debug!(
                    "registrando al cliente {client_id} en documento {doc_id} {doc_basename}",
                )
            );
        }

        print!(
            "{}",
            log::info!("cliente {client_id} accediendo al documento {doc_id}")
        );

        let content = match doc_kind.as_str() {
            "TEXT" => self.get_text_content(&doc_id),
            "SPREADSHEET" => self.get_spreadsheet_content(&doc_id),
            _ => todo!(),
        };

        let log_msg = log::debug!(
            "enviado al cliente {client_id} información de documento {doc_id} {doc_basename}"
        );

        let mut stream = TcpStream::connect(self.db_addr).map_err(Error::ConnectionError)?;

        let cmd = Command::PubSub(PubSubCommand::Publish(Publish {
            channel: doc_id.into(),
            message: format!("FETCH_ACK@{client_id}@{doc_kind}@{content}").into(),
        }));

        stream
            .write_all(&Vec::from(cmd))
            .map_err(Error::WriteError)?;

        print!("{log_msg}");

        Ok(())
    }

    pub fn disconnect_client(&mut self, client_id: String, doc_id: String) {
        if let Some(doc_clients) = self.connected_clients.get_mut(&doc_id) {
            doc_clients.remove(&client_id);

            print!(
                "{}",
                log::info!("desconectado al cliente {client_id} de documento {doc_id}")
            );
        }
    }

    pub fn publish_connected_clients(&mut self) -> Result<(), Error> {
        for (doc_id, clients) in &self.connected_clients {
            if !clients.is_empty() {
                let clients_ids: Vec<_> = clients.iter().map(|c| c.as_str()).collect();

                let msg = format!("CLIENTS@{}", clients_ids.join(","));

                let cmd = Command::PubSub(PubSubCommand::Publish(Publish {
                    channel: doc_id.into(),
                    message: msg.into(),
                }));

                let mut stream =
                    TcpStream::connect(self.db_addr).map_err(Error::ConnectionError)?;

                stream
                    .write_all(&Vec::from(cmd))
                    .map_err(Error::WriteError)?;
            }
        }

        Ok(())
    }
}
