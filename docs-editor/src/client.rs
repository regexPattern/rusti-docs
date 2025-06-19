use std::{
    collections::HashMap,
    io::prelude::*,
    net::{Shutdown, SocketAddr, TcpStream},
};

use redis_cmd::{
    Command,
    storage::{HGetAll, StorageCommand},
};
use redis_resp::{BulkString, RespDataType};

use crate::Error;

pub fn send_command(cmd: Command, addr: SocketAddr) -> Result<RespDataType, Error> {
    let mut client_stream = TcpStream::connect(addr).map_err(Error::OpenConn)?;

    client_stream
        .write_all(&Vec::from(cmd))
        .map_err(Error::SendCommand)?;

    client_stream
        .shutdown(Shutdown::Write)
        .map_err(Error::SendCommand)?;

    let mut reply_buffer = Vec::new();
    client_stream
        .read_to_end(&mut reply_buffer)
        .map_err(Error::ReadReply)?;

    Ok(RespDataType::try_from(reply_buffer.as_slice())?)
}

pub fn read_resp_map(
    addr: SocketAddr,
    key: &str,
) -> Result<HashMap<BulkString, BulkString>, Error> {
    let cmd = Command::Storage(StorageCommand::HGetAll(HGetAll { key: key.into() }));

    let reply = send_command(cmd, addr)?;

    let map = match reply {
        RespDataType::Map(map) => map,
        RespDataType::Null => {
            print!("{}", log::debug!("key '{key}' sin inicializar"));
            return Ok(HashMap::new());
        }
        RespDataType::SimpleError(_err) => todo!(),
        _ => {
            return Err(Error::ReplyRespType);
        }
    };
    let map = map.into_iter().filter_map(|kv| {
        if let (RespDataType::BulkString(key), RespDataType::BulkString(value)) = kv {
            Some((key, value))
        } else {
            None
        }
    });

    Ok(map.collect())
}
