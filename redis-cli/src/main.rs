use std::io::{BufRead, BufReader, Write};
use std::net::TcpStream;
use std::sync::{Arc, Mutex};
use std::thread;

fn main() {
    let server_addr = "127.0.0.1:6379";
    let stream = TcpStream::connect(server_addr).expect("No se pudo conectar al servidor Redis");
    println!("Conectado a {}", server_addr);

    let stream = Arc::new(Mutex::new(stream));

    // Enviamos SUBSCRIBE chan1
    {
        let mut stream_guard = stream.lock().unwrap();
        writeln!(stream_guard, "SUBSCRIBE chan1").unwrap();
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
                Ok(msg) => println!("[RECV] {}", msg),
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

            println!("[DEBUG] Antes del lock");
            let mut stream = stream_writer.lock().unwrap();

            if let Err(e) = writeln!(stream, "{}", input.trim()) {
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
