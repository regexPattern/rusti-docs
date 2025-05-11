use std::{
    io::{Read, Write},
    net::TcpStream,
};

use redis_client::client_struct;
// use redis_client::client_struct;

use commands::{Command, pub_sub::PubSubCommand};
fn main() {
    let server_addr = "127.0.0.1:6379";
    let mut stream = TcpStream::connect(server_addr).unwrap();

    let cliente = client_struct::Client::new(stream.try_clone().unwrap());
    //LE PASAMOS TCP,
    //UN TX_SENDER PARA COMANDOS STORAE,
    //OTRO TX_SENDER_PUBSUB
    // ALGO MAS Q ME ESTE OLVIDANDO...

    let (storage_tx, storage_rx) = mpsc::channel::<StoragEnvelope>();

    let (reply_pub_sub_tx, reply_pub_sub_rx) = mpsc::channel::<PubSubEnvelope>();

    // fn thread para renderizar consola de comandos storage ()) {

    cliente.ejecute_command(stream, StorgaCommand).unwrap();

    for reply in storage_rx {
        // Process the reply from the storage command
        println!("Storage reply: {:?}", reply);
    }

    // }

    // fn thread para rendrizar la consola de escritura de comandos pub sub (){

    //por reply_pub_sub_tx me MANDAN REPLIES DE LOS PUBLISHES Y DE LOS COMANDOS PUB SUB

    //por cmd_pub_sub_rx el cliente recibe mis comadnos (esto lo creaa el y me lo devuleve con mi
    //primer subscribe )
    // y yo le escribo por...

    //esto es la prrimera vuelta para el subscribe inicial
    cmd_pub_sub_tx = cliente
        .ejecute_command(stream, StorgaCommand, reply_pub_sub_tx)
        .unwrap();

    // ME MANDA UN CHANNEL  POR EL Q YO PUEDO CONTINUAR MANDANDOLE COMANDOS Y LO USO ACA ABJO.....
    loop {
        //input =
        //command = input.parse.(command)

        cmd_pub_sub_tx.send(PubSubEnvelope::Command(cmd)).unwrap();
    }
    // }

    // fn thread para renderizar la lectura lectura de comandos pub sub() {

    //ACA SOLO LEEMOS LO Q NOS MANDE POR PUB SUB,
    // OSEA ACA ME LLEVGAN LOS MSJS Q PUBLICARON  OTROS  CLIENTES  Y LAS REPLIES DEL SERVER
    //A LOS COMANDOS PUB SUB  QUE LE MANDE EN EL TRHEAD DE ARRIBV

    for reply in reply_pub_sub_rx {
        println!("PubSub reply: {:?}", reply);
    }

    // }

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
