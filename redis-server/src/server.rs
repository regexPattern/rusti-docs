mod error;
mod node;

use std::{
    fs::OpenOptions,
    io::Write,
    net::{IpAddr, SocketAddr, TcpListener},
    sync::{
        Arc,
        mpsc::{self, Sender},
    },
    thread,
};

use error::InternalError;
use log::LogMsg;
use node::Node;

use crate::{config::Config, thread_pool::ThreadPool};

#[derive(Debug)]
pub struct Server {
    ip: IpAddr,
    port: u16,
    thread_pool: ThreadPool,
    node: Arc<Node>,
    logger_tx: Sender<LogMsg>,
}

impl Server {
    pub fn new(config: Config) -> Result<Self, InternalError> {
        let (logger_tx, logger_rx) = mpsc::channel();

        let node = Node::start(config.appendfilename, logger_tx.clone())?;

        let mut log_file: Box<dyn Write + Send> = if let Some(path) = config.logfile {
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

        Ok(Self {
            ip: config.bind,
            port: config.port,
            thread_pool: ThreadPool::new(config.io_threads),
            node: Arc::new(node),
            logger_tx,
        })
    }

    pub fn start(self) -> Result<(), InternalError> {
        let addr = SocketAddr::new(self.ip, self.port);
        let listener = TcpListener::bind(addr).map_err(InternalError::AddrBind)?;

        self.logger_tx
            .send(log::info!("servidor escuchando en {:?}", addr))?;

        for conn in listener.incoming() {
            let conn = match conn {
                Ok(conn) => conn,
                Err(err) => {
                    self.logger_tx.send(log::error!("{err}"))?;
                    continue;
                }
            };

            let logger_tx = self.logger_tx.clone();
            let node = Arc::clone(&self.node);

            self.thread_pool.execute(move || {
                if let Err(err) = node.handle_client_conn(conn) {
                    logger_tx.send(log::error!("{err}")).unwrap();
                }
            })?;
        }

        Ok(())
    }
}
