use crate::error::Error;

pub const PREFIX: u8 = b'+';

#[derive(Debug, PartialEq)]
pub struct SimpleString(pub String);

impl From<&str> for SimpleString {
    fn from(content: &str) -> Self {
        Self(content.to_string())
    }
}

impl From<SimpleString> for String {
    fn from(ss: SimpleString) -> Self {
        ss.0
    }
}

impl TryFrom<&[u8]> for SimpleString {
    type Error = Error;

    fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
        let data_bytes = super::data_subslice(bytes, PREFIX)?;
        let data = String::from_utf8(data_bytes.to_vec()).map_err(|_| Error::InvalidEncoding)?;
        Ok(Self(data))
    }
}

impl From<SimpleString> for Vec<u8> {
    fn from(ss: SimpleString) -> Self {
        let resp_str = format!("+{}\r\n", ss.0);
        let bytes = resp_str.as_bytes();
        bytes.into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple_string_se_serializa_correctamente() {
        let ss = SimpleString::from("Hello, World!");

        let bytes: Vec<u8> = ss.into();

        assert_eq!(&bytes, b"+Hello, World!\r\n");
    }

    #[test]
    fn simple_string_se_deserializa_correctamente() {
        let bytes = "+Hello, World!\r\n".as_bytes();

        let ss = SimpleString::try_from(bytes).unwrap();

        assert_eq!(ss.0, "Hello, World!");
    }

    #[test]
    fn simple_string_vacio_se_deserializa_correctamente() {
        let bytes = "+\r\n".as_bytes();

        let ss = SimpleString::try_from(bytes).unwrap();

        assert_eq!(ss.0, "");
    }

    #[test]
    fn simple_string_con_chars_non_ascii_no_se_deserializa() {
        let bytes = "+4\r\n🦀\r\n".as_bytes();

        let err = SimpleString::try_from(bytes).unwrap_err();

        assert_eq!(err, Error::InvalidEncoding);
    }
}
