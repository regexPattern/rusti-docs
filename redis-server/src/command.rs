#[derive(Debug)]
pub enum Command {
    Storage(StorageCommand),
    PubSub(PubSubCommand),
}

#[derive(Debug)]
pub enum StorageCommand {
    Get { key: String },
    Set { key: String, value: String },
}

#[derive(Debug)]
pub enum PubSubCommand {
    Subscribe { channel: String },
    Publish { channel: String, message: String },
    UnSubscribe { channel: String },
}

#[derive(Debug)]
pub struct CommandError;

impl TryFrom<&[u8]> for Command {
    type Error = CommandError;

    fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
        let cmd = String::from_utf8_lossy(bytes).to_ascii_uppercase();
        let args: Vec<_> = cmd.split_ascii_whitespace().collect();

        Ok(if cmd.starts_with("SUBSCRIBE") {
            Self::PubSub(PubSubCommand::Subscribe {
                channel: args[1].to_string(),
            })
        } else if cmd.starts_with("PUBLISH") {
            Self::PubSub(PubSubCommand::Publish {
                channel: args[1].to_string(),
                message: args[2].to_string(),
            })
        } else if cmd.starts_with("GET") {
            Self::Storage(StorageCommand::Get {
                key: args[1].to_string(),
            })
        } else {
            Self::Storage(StorageCommand::Set {
                key: args[1].to_string(),
                value: args[2].to_string(),
            })
        })
    }
}

pub trait Keyed {
    fn key(&self) -> &str;
}

impl Keyed for StorageCommand {
    fn key(&self) -> &str {
        match self {
            StorageCommand::Get { key } => key,
            StorageCommand::Set { key, .. } => key,
        }
    }
}
