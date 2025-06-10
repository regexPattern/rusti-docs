use std::{fmt, io, sync::mpsc::SendError};

use log::Log;

use super::{
    cluster::ClusterAction,
    pub_sub::{self, PubSubEnvelope},
    storage::{self, StorageAction},
};

#[derive(Debug)]
pub enum InternalError {
    LogSend(SendError<Log>),
    ClusterActorSend(SendError<ClusterAction>),
    PubSubBroker(pub_sub::InternalError),
    PubSubBrokerSend(SendError<PubSubEnvelope>),
    StorageActor(storage::InternalError),
    StorageActorSend(SendError<StorageAction>),
    StreamRead(io::Error),
    StreamWrite(io::Error),
}

impl std::error::Error for InternalError {}

impl fmt::Display for InternalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            InternalError::LogSend(err) => write!(
                f,
                "error enviando action por el canal del cluster actor: {err}"
            ),
            InternalError::ClusterActorSend(err) => write!(f, "{err}"),
            InternalError::PubSubBroker(err) => write!(f, "{err}"),
            InternalError::PubSubBrokerSend(err) => write!(
                f,
                "error enviando action por el canal del pub/sub broker: {err}"
            ),
            InternalError::StorageActor(err) => write!(f, "{err}"),
            InternalError::StreamRead(err) => {
                write!(f, "error leyendo del stream del cliente: {err}")
            }
            InternalError::StorageActorSend(err) => write!(
                f,
                "error enviando action por el canal del storage actor: {err}"
            ),
            InternalError::StreamWrite(err) => {
                write!(f, "error escribiendo al stream del cliente: {err}")
            }
        }
    }
}

impl From<SendError<Log>> for InternalError {
    fn from(err: SendError<Log>) -> Self {
        Self::LogSend(err)
    }
}

impl From<SendError<ClusterAction>> for InternalError {
    fn from(err: SendError<ClusterAction>) -> Self {
        Self::ClusterActorSend(err)
    }
}

impl From<pub_sub::InternalError> for InternalError {
    fn from(err: pub_sub::InternalError) -> Self {
        Self::PubSubBroker(err)
    }
}

impl From<SendError<PubSubEnvelope>> for InternalError {
    fn from(err: SendError<PubSubEnvelope>) -> Self {
        Self::PubSubBrokerSend(err)
    }
}

impl From<storage::InternalError> for InternalError {
    fn from(err: storage::InternalError) -> Self {
        Self::StorageActor(err)
    }
}

impl From<SendError<StorageAction>> for InternalError {
    fn from(err: SendError<StorageAction>) -> Self {
        Self::StorageActorSend(err)
    }
}
