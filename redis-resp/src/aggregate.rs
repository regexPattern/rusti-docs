pub mod array;
pub mod bulk_string;
pub mod map;
pub mod set;

use std::slice::Windows;

pub use array::Array;
pub use bulk_string::BulkString;
pub use map::Map;
pub use set::Set;

use crate::Error;

#[derive(Debug)]
struct ContentLength<'b> {
    length: usize,
    end_idx: usize,
    bytes_windows_rest: Windows<'b, u8>,
}

fn content_length(bytes: &[u8], prefix: u8) -> Result<ContentLength<'_>, Error> {
    if bytes.is_empty() {
        return Err(Error::EmptyPayload);
    } else if bytes[0] != prefix {
        return Err(Error::WrongPrefix);
    }

    let mut bytes_windows = bytes.windows(2);

    let end_idx = bytes_windows
        .position(|w| w == b"\r\n")
        .ok_or(Error::MissingTerminator)?;

    let length = std::str::from_utf8(&bytes[1..end_idx])
        .map_err(|_| Error::InvalidEncoding)?
        .parse()
        .map_err(|_| Error::InvalidBulkLength)?;

    let bl = ContentLength {
        length,
        end_idx,
        bytes_windows_rest: bytes_windows,
    };

    Ok(bl)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bulk_length_retorna_length_correctamente() {
        let bytes = "$123\r\n".as_bytes();

        let bl = content_length(bytes, b'$').unwrap();

        assert_eq!(bl.length, 123);
    }

    #[test]
    fn bulk_length_retorna_indice_de_fin_de_length_correctamente() {
        let bytes = "$123\r\n".as_bytes();

        let bl = content_length(bytes, b'$').unwrap();

        assert_eq!(bl.end_idx, 4);
    }

    #[test]
    fn bulk_length_retorna_error_con_slice_vacio() {
        let bytes = "".as_bytes();

        let err = content_length(bytes, b'$').unwrap_err();

        assert_eq!(err, Error::EmptyPayload);
    }

    #[test]
    fn bulk_length_retorna_error_con_length_invalido() {
        let bytes = "$@1X\r\n".as_bytes();

        let err = content_length(bytes, b'$').unwrap_err();

        assert_eq!(err, Error::InvalidBulkLength);
    }
}
