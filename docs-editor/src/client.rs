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

pub fn send_command(cmd: Command, db_addr: SocketAddr) -> Result<RespDataType, Error> {
    let mut slot_addr = db_addr;
    let cmd = Vec::from(cmd);

    loop {
        let mut stream = TcpStream::connect(slot_addr).unwrap();
        stream.write_all(&cmd).unwrap();

        stream.shutdown(Shutdown::Write).unwrap();

        let mut buffer = Vec::new();
        stream.read_to_end(&mut buffer).unwrap();

        let reply = RespDataType::try_from(buffer.as_slice()).unwrap();

        if let RespDataType::SimpleError(err) = reply {
            let mut err = err.0.splitn(3, " ");
            let redir_slot = err.nth(1).unwrap().parse().unwrap();
            let redir_addr = err.next().unwrap().parse().unwrap();
            slot_addr = SocketAddr::new(redir_slot, redir_addr);
            continue;
        } else {
            return Ok(reply);
        }
    }
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
