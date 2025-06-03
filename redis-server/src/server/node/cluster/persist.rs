use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use rand::Rng;

use crate::config::ClusterConfig;

use super::{ClusterActor, node::ClusterNode};

impl TryFrom<ClusterConfig> for ClusterActor {
    type Error = ();

    fn try_from(config: ClusterConfig) -> Result<Self, Self::Error> {
        let mut id = [0_u8; 20];
        rand::rng().fill(&mut id);

        let myself = ClusterNode {
            id,
            ip: config.bind,
            port: config.port,
            cluster_port: config.port,
            config_epoch: 0,
        };

        Ok(Self {
            myself,
            cluster_view: Arc::new(RwLock::new(HashMap::new())),
            cluster_streams: Arc::new(RwLock::new(HashMap::new())),
        })
    }
}
