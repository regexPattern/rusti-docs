use std::{fmt, io, net::AddrParseError, num::ParseIntError};

#[derive(Debug)]
pub enum Error {
    BindParse(AddrParseError),
    PortParse(ParseIntError),
    IoThreadsParse(ParseIntError),
    LogFileOpen(io::Error),
    PersistenceFileOpen(io::Error),
}

impl std::error::Error for Error {}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::BindParse(err) => write!(f, "dirección IP del servidor inválida: {err}"),
            Error::PortParse(err) => write!(f, "puerto del servidor inválido: {err}"),
            Error::IoThreadsParse(err) => {
                write!(f, "número de threads del servidor inválido: {err}")
            }
            Error::LogFileOpen(err) => write!(f, "error abriendo archivo de logs: {err}"),
            Error::PersistenceFileOpen(err) => {
                write!(f, "error abriendo archivo de persistencia: {err}")
            }
        }
    }
}
