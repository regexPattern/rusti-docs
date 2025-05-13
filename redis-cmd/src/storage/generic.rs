use redis_resp::BulkString;

use crate::error::Error;

/// Removes the specified keys. A key is ignored if it does not exist.
///
/// https://redis.io/docs/latest/commands/del
#[derive(Clone, Debug, PartialEq)]
pub struct Del {
    pub keys: Vec<BulkString>,
}

impl Del {
    pub fn from_bulk_strings(args: impl Iterator<Item = BulkString>) -> Result<Self, Error> {
        let args: Vec<_> = args.collect();

        if args.is_empty() {
            return Err(Error::MissingArgument);
        }

        Ok(Self { keys: args })
    }
}
