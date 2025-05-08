use std::io::{BufRead, BufReader, Write};
use std::net::TcpStream;
use std::sync::{Arc, Mutex};
use std::thread;

use commands::Command;
use commands::pub_sub::Subscribe;

fn main() {
    let server_addr = "127.0.0.1:6379";
    let stream = TcpStream::connect(server_addr).expect("No se pudo conectar al servidor Redis");
    // println!("Conectado a {}", server_addr);

    let log_msg = log::info!("Conectado a {}", server_addr);
    println!("{}", log_msg);

    let stream = Arc::new(Mutex::new(stream));

    // Enviamos subscirbe con commands lib
    {
        let mut stream_guard = stream.lock().unwrap();

        let subscribe = Subscribe {
            channels: vec![resp::BulkString::from("chan1")],
        };
        let cmd = Command::PubSub(subscribe.into());
        let resp_bytes: Vec<u8> = cmd.into();

        println!(
            "{}",
            log::info!(
                "Comando SUBSCRIBE serializado: {:?}",
                String::from_utf8_lossy(&resp_bytes)
            )
        );

        stream_guard.write_all(&resp_bytes).unwrap();
        stream_guard.flush().unwrap();
    }

    let stream_reader = Arc::clone(&stream);
    let stream_writer = Arc::clone(&stream);

    // Thread para lectura del stream
    let reader_handle = thread::spawn(move || {
        let stream = stream_reader.lock().unwrap();
        let reader = BufReader::new(stream.try_clone().unwrap());
        drop(stream);

        for line in reader.lines() {
            match line {
                Ok(msg) => println!("\n[RECV] {}", msg),
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
            print!("Ingrese comando: ");
            std::io::Write::flush(&mut std::io::stdout()).unwrap();

            let mut input = String::new();
            std::io::stdin().read_line(&mut input).unwrap();

            let input_trimmed = input.trim();
            println!("[DEBUG] Antes del lock");
            let mut stream = stream_writer.lock().unwrap();

            let mut parts = input_trimmed.split_whitespace();
            if let Some(cmd) = parts.next() {
                if cmd.eq_ignore_ascii_case("subscribe") {
                    let channels: Vec<resp::BulkString> =
                        parts.map(resp::BulkString::from).collect();
                    if !channels.is_empty() {
                        let subscribe = Subscribe { channels };
                        let cmd = Command::PubSub(subscribe.into());
                        let resp_bytes: Vec<u8> = cmd.into();
                        println!(
                            "[DEBUG] Serializado: {:?}",
                            String::from_utf8_lossy(&resp_bytes)
                        );

                        if let Err(e) = stream.write_all(&resp_bytes) {
                            eprintln!("[ERROR ESCRITURA] {}", e);
                            break;
                        }
                        if let Err(e) = stream.flush() {
                            eprintln!("[ERROR FLUSH] {}", e);
                            break;
                        }
                        println!("[DEBUG] Después de flush");
                        continue;
                    }
                }
            }

            //por ahora en otros comandos mando solo bytes
            if let Err(e) = writeln!(stream, "{}", input_trimmed) {
                eprintln!("[ERROR ESCRITURA] {}", e);
                break;
            }
            if let Err(e) = stream.flush() {
                eprintln!("[ERROR FLUSH] {}", e);
                break;
            }
            println!("[DEBUG] Después de flush");
        }
    });

    reader_handle.join().unwrap();
    writer_handle.join().unwrap();
}
