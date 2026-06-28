use crate::emulator::EmulatorState;
use crate::escpos::printer::{PrinterState, ReceiptLine};
use egui::{Color32, ColorImage, RichText, ScrollArea, TextureHandle, TextureOptions, Ui};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct ReceiptViewer {
    show_paper_edges: bool,
    show_grid: bool,
    bitmap_cache: HashMap<u64, TextureHandle>,
}

impl Default for ReceiptViewer {
    fn default() -> Self {
        Self {
            show_paper_edges: true,
            show_grid: false,
            bitmap_cache: HashMap::new(),
        }
    }
}

fn hash_bytes(data: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for &byte in data {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

impl ReceiptViewer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn show(&mut self, ui: &mut Ui, emulator_state: &Arc<Mutex<EmulatorState>>) {
        ui.heading("🖨️ Receipt Viewer");
        ui.separator();

        // Controls
        ui.horizontal(|ui| {
            ui.checkbox(&mut self.show_paper_edges, "Show paper edges");
            ui.checkbox(&mut self.show_grid, "Show grid");

            if ui.button("🗑️ Clear").clicked() {
                if let Ok(mut state) = emulator_state.try_lock() {
                    state.clear_printer_buffer();
                }
                self.bitmap_cache.clear();
            }
        });

        ui.separator();

        // Receipt display
        ScrollArea::both().show(ui, |ui| {
            if let Ok(state) = emulator_state.try_lock() {
                self.render_receipt(ui, &state);
            } else {
                ui.label("Cannot load emulator state");
            }
        });
    }

    fn render_receipt(&mut self, ui: &mut Ui, state: &EmulatorState) {
        let printer_state = state.get_printer_state();
        let buffer = printer_state.get_buffer();

        if buffer.is_empty() {
            ui.vertical_centered(|ui| {
                ui.add_space(40.0);
                ui.label(RichText::new("📋 No receipt data").size(16.0).color(Color32::GRAY));
                ui.add_space(10.0);
                ui.label("Send ESC/POS commands to see the receipt here");
            });
            return;
        }

        let max_chars = printer_state.paper_width.get_max_chars(printer_state.font_size);

        // Paper simulation frame
        let frame = egui::Frame::none()
            .fill(Color32::WHITE)
            .inner_margin(egui::Margin::same(8.0))
            .stroke(if self.show_paper_edges {
                egui::Stroke::new(1.0, Color32::from_gray(200))
            } else {
                egui::Stroke::NONE
            })
            .shadow(if self.show_paper_edges {
                egui::epaint::Shadow {
                    offset: egui::vec2(2.0, 2.0),
                    blur: 4.0,
                    spread: 0.0,
                    color: Color32::from_black_alpha(30),
                }
            } else {
                egui::epaint::Shadow::NONE
            });

        frame.show(ui, |ui| {
            // Paper header info
            ui.horizontal(|ui| {
                ui.label(RichText::new(format!("📄 {:?}", printer_state.paper_width)).size(10.0).color(Color32::GRAY));
                ui.label(RichText::new(format!("🔤 {:?}", printer_state.current_font)).size(10.0).color(Color32::GRAY));
                if printer_state.codepage != 0 {
                    ui.label(RichText::new(format!("🌐 CP{}", printer_state.codepage)).size(10.0).color(Color32::GRAY));
                }
            });
            ui.separator();

            // Render each line with its own formatting
            for (line_num, line) in buffer.iter().enumerate() {
                match line {
                    ReceiptLine::Text(text_line) => {
                        if !text_line.content.is_empty() {
                            // Build layout based on justification
                            let layout = match text_line.justification {
                                crate::escpos::commands::Justification::Left => {
                                    egui::Layout::left_to_right(egui::Align::Min)
                                }
                                crate::escpos::commands::Justification::Center => {
                                    egui::Layout::top_down(egui::Align::Center)
                                }
                                crate::escpos::commands::Justification::Right => {
                                    egui::Layout::right_to_left(egui::Align::Min)
                                }
                            };

                            ui.with_layout(layout, |ui| {
                                // Build rich text with formatting
                                let mut rt = RichText::new(&text_line.content).monospace();

                                if text_line.emphasis {
                                    rt = rt.strong();
                                }
                                if text_line.underline {
                                    rt = rt.underline();
                                }
                                if text_line.italic {
                                    rt = rt.italics();
                                }

                                // Font size scaling
                                let base_size = match text_line.font_size {
                                    0..=8 => 11.0,
                                    9..=12 => 13.0,
                                    13..=20 => 15.0,
                                    21..=32 => 18.0,
                                    _ => 22.0,
                                };
                                rt = rt.size(base_size);
                                rt = rt.color(Color32::BLACK);

                                ui.label(rt);
                            });
                        } else {
                            ui.label("");
                        }
                    }
                    ReceiptLine::Bitmap { width_px, height_px, data } => {
                        self.render_bitmap(ui, *width_px, *height_px, data);
                    }
                    ReceiptLine::Separator => {
                        let sep = "─".repeat(max_chars as usize);
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(&sep).monospace().color(Color32::GRAY));
                        });
                    }
                }
            }

            // Paper footer
            ui.separator();
            ui.label(RichText::new("✂️ ─ ─ ─ ─ ─ ─ ─ ─ ─").color(Color32::GRAY).size(10.0));
        });
    }

    fn render_bitmap(&mut self, ui: &mut Ui, width_px: u32, height_px: u32, data: &[u8]) {
        let cache_key = hash_bytes(data);

        let texture = self.bitmap_cache.entry(cache_key).or_insert_with(|| {
            let rgb_image = PrinterState::bitmap_to_rgb(width_px, height_px, data);
            let size = [rgb_image.width() as usize, rgb_image.height() as usize];
            let pixels: Vec<egui::Color32> = rgb_image
                .pixels()
                .map(|p| egui::Color32::from_rgb(p[0], p[1], p[2]))
                .collect();
            let color_image = ColorImage { size, pixels };
            ui.ctx().load_texture(
                format!("bitmap_{}", cache_key),
                color_image,
                TextureOptions::NEAREST,
            )
        });

        let scale = (400.0 / width_px as f32).min(1.0);
        let display_size = egui::vec2(width_px as f32 * scale, height_px as f32 * scale);
        ui.image((texture.id(), display_size));
    }
}
