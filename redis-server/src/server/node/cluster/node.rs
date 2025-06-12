use std::{
    fmt,
    net::Ipv4Addr,
    time::{SystemTime, UNIX_EPOCH},
};

use chrono::{DateTime, Local};

use super::{
    NodeId,
    flags::{self, Flags},
    message::MessageHeader,
};

