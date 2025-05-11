mod aggregate;
mod error;
mod simple;

pub use aggregate::*;
pub use error::Error;
pub use simple::*;

#[derive(Debug, PartialEq, Clone)]
pub enum RespDataType {
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

impl From<RespDataType> for Vec<u8> {
    fn from(dt: RespDataType) -> Self {
        match dt {
            RespDataType::SimpleString(ss) => ss.into(),
            RespDataType::BulkString(bs) => bs.into(),
            RespDataType::SimpleError(se) => se.into(),
            RespDataType::Integer(i) => i.into(),
            RespDataType::Array(a) => a.into(),
            RespDataType::Null => null::BYTES.into(),
            RespDataType::Boolean(b) => b.into(),
            RespDataType::Double(d) => d.into(),
            RespDataType::Set(set) => set.into(),
            RespDataType::Map(m) => m.into(),
        }
    }
}

impl TryFrom<&[u8]> for RespDataType {
    type Error = Error;

    fn try_from(bytes: &[u8]) -> Result<RespDataType, Self::Error> {
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

impl From<Array> for RespDataType {
    fn from(arr: Array) -> Self {
        Self::Array(arr)
    }
}

impl From<Boolean> for RespDataType {
    fn from(b: Boolean) -> Self {
        Self::Boolean(b)
    }
}

impl From<BulkString> for RespDataType {
    fn from(bs: BulkString) -> Self {
        Self::BulkString(bs)
    }
}

impl From<Double> for RespDataType {
    fn from(d: Double) -> Self {
        Self::Double(d)
    }
}

impl From<Integer> for RespDataType {
    fn from(i: Integer) -> Self {
        Self::Integer(i)
    }
}

impl From<Map> for RespDataType {
    fn from(m: Map) -> Self {
        Self::Map(m)
    }
}

impl From<Set> for RespDataType {
    fn from(s: Set) -> Self {
        Self::Set(s)
    }
}

impl From<SimpleError> for RespDataType {
    fn from(se: SimpleError) -> Self {
        Self::SimpleError(se)
    }
}

impl From<SimpleString> for RespDataType {
    fn from(ss: SimpleString) -> Self {
        Self::SimpleString(ss)
    }
}
