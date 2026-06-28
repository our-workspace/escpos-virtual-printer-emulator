use crate::emulator::EmulatorState;
use egui::{RichText, ScrollArea, Ui};
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct CommandLog {
    show_timestamps: bool,
    show_device_ip: bool,
    max_display_lines: usize,
    filter_text: String,
}

impl Default for CommandLog {
    fn default() -> Self {
        Self {
            show_timestamps: true,
            show_device_ip: true,
            max_display_lines: 1000,
            filter_text: String::new(),
        }
    }
}

impl CommandLog {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn show(&mut self, ui: &mut Ui, emulator_state: &Arc<Mutex<EmulatorState>>) {
        ui.heading("📋 Command Log");
        ui.separator();

        // Controls
        ui.horizontal(|ui| {
            ui.checkbox(&mut self.show_timestamps, "Timestamps");
            ui.checkbox(&mut self.show_device_ip, "Device IP");
            ui.label("Filter:");
            ui.text_edit_singleline(&mut self.filter_text);

            if ui.button("🗑️ Clear").clicked() {
                if let Ok(mut state) = emulator_state.try_lock() {
                    state.clear_history();
                }
            }
        });

        ui.separator();

        ScrollArea::vertical().show(ui, |ui| {
            if let Ok(state) = emulator_state.try_lock() {
                self.render_command_list(ui, &state);
            } else {
                ui.label("Cannot load emulator state");
            }
        });
    }

    fn render_command_list(&self, ui: &mut Ui, state: &EmulatorState) {
        let history = state.get_command_history();

        if history.is_empty() {
            ui.label("No commands received");
            return;
        }

        // Apply filter
        let filtered_commands: Vec<_> = history.iter()
            .filter(|entry| {
                if self.filter_text.is_empty() {
                    return true;
                }
                let filter_lower = self.filter_text.to_lowercase();
                match &entry.command {
                    crate::escpos::commands::EscPosCommand::Text(text) => {
                        text.to_lowercase().contains(&filter_lower)
                    }
                    _ => {
                        format!("{:?}", entry.command).to_lowercase().contains(&filter_lower)
                    }
                }
            })
            .collect();

        let display_commands: Vec<_> = filtered_commands.iter()
            .rev()
            .take(self.max_display_lines)
            .collect();

        let display_count = display_commands.len();

        for entry in &display_commands {
            ui.horizontal(|ui| {
                // Timestamp — use chrono for proper display
                if self.show_timestamps {
                    if let Ok(duration) = entry.timestamp.elapsed() {
                        let secs = duration.as_secs();
                        let time_str = if secs < 60 {
                            format!("{}s ago", secs)
                        } else if secs < 3600 {
                            format!("{}m {}s ago", secs / 60, secs % 60)
                        } else {
                            format!("{}h {}m ago", secs / 3600, (secs % 3600) / 60)
                        };
                        ui.label(RichText::new(format!("⏰ {}", time_str))
                            .size(10.0)
                            .color(egui::Color32::GRAY));
                    }
                }

                // Device IP
                if self.show_device_ip {
                    ui.label(RichText::new(format!("[{}]", entry.device_ip))
                        .size(10.0)
                        .color(egui::Color32::from_rgb(130, 170, 255)));
                }

                // Command description
                let command_text = match &entry.command {
                    crate::escpos::commands::EscPosCommand::Text(text) => {
                        format!("📝 {}", text)
                    }
                    crate::escpos::commands::EscPosCommand::NewLine => "↵ NewLine".to_string(),
                    crate::escpos::commands::EscPosCommand::LineFeed => "⏬ LineFeed".to_string(),
                    crate::escpos::commands::EscPosCommand::CarriageReturn => "↩️ CR".to_string(),
                    crate::escpos::commands::EscPosCommand::SetFont(font) => {
                        format!("🔤 Font: {:?}", font)
                    }
                    crate::escpos::commands::EscPosCommand::SetJustification(just) => {
                        format!("📐 Align: {:?}", just)
                    }
                    crate::escpos::commands::EscPosCommand::SetEmphasis(on) => {
                        format!("💪 Bold: {}", if *on { "ON" } else { "OFF" })
                    }
                    crate::escpos::commands::EscPosCommand::SetUnderline(on) => {
                        format!("➖ Underline: {}", if *on { "ON" } else { "OFF" })
                    }
                    crate::escpos::commands::EscPosCommand::SetItalic(on) => {
                        format!("📝 Italic: {}", if *on { "ON" } else { "OFF" })
                    }
                    crate::escpos::commands::EscPosCommand::CutPaper => "✂️ Cut Paper".to_string(),
                    crate::escpos::commands::EscPosCommand::PrintImage(_) => "🖼️ Bit Image (ESC *)".to_string(),
                    crate::escpos::commands::EscPosCommand::PrintRasterImage { width_bytes, height, .. } => {
                        format!("🖼️ Raster {}×{}", width_bytes * 8, height)
                    }
                    crate::escpos::commands::EscPosCommand::SetCodepage(cp) => format!("🌐 CP{}", cp),
                    crate::escpos::commands::EscPosCommand::SetLineHeight(h) => format!("📏 LineH: {}", h),
                    crate::escpos::commands::EscPosCommand::SetFontSize(s) => format!("🔤 Size: {}", s),
                    crate::escpos::commands::EscPosCommand::InitializePrinter => "🔄 Init Printer".to_string(),
                    crate::escpos::commands::EscPosCommand::Unknown(d) => format!("❓ Unknown ({} bytes)", d.len()),
                };
                ui.label(command_text);
            });
        }

        ui.separator();
        ui.label(format!("Total: {} | Displayed: {} | Filtered: {}",
            history.len(), display_count, filtered_commands.len()));
    }
}
