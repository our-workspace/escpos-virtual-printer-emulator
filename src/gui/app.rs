use crate::config::AppMode;
use crate::emulator::EmulatorState;
use crate::gui::{CommandLog, ReceiptViewer, SaveStatusPanel, SettingsPanel};
use eframe::egui::{self, CentralPanel, Color32, RichText, TopBottomPanel};
use std::time::Duration;

#[derive(Debug, Clone, PartialEq)]
pub enum Tab {
    Receipt,
    Commands,
    Settings,
}

impl Default for Tab {
    fn default() -> Self {
        Tab::Receipt
    }
}

pub struct EscPosEmulatorApp {
    pub emulator_state: std::sync::Arc<tokio::sync::Mutex<EmulatorState>>,
    selected_tab: Tab,
    receipt_viewer: ReceiptViewer,
    command_log: CommandLog,
    settings_panel: SettingsPanel,
    save_status_panel: SaveStatusPanel,
}

impl Default for EscPosEmulatorApp {
    fn default() -> Self {
        Self {
            emulator_state: std::sync::Arc::new(tokio::sync::Mutex::new(EmulatorState::new())),
            selected_tab: Tab::Receipt,
            receipt_viewer: ReceiptViewer::new(),
            command_log: CommandLog::new(),
            settings_panel: SettingsPanel::default(),
            save_status_panel: SaveStatusPanel::new(),
        }
    }
}

impl EscPosEmulatorApp {
    pub fn new(emulator_state: std::sync::Arc<tokio::sync::Mutex<EmulatorState>>) -> Self {
        Self {
            emulator_state,
            ..Default::default()
        }
    }
}

impl eframe::App for EscPosEmulatorApp {
    fn update(&mut self, ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) {
        // Auto-repaint for real-time updates
        ctx.request_repaint_after(Duration::from_millis(200));

        self.show(ctx);
    }
}

impl EscPosEmulatorApp {
    fn show(&mut self, ctx: &eframe::egui::Context) {
        // ── Top Panel: Tabs + Mode indicator ──
        TopBottomPanel::top("tabs").show(ctx, |ui| {
            ui.horizontal(|ui| {
                // Mode indicator
                if let Ok(state) = self.emulator_state.try_lock() {
                    let (mode_icon, mode_text, mode_color) = match state.mode {
                        AppMode::PrintAndView => ("🖨️", "Print & View", Color32::from_rgb(100, 200, 100)),
                        AppMode::SaveAsPdf => ("📄", "Save as PDF", Color32::from_rgb(100, 150, 255)),
                    };
                    ui.label(RichText::new(format!("{} {}", mode_icon, mode_text))
                        .color(mode_color)
                        .strong()
                        .size(12.0));
                    ui.separator();
                }

                // Tab buttons
                ui.selectable_value(&mut self.selected_tab, Tab::Receipt,
                    RichText::new("🖨️ Receipt").size(13.0));
                ui.selectable_value(&mut self.selected_tab, Tab::Commands,
                    RichText::new("📋 Commands").size(13.0));
                ui.selectable_value(&mut self.selected_tab, Tab::Settings,
                    RichText::new("⚙️ Settings").size(13.0));
            });
        });

        // ── Bottom Panel: Status Bar ──
        TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if let Ok(state) = self.emulator_state.try_lock() {
                    // Server status
                    ui.label(RichText::new("🟢 Listening").color(Color32::from_rgb(100, 200, 100)).size(11.0));
                    ui.separator();

                    // Connections
                    let conn_color = if state.active_connections > 0 {
                        Color32::from_rgb(100, 200, 255)
                    } else {
                        Color32::GRAY
                    };
                    ui.label(RichText::new(format!("📡 {} connections", state.active_connections))
                        .color(conn_color).size(11.0));
                    ui.separator();

                    // Commands count
                    ui.label(RichText::new(format!("📊 {} commands", state.total_commands))
                        .size(11.0).color(Color32::GRAY));
                    ui.separator();

                    // Mode-specific info
                    match state.mode {
                        AppMode::PrintAndView => {
                            let lines = state.printer_state.get_buffer().len();
                            ui.label(RichText::new(format!("📃 {} lines", lines))
                                .size(11.0).color(Color32::GRAY));
                        }
                        AppMode::SaveAsPdf => {
                            let pdfs = state.saved_pdfs.len();
                            ui.label(RichText::new(format!("📄 {} PDFs saved", pdfs))
                                .size(11.0).color(Color32::from_rgb(100, 150, 255)));
                        }
                    }

                    // Uptime
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if let Ok(uptime) = state.start_time.elapsed() {
                            let secs = uptime.as_secs();
                            let uptime_str = if secs < 60 {
                                format!("{}s", secs)
                            } else if secs < 3600 {
                                format!("{}m {}s", secs / 60, secs % 60)
                            } else {
                                format!("{}h {}m", secs / 3600, (secs % 3600) / 60)
                            };
                            ui.label(RichText::new(format!("🕐 {}", uptime_str))
                                .size(11.0).color(Color32::GRAY));
                        }
                    });
                }
            });
        });

        // ── Central Panel: Content ──
        CentralPanel::default().show(ctx, |ui| {
            match self.selected_tab {
                Tab::Receipt => {
                    // In PDF mode, show save status instead of receipt viewer
                    if let Ok(state) = self.emulator_state.try_lock() {
                        if state.mode == AppMode::SaveAsPdf {
                            drop(state);
                            self.save_status_panel.show(ui, &self.emulator_state);
                            return;
                        }
                    }
                    self.receipt_viewer.show(ui, &self.emulator_state);
                }
                Tab::Commands => {
                    self.command_log.show(ui, &self.emulator_state);
                }
                Tab::Settings => {
                    if let Ok(mut state) = self.emulator_state.try_lock() {
                        self.settings_panel.show(ui, &mut state);
                    }
                }
            }
        });
    }
}
