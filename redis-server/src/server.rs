#![allow(clippy::result_large_err)]

mod error;
mod node;

use std::{
    fs::OpenOptions,
    io::{Read, Write},
    net::{Ipv4Addr, SocketAddr, TcpListener, TcpStream},
    path::PathBuf,
    sync::{
        Arc,
        mpsc::{self, Receiver, Sender},
    },
    thread,
};

use error::InternalError;
use log::Log;
use node::Node;
use rustls::{
    ServerConfig, StreamOwned,
    pki_types::{CertificateDer, PrivateKeyDer, pem::PemObject},
    server::ServerConnection,
};

use crate::{
    config::{Config, TlsConfig},
    thread_pool::ThreadPool,
};

#[derive(Debug)]
pub struct Server {
    ip: Ipv4Addr,
    port: u16,
    thread_pool: ThreadPool,
    node: Arc<Node>,
    tls_config: Option<TlsConfig>,
    logger_tx: Sender<Log>,
}

#[derive(Debug)]
#[allow(clippy::large_enum_variant)]
pub enum ClientStream {
    Encrypted(StreamOwned<ServerConnection, TcpStream>),
    Unencrypted(TcpStream),
}

impl Server {
    /// Crea una nueva instancia del servidor Redis, inicializando el nodo, logger y modo cluster/TLS.
    /// Devuelve el servidor listo para aceptar conexiones de clientes.
    pub fn new(config: Config) -> Result<Self, InternalError> {
        let (logger_tx, logger_rx) = mpsc::channel();
        Self::setup_logger(config.logfile, logger_rx)?;

        let mut node = Node::start(config.appendfilename, logger_tx.clone())?;

        if let Some(cluster_config) = config.cluster {
            logger_tx.send(log::info!("iniciando servidor en modo cluster"))?;
            node.enable_cluster_mode(cluster_config)?;
        } else {
            logger_tx.send(log::info!("iniciando servidor en modo standalone"))?;
        }

        if config.tls.is_some() {
            logger_tx.send(log::info!(
                "iniciando servidor con encriptación TLS activada"
            ))?;
        }

        Ok(Self {
            ip: config.bind,
            port: config.port,
            thread_pool: ThreadPool::new(config.io_threads),
            node: Arc::new(node),
            tls_config: config.tls,
            logger_tx,
        })
    }

    fn setup_logger(
        logfile: Option<PathBuf>,
        logger_rx: Receiver<Log>,
    ) -> Result<(), InternalError> {
        let mut log_file: Box<dyn Write + Send> = if let Some(path) = logfile {
            Box::new(
                OpenOptions::new()
                    .append(true)
                    .create(true)
                    .open(path)
                    .map_err(InternalError::LogFileOpen)?,
            )
        } else {
            Box::new(std::io::stdout())
        };

        thread::spawn(move || {
            while let Ok(msg) = logger_rx.recv() {
                if let Err(err) = write!(log_file, "{msg}") {
                    eprintln!("{}", log::error!("{}", InternalError::LogFileWrite(err)));
                    break;
                }
            }
        });

        Ok(())
    }

    /// Inicia el ciclo principal del servidor, aceptando conexiones y delegando a los threads del pool.
    /// Gestiona conexiones TLS si está configurado y reporta errores por log.
    pub fn start(self) -> Result<(), InternalError> {
        let addr = SocketAddr::new(self.ip.into(), self.port);
        let listener = TcpListener::bind(addr).map_err(InternalError::AddrBind)?;

        self.logger_tx
            .send(log::info!("servidor escuchando clientes en {:?}", addr))?;

        let tls_config = if let Some(tls_config) = self.tls_config {
            let certs = CertificateDer::pem_file_iter(tls_config.cert_file)?
                .filter_map(|cert| cert.ok())
                .collect();

            let private_key = PrivateKeyDer::from_pem_file(tls_config.key_file)?;

            Some(Arc::new(
                ServerConfig::builder()
                    .with_no_client_auth()
                    .with_single_cert(certs, private_key)?,
            ))
        } else {
            None
        };

        for client in listener.incoming() {
            let tcp_stream = match client {
                Ok(tcp_stream) => tcp_stream,
                Err(err) => {
                    self.logger_tx.send(log::error!("{err}"))?;
                    continue;
                }
            };

            let logger_tx = self.logger_tx.clone();
            let node = Arc::clone(&self.node);

            let stream = if let Some(tls_config) = &tls_config {
                let tls_stream =
                    StreamOwned::new(ServerConnection::new(Arc::clone(tls_config))?, tcp_stream);
                ClientStream::Encrypted(tls_stream)
            } else {
                ClientStream::Unencrypted(tcp_stream)
            };

            self.thread_pool.execute(move || {
                if let Err(err) = node.handle_client(stream) {
                    let _ = logger_tx.send(log::error!("{err}"));
                }
            })?;
        }

        Ok(())
    }
}

impl Read for ClientStream {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match self {
            ClientStream::Encrypted(stream) => stream.read(buf),
            ClientStream::Unencrypted(stream) => stream.read(buf),
        }
    }
}

impl Write for ClientStream {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        match self {
            ClientStream::Encrypted(stream) => stream.write(buf),
            ClientStream::Unencrypted(stream) => stream.write(buf),
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        match self {
            ClientStream::Encrypted(stream) => stream.flush(),
            ClientStream::Unencrypted(stream) => stream.flush(),
        }
    }
}
