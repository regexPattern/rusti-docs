use std::net::{IpAddr, Ipv4Addr};

#[derive(Debug)]
pub struct ClusterMsg {
    pub id: String,
    pub ip: Ipv4Addr,
    pub port: u16,
    pub cluster_port: u16,
    pub data: ClusterMsgData,
}

impl ClusterMsg {
    pub fn into_bytes(self) -> Vec<u8> {
        match self.data {
            ClusterMsgData::Fail { id } => todo!(),
            ClusterMsgData::Gossip => todo!(),
            ClusterMsgData::Module => todo!(),
            ClusterMsgData::Ping(gossip_datas) => format!(
                "PING;{};{};{};{}",
                self.id, self.ip, self.port, self.cluster_port
            )
            .into_bytes(),
            ClusterMsgData::Pong(gossip_datas) => todo!(),
            ClusterMsgData::Meet(gossip_datas) => format!(
                "MEET;{};{};{};{}",
                self.id, self.ip, self.port, self.cluster_port
            )
            .into_bytes(),
            ClusterMsgData::Publish => todo!(),
            ClusterMsgData::Update => todo!(),
        }
    }
}

#[derive(Debug)]
pub enum ClusterMsgData {
    Fail { id: String },
    Gossip,
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
    pub ip: IpAddr,
    pub port: u16,
    pub cluster_port: u16,
}
