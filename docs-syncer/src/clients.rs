use std::{
    io::Write,
    net::TcpStream,
    sync::mpsc::Sender,
    thread::{self, JoinHandle},
    time::Duration,
};

use redis_cmd::{
    Command,
    pub_sub::{PubSubCommand, Publish},
};

use crate::{DocsSyncer, DocsSyncerAction, error::Error};

/// Intervalo de publicación de listado de clientes conectados por documento.
const CLIENTS_LIST_INTERVAL: Duration = Duration::from_millis(1000);

impl DocsSyncer {
    /// Inicia un thread que publica periódicamente la lista de clientes conectados.
    /// Envía la acción PublishConnectedClients cada segundo.
    pub fn start_connected_clients_publisher(
        actions_tx: Sender<DocsSyncerAction>,
    ) -> JoinHandle<Result<(), Error>> {
        thread::spawn(move || {
            loop {
                thread::sleep(CLIENTS_LIST_INTERVAL);
                let _ = actions_tx.send(DocsSyncerAction::PublishConnectedClients);
            }
        })
    }

    /// Registra un cliente como conectado a un documento y le envía el contenido inicial.
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

        let doc_content = match doc_kind.as_str() {
            "TEXT" => self.get_text_content(&doc_id)?,
            "SPREADSHEET" => self.get_spreadsheet_content(&doc_id)?,
            _ => unreachable!(),
        };

        let log_msg = log::debug!(
            "enviado al cliente {client_id} información de documento {doc_id} {doc_basename}"
        );

        let mut stream = TcpStream::connect(self.db_addr).map_err(Error::OpenConn)?;

        // FETCH_ACK@<CLIENT_ID>@<DOC_KIND>@<DOC_CONTENT>
        let msg = format!("FETCH_ACK@{client_id}@{doc_kind}@{doc_content}");

        let cmd = Command::PubSub(PubSubCommand::Publish(Publish {
            channel: doc_id.into(),
            message: msg.into(),
        }));

        stream
            .write_all(&Vec::from(cmd))
            .map_err(Error::SendCommand)?;

        print!("{log_msg}");

        Ok(())
    }

    /// Elimina un cliente de la lista de conectados a un documento.
    pub fn disconnect_client(&mut self, client_id: String, doc_id: String) {
        if let Some(doc_clients) = self.connected_clients.get_mut(&doc_id) {
            doc_clients.remove(&client_id);

            print!(
                "{}",
                log::info!("desconectado al cliente {client_id} de documento {doc_id}")
            );
        }
    }

    /// Publica la lista de clientes conectados para cada documento.
    pub fn publish_connected_clients(&mut self) -> Result<(), Error> {
        for (doc_id, clients) in &self.connected_clients {
            if !clients.is_empty() {
                let clients_ids: Vec<_> = clients.iter().map(|c| c.as_str()).collect();

                // CLIENTS@<CLIENT_ID_1>,<CLIENT_ID_2>,<CLIENT_ID_3>,...
                let msg = format!("CLIENTS@{}", clients_ids.join(","));

                let cmd = Command::PubSub(PubSubCommand::Publish(Publish {
                    channel: doc_id.into(),
                    message: msg.into(),
                }));

                let mut stream = TcpStream::connect(self.db_addr).map_err(Error::OpenConn)?;

                stream
                    .write_all(&Vec::from(cmd))
                    .map_err(Error::SendCommand)?;
            }
        }

        Ok(())
    }
}
