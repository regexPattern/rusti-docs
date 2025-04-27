use crate::error::Error;

pub const PREFIX: u8 = b'#';

#[derive(Debug, PartialEq, Clone)]
pub struct Boolean(bool);

impl TryFrom<&[u8]> for Boolean {
    type Error = Error;

    fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
        let data = super::data_subslice(bytes, PREFIX)?;
        match data {
            b"t" => Ok(Self(true)),
            b"f" => Ok(Self(false)),
            _ => Err(Error::InvalidEncoding),
        }
    }
}

impl From<Boolean> for Vec<u8> {
    fn from(b: Boolean) -> Self {
        let value = if b.0 { "t" } else { "f" };
        format!("{}{}\r\n", PREFIX as char, value).into_bytes()
    }
}

impl From<bool> for Boolean {
    fn from(b: bool) -> Self {
        Self(b)
    }
}

impl From<Boolean> for bool {
    fn from(b: Boolean) -> Self {
        b.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn boolean_true_se_serializa_correctamente() {
        let b = Boolean(true);
        let bytes: Vec<u8> = b.into();
        assert_eq!(&bytes, b"#t\r\n");
    }

    #[test]
    fn boolean_false_se_serializa_correctamente() {
        let b = Boolean(false);
        let bytes: Vec<u8> = b.into();
        assert_eq!(&bytes, b"#f\r\n");
    }

    #[test]
    fn boolean_true_se_deserializa_correctamente() {
        let bytes = "#t\r\n".as_bytes();
        let b = Boolean::try_from(bytes).unwrap();
        assert_eq!(b, Boolean(true));
    }

    #[test]
    fn boolean_false_se_deserializa_correctamente() {
        let bytes = "#f\r\n".as_bytes();
        let b = Boolean::try_from(bytes).unwrap();
        assert_eq!(b, Boolean(false));
    }

    #[test]
    fn boolean_con_formato_invalido_falla() {
        let bytes = "#x\r\n".as_bytes();
        let err = Boolean::try_from(bytes).unwrap_err();
        assert_eq!(err, Error::InvalidEncoding);
    }
}
