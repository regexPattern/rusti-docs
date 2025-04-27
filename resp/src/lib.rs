mod bulk;
mod error;
mod simple;

pub use bulk::*;
pub use error::Error;
pub use simple::*;

#[derive(Debug, PartialEq)]
pub enum DataType {
    Array(Array),
    Boolean(Boolean),
    BulkString(BulkString),
    Double(Double),
    Integer(Integer),
    Map(Map),
    Null,
    Set(Set),
    SimpleError(SimpleError),
    SimpleString(SimpleString),
}

impl From<DataType> for Vec<u8> {
    fn from(dt: DataType) -> Self {
        match dt {
            DataType::SimpleString(ss) => ss.into(),
            DataType::BulkString(bs) => bs.into(),
            DataType::SimpleError(se) => se.into(),
            DataType::Integer(i) => i.into(),
            DataType::Array(a) => a.into(),
            DataType::Null => null::BYTES.into(),
            DataType::Boolean(b) => b.into(),
            DataType::Double(d) => d.into(),
            DataType::Set(set) => set.into(),
            DataType::Map(m) => m.into(),
        }
    }
}

impl TryFrom<&[u8]> for DataType {
    type Error = Error;

    fn try_from(bytes: &[u8]) -> Result<DataType, Self::Error> {
        match bytes.first() {
            Some(&simple_string::PREFIX) => Ok(Self::SimpleString(SimpleString::try_from(bytes)?)),
            Some(&bulk_string::PREFIX) => Ok(Self::BulkString(BulkString::try_from(bytes)?)),
            Some(&simple_error::PREFIX) => Ok(Self::SimpleError(SimpleError::try_from(bytes)?)),
            Some(&integer::PREFIX) => Ok(Self::Integer(Integer::try_from(bytes)?)),
            Some(&array::PREFIX) => Ok(Self::Array(Array::try_from(bytes)?)),
            Some(&null::PREFIX) => Ok(Self::Null),
            Some(&boolean::PREFIX) => Ok(Self::Boolean(Boolean::try_from(bytes)?)),
            Some(&double::PREFIX) => Ok(Self::Double(Double::try_from(bytes)?)),
            Some(&set::PREFIX) => Ok(Self::Set(Set::try_from(bytes)?)),
            Some(&map::PREFIX) => Ok(Self::Map(Map::try_from(bytes)?)),
            _ => Err(Error::WrongPrefix),
        }
    }
}
