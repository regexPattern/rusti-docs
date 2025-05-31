use std::net::Ipv4Addr;

use super::error::Error;

const FIELDS_SEP: u8 = b' ';

#[derive(Debug)]
pub struct PacketHeader {
    pub id: String,
    pub ip: Ipv4Addr,
    pub port: u16,
    pub cluster_port: u16,
}

impl TryFrom<&[u8]> for PacketHeader {
    type Error = Error;

    fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
        let mut fields = bytes.split(|b| *b == b' ');
        Ok(Self {
            id: String::from_utf8_lossy(fields.next().unwrap()).to_string(),
            ip: String::from_utf8_lossy(fields.next().unwrap())
                .parse()
                .unwrap(),
            port: String::from_utf8_lossy(fields.next().unwrap())
                .parse()
                .unwrap(),
            cluster_port: String::from_utf8_lossy(fields.next().unwrap())
                .parse()
                .unwrap(),
        })
    }
}

impl From<PacketHeader> for Vec<u8> {
    fn from(header: PacketHeader) -> Self {
        format!(
            "{} {} {} {}",
            header.id, header.ip, header.port, header.cluster_port
        )
        .into_bytes()
    }
}
