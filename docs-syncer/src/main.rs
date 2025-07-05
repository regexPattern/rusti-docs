use std::net::{Ipv4Addr, SocketAddr};

use docs_syncer::DocsSyncer;

fn main() {
    let ip: Ipv4Addr = match std::env::var("REDIS_HOST") {
        Ok(ip) => match ip.parse() {
            Ok(ip) => ip,
            Err(err) => {
                print!("{}", log::error!("ip inválido: {err}"));
                return;
            }
        },
        Err(_) => Ipv4Addr::new(0, 0, 0, 0),
    };

    let port: u16 = match std::env::var("REDIS_PORT") {
        Ok(port) => match port.parse() {
            Ok(port) => port,
            Err(err) => {
                print!("{}", log::error!("puerto inválido: {err}"));
                return;
            }
        },
        Err(_) => 6379,
    };

    let db_addr = SocketAddr::new(ip.into(), port);

    let docs_syncer = match DocsSyncer::new(db_addr) {
        Ok(docs_syncer) => docs_syncer,
        Err(err) => {
            print!("{}", log::error!("{err}"));
            return;
        }
    };

    match docs_syncer.start() {
        Ok(handle) => {
            if let Err(err) = handle.join().unwrap() {
                print!("{}", log::error!("{err}"));
            }
        }
        Err(err) => print!("{}", log::error!("{err}")),
    }
}
