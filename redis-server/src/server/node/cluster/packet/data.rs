use std::net::Ipv4Addr;

use super::error::Error;

const FIELDS_SEP: u8 = b' ';

#[derive(Debug)]
pub enum PacketData {
    Fail { id: String },
    Module,
    Ping(Vec<GossipData>),
    Pong(Vec<GossipData>),
    Meet(Vec<GossipData>),
    Publish,
    Update,
}

#[derive(Debug)]
pub struct GossipData {
    pub id: String,
    pub ip: Ipv4Addr,
    pub port: u16,
    pub cluster_port: u16,
}

impl TryFrom<&[u8]> for PacketData {
    type Error = Error;

    fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
        let mut fields = bytes.split(|b| *b == FIELDS_SEP);
        let kind = String::from_utf8_lossy(fields.next().unwrap()).to_string();
        Ok(match kind.as_str() {
            "MEET" => Self::Meet(vec![]),
            "PING" => Self::Ping(vec![]),
            "PONG" => Self::Pong(vec![]),
            _ => unimplemented!(),
        })
    }
}

impl From<PacketData> for Vec<u8> {
    fn from(data: PacketData) -> Self {
        (match data {
            PacketData::Fail { id: _id } => todo!(),
            PacketData::Module => todo!(),
            PacketData::Ping(_gd) => format!("PING"),
            PacketData::Pong(_gd) => format!("PONG"),
            PacketData::Meet(_gd) => format!("MEET"),
            PacketData::Publish => todo!(),
            PacketData::Update => todo!(),
        })
        .into_bytes()
    }
}
