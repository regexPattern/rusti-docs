use std::{
    fmt, io,
    sync::{MutexGuard, PoisonError, mpsc::SendError},
};

use super::State;

#[derive(Debug)]
pub enum Error {
    ClientDisconnect,
    MsgDelivery(SendError<Vec<u8>>),
    PoisonState,

    Io(io::Error),
    Log(log::Error),
}

impl std::error::Error for Error {}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::ClientDisconnect => write!(f, "cliente desconectado"),
            Error::MsgDelivery(err) => write!(f, "error enviando mensaje al cliente: {err}"),
            Error::PoisonState => write!(f, "error lockeando mutex"),
            Error::Io(err) => write!(f, "{err}"),
            Error::Log(err) => write!(f, "{err}"),
        }
    }
}

impl From<SendError<Vec<u8>>> for Error {
    fn from(err: SendError<Vec<u8>>) -> Self {
        Self::MsgDelivery(err)
    }
}

impl From<PoisonError<MutexGuard<'_, State>>> for Error {
    fn from(err: PoisonError<MutexGuard<'_, State>>) -> Self {
        Self::PoisonState
    }
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Self {
        Self::Io(err)
    }
}

impl From<log::Error> for Error {
    fn from(err: log::Error) -> Self {
        Self::Log(err)
    }
}
