use std::{fmt, io, net::AddrParseError, num::ParseIntError};

#[derive(Debug)]
pub enum Error {
    BindParse(AddrParseError),
    InvalidClusterEnabledValue,
    IoThreadsParse(ParseIntError),
    LogFileOpen(io::Error),
    NodeTimeoutParse(ParseIntError),
    PersistenceFileOpen(io::Error),
    PortParse(ParseIntError),
}

impl std::error::Error for Error {}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::BindParse(err) => write!(f, "dirección IP del servidor inválida: {err}"),
            Error::PortParse(err) => write!(f, "puerto del servidor inválido: {err}"),
            Error::InvalidClusterEnabledValue => {
                write!(f, "valor de opción 'cluster-enabled' debe ser 'yes' o 'no'")
            }
            Error::IoThreadsParse(err) => {
                write!(f, "número de threads del servidor inválido: {err}")
            }
            Error::LogFileOpen(err) => write!(f, "error abriendo archivo de logs: {err}"),
            Error::NodeTimeoutParse(err) => write!(f, "timeout del nodo inválido: {err}"),
            Error::PersistenceFileOpen(err) => {
                write!(f, "error abriendo archivo de persistencia: {err}")
            }
        }
    }
}
