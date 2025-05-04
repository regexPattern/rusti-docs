use std::io;

use crate::thread_pool;

#[derive(Debug)]
pub enum Error {
    ThreadPool(thread_pool::Error),
    Io(io::Error),
    LogSend, // `SendError` para el canal de logs.
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Self {
        Self::Io(err)
    }
}

impl From<thread_pool::Error> for Error {
    fn from(err: thread_pool::Error) -> Self {
        Self::ThreadPool(err)
    }
}
