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
            Error::ValueNotAnInteger => SimpleError::from("ERROR value is not an integer"),
            Error::WrongNumberOfArgs => {
                SimpleError::from("ERROR wrong number of arguments for command")
            }
            Error::WrongType => SimpleError::from(
                "WRONGTYPE operation against a key holding the wrong kind of value",
            ),
        }
    }
}
