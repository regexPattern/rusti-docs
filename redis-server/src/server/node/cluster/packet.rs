mod data;
mod error;
mod header;

use data::PacketData;
use error::Error;
use header::PacketHeader;

const HEADER_DATA_SEP: u8 = b'@';
pub const TERMINATOR: u8 = b'\n';

#[derive(Debug)]
pub struct Packet {
    pub header: PacketHeader,
    pub data: PacketData,
}

impl TryFrom<Vec<u8>> for Packet {
    type Error = Error;

    fn try_from(bytes: Vec<u8>) -> Result<Self, Self::Error> {
        let mut bytes = bytes.split(|b| *b == b'@');

        let header = PacketHeader::try_from(bytes.next().unwrap()).unwrap();
        let data = PacketData::try_from(bytes.next().unwrap()).unwrap();

        Ok(Self { header, data })
    }
}

impl From<Packet> for Vec<u8> {
    fn from(packet: Packet) -> Self {
        let mut bytes = Vec::from(packet.header);
        let data = Vec::from(packet.data);

        bytes.reserve_exact(1 + data.len() + 1);

        bytes.push(HEADER_DATA_SEP);
        bytes.extend(data);
        bytes.push(TERMINATOR);

        bytes
    }
}
