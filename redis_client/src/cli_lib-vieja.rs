//CREO Q ESTO BOOOOM!
//CREO Q ESTO BOOOOM!

use std::io::{BufRead, BufReader, Write};
use std::net::TcpStream;
use std::sync::{Arc, Mutex, mpsc::Sender};
use std::thread;

use crate::command_sender::{self, CommandAction};

// ESTA LIBREREIA TIENE ESTA FUICNION QUE SE LLAMA ASI (ES UNA COPIA DEL MAIN QUE ESTYA EN CLI.RS)

//HANDLE_UI_ACTYION_

///HANDLE_UI_COMMAND TAMBIEN SON BUENOS NOMBRES

pub fn handle_command_input(input: &str, stream: &Arc<Mutex<TcpStream>>, tx_client: Sender<String>) -> Result<bool, String> {
    

    // //pedimos comando inical
    // print!("Ingrese comando: ");

    // // esto va a ser por un boton o algo que hardcodee el input...

    // std::io::Write::flush(&mut std::io::stdout()).unwrap();
    // let mut input = String::new();
    // std::io::stdin().read_line(&mut input).unwrap();

    let action = command_sender::parse_and_serialize_command(&input);

    // // una vez q sabemos que vamos a acer conectamos
    // let server_addr = "127.0.0.1:6379";
    // let stream = TcpStream::connect(server_addr).expect("No se pudo conectar al servidor Redis");

    // let log_msg = log::info!("Conectado a {}", server_addr);
    // println!("{}", log_msg);

    // let stream: Arc<Mutex<TcpStream>> = Arc::new(Mutex::new(stream));

    // Segun el comando enviamos  y manejamos
    match action {
        CommandAction::Send(bytes) => {
            //para set get publish
            let mut stream = stream.lock().map_err(|_| "Lock error".to_string())?;
            stream.write_all(&bytes).unwrap();
            stream.flush().unwrap();
            println!("[DEBUG] Comando enviado esperando rta ...");
            
            let server_stream_reader = stream.try_clone().map_err(|e| e.to_string())?;
            let mut server_reader = BufReader::new(server_stream_reader);
        

            let buffer = server_reader.fill_buf().map_err(|e| e.to_string())?;
            if buffer.is_empty() {
                tx_client.send("[DEBUG] Conexión cerrada por el servidor.".to_string()).ok();
            } else {
                
                //TEMP: 
                // Hay q mandar resp x ahora mando string xa simplificar..
                
                let server_reply = String::from_utf8_lossy(buffer).trim_end().to_string();
                tx_client.send(server_reply).ok();

                let len = buffer.len();
                server_reader.consume(len);
            }

            Ok(true)
        }
        CommandAction::HandleSubscribe(bytes, _channels) => {
            let stream_reader = Arc::clone(&stream);
            let stream_writer = Arc::clone(&stream);

            // mandamos el SUBSCRIBE primero
            {
                let mut stream = stream_writer.lock().unwrap();
                stream.write_all(&bytes).unwrap();
                stream.flush().unwrap();
                let log_msg = log::info!("[DEBUG] SUBSCRIBE enviado. escuhando al server...");
                println!("{}", log_msg);
            }

            // Thread para lectura del stream
            let reader_handle = thread::spawn(move || {
                let stream_reader = {
                    let stream_lock = stream_reader.lock().unwrap();
                    stream_lock.try_clone().unwrap()
                };

                let reader = BufReader::new(stream_reader);
                for line in reader.lines() {
                    //TODO REVISAR READER .LINES OK?
                    match line {
                        Ok(msg) => println!("[PUBSUB] {}", msg),
                        Err(e) => {
                            eprintln!("[ERROR LECTURA] {}", e);
                            break;
                        }
                    }
                }
            });
            // Thread para escritura de comandos
            let writer_handle = thread::spawn(move || {
                loop {
                    println!(
                        "\n--- consola esperando ingreso de comando (subscribe/unsubscribe/quit): ---\n"
                    );

                    // esto va a ser por un boton o algo que hardcodee el input...

                    std::io::Write::flush(&mut std::io::stdout()).unwrap();
                    let mut input = String::new();
                    std::io::stdin().read_line(&mut input).unwrap();

                    let action = command_sender::parse_and_serialize_command(&input);
                    match action {
                        CommandAction::Send(bytes) => {
                            //no debiera pero si lo hace nos devolvera error y lo leemos
                            //en el reader del thread
                            let mut stream = stream_writer.lock().unwrap();
                            stream.write_all(&bytes).unwrap();
                            stream.flush().unwrap();
                        }
                        CommandAction::HandleUnsubscribe(bytes, _channels) => {
                            let mut stream = stream_writer.lock().unwrap();
                            stream.write_all(&bytes).unwrap();
                            stream.flush().unwrap();
                            println!("[DEBUG] UNSUBSCRIBE enviado.");
                        }
                        CommandAction::Quit => {
                            println!("Cerrando conexión (quit).");
                            break;
                        }
                        CommandAction::Unknown(cmd) => {
                            eprintln!("Comando no reconocido: {}", cmd);
                        }
                        CommandAction::HandleSubscribe(bytes, _channels) => {
                            // println!(
                            //     "{}",
                            //     log::info!(
                            //         "Comando SUBSCRIBE serializado: {:?}",
                            //         String::from_utf8_lossy(&bytes)
                            //     )
                            // );

                            //manjear suscripciones aca en algun atributo del cliente eso falata
                            //crearemos una entidad?

                            {
                                let mut stream = stream_writer.lock().unwrap();
                                println!("[DEBUG] SUBSCRIBE ANIDADO: ");
                                stream.write_all(&bytes).unwrap();
                                stream.flush().unwrap();
                                println!("[DEBUG] SUBSCRIBE ANIDADO enviado");
                            }
                        }
                    }
                }
            });

            reader_handle.join().unwrap();
            writer_handle.join().unwrap();

            Ok(true)

        }
        CommandAction::HandleUnsubscribe(bytes, _channels) => {
            let mut stream = stream.lock().unwrap();
            stream.write_all(&bytes).unwrap();
            stream.flush().unwrap();
            println!("[DEBUG] UNSUBSCRIBE enviado.");

            Ok(true)

        }
        CommandAction::Quit => {
            println!("Cerrando conexión.");
            Ok(true)

        }
        CommandAction::Unknown(cmd) => {
            eprintln!("Comando no reconocido: {}", cmd);
            Ok(true)

        }
    }
}
