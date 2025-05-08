#[derive(Debug)]
pub enum Error {
    Log(log::Error),
}

impl std::error::Error for Error {}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "node-error")
    }
}

impl From<log::Error> for Error {
    fn from(err: log::Error) -> Self {
        Self::Log(err)
    }
}
