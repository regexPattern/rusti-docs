use crate::{DataType, Error};

use super::Array;

pub const PREFIX: u8 = b'%';

#[derive(Debug, PartialEq)]
pub struct Map(Vec<(DataType, DataType)>);

impl TryFrom<&[u8]> for Map {
    type Error = Error;

    fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
        let (elements, _) = Array::parse_elements_recursive(bytes, PREFIX)?;
        Ok(Self::from(elements))
    }
}

impl From<Vec<DataType>> for Map {
    fn from(elements: Vec<DataType>) -> Self {
        let mut entries = Vec::with_capacity(elements.len());
        let mut elements = elements.into_iter();

        while let (Some(k), Some(v)) = (elements.next(), elements.next()) {
            entries.push((k, v));
        }

        Self(entries)
    }
}

impl From<Map> for Vec<u8> {
    fn from(m: Map) -> Self {
        let mut result = Vec::from(format!("{}{}\r\n", PREFIX as char, m.0.len()));

        for (key, value) in m.0 {
            result.extend(Vec::from(key));
            result.extend(Vec::from(value));
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use crate::{BulkString, Integer, SimpleString};

    use super::*;

    #[test]
    fn map_se_deserializa_correctamente() {
        let bytes = "%2\r\n+first\r\n:1\r\n+second\r\n:2\r\n".as_bytes();

        let m = Map::try_from(bytes).unwrap();

        assert_eq!(
            m.0,
            [
                (
                    DataType::SimpleString(SimpleString::from("first")),
                    DataType::Integer(Integer::from(1))
                ),
                (
                    DataType::SimpleString(SimpleString::from("second")),
                    DataType::Integer(Integer::from(2))
                )
            ]
        );
    }

    #[test]
    fn map_con_keys_de_diferentes_tipos_se_deserializa_correctamente() {
        let bytes = "%2\r\n+first\r\n:1\r\n$13\r\nHello, World!\r\n:2\r\n".as_bytes();

        let m = Map::try_from(bytes).unwrap();

        assert_eq!(
            m.0,
            [
                (
                    DataType::SimpleString(SimpleString::from("first")),
                    DataType::Integer(Integer::from(1))
                ),
                (
                    DataType::BulkString(BulkString::from("Hello, World!")),
                    DataType::Integer(Integer::from(2))
                )
            ]
        );
    }

    #[test]
    fn map_con_values_de_diferentes_tipos_se_deserializa_correctamente() {
        let bytes = "%2\r\n+first\r\n:1\r\n+second\r\n+Hello, World!\r\n".as_bytes();

        let m = Map::try_from(bytes).unwrap();

        assert_eq!(
            m.0,
            [
                (
                    DataType::SimpleString(SimpleString::from("first")),
                    DataType::Integer(Integer::from(1))
                ),
                (
                    DataType::SimpleString(SimpleString::from("second")),
                    DataType::SimpleString(SimpleString::from("Hello, World!"))
                )
            ]
        );
    }

    #[test]
    fn map_con_map_anidado_se_deserializa_correctamente() {
        let bytes = concat!(
            "%2\r\n",
            "+first\r\n",
            "%2\r\n+one\r\n:1\r\n+two\r\n:2\r\n",
            "+second\r\n",
            "%2\r\n+three\r\n:3\r\n+four\r\n:4\r\n",
        )
        .as_bytes();

        let m = Map::try_from(bytes).unwrap();

        assert_eq!(
            m.0,
            [
                (
                    DataType::SimpleString(SimpleString::from("first")),
                    DataType::Map(Map([
                        (
                            DataType::SimpleString(SimpleString::from("one")),
                            DataType::Integer(Integer::from(1))
                        ),
                        (
                            DataType::SimpleString(SimpleString::from("two")),
                            DataType::Integer(Integer::from(2))
                        ),
                    ]
                    .into()))
                ),
                (
                    DataType::SimpleString(SimpleString::from("second")),
                    DataType::Map(Map([
                        (
                            DataType::SimpleString(SimpleString::from("three")),
                            DataType::Integer(Integer::from(3))
                        ),
                        (
                            DataType::SimpleString(SimpleString::from("four")),
                            DataType::Integer(Integer::from(4))
                        ),
                    ]
                    .into()))
                )
            ]
        );
    }

    #[test]
    fn map_vacio_se_serializa_correctamente() {
        let m = Map(vec![]);

        let bytes: Vec<_> = m.into();

        assert_eq!(&bytes, b"%0\r\n");
    }

    #[test]
    fn map_de_un_solo_tipo_se_serializa_correctamente() {
        let m = Map(vec![
            (
                DataType::SimpleString(SimpleString::from("first")),
                DataType::Integer(Integer::from(1)),
            ),
            (
                DataType::SimpleString(SimpleString::from("second")),
                DataType::Integer(Integer::from(2)),
            ),
        ]);

        let bytes: Vec<_> = m.into();

        assert_eq!(&bytes, b"%2\r\n+first\r\n:+1\r\n+second\r\n:+2\r\n");
    }

    #[test]
    fn map_con_keys_de_diferentes_tipos_se_serializa_correctamente() {
        let m = Map(vec![
            (
                DataType::SimpleString(SimpleString::from("first")),
                DataType::Integer(Integer::from(1)),
            ),
            (
                DataType::BulkString(BulkString::from("Hello, World!")),
                DataType::Integer(Integer::from(2)),
            ),
        ]);

        let bytes: Vec<_> = m.into();

        let esperado = b"%2\r\n+first\r\n:+1\r\n$13\r\nHello, World!\r\n:+2\r\n";

        assert_eq!(&bytes, esperado);
    }

    #[test]
    fn map_con_values_de_diferentes_tipos_se_serializa_correctamente() {
        let m = Map(vec![
            (
                DataType::SimpleString(SimpleString::from("first")),
                DataType::Integer(Integer::from(1)),
            ),
            (
                DataType::SimpleString(SimpleString::from("second")),
                DataType::SimpleString(SimpleString::from("Hello, World!")),
            ),
        ]);

        let bytes: Vec<_> = m.into();

        let esperado = b"%2\r\n+first\r\n:+1\r\n+second\r\n+Hello, World!\r\n";

        assert_eq!(&bytes, esperado);
    }

    #[test]
    fn map_con_map_anidado_se_serializa_correctamente() {
        let m = Map(vec![
            (
                DataType::SimpleString(SimpleString::from("first")),
                DataType::Map(Map(vec![
                    (
                        DataType::SimpleString(SimpleString::from("one")),
                        DataType::Integer(Integer::from(1)),
                    ),
                    (
                        DataType::SimpleString(SimpleString::from("two")),
                        DataType::Integer(Integer::from(2)),
                    ),
                ])),
            ),
            (
                DataType::SimpleString(SimpleString::from("second")),
                DataType::Map(Map(vec![
                    (
                        DataType::SimpleString(SimpleString::from("three")),
                        DataType::Integer(Integer::from(3)),
                    ),
                    (
                        DataType::SimpleString(SimpleString::from("four")),
                        DataType::Integer(Integer::from(4)),
                    ),
                ])),
            ),
        ]);

        let bytes: Vec<_> = m.into();

        let inner_1 = "%2\r\n+one\r\n:+1\r\n+two\r\n:+2\r\n";
        let inner_2 = "%2\r\n+three\r\n:+3\r\n+four\r\n:+4\r\n";
        let esperado = format!("%2\r\n+first\r\n{inner_1}+second\r\n{inner_2}").into_bytes();

        assert_eq!(bytes, esperado);
    }
}
