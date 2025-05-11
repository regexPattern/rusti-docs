use std::{
    io::{Read, Write},
    net::TcpStream,
};

use commands::{Command, pub_sub::PubSubCommand};

fn main() {
    let server_addr = "127.0.0.1:6379";
    let mut stream = TcpStream::connect(server_addr).unwrap();

    loop {
        let mut input = String::new();
        std::io::stdin().read_line(&mut input).unwrap();

        let cmd = redis_client::cli::parse_command(&input).unwrap();
        let es_pub_sub = matches!(cmd, Command::PubSub(PubSubCommand::Subscribe(_)));

        stream.write_all(&Vec::from(cmd)).unwrap();

        let mut reply = Vec::new();
        stream.read_to_end(&mut reply).unwrap();

        println!(
            "{}",
            String::from_utf8_lossy(&reply)
                .replace("\r", "\\r")
                .replace("\n", "\\n")
        );

        if es_pub_sub {
            println!("entrando a subscribe state...");
        } else {
            stream = TcpStream::connect(server_addr).unwrap();
        }
    }
}
