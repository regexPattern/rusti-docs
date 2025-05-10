use std::{
    io::{BufRead, BufReader, Write},
    net::TcpStream,
    sync::{Arc, Mutex},
    thread,
};


//
//esta seria la gui dde aca enb vez de mandar cosas por consola  las mandamoss  dde la interfazzzz
//


fn main() {
    let server_addr = "127.0.0.1:6379";
    // Conn para comandos generales
    let general_stream = Arc::new(Mutex::new(TcpStream::connect(server_addr).unwrap()));
    // Connn para pub/sub
    let pubsub_stream = Arc::new(Mutex::new(TcpStream::connect(server_addr).unwrap()));

    // Thread para comandos generales
    let general_stream_clone = Arc::clone(&general_stream);
    let general_handle = thread::spawn(move || {
        loop {
            let mut input = String::new();
            std::io::stdin().read_line(&mut input).unwrap();


            ///HANDLE_UI_COMMAND TAMBIEN SON BUENOS NOMBRES
            //aca usamos la lib q hay en cli-lib.rs, rescibimos la RTA
            // POR UN HCANNNELLLLLLL

            let seguir = redis_client::cli-lib2::handle_command_input(&input, &general_stream_clone, TX_CHANNEL);
            
            //asiiiiiiii
            ////siuuuu creo q va por acaaaa
            for msg in rx.recv() {
                println!("[GENERAL] {}", msg);
            }

            if !seguir {
                break;
            }
        }
    });

    // Thread para pub/sub
    let pubsub_stream_clone = Arc::clone(&pubsub_stream);
    let pubsub_handle = thread::spawn(move || {
        // Aquí puedes pedir el canal a suscribirse, o dejarlo esperando comandos SUBSCRIBE
        loop {
            let mut input = String::new();
            std::io::stdin().read_line(&mut input).unwrap();

            //aca usamos la lib q hay en cli-lib.rs
            //MANDAMOS UN CHANELLLL PARA RECIBI LOS MSJS Y HABLAR ENTRE EKL THREAD DEL SUSVCRIUBE DEL 
            ///HANLDE COMMAND INPUTR Y ESTE  DEACA
            
            ///HANDLE_UI_COMMAND TAMBIEN SON BUENOS NOMBRES
            let seguir = redis_client::cli-lib2::handle_command_input(&input, &pubsub_stream_clone, tex_channell);
            
            //si es un quit

            if !seguir {
                break;
            }

            // Después de un SUBSCRIBE dejamoss este thread esperando mensajes

            //ACA ABAJO EN VEZ RESCIBIR LOS MSJS POR EL STREAM LOS PODEMOS RECIBIR POR UN CHANELLLLLLL
            //CJHANELLLL
            
            let stream = pubsub_stream_clone.lock().unwrap();
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            let mut msg = String::new();
            while let Ok(n) = reader.read_line(&mut msg) {
              
            }
            
            // en vez de eso....

            //algo asiiiii
            for msg in rx.recv() {
                println!("[PUBSUB] {}", msg);
    
                }
            
        }
    });

    general_handle.join().unwrap();
    pubsub_handle.join().unwrap();
}