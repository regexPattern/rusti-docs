use std::net::TcpStream;
use std::sync::mpsc::{self, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use crate::consola::Consola;
use eframe::egui;

use log::LogMsg;
// use redis_cmd::pub_sub::PubSubCommand;
use redis_lib::client_struct::Client;
use redis_resp::RespDataType;

pub struct ThreadedApp {
    consola1: Consola,
    consola2: Consola,
}

//TODO PARA TODOS LOS THREADS PASRA is_runnign como aparametro y hacer un
//     while *is_running_clone.lock().unwrap() {
//      Pequeño sleep para evitar busy-waiting
//      std::thread::sleep(std::time::Duration::from_millis(50));
// para que termienne bien los threads
//*self.is_running.lock().unwrap() = false;
//al cerrar la app

impl Default for ThreadedApp {
    fn default() -> Self {
        //va a haber 2 clinetes distintos,
        //no hace faltan  los cloneeee

        //cliente_strg = cliente.new //este new se hace xca cada consulta
        //cliente_pubsub  = cliente.new

        // Closure para storage (comandos generales)
        let tarea_storage = |cmd: String,
                             hist: Arc<Mutex<Vec<String>>>,
                             _flag: Arc<Mutex<bool>>| {
            // Logger dummy
            let (logger_tx, _logger_rx) = mpsc::channel::<LogMsg>();

            let client = Client::new(logger_tx);

            let executable_cmd = match redis_lib::parse_command::parse_command(&cmd) {
                Ok(cmd) => cmd,
                Err(e) => {
                    hist.lock()
                        .unwrap()
                        .push(format!("Error al parsear el comando: {:?}", e));
                    return;
                }
            };

            let server_addr = "127.0.0.1:6379";
            let stream = TcpStream::connect(server_addr).expect("No se pudo conectar al servidor");

            let resp = match client.execute_non_pub_sub_mode_command(stream, executable_cmd) {
                Ok(resp) => resp,
                Err(e) => {
                    hist.lock()
                        .unwrap()
                        .push(format!("Error ejecutando comando: {:?}", e));
                    return;
                }
            };

            thread::sleep(Duration::from_millis(100));
            hist.lock().unwrap().push(format!("Procesado: {}", cmd));

            hist.lock().unwrap().push(format!("Resultado: {:?}", resp));
        };

        // Closure para pubsub
        let cmd_pub_sub_tx_mine = Arc::new(Mutex::new(None::<Sender<redis_cmd::Command>>));

        let thread_started = Arc::new(Mutex::new(false));
        // Para no lanzar el thread  varias veces

        let tarea_pubsub = {
            let thread_started = Arc::clone(&thread_started);
            let cmd_pub_sub_tx_mine = Arc::clone(&cmd_pub_sub_tx_mine);

            move |cmd: String, hist: Arc<Mutex<Vec<String>>>, _flag: Arc<Mutex<bool>>| {
                let mut thread_guard = thread_started.lock().unwrap();
                hist.lock()
                    .unwrap()
                    .push("ejecutando tarea pub sub".to_string());

                let mut tx_guard = cmd_pub_sub_tx_mine.lock().unwrap();
                if tx_guard.is_none() {
                    // Primera vez: inicializa la conexión y obtiene el channel
                    let (logger_tx, _logger_rx) = mpsc::channel::<LogMsg>();

                    let mut client = Client::new(logger_tx);

                    let executable_cmd = match redis_lib::parse_command::parse_command(&cmd) {
                        Ok(cmd) => cmd,
                        Err(e) => {
                            hist.lock()
                                .unwrap()
                                .push(format!("Error al parsear el comando: {:?}", e));
                            return;
                        }
                    };

                    let server_addr = "127.0.0.1:6379";
                    let stream =
                        TcpStream::connect(server_addr).expect("No se pudo conectar al servidor");

                    let pubsub_exec_cmd = match executable_cmd {
                        redis_cmd::Command::PubSub(cmd) => cmd,
                        redis_cmd::Command::Storage(_) => {
                            hist.lock()
                                .unwrap()
                                .push("Error: Storage no soportado en modo PubSub".to_string());
                            return;
                        }
                        redis_cmd::Command::Cluster(_) => {
                            hist.lock()
                                .unwrap()
                                .push("Error: Cluster no soportado en modo PubSub".to_string());
                            return;
                        }
                    };
                    let (reply_pub_sub_tx, reply_pub_sub_rx) = mpsc::channel::<RespDataType>();
                    //canal para el envio de comandos al client-sdk
                    let cmd_pub_sub_tx = match client.execute_pubsub_mode_cmd(
                        pubsub_exec_cmd,
                        stream,
                        reply_pub_sub_tx,
                    ) {
                        Ok(tx) => tx,
                        Err(e) => {
                            hist.lock()
                                .unwrap()
                                .push(format!("Error ejecutando comando PubSub: {:?}", e));
                            return;
                        }
                    };

                    *tx_guard = cmd_pub_sub_tx;

                    hist.lock()
                        .unwrap()
                        .push(format!("Inicializado PubSub con: {}", cmd));

                    // Lanzo un thread para escuchar respuestas y mostrarlas en el historial
                    // let is_running_clone = Arc::clone(&is_running);   //OJO FALTA CODIGO CERRAR BIEN THREADS !
                    if !*thread_guard {
                        let hist_clone = Arc::clone(&hist);
                        thread::spawn(move || {
                            for mensaje in reply_pub_sub_rx {
                                hist_clone
                                    .lock()
                                    .unwrap()
                                    .push(format!("[PUBSUB]: {:?}", mensaje));
                            }
                        });
                        *thread_guard = true;
                    }
                } else {
                    hist.lock().unwrap().push("no tengo el lock ".to_string());
                    // Ya inicializado: solo envía el comandos por el channel
                    if let Some(ref tx) = *tx_guard {
                        hist.lock().unwrap().push("tengo el lock ".to_string());
                        let executable_cmd = match redis_lib::parse_command::parse_command(&cmd) {
                            Ok(cmd) => cmd,
                            Err(e) => {
                                hist.lock()
                                    .unwrap()
                                    .push(format!("Error al parsear el comando: {:?}", e));
                                return;
                            }
                        };
                        tx.send(executable_cmd).expect("Failed to send command");
                        hist.lock()
                            .unwrap()
                            .push(format!("Enviado por channel: {}", cmd));
                    }
                }
                // thread::sleep(Duration::from_millis(100));
                hist.lock()
                    .unwrap()
                    .push(format!("Procesado (pubsub): {}", cmd));
            }
        };

        Self {
            consola1: Consola::new("Storage", tarea_storage),
            consola2: Consola::new("PubSub", tarea_pubsub),
        }
    }
}

impl eframe::App for ThreadedApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical()
                .id_source("consolas_scroll_area")
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.vertical(|ui| {
                            ui.label(
                                egui::RichText::new("Consola PUBLISH + STORAGE Commands")
                                    .heading()
                                    .strong()
                                    .color(egui::Color32::from_rgb(255, 255, 255)),
                            );
                            self.consola1.ui(ui, "");
                        });
                        ui.vertical(|ui| {
                            ui.label(
                                egui::RichText::new("Consola SUBSCRIBE Commands")
                                    .heading()
                                    .strong()
                                    .color(egui::Color32::from_rgb(255, 255, 255)),
                            );
                            self.consola2.ui(ui, "");
                        });
                    });
                });
        });

        if *self.consola1.is_running.lock().unwrap() || *self.consola2.is_running.lock().unwrap() {
            ctx.request_repaint();
        }
    }
}
