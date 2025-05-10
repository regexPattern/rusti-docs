use crate::error::Error;

pub const PREFIX: u8 = b':';

#[derive(Debug, PartialEq, Clone)]
pub struct Integer(i64);

impl From<i64> for Integer {
    fn from(value: i64) -> Self {
        Self(value)
    }
}

impl TryFrom<&[u8]> for Integer {
    type Error = Error;

    fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
        let data_bytes = super::data_subslice(bytes, PREFIX)?;

        let data = String::from_utf8(data_bytes.to_vec())
            .map_err(|_| Error::InvalidEncoding)?
            .parse()
            .map_err(|_| Error::InvalidEncoding)?;

        Ok(Self(data))
    }
}

impl From<Integer> for Vec<u8> {
    fn from(i: Integer) -> Self {
        format!("{}{}\r\n", PREFIX as char, i.0).into_bytes()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn integer_positivo_se_serializa_correctamente() {
        let i = Integer::from(42);

        let bytes: Vec<u8> = i.into();

        assert_eq!(bytes, b":42\r\n");
    }

    #[test]
    fn integer_negativo_se_serializa_correctamente() {
        let i = Integer::from(-42);

        let bytes: Vec<u8> = i.into();

        assert_eq!(bytes, b":-42\r\n");
    }

    #[test]
    fn integer_se_serializa_cero_correctamente() {
        let i = Integer::from(0);

        let bytes: Vec<u8> = i.into();

        assert_eq!(bytes, b":0\r\n");
    }

    #[test]
    fn integer_se_deserializa_correctamente() {
        let bytes = ":42\r\n".as_bytes();

        let integer = Integer::try_from(bytes).unwrap();

        assert_eq!(integer, Integer::from(42));
    }

    #[test]
    fn integer_sin_numero_no_se_deserializa() {
        let bytes = ":\r\n".as_bytes();

        let err = Integer::try_from(bytes).unwrap_err();

        assert_eq!(err, Error::InvalidEncoding);
    }

    #[test]
    fn integer_con_signo_positivo_se_deserializa_correctamente() {
        let bytes = ":+42\r\n";

        let i = Integer::try_from(bytes.as_bytes()).unwrap();

        assert_eq!(i, Integer::from(42));
    }

    #[test]
    fn integer_con_signo_negativo_se_deserializa_correctamente() {
        let bytes = ":-42\r\n".as_bytes();

        let i = Integer::try_from(bytes).unwrap();

        assert_eq!(i, Integer::from(-42));
    }

    #[test]
    fn integer_invalido_no_se_deserializa() {
        let bytes = ":12a3\r\n".as_bytes();

        let err = Integer::try_from(bytes).unwrap_err();

        assert_eq!(err, Error::InvalidEncoding);
    }
}
