use std::io;

use crate::thread_pool;

#[derive(Debug)]
pub enum Error {
    Io(io::Error),
    Log(log::Error),
    ThreadPool(thread_pool::Error),
}

impl std::error::Error for Error {}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        todo!()
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

impl From<thread_pool::Error> for Error {
    fn from(err: thread_pool::Error) -> Self {
        Self::ThreadPool(err)
    }
}
