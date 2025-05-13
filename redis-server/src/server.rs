mod error;

use std::{
    fs::File,
    io::Write,
    net::{IpAddr, SocketAddr, TcpListener},
    sync::{
        Arc,
        mpsc::{self, Sender},
    },
    thread,
};

use error::Error;
use log::LogMsg;

use crate::{config::Config, node::Node, thread_pool::ThreadPool};

#[derive(Debug)]
pub struct Server {
    ip: IpAddr,
    port: u16,
    thread_pool: ThreadPool,
    node: Arc<Node>,
    logger_tx: Sender<LogMsg>,
}

impl Server {
    pub fn try_new(config: Config) -> Result<Self, Error> {
        let (logger_tx, logger_rx) = mpsc::channel::<LogMsg>();

        let node = Node::start(config.appendfilename, logger_tx.clone()).unwrap();

        let mut logfile: Box<dyn Write + Send> = if let Some(logfile) = config.logfile {
            Box::new(File::open(logfile).unwrap())
        } else {
            Box::new(std::io::stdout())
        };

        thread::spawn(move || {
            while let Ok(msg) = logger_rx.recv() {
                if let Err(err) = write!(logfile, "{msg}") {
                    eprintln!("{}", log::error!("error escribiendo logs: {err}"));

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

    pub fn start(self) -> Result<(), Error> {
        let addr = SocketAddr::new(self.ip, self.port);
        let listener = TcpListener::bind(addr)?;

        self.logger_tx
            .send(log::info!("servidor escuchando en {:?}", addr))?;

        for conn in listener.incoming() {
            let conn = conn?;

            let logger_tx = self.logger_tx.clone();
            let node = Arc::clone(&self.node);

            self.thread_pool.execute(move || {
                if let Err(err) = node.handle_client_conn(conn) {
                    let _ = logger_tx.send(log::error!("{err}"));
                }
            })?;
        }

        Ok(())
    }
}
