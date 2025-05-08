use crate::RespDataType;
use crate::error::Error;

use super::{BulkString, ContentLength, Map, Set, bulk_string, map, set};

pub const PREFIX: u8 = b'*';

#[derive(Debug, PartialEq, Clone)]
pub struct Array(Vec<RespDataType>);

impl Array {
    pub fn parse_elements_recursive(
        bytes: &[u8],
        prefix: u8,
    ) -> Result<(Vec<RespDataType>, usize), Error> {
        let ContentLength {
            mut length,
            end_idx: length_end_idx,
            bytes_windows_rest: _,
        } = super::content_length(bytes, prefix)?;

        let mut bytes_idx = length_end_idx + 2;

        if prefix == map::PREFIX {
            length *= 2;
        }
        let mut elements = Vec::with_capacity(length);

        for _ in 0..length {
            if bytes_idx >= bytes.len() {
                return Err(Error::WrongBulkLength);
            }

            let (element, offset) = Self::parse_element(&bytes[bytes_idx..])?;
            elements.push(element);
            bytes_idx += offset;
        }

        Ok((elements, bytes_idx))
    }

    pub fn parse_simple_element(bytes: &[u8]) -> Result<(RespDataType, usize), Error> {
        let offset = bytes
            .windows(2)
            .position(|w| w == b"\r\n")
            .ok_or(Error::MissingTerminator)?
            + 2;

        Ok((RespDataType::try_from(&bytes[..offset])?, offset))
    }

    fn parse_bulk_string(bytes: &[u8]) -> Result<(RespDataType, usize), Error> {
        let mut bytes_windows = bytes.windows(2);

        let length_end_idx = bytes_windows
            .position(|w| w == b"\r\n")
            .ok_or(Error::MissingTerminator)?;

        bytes_windows.next();

        let content_end_idx = bytes_windows
            .position(|w| w == b"\r\n")
            .ok_or(Error::MissingTerminator)?;

        let offset = (length_end_idx + 2) + (content_end_idx + 2);

        Ok((RespDataType::try_from(&bytes[..offset])?, offset))
    }

    fn parse_element(bytes: &[u8]) -> Result<(RespDataType, usize), Error> {
        match bytes.first() {
            Some(&bulk_string::PREFIX) => Self::parse_bulk_string(bytes),
            Some(&map::PREFIX) | Some(&set::PREFIX) | Some(&PREFIX) => Self::parse_nested(bytes),
            Some(_) => Self::parse_simple_element(bytes),
            None => Err(Error::WrongBulkLength),
        }
    }

    fn parse_nested(bytes: &[u8]) -> Result<(RespDataType, usize), Error> {
        let prefix = bytes.first().ok_or(Error::WrongBulkLength)?;
        let (inner, offset) = Self::parse_elements_recursive(bytes, *prefix)?;
        let element = match *prefix {
            map::PREFIX => RespDataType::Map(Map::from(inner)),
            set::PREFIX => RespDataType::Set(Set::from(inner)),
            PREFIX => RespDataType::Array(Self(inner)),
            _ => unreachable!(),
        };
        Ok((element, offset))
    }
}

impl TryFrom<&[u8]> for Array {
    type Error = Error;

    fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
        let (elements, _) = Array::parse_elements_recursive(bytes, PREFIX)?;
        Ok(Self(elements))
    }
}

impl From<Array> for Vec<u8> {
    fn from(arr: Array) -> Self {
        let mut result = Vec::from(format!("{}{}\r\n", PREFIX as char, arr.0.len()));
        for element in arr.0 {
            result.extend(Vec::from(element));
        }
        result
    }
}

impl Array {
    pub fn to_resp_vec(col: &[&BulkString]) -> Vec<u8> {
        let mut result = Vec::from(format!("{}{}\r\n", PREFIX as char, col.len()));
        for element in col {
            result.extend(Vec::from(*element));
        }
        result
    }
}

impl From<Vec<RespDataType>> for Array {
    fn from(elements: Vec<RespDataType>) -> Self {
        Self(elements)
    }
}

impl From<Array> for Vec<RespDataType> {
    fn from(arr: Array) -> Self {
        arr.0
    }
}

#[cfg(test)]
mod tests {
    use crate::{BulkString, Integer, SimpleError, SimpleString};

    use super::*;

    #[test]
    fn array_de_un_solo_tipo_se_serializa_correctamente() {
        let arr = Array(vec![
            RespDataType::BulkString(BulkString::from("hello")),
            RespDataType::BulkString(BulkString::from("world")),
        ]);

        let bytes: Vec<u8> = arr.into();

        let esperado = b"*2\r\n$5\r\nhello\r\n$5\r\nworld\r\n";

        assert_eq!(&bytes, esperado);
    }

    #[test]
    fn array_de_multiples_tipos_se_serializa_correctamente() {
        let arr = Array(vec![
            RespDataType::Integer(Integer::from(-6)),
            RespDataType::SimpleError(SimpleError {
                prefix: "ERROR".to_string(),
                msg: None,
            }),
            RespDataType::BulkString(BulkString::from("Hello, World!")),
        ]);

        let bytes: Vec<u8> = arr.into();

        let esperado = b"*3\r\n:-6\r\n-ERROR\r\n$13\r\nHello, World!\r\n";

        assert_eq!(&bytes, esperado);
    }

    #[test]
    fn array_vacio_se_serializa_correctamente() {
        let arr = Array(vec![]);

        let bytes: Vec<u8> = arr.into();

        let esperado = b"*0\r\n";

        assert_eq!(&bytes, esperado);
    }

    #[test]
    fn arrays_anidados_se_serializan_correctamente() {
        let arr = Array(vec![
            RespDataType::Array(Array(vec![
                RespDataType::Integer(Integer::from(1)),
                RespDataType::Integer(Integer::from(2)),
                RespDataType::Integer(Integer::from(-1)),
            ])),
            RespDataType::Array(Array(vec![
                RespDataType::SimpleString(SimpleString::from("Hello")),
                RespDataType::SimpleError(SimpleError {
                    prefix: "World".to_string(),
                    msg: None,
                }),
            ])),
        ]);

        let bytes: Vec<u8> = arr.into();

        let inner_1 = "*3\r\n:+1\r\n:+2\r\n:-1\r\n";
        let inner_2 = "*2\r\n+Hello\r\n-World\r\n";
        let outer = format!("*2\r\n{inner_1}{inner_2}").into_bytes();

        assert_eq!(bytes, outer);
    }

    #[test]
    fn array_de_un_solo_tipo_se_deserializa_correctamente() {
        let bytes = "*3\r\n:1\r\n:2\r\n:3\r\n".as_bytes();

        let arr = Array::try_from(bytes).unwrap();

        assert_eq!(
            arr.0,
            [
                RespDataType::Integer(Integer::from(1)),
                RespDataType::Integer(Integer::from(2)),
                RespDataType::Integer(Integer::from(3))
            ]
        );
    }

    #[test]
    fn array_de_bulk_string_se_deserializa_correctamente() {
        let bytes = "*1\r\n$7\r\nABC\rDEF\r\n".as_bytes();

        let arr = Array::try_from(bytes).unwrap();

        assert_eq!(
            arr.0,
            [RespDataType::BulkString(BulkString::from("ABC\rDEF")),]
        );
    }

    #[test]
    fn array_sin_elementos_se_deserializa_correctamente() {
        let bytes = "*0\r\n".as_bytes();

        let arr = Array::try_from(bytes).unwrap();

        assert_eq!(arr.0, []);
    }

    #[test]
    fn array_de_multiples_tipos_se_deserializa_correctamente() {
        let bytes = "*3\r\n:-6\r\n-ERROR\r\n$13\r\nHello, World!\r\n".as_bytes();

        let arr = Array::try_from(bytes).unwrap();

        assert_eq!(
            arr.0,
            [
                RespDataType::Integer(Integer::from(-6)),
                RespDataType::SimpleError(SimpleError {
                    prefix: "ERROR".to_string(),
                    msg: None
                }),
                RespDataType::BulkString(BulkString::from("Hello, World!")),
            ]
        );
    }

    #[test]
    fn array_con_null_type_se_serializa_correctamente() {
        let arr = Array(vec![
            RespDataType::Integer(Integer::from(42)),
            RespDataType::Null,
            RespDataType::BulkString(BulkString::from("hello")),
        ]);

        let bytes: Vec<u8> = arr.into();
        let esperado = b"*3\r\n:+42\r\n_\r\n$5\r\nhello\r\n";
        assert_eq!(&bytes, esperado);
    }

    #[test]
    fn array_con_null_type_se_deserializa_correctamente() {
        let bytes = b"*3\r\n:42\r\n_\r\n$5\r\nhello\r\n";

        let arr = Array::try_from(bytes.as_ref()).unwrap();

        assert_eq!(
            arr.0,
            [
                RespDataType::Integer(Integer::from(42)),
                RespDataType::Null,
                RespDataType::BulkString(BulkString::from("hello")),
            ]
        );
    }

    #[test]
    fn array_con_length_invalido_no_se_deserializa() {
        let bytes = "$3\r\nA\r\n".as_bytes();

        let err = BulkString::try_from(bytes).unwrap_err();

        assert_eq!(err, Error::WrongBulkLength);
    }

    #[test]
    fn arrays_anidados_se_deserializan_correctamente() {
        let inner_1 = "*3\r\n:+1\r\n:+2\r\n:-1\r\n";
        let inner_2 = "*2\r\n+Hello\r\n-World\r\n";
        let outer = format!("*2\r\n{inner_1}{inner_2}");

        let arr = Array::try_from(outer.as_bytes()).unwrap();

        assert_eq!(
            arr.0,
            [
                RespDataType::Array(Array(vec![
                    RespDataType::Integer(Integer::from(1)),
                    RespDataType::Integer(Integer::from(2)),
                    RespDataType::Integer(Integer::from(-1)),
                ])),
                RespDataType::Array(Array(vec![
                    RespDataType::SimpleString(SimpleString::from("Hello")),
                    RespDataType::SimpleError(SimpleError {
                        prefix: "World".to_string(),
                        msg: None,
                    }),
                ])),
            ]
        );
    }

    #[test]
    fn parse_simple_elemento_retorna_cantidad_de_bytes_correctamente() {
        let bytes = b"+Hello, World!\r\n";

        let (elem, _) = Array::parse_simple_element(bytes).unwrap();

        assert_eq!(
            elem,
            RespDataType::SimpleString(SimpleString::from("Hello, World!"))
        );
    }

    #[test]
    fn parse_simple_element_retorna_error_si_no_encuentra_fin_del_elemento() {
        let bytes = b"+Hello, World!";

        let err = Array::parse_bulk_string(bytes).unwrap_err();

        assert_eq!(err, Error::MissingTerminator);
    }

    #[test]
    fn parse_bulk_string_delega_serializacion_correctamente() {
        let bytes = b"$13\r\nHello, World!\r\n";

        let (bs, _) = Array::parse_bulk_string(bytes).unwrap();

        assert_eq!(
            bs,
            RespDataType::BulkString(BulkString::from("Hello, World!"))
        );
    }

    #[test]
    fn parse_bulk_string_retorna_cantidad_de_bytes_correctamente() {
        let bytes = b"$13\r\nHello, World!\r\n";

        let (_, offset) = Array::parse_bulk_string(bytes).unwrap();

        assert_eq!(offset, bytes.len());
    }

    #[test]
    fn parse_bulk_string_retorna_error_si_no_encuentra_fin_del_bulk_string() {
        let bytes = b"$13\r\nHello, World!";

        let err = Array::parse_bulk_string(bytes).unwrap_err();

        assert_eq!(err, Error::MissingTerminator);
    }

    #[test]
    fn parse_elements_recursive_con_map_anidado() {
        let arr = Array(vec![RespDataType::Map(Map::from(vec![
            RespDataType::SimpleString(SimpleString::from("first")),
            RespDataType::Integer(Integer::from(1)),
        ]))]);

        let bytes: Vec<u8> = arr.into();

        let arr = Array::try_from(bytes.as_slice()).unwrap();

        assert_eq!(
            arr,
            Array(vec![RespDataType::Map(Map::from(vec![
                RespDataType::SimpleString(SimpleString::from("first")),
                RespDataType::Integer(Integer::from(1)),
            ]))])
        );
    }

    #[test]
    fn parse_elements_recursive_con_set_anidado() {
        let arr = Array(vec![RespDataType::Set(Set::from(vec![
            RespDataType::SimpleString(SimpleString::from("Hello, World!")),
            RespDataType::Integer(Integer::from(1)),
        ]))]);

        let bytes: Vec<u8> = arr.into();

        let arr = Array::try_from(bytes.as_slice()).unwrap();

        assert_eq!(
            arr,
            Array(vec![RespDataType::Set(Set::from(vec![
                RespDataType::SimpleString(SimpleString::from("Hello, World!")),
                RespDataType::Integer(Integer::from(1)),
            ]))])
        );
    }

    #[test]
    fn se_serializa_referencia_a_slice_de_bulk_strings_como_array() {
        let arr_ref = &[
            &BulkString::from("first"),
            &BulkString::from("second"),
            &BulkString::from("third"),
        ];

        let arr_owned = Array(vec![
            BulkString::from("first").into(),
            BulkString::from("second").into(),
            BulkString::from("third").into(),
        ]);

        let ref_bytes = Array::to_resp_vec(arr_ref);
        let owned_bytes = Vec::from(arr_owned);

        assert_eq!(ref_bytes, owned_bytes);
    }
}
