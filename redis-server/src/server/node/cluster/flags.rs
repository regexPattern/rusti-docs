use std::fmt;

pub const FLAG_MYSELF: u16 = 0b0000_0000_0000_0001;
pub const FLAG_MASTER: u16 = 0b0000_0000_0000_0010;
pub const FLAG_SLAVE: u16 = 0b0000_0000_0000_0100;
pub const FLAG_PFAIL: u16 = 0b0000_0000_0000_1000;
pub const FLAG_FAIL: u16 = 0b0000_0000_0001_0000;
pub const FLAG_HANDSHAKE: u16 = 0b0000_0000_0010_0000;
pub const FLAG_NOADDR: u16 = 0b0000_0000_0100_0000;
pub const FLAG_NOFAILOVER: u16 = 0b0000_0000_1000_0000;

#[derive(Copy, Clone, PartialEq)]
pub struct Flags(pub u16);

impl Flags {
    pub fn contains(&self, flag: u16) -> bool {
        self.0 & flag == flag
    }
}

impl fmt::Display for Flags {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut flags = Vec::with_capacity(8);

        let mut push_flag = |flag, string| {
            if self.contains(flag) {
                flags.push(string)
            }
        };

        push_flag(FLAG_MYSELF, "myself");
        push_flag(FLAG_MASTER, "master");
        push_flag(FLAG_SLAVE, "slave");
        push_flag(FLAG_PFAIL, "pfail");
        push_flag(FLAG_FAIL, "fail");
        push_flag(FLAG_HANDSHAKE, "handshake");
        push_flag(FLAG_NOADDR, "noaddr");
        push_flag(FLAG_NOFAILOVER, "nofailover");

        write!(f, "{}", flags.join(","))
    }
}

impl fmt::Debug for Flags {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self}")
    }
}
