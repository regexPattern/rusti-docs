use crate::Error;

use super::BulkLength;

pub const PREFIX: u8 = b'$';

#[derive(Debug, PartialEq)]
pub struct BulkString(String);

impl From<&str> for BulkString {
    fn from(content: &str) -> Self {
        Self(content.to_string())
    }
}

impl From<BulkString> for String {
    fn from(bs: BulkString) -> Self {
        bs.0
    }
}

impl TryFrom<&[u8]> for BulkString {
    type Error = Error;

    fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
        let BulkLength {
            length,
            end_idx: length_end_idx,
            bytes_windows_rest: mut bytes_windows,
        } = super::bulk_length(bytes, PREFIX)?;

        bytes_windows.next();

        let content_end_idx = bytes_windows
            .position(|w| w == b"\r\n")
            .ok_or(Error::MissingTerminator)?;

        let content_start_idx = length_end_idx + 2;
        let content_end_idx = content_start_idx + content_end_idx;

        let data_bytes = &bytes[content_start_idx..content_end_idx];
        if data_bytes.len() != length {
            return Err(Error::WrongBulkLength);
        }

        let data = String::from_utf8(data_bytes.to_vec()).map_err(|_| Error::InvalidEncoding)?;

        Ok(Self(data))
    }
}

impl From<BulkString> for Vec<u8> {
    fn from(bs: BulkString) -> Self {
        let resp_str = format!("${}\r\n{}\r\n", bs.0.len(), bs.0);
        let bytes = resp_str.as_bytes();
        bytes.into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bulk_string_con_chars_ascii_se_serializa_correctamente() {
        let ss = BulkString::from("ABCDE");

        let bytes: Vec<u8> = ss.into();

        assert_eq!(&bytes, b"$5\r\nABCDE\r\n");
    }

    #[test]
    fn bulk_string_con_chars_utf8_se_serializa_correctamente() {
        let ss = BulkString::from("🦀");

        let bytes: Vec<u8> = ss.into();

        assert_eq!(&bytes, "$4\r\n🦀\r\n".as_bytes());
    }

    #[test]
    fn bulk_string_con_chars_ascii_se_deserializa_correctamente() {
        let bytes = "$3\r\nABC\r\n".as_bytes();

        let bs = BulkString::try_from(bytes).unwrap();

        assert_eq!(bs.0, "ABC");
    }

    #[test]
    fn bulk_string_con_chars_utf8_se_deserializa_correctamente() {
        let bytes = "$4\r\n🦀\r\n".as_bytes();

        let bs = BulkString::try_from(bytes).unwrap();

        assert_eq!(bs.0, "🦀");
        assert_eq!(bs.0.len(), 4);
    }

    #[test]
    fn bulk_string_con_newlines_se_deserializa_correctamente() {
        let bytes = "$7\r\nABC\nDEF\r\n".as_bytes();

        let bs = BulkString::try_from(bytes).unwrap();

        assert_eq!(bs.0, "ABC\nDEF");
    }

    #[test]
    fn bulk_string_con_cr_se_deserializa_correctamente() {
        let bytes = "$7\r\nABC\rDEF\r\n".as_bytes();

        let bs = BulkString::try_from(bytes).unwrap();

        assert_eq!(bs.0, "ABC\rDEF");
    }

    #[test]
    fn bulk_string_sin_termination_al_final_no_se_deserializa() {
        let bytes = "$3\r\nABC".as_bytes();

        let err = BulkString::try_from(bytes).unwrap_err();

        assert_eq!(err, Error::MissingTerminator);
    }

    #[test]
    fn bulk_string_con_length_incorrecta_no_se_deserializa() {
        let bytes = "$1\r\nABC\r\n".as_bytes();

        let err = BulkString::try_from(bytes).unwrap_err();

        assert_eq!(err, Error::WrongBulkLength);
    }

    #[test]
    fn bulk_string_con_chars_utf8_invalidos_no_se_deserializa() {
        let bytes = [
            b'$', b'4', b'\r', b'\n', 0xF4, 0x90, 0x80, 0x80, b'\r', b'\n',
        ];

        let err = BulkString::try_from(bytes.as_slice()).unwrap_err();

        assert_eq!(err, Error::InvalidEncoding);
    }
}
