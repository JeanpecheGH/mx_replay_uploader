mod app;
mod gbx_parser;
mod mx_client;
mod pref;
mod watcher;

use crate::app::ReplayUploaderApp;
use crate::pref::Pref;
use eframe::{NativeOptions, egui};
use egui::{IconData, Vec2, ViewportBuilder};
use std::sync::Arc;

const WIDTH: f32 = 700.0;
const HEIGHT: f32 = 500.0;
const VEC2_SIZE: Vec2 = Vec2 {
    x: WIDTH,
    y: HEIGHT,
};

#[tokio::main]
async fn main() -> eframe::Result {
    let prefs = pref::load_pref().unwrap_or_else(|e| {
        eprintln!("Error loading preferences : {}", e);
        Pref::default()
    });
    let app = ReplayUploaderApp::with_pref(prefs);

    //Include the icon directly into the binary
    let icon_bytes = include_bytes!("..\\media\\icon-128.png");
    let icon = image::load_from_memory(icon_bytes)
        .expect("Failed to load icon data")
        .to_rgba8();
    let (icon_width, icon_height) = icon.dimensions();

    let viewport = ViewportBuilder::default()
        .with_inner_size(VEC2_SIZE)
        .with_min_inner_size(VEC2_SIZE)
        .with_icon(Arc::new(IconData {
            rgba: icon.into_raw(),
            width: icon_width,
            height: icon_height,
        }));

    let options = NativeOptions {
        viewport,
        ..NativeOptions::default()
    };
    eframe::run_native(
        "ManiaExchange Replay Uploader",
        options,
        Box::new(|_cc| Ok(Box::new(app))),
    )
}
