fn main() {}

// use redis_lib::client_struct::Client;
// use redis_cmd::pub_sub::{PubSubCommand, Subscribe};
// use redis_cmd::Command;
// use redis_resp::RespDataType;
// use redis_lib::command_sender::CommandAction;
// use log::LogMsg;
// use std::net::TcpStream;
// use std::sync::mpsc::{self, Sender};
// use std::thread;

// use redis_lib::command_sender;
// use std::io::{self, Write};

// fn main() {

//     print!("Ingrese comando: ");
//     io::stdout().flush().unwrap();
//     let mut input = String::new();
//     io::stdin().read_line(&mut input).unwrap();

//     let action = command_sender::parse_and_serialize_command(&input);

//     match action {
//         CommandAction::Send(bytes) => {

//             // Logger dummy
//             let (logger_tx, _logger_rx) = mpsc::channel::<LogMsg>();

//             let mut client = Client::new(logger_tx);

//             let server_addr = "127.0.0.1:6379";
//             let stream = TcpStream::connect(server_addr).expect("No se pudo conectar al servidor");

//             let cmd = redis_cmd::Command::try_from(bytes.as_slice()).expect("Comando inválido");
//             let resp = client
//                 .execute_non_pub_sub_mode_command(stream, cmd)
//                 .expect("Error ejecutando comando");

//             println!("Respuesta: {:?}", resp);

//         }
//         command_sender::CommandAction::HandleSubscribe(channel, callback) => {
//             let (logger_tx, logger_rx) = mpsc::channel::<LogMsg>();

//             let mut client = Client::new(logger_tx);

//             let (pubsub_reply_tx, pubsub_reply_rx) = mpsc::channel::<RespDataType>();

//             let server_addr = "127.0.0.1:6379";
//             let stream = TcpStream::connect(server_addr).expect("No se pudo conectar al servidor");

//             // Mandamos un subscribe
//             let subscribe_cmd = PubSubCommand::Subscribe(Subscribe {
//                 channels: vec!["canal1".into()],
//             });

//             let pubsub_cmd_sender = client
//                 .execute_pubsub_mode_cmd(subscribe_cmd, stream.try_clone().unwrap(), pubsub_reply_tx)
//                 .expect("Fallo al ejecutar subscribe")
//                 .expect("No se devolvió el sender de comandos pubsub");

//             //Thread para recibir mensajes de pubsub + PUBLISHES en canales
//             thread::spawn(move || {
//                 for msg in pubsub_reply_rx {
//                     println!("[PUBSUB] {:?}", msg);
//                 }
//             });

//             use std::io::{self, Write};

//             loop {
//                 print!("Comando pubsub (subscribe/unsubscribe/quit): ");
//                 io::stdout().flush().unwrap();
//                 let mut input = String::new();
//                 io::stdin().read_line(&mut input).unwrap();
//                 let input = input.trim();

//                 if input.eq_ignore_ascii_case("quit") {
//                     break;
//                 }

//                 // Parsear el input a un PubSubCommand (ejemplo simple)
//                 let parts: Vec<&str> = input.split_whitespace().collect();
//                 if parts.is_empty() {
//                     continue;
//                 }

//                 match parts[0].to_ascii_lowercase().as_str() {
//                     "subscribe" if parts.len() > 1 => {
//                         let cmd = Command::PubSub(PubSubCommand::Subscribe(Subscribe {
//                             channels: vec![parts[1].into()],
//                         }));
//                         pubsub_cmd_sender.send(cmd).unwrap();
//                     }
//                     "unsubscribe" if parts.len() > 1 => {
//                         use redis_cmd::pub_sub::Unsubscribe;
//                         let cmd = Command::PubSub(PubSubCommand::Unsubscribe(Unsubscribe {
//                             channels: vec![parts[1].into()],
//                         }));
//                         pubsub_cmd_sender.send(cmd).unwrap();
//                     }
//                     _ => {
//                         println!("Comando no reconocido o faltan argumentos.");
//                     }
//                 }
//             }
//         }
//         _ => {}

//     }

// }
