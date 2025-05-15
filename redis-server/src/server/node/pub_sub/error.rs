use std::{
    fmt, io,
    sync::{MutexGuard, PoisonError, mpsc::SendError},
};

use redis_resp::SimpleError;

use super::State;

/// Errores que pueden ocurrir en el funcionamiento interno del pub/sub broker, no relacionados a las operaciones de pub/sub realizadas por los clientes.
#[derive(Debug)]
pub enum InternalError {
    MsgDelivery(SendError<Vec<u8>>),
    PoisonState,
    Io(io::Error),
}

impl std::error::Error for InternalError {}

impl fmt::Display for InternalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            InternalError::MsgDelivery(err) => {
                write!(f, "error enviando mensaje al cliente: {err}")
            }
            InternalError::PoisonState => write!(f, "error lockeando mutex"),
            InternalError::Io(err) => write!(f, "{err}"),
        }
    }
}

impl From<SendError<Vec<u8>>> for InternalError {
    fn from(err: SendError<Vec<u8>>) -> Self {
        Self::MsgDelivery(err)
    }
}

impl From<PoisonError<MutexGuard<'_, State>>> for InternalError {
    fn from(_: PoisonError<MutexGuard<'_, State>>) -> Self {
        Self::PoisonState
    }
}

impl From<io::Error> for InternalError {
    fn from(err: io::Error) -> Self {
        Self::Io(err)
    }
}

#[derive(Debug)]
pub enum OperationError {
    NotAPubSubCommand,
}

impl From<OperationError> for SimpleError {
    fn from(err: OperationError) -> Self {
        match err {
            OperationError::NotAPubSubCommand => SimpleError::from(
                "ERR solo se permiten los comandos `SUBSCRIBE` / `UNSUBSCRIBE` / `QUIT` mientras se está suscrito",
            ),
        }
    }
}
