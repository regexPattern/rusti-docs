use std::{fmt, io};

use super::{pub_sub, storage};

#[derive(Debug)]
pub enum InternalError {
    PubSubBroker(pub_sub::InternalError),
    StorageActor(storage::InternalError),
    StreamRead(io::Error),
    StreamWrite(io::Error),
}

impl std::error::Error for InternalError {}

impl fmt::Display for InternalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            InternalError::PubSubBroker(err) => write!(f, "{err}"),
            InternalError::StorageActor(err) => write!(f, "{err}"),
            InternalError::StreamRead(err) => {
                write!(f, "error leyendo del stream del cliente: {err}")
            }
            InternalError::StreamWrite(err) => {
                write!(f, "error escribiendo al stream del cliente: {err}")
            }
        }
    }
}

impl From<pub_sub::InternalError> for InternalError {
    fn from(err: pub_sub::InternalError) -> Self {
        Self::PubSubBroker(err)
    }
}

impl From<storage::InternalError> for InternalError {
    fn from(err: storage::InternalError) -> Self {
        Self::StorageActor(err)
    }
}
