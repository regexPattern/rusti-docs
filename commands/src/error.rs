use std::fmt;

use resp::SimpleError;

#[derive(Debug, PartialEq)]
pub enum Error {
    InvalidDataType,
    MissingCommand,
    CommandNotSupported,
    MissingArgument,
    RespError(resp::Error),
}

impl std::error::Error for Error {}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::InvalidDataType => write!(f, "tipo de dato inválido"),
            Error::MissingCommand => write!(f, "ningún comando encontrado"),
            Error::CommandNotSupported => write!(f, "comando no soportado"),
            Error::MissingArgument => write!(f, "falta argumento de comando"),
            Error::RespError(err) => write!(f, "error de protocolo RESP: {err}"),
        }
    }
}

impl From<resp::Error> for Error {
    fn from(err: resp::Error) -> Self {
        Self::RespError(err)
    }
}

impl From<Error> for SimpleError {
    fn from(err: Error) -> Self {
        SimpleError::from(format!("CMDERROR {err}"))
    }
}
