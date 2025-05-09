use commands::Command;
use commands::pub_sub::PubSubCommand;

use crate::client_command;

use resp::BulkString;

pub enum CommandAction {
    Send(Vec<u8>),
    HandleSubscribe(Vec<u8>, Vec<BulkString>),
    HandleUnsubscribe(Vec<u8>, Vec<BulkString>),
    Quit,
    // OJO FALTA QUITTTTT TODO
    //hara algo parecido a handle_unscubcribe supongo
    Unknown(String),
}

//y aca la serelizacion de todos los comanodos..
pub fn parse_and_serialize_command(input: &str) -> CommandAction {
    todo!()
    // match client_command::parse_command(input) {
    //     Some(Command::PubSub(pubsub_cmd)) => {
    //         use commands::pub_sub::PubSubCommand::*;
    //         match pubsub_cmd {
    //             Subscribe(subscribe) => {
    //                 let chans = subscribe.channels.clone();
    //
    //                 //ETSA BUENO MANEJAR TODO COMO BULK STRINGS INLCUSO INTERNAMENTE?
    //                 // EN EL SERVER LO HICIMOS PERO QSY
    //
    //                 let resp_bytes: Vec<u8> =
    //                     Command::PubSub(PubSubCommand::Subscribe(subscribe)).into();
    //                 CommandAction::HandleSubscribe(resp_bytes, chans)
    //             }
    //             Unsubscribe(unsubscribe) => {
    //                 let chans = unsubscribe.channels.clone();
    //                 let resp_bytes: Vec<u8> =
    //                     Command::PubSub(PubSubCommand::Unsubscribe(unsubscribe)).into();
    //                 CommandAction::HandleUnsubscribe(resp_bytes, chans)
    //             }
    //             // OJO FALTA QUITTTTT
    //             _ => {
    //                 let resp_bytes: Vec<u8> = Command::PubSub(pubsub_cmd).into();
    //                 CommandAction::Send(resp_bytes)
    //             }
    //         }
    //     }
    //     //storage commandsa get set publish etc...
    //     Some(cmd) => {
    //         let resp_bytes: Vec<u8> = cmd.into();
    //         CommandAction::Send(resp_bytes)
    //     }
    //     None => CommandAction::Unknown(input.trim().to_string()),
    // }
}
