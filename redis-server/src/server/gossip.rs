use std::collections::HashSet;


#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NodeStatus {
    Alive,
    Suspect,
    Down,
    Left,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NodeRole {
    Master,
    Replica,
}

#[derive(Debug, Clone)]
pub struct NodeInfo {
    pub id: String,
    pub ip: String,
    pub gossip_port: u16,
    pub role: NodeRole,
    pub status: NodeStatus,
    pub slots: Option<Vec<u16>>, //Solo si es master
    pub replicas: Option<Vec<String>>, //Solo si es master
    pub master_id: Option<String>, //Solo si es replica
    pub epoch: u64, //timestamp (segundos)
}

// Para ver si un nodo peude estar fallando o no
#[derive(Debug, Clone)]
pub struct SuspectNode {
    pub id: String,
    pub reason: String,
    pub epoch: u64,
}

#[derive(Debug, Clone)]
pub struct GossipMessage {
    pub from: NodeInfo,
    //LA INFO MIA XD
    pub known_nodes: Vec<NodeInfo>, //vector de nodos conocidos con su respectiva info
    //podria ser un HashSet tmb
    pub suspect_nodes: Vec<SuspectNode>,
    pub timestamp: u64,
}
