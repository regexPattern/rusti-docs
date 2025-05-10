use crate::RespDataType;
use crate::error::Error;

use super::Array;

pub const PREFIX: u8 = b'~';

#[derive(Debug, PartialEq, Clone)]
pub struct Set(Vec<RespDataType>);

impl TryFrom<&[u8]> for Set {
    type Error = Error;

    fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
        let (elements, _) = Array::parse_elements_recursive(bytes, PREFIX)?;
        Ok(Self::from(elements))
    }
}

impl From<Vec<RespDataType>> for Set {
    fn from(elements: Vec<RespDataType>) -> Self {
        Self(elements)
    }
}

impl From<Set> for Vec<u8> {
    fn from(s: Set) -> Self {
        let mut result = Vec::from(format!("{}{}\r\n", PREFIX as char, s.0.len()));
        for element in s.0 {
            result.extend(Vec::from(element));
        }
        result
    }
}

impl Set {
    pub fn to_resp_vec(col: &std::collections::HashSet<super::BulkString>) -> Vec<u8> {
        let mut result = Vec::from(format!("{}{}\r\n", PREFIX as char, col.len()));
        for element in col {
            result.extend(Vec::from(element));
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use crate::{Array, BulkString, Integer, RespDataType, SimpleString};

    use super::*;

    #[test]
    fn set_se_serializa_correctamente() {
        let s = Set(vec![
            RespDataType::SimpleString(SimpleString::from("Hello, World!")),
            RespDataType::Integer(Integer::from(1)),
        ]);

        let bytes: Vec<_> = s.into();

        assert_eq!(bytes, b"~2\r\n+Hello, World!\r\n:1\r\n");
    }

    #[test]
    fn set_vacio_se_serializa_correctamente() {
        let s = Set(vec![]);

        let bytes: Vec<_> = s.into();

        assert_eq!(bytes, b"~0\r\n");
    }

    #[test]
    fn set_con_set_anidado_se_serializa_correctamente() {
        let s = Set(vec![
            RespDataType::Set(Set(vec![RespDataType::SimpleString(SimpleString::from(
                "Hello, World!",
            ))])),
            RespDataType::Integer(Integer::from(1)),
        ]);

        let bytes: Vec<_> = s.into();

        assert_eq!(bytes, b"~2\r\n~1\r\n+Hello, World!\r\n:1\r\n");
    }

    #[test]
    fn set_con_array_anidado_se_serializa_correctamente() {
        let s = Set(vec![RespDataType::Array(Array::from(vec![
            RespDataType::SimpleString(SimpleString::from("Hello, World!")),
        ]))]);

        let bytes: Vec<u8> = s.into();

        let esperado = b"~1\r\n*1\r\n+Hello, World!\r\n";

        assert_eq!(&bytes, esperado);
    }

    #[test]
    fn set_con_array_anidado_se_deserializa_correctamente() {
        let bytes = b"~1\r\n*1\r\n+Hello, World!\r\n";

        let s = Set::try_from(bytes.as_slice()).unwrap();

        assert_eq!(
            s,
            Set(vec![RespDataType::Array(Array::from(vec![
                RespDataType::SimpleString(SimpleString::from("Hello, World!")),
            ]))])
        );
    }

    #[test]
    fn set_vacio_se_deserializa_correctamente() {
        let bytes = b"~0\r\n";

        let s = Set::try_from(bytes.as_slice()).unwrap();

        assert_eq!(s, Set(vec![]));
    }

    #[test]
    fn se_serializa_referencia_a_hashset_de_bulk_strings_como_set() {
        let set_ref = &HashSet::from([BulkString::from("first")]);

        let set_owned = Set(vec![BulkString::from("first").into()]);

        let ref_bytes = Set::to_resp_vec(set_ref);
        let owned_bytes = Vec::from(set_owned);

        assert_eq!(ref_bytes, owned_bytes);
    }
}
