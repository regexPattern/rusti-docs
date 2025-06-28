use std::fmt;

#[derive(Debug)]
pub enum Error {
    SendError(String),
    NoSender,
}

impl std::error::Error for Error {}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let err = match self {
            Error::SendError(err) => err,
            Error::NoSender => &"worker no tiene sender para enviar tarea".to_string(),
        };

        write!(f, "thread-pool error: {err}")
    }
}
