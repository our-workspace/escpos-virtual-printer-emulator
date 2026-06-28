use crate::emulator::{EmulatorState, SaveStatus};
use egui::{Color32, RichText, ScrollArea, Ui};
use std::sync::Arc;
use tokio::sync::Mutex;

/// Panel displaying PDF save status and history
pub struct SaveStatusPanel;

impl Default for SaveStatusPanel {
    fn default() -> Self {
        Self
    }
}

impl SaveStatusPanel {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn show(&mut self, ui: &mut Ui, emulator_state: &Arc<Mutex<EmulatorState>>) {
        ui.heading("📄 Save as PDF — Status");
        ui.separator();

        if let Ok(mut state) = emulator_state.try_lock() {
            // Summary stats
            let total = state.saved_pdfs.len();
            let success_count = state.saved_pdfs.iter()
                .filter(|p| matches!(p.status, SaveStatus::Success))
                .count();
            let failed_count = total - success_count;

            ui.horizontal(|ui| {
                ui.label(RichText::new(format!("✅ Saved: {}", success_count)).color(Color32::from_rgb(100, 200, 100)));
                ui.label(RichText::new(format!("❌ Failed: {}", failed_count)).color(Color32::from_rgb(200, 100, 100)));
                ui.label(format!("📊 Total: {}", total));

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("🗑️ Clear Log").clicked() {
                        state.clear_saved_pdfs();
                    }
                    if ui.button("📂 Open Folder").clicked() {
                        let path = state.pdf_save_path.clone();
                        let _ = std::fs::create_dir_all(&path);
                        #[cfg(target_os = "windows")]
                        {
                            let _ = std::process::Command::new("explorer")
                                .arg(&path)
                                .spawn();
                        }
                    }
                });
            });

            ui.separator();

            // Save path display
            ui.horizontal(|ui| {
                ui.label("📁 Save path:");
                ui.label(RichText::new(state.pdf_save_path.to_string_lossy().to_string()).monospace());
            });

            ui.separator();

            // PDF save log
            if state.saved_pdfs.is_empty() {
                ui.vertical_centered(|ui| {
                    ui.add_space(40.0);
                    ui.label(RichText::new("📋 No receipts saved yet").size(16.0).color(Color32::GRAY));
                    ui.add_space(10.0);
                    ui.label("Receipts will be automatically saved as PDF when received");
                    ui.label("Each device's receipts are organized in separate folders by IP address");
                });
            } else {
                ScrollArea::vertical().show(ui, |ui| {
                    // Show most recent first
                    for entry in state.saved_pdfs.iter().rev() {
                        ui.group(|ui| {
                            ui.horizontal(|ui| {
                                // Status icon
                                let (icon, color) = match &entry.status {
                                    SaveStatus::Success => ("✅", Color32::from_rgb(100, 200, 100)),
                                    SaveStatus::Failed(_) => ("❌", Color32::from_rgb(200, 100, 100)),
                                };
                                ui.label(RichText::new(icon).size(14.0));

                                // Timestamp
                                ui.label(RichText::new(&entry.timestamp).monospace().size(11.0));

                                // Device IP
                                ui.label(RichText::new(format!("📱 {}", entry.device_ip))
                                    .color(Color32::from_rgb(130, 170, 255)));

                                // Line count
                                ui.label(format!("({} lines)", entry.line_count));
                            });

                            // File path or error
                            match &entry.status {
                                SaveStatus::Success => {
                                    ui.horizontal(|ui| {
                                        ui.label("   📄");
                                        ui.label(RichText::new(&entry.file_path)
                                            .monospace()
                                            .size(10.0)
                                            .color(Color32::GRAY));
                                    });
                                }
                                SaveStatus::Failed(err) => {
                                    ui.horizontal(|ui| {
                                        ui.label("   ⚠️");
                                        ui.label(RichText::new(err)
                                            .size(10.0)
                                            .color(Color32::from_rgb(255, 150, 150)));
                                    });
                                }
                            }
                        });
                    }
                });
            }
        } else {
            ui.label("Cannot load emulator state");
        }
    }
}
