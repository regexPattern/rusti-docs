use redis_server::{Config, Server, cli};

fn main() {
    let args = cli::parse();

    let config = match args.config_path {
        Some(path) => {
            let src = match std::fs::read_to_string(path) {
                Ok(src) => src,
                Err(err) => {
                    eprintln!("ERROR error leyendo archivo de configuración: {err}");
                    return;
                }
            };

            match Config::from_config_file(&src) {
                Ok(config) => config,
                Err(err) => {
                    eprintln!("ERROR error parseando archivo de configuración: {err}");
                    return;
                }
            }
        }
        None => Config::default(),
    };

    let server = Server::new(config).unwrap();
    server.start().unwrap();
}
