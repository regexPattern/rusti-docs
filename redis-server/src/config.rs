mod error;

use std::collections::HashMap;
use std::net::Ipv4Addr;
use std::path::PathBuf;

use error::Error;

/// Opciones de configuración del servidor soportadas.
/// https://github.com/redis/redis/blob/51ad2f8d003504a0523a050e7142a44639d8c6ce/redis.conf
#[derive(Debug)]
pub struct Config {
    pub bind: Ipv4Addr,
    pub port: u16,
    pub io_threads: usize,
    pub logfile: Option<PathBuf>,
    pub appendfilename: PathBuf,
    pub tls: Option<TlsConfig>,
    pub cluster: Option<ClusterConfig>,
}

/// Opciones específicas para soporte de conexiones TLS.
/// https://github.com/redis/redis/blob/51ad2f8d003504a0523a050e7142a44639d8c6ce/redis.conf#L189
#[derive(Debug)]
pub struct TlsConfig {
    pub cert_file: PathBuf,
    pub key_file: PathBuf,
    pub ca_cert_file: PathBuf,
}

/// Opciones específicas para soporte para modo cluster.
/// https://github.com/redis/redis/blob/51ad2f8d003504a0523a050e7142a44639d8c6ce/redis.conf#L1596
#[derive(Debug)]
pub struct ClusterConfig {
    pub bind: Ipv4Addr,
    pub port: u16,
    pub cluster_port: u16,
    pub config_file: PathBuf,
    pub node_timeout: u16,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            bind: Ipv4Addr::new(127, 0, 0, 1),
            port: 6379,
            io_threads: 8,
            logfile: None,
            appendfilename: PathBuf::from("./appendonly.aof"),
            tls: None,
            cluster: None,
        }
    }
}

impl ClusterConfig {
    pub fn default(bind: Ipv4Addr, port: u16) -> Self {
        Self {
            bind,
            port,
            cluster_port: port + 10000,
            config_file: PathBuf::from("./nodes.conf"),
            node_timeout: 15000,
        }
    }
}

impl Config {
    /// Construye una configuración del servidor a partir de un archivo de configuración que sigue el formato de [redis.conf](https://github.com/redis/redis/blob/unstable/redis.conf).
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
        if let Some(cluster_enabled) = opts.get("cluster-enabled") {
            if cluster_enabled == "yes" {
                let mut cluster_config = ClusterConfig::default(config.bind, config.port);

                if let Some(cluster_port) = opts.get("cluster-port") {
                    cluster_config.cluster_port = cluster_port.parse().map_err(Error::PortParse)?;
                }
                if let Some(config_file) = opts.get("cluster-config-file") {
                    let config_file = config_file.trim_matches('"');
                    cluster_config.config_file = config_file.into();
                }
                if let Some(node_timeout) = opts.get("cluster-node-timeout") {
                    cluster_config.node_timeout =
                        node_timeout.parse().map_err(Error::NodeTimeoutParse)?;
                }

                config.cluster = Some(cluster_config);
            } else if cluster_enabled != "no" {
                return Err(Error::InvalidClusterEnabledValue);
            }
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
