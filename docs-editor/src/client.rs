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

/// Envía un comando Redis al servidor y maneja redirecciones MOVED.
/// Devuelve la respuesta como RespDataType o un error.
pub fn send_command(cmd: Command, db_addr: SocketAddr) -> Result<RespDataType, Error> {
    let mut slot_addr = db_addr;
    let cmd = Vec::from(cmd);

    loop {
        let mut stream = TcpStream::connect(slot_addr).map_err(Error::OpenConn)?;
        stream.write_all(&cmd).map_err(Error::SendCommand)?;

        stream.shutdown(Shutdown::Write).map_err(Error::OpenConn)?;

        let mut buffer = Vec::new();
        stream.read_to_end(&mut buffer).map_err(Error::ReadReply)?;

        let reply = RespDataType::try_from(buffer.as_slice()).map_err(|_| Error::ReplyRespType)?;

        if let RespDataType::SimpleError(err) = reply {
            if err.0.contains("MOVED") {
                let port = err.0.split(":").last().ok_or(Error::MissingData)?;
                let port = port.parse().unwrap();
                print!("{}", log::debug!("redirigiendo a nodo en puerto {port}"));
                slot_addr.set_port(port);
                continue;
            } else {
                print!("{}", log::debug!("comando enviado a nodo en {slot_addr:?}"));
                return Err(Error::RedisClient(err.0));
            }
        } else {
            print!("{}", log::debug!("comando enviado a nodo en {slot_addr:?}"));
            return Ok(reply);
        }
    }
}

/// Lee un resp map usando HGETALL y lo retorna como HashMap.
/// Si la clave no existe, retorna un HashMap vacío.
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
