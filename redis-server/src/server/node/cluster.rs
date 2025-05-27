mod attrs;

use std::{
    net::{IpAddr, SocketAddr, TcpStream},
    sync::mpsc::{self, Sender},
};

use attrs::{Attrs, Flag};
use log::LogMsg;
use rand::Rng;

use crate::config::ClusterConfig;

#[derive(Debug)]
pub struct ClusterActor {
    attrs: Attrs,
}

impl ClusterActor {
    pub fn start(
        ip: IpAddr,
        config: ClusterConfig,
        _logger_tx: Sender<LogMsg>,
    ) -> Result<Sender<()>, ()> {
        let (tx, _rx) = mpsc::channel();
        let _stream = TcpStream::connect(SocketAddr::new(ip, config.port));

        let mut id = [0_u8; 20];
        rand::rng().fill(&mut id);

        let _attrs = Attrs {
            id,
            ip,
            port: config.port,
            flags: vec![Flag::Myself, Flag::Master],
            master_id: None,
            // TODO: vamos a tener que generar esto dinamicamente cuando hacemos que el server sea
            // parte de un nodo, lo que significa que probablemente este actor no deberia iniciarse
            // cuando se inicia un nodo, sino que mas bien cuando un nodo arranca como parte de un
            // cluster.
            slot_range: (0, 16384),
        };

        Ok(tx)
    }
}
