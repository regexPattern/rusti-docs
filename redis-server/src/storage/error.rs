use resp::SimpleError;

#[derive(Debug)]
pub enum Error {
    ValueNotAnInteger,
    WrongNumberOfArgs,
    WrongType,
}

impl std::error::Error for Error {}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}

impl From<Error> for SimpleError {
    fn from(err: Error) -> Self {
        match err {
            Error::ValueNotAnInteger => SimpleError {
                prefix: "ERROR".to_string(),
                msg: Some("value is not an integer".to_string()), // TODO: aca realmente no cambio
                                                                  // tengo prefix
            },
            Error::WrongNumberOfArgs => SimpleError {
                prefix: "ERROR".to_string(),
                msg: Some("wrong number of arguments for command".to_string()),
            },
            Error::WrongType => SimpleError {
                prefix: "WRONGTYPE".to_string(),
                msg: Some("operation against a key holding the wrong kind of value".to_string()),
            },
        }
    }
}
