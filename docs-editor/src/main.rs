use std::{
    net::{Ipv4Addr, SocketAddr},
    sync::Arc,
};

use docs_editor::DocsEditorApp;
use eframe::{
    NativeOptions,
    egui::{Vec2, ViewportBuilder},
};

fn main() {
    let icon = match eframe::icon_data::from_png_bytes(include_bytes!("../assets/docs-editor.png"))
    {
        Ok(icon) => icon,
        Err(_) => {
            print!(
                "{}",
                log::error!("no se pudo procesar icono de la aplicación")
            );
            return;
        }
    };

    let opts = NativeOptions {
        viewport: ViewportBuilder {
            title: Some("Editor de Documentos".to_string()),
            active: Some(true),
            icon: Some(Arc::new(icon)),
            inner_size: Some(Vec2::new(760.0, 675.0)),
            ..Default::default()
        },
        ..Default::default()
    };

    let port: u16 = match std::env::var("REDIS_PORT") {
        Ok(port) => match port.parse() {
            Ok(port) => port,
            Err(err) => {
                print!("{}", log::error!("puerto inválido: {err}"));
                return;
            }
        },
        Err(_) => 6379,
    };

    let db_addr = SocketAddr::new(Ipv4Addr::new(0, 0, 0, 0).into(), port);

    if let Err(err) = eframe::run_native(
        "docs-editor",
        opts,
        Box::new(|cc| Ok(Box::new(DocsEditorApp::new(db_addr, cc)?))),
    ) {
        print!("{}", log::error!("error inicializando editor: {err}"));
    }
}
