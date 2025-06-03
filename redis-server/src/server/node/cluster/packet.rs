mod data;
mod error;
mod header;

pub use data::{GossipData, GossipDataKind, PacketData};
use error::Error;
pub use header::{HEADER_BYTES, PacketHeader};

#[derive(Debug)]
pub struct Packet {
    pub header: PacketHeader,
    pub data: PacketData,
}

impl TryFrom<&[u8]> for Packet {
    type Error = Error;

    fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
        let header = PacketHeader::try_from(&bytes[0..HEADER_BYTES]).unwrap();
        let data = PacketData::try_from(&bytes[HEADER_BYTES..]).unwrap();

        Ok(Self { header, data })
    }
}

impl From<Packet> for Vec<u8> {
    fn from(packet: Packet) -> Self {
        let mut bytes = Vec::from(packet.header);
        bytes.extend(Vec::from(packet.data));
        bytes
    }
}
