use std::{
    io::Write,
    net::{IpAddr, SocketAddr, TcpListener},
    sync::{
        Arc,
        mpsc::{self, Sender},
    },
    thread,
};

use crate::{config::Config, error::Error, node::Node, thread_pool::ThreadPool};

#[derive(Debug)]
pub struct Server {
    ip: IpAddr,
    port: u16,
    thread_pool: ThreadPool,
    node: Arc<Node>,
    logger_tx: Sender<String>,
}

impl Server {
    pub fn new(mut config: Config) -> Self {
        let (logger_tx, logger_rx) = mpsc::channel();

        // TODO: deberiamos hacer que el nodo entero caiga si no tenemos archivo de logs para
        // escribir?

        // logger-actor
        thread::spawn(move || {
            while let Ok(msg) = logger_rx.recv() {
                if let Err(err) = writeln!(config.logfile, "{msg}") {
                    eprintln!("ERROR error escribiendo logs: {err}");
                    break;
                }
            }
        });

        Self {
            ip: config.bind,
            port: config.port,
            thread_pool: ThreadPool::new(config.io_threads),
            node: Arc::new(Node::new(logger_tx.clone())),
            logger_tx,
        }
    }

    pub fn start(self) -> Result<(), Error> {
        let addr = SocketAddr::new(self.ip, self.port);
        let listener = TcpListener::bind(addr)?;

        self.logger_tx
            .send(format!("INFO servidor escuchando en {:?}", addr))
            .map_err(|_| Error::LogSend)?;

        for conn in listener.incoming() {
            let conn = conn?;

            let logger_tx = self.logger_tx.clone();
            let node = Arc::clone(&self.node);

            self.thread_pool.execute(move || {
                if let Err(err) = node.handle_conn(conn, logger_tx.clone()) {
                    let _ = logger_tx.send(format!("ERROR {err}"));
                }
            })?;
        }

        Ok(())
    }
}
