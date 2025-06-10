use std::{fmt, time::SystemTime};

use chrono::{DateTime, Local};

use crate::server::node::cluster::NodeId;

pub struct FailureReport {
    pub reporter_id: NodeId,
    pub time: SystemTime,
}

impl fmt::Display for FailureReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let time: DateTime<Local> = self.time.into();

        f.debug_struct("FailureReport")
            .field("reporter_id", &hex::encode(self.reporter_id))
            .field("time", &time.format("%d-%m-%Y %H:%M:%S").to_string())
            .finish()
    }
}

impl fmt::Debug for FailureReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self}")
    }
}
