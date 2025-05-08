#[derive(Debug, PartialEq)]
pub enum Error {
    InvalidDataType,
    MissingCommand,
    CommandNotSupported,
    MissingArgument,
    RespError(resp::Error),
}

impl From<resp::Error> for Error {
    fn from(err: resp::Error) -> Self {
        Self::RespError(err)
    }
}
