use std::{fmt, io, sync::mpsc::SendError};

// use super::State;

#[derive(Debug)]
pub enum Error {
    ClientDisconnect,
    MsgDelivery(SendError<Vec<u8>>),
    PoisonState,
    InvalidCommand(String),
    Io(io::Error),
    Log(log::Error),
}

impl std::error::Error for Error {}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            //TODO CAMBIAR NOMBRES DE ERRORES
            Error::ClientDisconnect => write!(f, "server desconectado"),
            Error::MsgDelivery(err) => write!(f, "error enviando mensaje al server: {err}"),
            Error::PoisonState => write!(f, "error lockeando mutex"), //no hara falta....
            Error::Io(err) => write!(f, "{err}"),
            Error::Log(err) => write!(f, "{err}"),
            Error::InvalidCommand(err) => write!(f, "error en el comando: {err}"),
        }
    }
}

impl From<SendError<Vec<u8>>> for Error {
    fn from(err: SendError<Vec<u8>>) -> Self {
        Self::MsgDelivery(err)
    }
}

// impl From<PoisonError<MutexGuard<'_, State>>> for Error {
//     fn from(err: PoisonError<MutexGuard<'_, State>>) -> Self {
//         Self::PoisonState
//     }
// }

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
