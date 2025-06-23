use std::{fmt, io, sync::mpsc::SendError};

use log::Log;

#[derive(Debug)]
pub enum InternalError {
    ClusterPortBind(io::Error),
    LogSend(SendError<Log>),
    ReplicationLinkConnect(io::Error),
}

impl std::error::Error for InternalError {}

impl fmt::Display for InternalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            InternalError::ClusterPortBind(err) => {
                write!(f, "error asignando puerto para cluster stream: {err}")
            }
            InternalError::LogSend(err) => {
                write!(f, "error enviando mensaje de log por el canal: {err}")
            }
            InternalError::ReplicationLinkConnect(err) => {
                write!(f, "error conectando con replication link: {err}")
            }
        }
    }
}

impl From<SendError<Log>> for InternalError {
    fn from(err: SendError<Log>) -> Self {
        Self::LogSend(err)
    }
}
