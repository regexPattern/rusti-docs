use redis_resp::BulkString;

use crate::error::Error;

/// Removes the specified keys. A key is ignored if it does not exist.
///
/// https://redis.io/docs/latest/commands/del
#[derive(Clone, Debug, PartialEq)]
pub struct Del {
    pub key: BulkString,
    pub keys: Vec<BulkString>,
}

impl Del {
    pub fn from_args(mut args: impl Iterator<Item = BulkString>) -> Result<Self, Error> {
        let key = args.next().ok_or(Error::MissingArgument)?;
        let keys: Vec<BulkString> = args.collect();

        if keys.is_empty() {
            return Err(Error::MissingArgument);
        }

        Ok(Self { key, keys })
    }
}

impl From<Del> for Vec<BulkString> {
    fn from(cmd: Del) -> Self {
        let mut args = vec![BulkString::from("DEL"), cmd.key];
        args.extend(cmd.keys);
        args
    }
}
