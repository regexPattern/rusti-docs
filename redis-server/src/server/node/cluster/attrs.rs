use std::{fmt, net::IpAddr};

type Id = [u8; 20];

/// Atributos de configuración de un nodo de un cluster de Redis.
/// https://redis.io/docs/latest/commands/cluster-nodes#serialization-format
#[derive(Debug)]
pub struct Attrs {
    pub id: Id,
    pub ip: IpAddr,
    pub port: u16,
    pub flags: Vec<Flag>,
    pub master_id: Option<Id>,
    pub slot_range: (u16, u16),
}

#[derive(Debug)]
pub enum Flag {
    Myself,
    Master,
    Slave,
    PFail,
    Fail,
    Handshake,
    NoAddr,
    NoFailover,
    NoFlags,
}

impl fmt::Display for Attrs {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let id = id_to_string(self.id);
        let (ip, port) = (self.ip, self.port);

        let flags: Vec<_> = self.flags.iter().map(|f| format!("{f}")).collect();
        let flags = flags.join(",");

        let master = if let Some(master_id) = self.master_id {
            id_to_string(master_id)
        } else {
            "-".to_string()
        };

        write!(f, "{id} {ip}:{port} {flags} {master}")?;

        if self.master_id.is_none() {
            write!(f, " {}-{}", self.slot_range.0, self.slot_range.1)?;
        }

        Ok(())
    }
}

impl fmt::Display for Flag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Flag::Myself => write!(f, "myself"),
            Flag::Master => write!(f, "master"),
            Flag::Slave => write!(f, "slave"),
            Flag::PFail => write!(f, "fail?"),
            Flag::Fail => write!(f, "fail"),
            Flag::Handshake => write!(f, "handshake"),
            Flag::NoAddr => write!(f, "noaddr"),
            Flag::NoFailover => write!(f, "nofailover"),
            Flag::NoFlags => write!(f, "noflags"),
        }
    }
}

fn id_to_string(id: Id) -> String {
    let mut id_str = String::with_capacity(40);
    for b in id {
        id_str.push_str(&format!("{:02x}", b));
    }
    id_str
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id_from_str(id: &str) -> Id {
        let s = id.as_bytes();
        let mut id = [0u8; 20];
        let mut i = 0;
        while i < 20 {
            let hi = nibble(s[2 * i]);
            let lo = nibble(s[2 * i + 1]);
            id[i] = (hi << 4) | lo;
            i += 1;
        }
        id
    }

    fn nibble(c: u8) -> u8 {
        match c {
            b'0'..=b'9' => c - b'0',
            b'a'..=b'f' => c - b'a' + 10,
            b'A'..=b'F' => c - b'A' + 10,
            _ => unreachable!(),
        }
    }

    #[test]
    fn se_formatean_correctamente_attrs_de_nodo_master() {
        let attrs = Attrs {
            id: id_from_str("07c37dfeb235213a872192d90877d0cd55635b91"),
            ip: "127.0.0.1".parse().unwrap(),
            port: 6379,
            flags: vec![Flag::Master],
            master_id: None,
            slot_range: (0, 16384),
        };

        assert_eq!(
            format!("{attrs}"),
            "07c37dfeb235213a872192d90877d0cd55635b91 127.0.0.1:6379 master - 0-16384"
        );
    }

    #[test]
    fn se_formatean_correctamente_attrs_de_nodo_slave() {
        let attrs = Attrs {
            id: id_from_str("07c37dfeb235213a872192d90877d0cd55635b91"),
            ip: "127.0.0.1".parse().unwrap(),
            port: 6379,
            flags: vec![Flag::Slave],
            master_id: Some(id_from_str("e7d1eecce10fd6bb5eb35b9f99a514335d9ba9ca")),
            slot_range: (0, 16384),
        };

        assert_eq!(
            format!("{attrs}"),
            "07c37dfeb235213a872192d90877d0cd55635b91 127.0.0.1:6379 slave e7d1eecce10fd6bb5eb35b9f99a514335d9ba9ca"
        );
    }

    #[test]
    fn se_formatean_correctamente_attrs_de_nodo_con_multiples_flags() {
        let attrs = Attrs {
            id: id_from_str("07c37dfeb235213a872192d90877d0cd55635b91"),
            ip: "127.0.0.1".parse().unwrap(),
            port: 6379,
            flags: vec![Flag::Myself, Flag::Master],
            master_id: None,
            slot_range: (0, 16384),
        };

        assert!(format!("{attrs}").contains("myself,master"));
    }
}
