use std::{fmt, io, sync::mpsc::SendError};

use crate::DocsSyncerAction;

#[derive(Debug)]
pub enum Error {
    Connection(io::Error),
    Write(io::Error),
    Read(io::Error),
    InvalidRespReply,
    Send,
    MissingData,
    RedisClient(String),
}

impl std::error::Error for Error {}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Connection(error) => write!(f, "error conectando con servidor: {error}"),
            Error::Write(error) => {
                write!(f, "error escribiendo al stream del servidor: {error}")
            }
            Error::Read(error) => write!(f, "error leyendo del stream del servidor: {error}"),
            Error::InvalidRespReply => {
                write!(f, "respuesta del servidor no es del tipo RESP")
            }
            Error::Send => write!(f, "error de comunicación con channel de actor"),
            Error::MissingData => write!(f, "datos faltantes en mensaje de clientes"),
            Error::RedisClient(err) => write!(f, "error enviado del servidor: {err}"),
        }
    }
}

impl From<SendError<DocsSyncerAction>> for Error {
    fn from(_: SendError<DocsSyncerAction>) -> Self {
        Self::Send
    }
}
