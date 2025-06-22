use docs_syncer::DocsSyncer;

fn main() {
    let docs_syncer = match DocsSyncer::new() {
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
