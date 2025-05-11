pub mod boolean;
pub mod double;
pub mod integer;
pub mod null;
pub mod simple_error;
pub mod simple_string;

pub use boolean::Boolean;
pub use double::Double;
pub use integer::Integer;
pub use simple_error::SimpleError;
pub use simple_string::SimpleString;

use crate::Error;

fn data_subslice(bytes: &[u8], prefix: u8) -> Result<&[u8], Error> {
    if bytes.is_empty() {
        return Err(Error::EmptyPayload);
    } else if bytes[0] != prefix {
        return Err(Error::WrongPrefix);
    } else if !bytes.ends_with(b"\r\n") {
        return Err(Error::MissingTerminator);
    } else if !bytes.is_ascii() {
        return Err(Error::InvalidEncoding);
    }

    Ok(&bytes[1..bytes.len() - 2])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn data_subslice_son_invalidos_si_estan_vacios() {
        let bytes = "".as_bytes();

        let err = data_subslice(bytes, b'+').unwrap_err();

        assert_eq!(err, Error::EmptyPayload);
    }

    #[test]
    fn data_subslice_son_invalidos_si_tienen_prefijo_incorrecto() {
        let bytes = "$".as_bytes();

        let err = data_subslice(bytes, b'+').unwrap_err();

        assert_eq!(err, Error::WrongPrefix);
    }

    #[test]
    fn data_subslice_son_invalidos_si_no_son_ascii() {
        let bytes = "+🦀\r\n".as_bytes();

        let err = data_subslice(bytes, b'+').unwrap_err();

        assert_eq!(err, Error::InvalidEncoding);
    }

    #[test]
    fn data_subslice_retorna_subslice_de_data() {
        let bytes = "+Hello, World!\r\n".as_bytes();

        let data_bytes = data_subslice(bytes, b'+').unwrap();

        assert_eq!(data_bytes, b"Hello, World!");
    }
}
