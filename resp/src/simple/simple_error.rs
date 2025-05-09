use crate::error::Error;

pub const PREFIX: u8 = b'-';

#[derive(Debug, PartialEq, Clone)]
pub struct SimpleError(String);

impl TryFrom<&[u8]> for SimpleError {
    type Error = Error;

    fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
        let data_bytes = super::data_subslice(bytes, PREFIX)?;
        let data = String::from_utf8(data_bytes.to_vec()).map_err(|_| Error::InvalidEncoding)?;
        Ok(Self(data))
    }
}

impl From<SimpleError> for Vec<u8> {
    fn from(se: SimpleError) -> Self {
        format!("{}{}\r\n", PREFIX as char, se.0).into_bytes()
    }
}

impl From<&str> for SimpleError {
    fn from(message: &str) -> Self {
        Self(message.to_string())
    }
}

impl From<String> for SimpleError {
    fn from(message: String) -> Self {
        Self(message)
    }
}

impl From<&String> for SimpleError {
    fn from(message: &String) -> Self {
        Self(message.to_string())
    }
}

impl From<SimpleError> for String {
    fn from(se: SimpleError) -> Self {
        se.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple_error_se_serializa_correctamente() {
        let se = SimpleError::from("ERR message");

        let bytes: Vec<_> = se.into();

        assert_eq!(&bytes, b"-ERR message\r\n");
    }

    #[test]
    fn simple_error_se_deserializa_correctamente() {
        let bytes = "-ERR message\r\n".as_bytes();

        let se = SimpleError::try_from(bytes).unwrap();

        assert_eq!(se.0, "ERR message");
    }
}
