use std::{fmt, io};

use crate::thread_pool;

use super::node;

#[derive(Debug)]
pub enum InternalError {
    AddrBind(io::Error),
    LogFileOpen(io::Error),
    LogFileWrite(io::Error),
    Node(node::InternalError),
    ThreadPool(thread_pool::Error),
}

impl std::error::Error for InternalError {}

impl fmt::Display for InternalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            InternalError::AddrBind(err) => {
                write!(f, "error abriendo conexión del servidor: {err}")
            }
            InternalError::LogFileOpen(err) => write!(f, "error abriendo archivo de logs: {err}"),
            InternalError::LogFileWrite(err) => {
                write!(f, "error escribiendo al archivo de logs: {err}")
            }
            InternalError::Node(err) => write!(f, "{err}"),
            InternalError::ThreadPool(err) => write!(f, "{err}"),
        }
    }
}

impl From<node::InternalError> for InternalError {
    fn from(err: node::InternalError) -> Self {
        Self::Node(err)
    }
}

impl From<thread_pool::Error> for InternalError {
    fn from(err: thread_pool::Error) -> Self {
        Self::ThreadPool(err)
    }
}
