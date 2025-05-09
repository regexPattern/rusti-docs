use commands::Command;
use commands::pub_sub::{Subscribe, Unsubscribe};
use resp::BulkString;

//aca se harian todos klos parseos de todos los comandos...
pub fn parse_command(input: &str) -> Option<Command> {
    let mut parts = input.split_whitespace();
    match parts.next().map(|s| s.to_ascii_lowercase()) {
        Some(cmd) if cmd == "subscribe" => {
            let channels: Vec<BulkString> = parts.map(BulkString::from).collect();
            if !channels.is_empty() {
                let subscribe = Subscribe { channels };
                Some(Command::PubSub(subscribe.into()))
            } else {
                None
            }
        }
        Some(cmd) if cmd == "unsubscribe" => {
            let channels: Vec<BulkString> = parts.map(BulkString::from).collect();
            let unsubscribe = Unsubscribe { channels };
            Some(Command::PubSub(unsubscribe.into()))
        }
        _ => None,
    }
}
