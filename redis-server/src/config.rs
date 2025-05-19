mod error;

use std::collections::HashMap;
use std::net::IpAddr;
use std::net::Ipv4Addr;
use std::path::PathBuf;

use error::Error;

pub struct Config {
    pub bind: IpAddr,
    pub port: u16,
    pub io_threads: usize,
    pub logfile: Option<PathBuf>,
    pub appendfilename: PathBuf,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            bind: IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
            port: 6379,
            io_threads: 2,
            logfile: None,
            appendfilename: PathBuf::from("./appendonly.aof"),
        }
    }
}

impl Config {
    pub fn from_config_file(src: &str) -> Result<Self, Error> {
        let mut config = Self::default();
        let opts = Self::read_opts(src);

        if let Some(bind) = opts.get("bind") {
            config.bind = bind.parse().map_err(Error::BindParse)?;
        }
        if let Some(port) = opts.get("port") {
            config.port = port.parse().map_err(Error::PortParse)?;
        }
        if let Some(io_threads) = opts.get("io-threads") {
            config.io_threads = io_threads.parse().map_err(Error::IoThreadsParse)?;
        }
        if let Some(logfile) = opts.get("logfile") {
            let logfile = logfile.trim_matches('"');
            if !logfile.is_empty() {
                config.logfile = Some(logfile.into());
            }
        }
        if let Some(appendfilename) = opts.get("appendfilename") {
            let appendfilename = appendfilename.trim_matches('"');
            config.appendfilename = appendfilename.into();
        }

        Ok(config)
    }

    fn read_opts(src: &str) -> HashMap<String, String> {
        let mut opts = HashMap::new();

        for line in src.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            let mut parts = line.splitn(2, ' ');

            let (key, value) = match (parts.next(), parts.next()) {
                (Some(key), Some(value)) => (key.trim().to_lowercase(), value.trim().to_string()),
                (Some(key), None) => {
                    eprintln!("missing value for key: `{key}`");
                    continue;
                }
                _ => continue,
            };

            opts.insert(key, value);
        }

        opts
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn se_extraen_opts_del_config_file_correctamente() {
        let src = r#"
bind 127.0.0.1
port 6379
io-threads 2
        "#;

        let opts = Config::read_opts(src);

        assert_eq!(opts.len(), 3);
        assert_eq!(opts.get("bind").unwrap(), "127.0.0.1");
        assert_eq!(opts.get("port").unwrap(), "6379");
        assert_eq!(opts.get("io-threads").unwrap(), "2");
    }

    #[test]
    fn se_ignoran_comentarios_del_config_file() {
        let src = r#"
# comentario 1
# comentario 2
# comentario 3
bind 127.0.0.1
# comentario 4
        "#;

        let opts = Config::read_opts(src);

        assert_eq!(opts.len(), 1);
        assert!(opts.contains_key("bind"));
    }

    #[test]
    fn se_ignoran_whitespaces_del_config_file() {
        let src = r#"

   bind    127.0.0.1

port 6379
        "#;

        let opts = Config::read_opts(src);

        assert_eq!(opts.len(), 2);
        assert_eq!(opts.get("bind").unwrap(), "127.0.0.1");
        assert_eq!(opts.get("port").unwrap(), "6379");
    }

    #[test]
    fn extraccion_de_opts_del_config_file_es_case_insensitive() {
        let src = r#"
BIND 127.0.0.1
pORt 6379
        "#;

        let opts = Config::read_opts(src);

        assert_eq!(opts.len(), 2);
        assert!(opts.contains_key("bind"));
        assert!(opts.contains_key("port"));
    }
}
