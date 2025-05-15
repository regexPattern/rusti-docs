use std::{error, path::Path};

use redis_server::{Config, Server, cli};

fn main() {
    let args = cli::parse();

    let config = match args.config_path {
        Some(path) => match read_config_file(&path) {
            Ok(config) => config,
            Err(err) => {
                eprint!(
                    "{}",
                    log::error!("error cargando archivo de configuración: {err}")
                );
                return;
            }
        },
        None => Config::default(),
    };

    if let Err(err) = run(config) {
        eprint!(
            "{}",
            log::error!("error durante la ejecución del servidor: {err}")
        );
    }
}

fn run(config: Config) -> Result<(), Box<dyn error::Error>> {
    let server = Server::new(config)?;
    Ok(server.start()?)
}

fn read_config_file(path: &Path) -> Result<Config, Box<dyn error::Error>> {
    let src = std::fs::read_to_string(path)?;
    Ok(Config::from_config_file(&src)?)
}
