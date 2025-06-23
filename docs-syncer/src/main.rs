use docs_syncer::DocsSyncer;

fn main() {
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

    let docs_syncer = match DocsSyncer::new(port) {
        Ok(docs_syncer) => docs_syncer,
        Err(err) => {
            print!("{}", log::error!("{err}"));
            return;
        }
    };

    match docs_syncer.start() {
        Ok(handles) => {
            for h in handles.into_iter() {
                if let Err(err) = h.join().unwrap() {
                    print!("{}", log::error!("{err}"));
                }
            }
        }
        Err(err) => print!("{}", log::error!("{err}")),
    }
}
