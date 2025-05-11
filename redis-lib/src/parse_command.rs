//esta puede quedar para mandar cosas por consola facilmente.,..
//no boooom!!

use commands::Command;
use resp::{Array, BulkString, RespDataType};

pub fn parse_command(input: &str) -> Result<Command, commands::Error> {
    let args: Vec<_> = input
        .split_whitespace()
        .map(BulkString::from)
        .map(RespDataType::from)
        .collect();

    let bytes: Vec<_> = Vec::from(Array::from(args));

    Command::try_from(bytes.as_slice())
}
