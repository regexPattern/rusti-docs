use std::{fmt, io, sync::mpsc::SendError};

use crate::DocsSyncerAction;

#[derive(Debug)]
pub enum Error {
    ConnectionError(io::Error),
    WriteError(io::Error),
    ReadError(io::Error),
    InvalidRespReply,
    SendError,
}

impl std::error::Error for Error {}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::ConnectionError(error) => write!(f, "error conectando con servidor: {error}"),
            Error::WriteError(error) => {
                write!(f, "error escribiendo al stream del servidor: {error}")
            }
            Error::ReadError(error) => write!(f, "error leyendo del stream del servidor: {error}"),
            Error::InvalidRespReply => {
                write!(f, "respuesta del servidor no es del tipo RESP")
            }
            Error::SendError => write!(f, "error de comunicación con channel de actor"),
        }
    }
}

impl From<SendError<DocsSyncerAction>> for Error {
    fn from(_: SendError<DocsSyncerAction>) -> Self {
        Self::SendError
    }
}
