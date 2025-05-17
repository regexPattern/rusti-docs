use eframe::egui;
use std::sync::{
    Arc, Mutex,
    mpsc::{self, Receiver, Sender},
};
use std::thread;

pub struct Consola {
    input: String,
    historial: Arc<Mutex<Vec<String>>>,
    tx: Sender<String>,
    rx: Receiver<String>,
    pub is_running: Arc<Mutex<bool>>,
}

impl Consola {
    pub fn new<F>(nombre: &str, tarea: F) -> Self
    where
        F: Fn(String, Arc<Mutex<Vec<String>>>, Arc<Mutex<bool>>) + Send + 'static + Clone,
    {
        let (tx_cmd, rx_cmd) = mpsc::channel::<String>();
        let (tx_out, rx_out) = mpsc::channel::<String>();

        let historial = Arc::new(Mutex::new(vec![format!(
            "{} iniciada en stream1 -stream-",
            nombre
        )]));
        let historial_clone = Arc::clone(&historial);
        let is_running = Arc::new(Mutex::new(true));
        let is_running_clone = Arc::clone(&is_running);

        thread::spawn(move || {
            while let Ok(cmd) = rx_cmd.recv() {
                if !*is_running_clone.lock().unwrap() {
                    break;
                }

                // let respuesta = tarea(cmd, Arc::clone(&historial_clone), Arc::clone(&is_running_clone));
                // tx_out.send(respuesta).ok();
                // creo q es o esto de arriba o
                tarea(
                    cmd,
                    Arc::clone(&historial_clone),
                    Arc::clone(&is_running_clone),
                );
                tx_out.send("----".to_string()).ok();
            }
        });

        Self {
            input: String::new(),
            historial,
            tx: tx_cmd,
            rx: rx_out,
            is_running,
        }
    }

    pub fn ui(&mut self, ui: &mut egui::Ui, nombre: &str) {
        ui.group(|ui| {
            ui.label(nombre);

            let historial = self.historial.lock().unwrap().clone();

            for line in historial.iter() {
                if line.starts_with("> ") {
                    // Comando ingresado por el usuario
                    ui.label(
                        egui::RichText::new(line)
                            .color(egui::Color32::from_rgb(0, 120, 220))
                            .strong()
                            .size(22.0),
                    );
                } else if let Some(rest) = line.strip_prefix("Resultado:") {
                    ui.label(
                        egui::RichText::new("Resultado:")
                            .color(egui::Color32::GRAY)
                            .size(16.0)
                            .strong(),
                    );
                    ui.label(
                        egui::RichText::new(rest.trim())
                            .color(egui::Color32::from_rgb(0, 180, 0))
                            .size(18.0)
                            .strong(),
                    );
                } else if let Some(rest) = line.strip_prefix("[PUBSUB]:") {
                    ui.label(
                        egui::RichText::new("[PUBSUB]:")
                            .color(egui::Color32::GRAY)
                            .size(16.0)
                            .strong(),
                    );
                    ui.label(
                        egui::RichText::new(rest.trim())
                            .color(egui::Color32::from_rgb(0, 180, 0))
                            .size(18.0)
                            .strong(),
                    );
                } else {
                    // Otros mensajes
                    ui.label(
                        egui::RichText::new(line)
                            .color(egui::Color32::GRAY)
                            .size(12.0),
                    );
                }
            }

            // input de comando
            let response = ui.add(
                egui::TextEdit::singleline(&mut self.input)
                    .hint_text("Comando...")
                    .desired_width(400.0),
            );
            if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                let cmd = self.input.trim().to_string();
                if !cmd.is_empty() {
                    self.tx.send(cmd.clone()).ok();
                    self.historial.lock().unwrap().push(format!("> {}", cmd));
                    self.input.clear();
                }
            }

            // Lectura de respuestas del thread
            while let Ok(resp) = self.rx.try_recv() {
                self.historial.lock().unwrap().push(format!("→ {}", resp));
            }
        });
    }
}
