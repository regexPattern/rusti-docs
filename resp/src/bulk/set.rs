use crate::DataType;
use crate::error::Error;

use super::Array;

pub const PREFIX: u8 = b'~';

#[derive(Debug, PartialEq)]
pub struct Set(Vec<DataType>);

impl TryFrom<&[u8]> for Set {
    type Error = Error;

    fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
        let (elements, _) = Array::parse_elements_recursive(bytes, PREFIX)?;
        Ok(Self::from(elements))
    }
}

impl From<Vec<DataType>> for Set {
    fn from(elements: Vec<DataType>) -> Self {
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

#[cfg(test)]
mod tests {
    use crate::{Array, DataType, Integer, SimpleString};

    use super::*;

    #[test]
    fn set_se_serializa_correctamente() {
        let s = Set(vec![
            DataType::SimpleString(SimpleString::from("Hello, World!")),
            DataType::Integer(Integer::from(1)),
        ]);

        let bytes: Vec<_> = s.into();

        assert_eq!(bytes, b"~2\r\n+Hello, World!\r\n:+1\r\n");
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
            DataType::Set(Set(vec![DataType::SimpleString(SimpleString::from(
                "Hello, World!",
            ))])),
            DataType::Integer(Integer::from(1)),
        ]);

        let bytes: Vec<_> = s.into();

        assert_eq!(bytes, b"~2\r\n~1\r\n+Hello, World!\r\n:+1\r\n");
    }

    #[test]
    fn set_con_array_anidado_se_serializa_correctamente() {
        let s = Set(vec![DataType::Array(Array::from(vec![
            DataType::SimpleString(SimpleString::from("Hello, World!")),
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
            Set(vec![DataType::Array(Array::from(vec![
                DataType::SimpleString(SimpleString::from("Hello, World!")),
            ]))])
        );
    }

    #[test]
    fn set_vacio_se_deserializa_correctamente() {
        let bytes = b"~0\r\n";

        let s = Set::try_from(bytes.as_slice()).unwrap();

        assert_eq!(s, Set(vec![]));
    }
}
