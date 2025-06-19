use std::{
    net::{Ipv4Addr, SocketAddr},
    sync::Arc,
};

use docs_editor::DocsEditorApp;
use eframe::{NativeOptions, egui::ViewportBuilder};

fn main() {
    let icon =
        eframe::icon_data::from_png_bytes(include_bytes!("../assets/docs-editor.png")).unwrap();

    let opts = NativeOptions {
        viewport: ViewportBuilder {
            title: Some("Editor de Documentos".to_string()),
            active: Some(true),
            icon: Some(Arc::new(icon)),
            ..Default::default()
        },
        ..Default::default()
    };

    let db_addr = SocketAddr::new(Ipv4Addr::new(127, 0, 0, 1).into(), 7000);

    eframe::run_native(
        "docs-editor",
        opts,
        Box::new(|cc| Ok(Box::new(DocsEditorApp::new(db_addr, cc)))),
    )
    .unwrap();
}
