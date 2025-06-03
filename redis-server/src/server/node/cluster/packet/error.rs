use std::fmt;

#[derive(Debug)]
pub enum Error {
    GossipDataKind,
    PacketDataKind,
    PacketDataLength,
    PacketHeaderLength,
}

impl std::error::Error for Error {}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::GossipDataKind => {
                write!(f, "discriminante de tipo de gossip data es inválido")
            }
            Error::PacketDataKind => {
                write!(f, "discriminante de tipo de packet data es inválido")
            }
            Error::PacketDataLength => {
                write!(f, "cantidad de bytes de packet data incorrectos")
            }
            Error::PacketHeaderLength => {
                write!(f, "cantidad de bytes de packet header incorrectos")
            }
        }
    }
}
