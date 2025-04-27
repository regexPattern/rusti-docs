use crate::error::Error;

pub const PREFIX: u8 = b'-';

#[derive(Debug, PartialEq)]
pub struct SimpleError {
    pub prefix: String,
    pub msg: Option<String>,
}

impl TryFrom<&[u8]> for SimpleError {
    type Error = Error;

    fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
        let data_bytes = super::data_subslice(bytes, PREFIX)?;

        let data = String::from_utf8(data_bytes.to_vec()).map_err(|_| Error::InvalidEncoding)?;

        let mut components = data.split(' ');

        match (components.next(), components.next()) {
            (Some(prefix), msg) => Ok(Self {
                prefix: prefix.to_string(),
                msg: msg.map(|m| m.to_string()),
            }),
            _ => Err(Error::EmptyPayload),
        }
    }
}

impl From<SimpleError> for Vec<u8> {
    fn from(se: SimpleError) -> Self {
        if let Some(msg) = se.msg {
            format!("-{} {}\r\n", se.prefix, msg).into()
        } else {
            format!("-{}\r\n", se.prefix).into()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple_error_con_mensaje_se_serializa_correctamente() {
        let se = SimpleError {
            prefix: "ERR".to_string(),
            msg: Some("message".to_string()),
        };

        let bytes: Vec<_> = se.into();

        assert_eq!(&bytes, b"-ERR message\r\n");
    }

    #[test]
    fn simple_error_sin_mensaje_se_serializa_correctamente() {
        let se: SimpleError = SimpleError {
            prefix: "ERR".to_string(),
            msg: None,
        };

        let bytes: Vec<_> = se.into();

        assert_eq!(&bytes, b"-ERR\r\n");
    }

    #[test]
    fn simple_error_con_mensaje_se_deserializa_correctamente() {
        let bytes = "-ERR message\r\n".as_bytes();

        let se = SimpleError::try_from(bytes).unwrap();

        assert_eq!(se.prefix, "ERR");
        assert_eq!(se.msg.unwrap(), "message");
    }

    #[test]
    fn simple_error_sin_mensaje_se_deserializa_correctamente() {
        let bytes = "-ERR\r\n".as_bytes();

        let se = SimpleError::try_from(bytes).unwrap();

        assert_eq!(se.prefix, "ERR");
        assert_eq!(se.msg, None);
    }
}
