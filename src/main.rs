use escpos_emulator::config::{AppConfig, AppMode};
use escpos_emulator::emulator::EmulatorState;
use escpos_emulator::gui::EscPosEmulatorApp;
use escpos_emulator::networking::server;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{info, Level};

#[tokio::main]
async fn main() -> Result<(), eframe::Error> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .init();

    info!("🚀 Starting ESC/POS Emulator v2.0...");

    // Load saved configuration
    let config = AppConfig::load();
    info!("📋 Mode: {:?}", config.mode);

    // Create shared emulator state
    let mut emulator = EmulatorState::new();
    emulator.set_mode(config.mode.clone());
    emulator.set_pdf_save_path(config.pdf_save_path.clone());
    emulator.set_paper_width(config.paper_width_mm);
    emulator.max_history_size = config.max_history_size;

    let emulator_state = Arc::new(Mutex::new(emulator));

    // Start network server in background
    let server_state = emulator_state.clone();
    tokio::spawn(async move {
        if let Err(e) = server::start_server(server_state).await {
            eprintln!("❌ Server error: {}", e);
        }
    });

    info!("✅ Emulator initialized successfully");

    // Launch GUI
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("ESC/POS Virtual Printer Emulator v2.0")
            .with_inner_size([800.0, 600.0])
            .with_min_inner_size([600.0, 400.0]),
        ..Default::default()
    };

    eframe::run_native(
        "ESC/POS Virtual Printer Emulator",
        options,
        Box::new(|_cc| Box::new(EscPosEmulatorApp::new(emulator_state))),
    )
}
