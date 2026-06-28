use crate::config::AppMode;
use crate::escpos::commands::EscPosCommand;
use crate::escpos::printer::{PrinterState, PaperWidth};
use crate::export::pdf_export;
use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;
use std::time::SystemTime;
use chrono::Local;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmulatorState {
    pub printer_state: PrinterState,
    pub command_history: VecDeque<CommandEntry>,
    pub max_history_size: usize,
    pub mode: AppMode,
    pub pdf_save_path: PathBuf,
    /// Per-device printer states for PDF mode (keyed by IP)
    #[serde(skip)]
    pub device_printers: HashMap<String, PrinterState>,
    /// Log of saved PDFs
    pub saved_pdfs: Vec<SavedPdfEntry>,
    /// Active TCP connections count
    #[serde(skip)]
    pub active_connections: u32,
    /// Total commands processed
    pub total_commands: u64,
    #[serde(skip, default = "default_system_time")]
    pub start_time: SystemTime,
}

fn default_system_time() -> SystemTime {
    SystemTime::now()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandEntry {
    #[serde(skip, default = "default_system_time")]
    pub timestamp: SystemTime,
    pub command: EscPosCommand,
    pub device_ip: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavedPdfEntry {
    pub timestamp: String,
    pub device_ip: String,
    pub file_path: String,
    pub line_count: usize,
    pub status: SaveStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SaveStatus {
    Success,
    Failed(String),
}

impl EmulatorState {
    pub fn new() -> Self {
        let default_save_path = dirs::document_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("ESC_POS_Receipts");

        Self {
            printer_state: PrinterState::new(),
            command_history: VecDeque::new(),
            max_history_size: 1000,
            mode: AppMode::PrintAndView,
            pdf_save_path: default_save_path,
            device_printers: HashMap::new(),
            saved_pdfs: Vec::new(),
            active_connections: 0,
            total_commands: 0,
            start_time: SystemTime::now(),
        }
    }

    /// Process an incoming ESC/POS command from a specific device
    pub fn process_command(&mut self, command: &EscPosCommand, device_ip: &str) {
        // Record in command history
        let entry = CommandEntry {
            timestamp: SystemTime::now(),
            command: command.clone(),
            device_ip: device_ip.to_string(),
        };
        self.command_history.push_back(entry);
        while self.command_history.len() > self.max_history_size {
            self.command_history.pop_front();
        }
        self.total_commands += 1;

        match self.mode {
            AppMode::PrintAndView => {
                // Normal mode: process into the main printer state for GUI display
                self.printer_state.process_command(command);
            }
            AppMode::SaveAsPdf => {
                // PDF mode: process into per-device printer state
                let device_printer = self.device_printers
                    .entry(device_ip.to_string())
                    .or_insert_with(PrinterState::new);

                device_printer.process_command(command);

                // On CutPaper, save the accumulated receipt as PDF
                if matches!(command, EscPosCommand::CutPaper) {
                    self.save_receipt_pdf(device_ip);
                }
            }
        }
    }

    /// Save the current receipt buffer for a device as a PDF file
    fn save_receipt_pdf(&mut self, device_ip: &str) {
        let now = Local::now();
        let timestamp_file = now.format("%Y-%m-%d_%H-%M-%S%.3f").to_string();
        let timestamp_display = now.format("%Y-%m-%d %H:%M:%S").to_string();

        // Create device-specific folder
        let device_folder = self.pdf_save_path.join(device_ip);
        if let Err(e) = std::fs::create_dir_all(&device_folder) {
            self.saved_pdfs.push(SavedPdfEntry {
                timestamp: timestamp_display,
                device_ip: device_ip.to_string(),
                file_path: String::new(),
                line_count: 0,
                status: SaveStatus::Failed(format!("Cannot create folder: {}", e)),
            });
            return;
        }

        let file_name = format!("receipt_{}.pdf", timestamp_file);
        let file_path = device_folder.join(&file_name);

        // Take the buffer from the device printer
        if let Some(device_printer) = self.device_printers.get_mut(device_ip) {
            let buffer = device_printer.take_buffer();
            let line_count = buffer.len();
            let paper_width = device_printer.paper_width.clone();

            match pdf_export::save_receipt_pdf(&buffer, &file_path, &paper_width) {
                Ok(()) => {
                    self.saved_pdfs.push(SavedPdfEntry {
                        timestamp: timestamp_display,
                        device_ip: device_ip.to_string(),
                        file_path: file_path.to_string_lossy().to_string(),
                        line_count,
                        status: SaveStatus::Success,
                    });
                    tracing::info!("📄 PDF saved: {}", file_path.display());
                }
                Err(e) => {
                    // Put buffer back on failure so data isn't lost
                    self.saved_pdfs.push(SavedPdfEntry {
                        timestamp: timestamp_display,
                        device_ip: device_ip.to_string(),
                        file_path: file_path.to_string_lossy().to_string(),
                        line_count,
                        status: SaveStatus::Failed(e.to_string()),
                    });
                    tracing::error!("❌ PDF save failed: {}", e);
                }
            }
        }
    }

    pub fn get_command_history(&self) -> &VecDeque<CommandEntry> {
        &self.command_history
    }

    pub fn clear_history(&mut self) {
        self.command_history.clear();
    }

    pub fn clear_printer_buffer(&mut self) {
        self.printer_state.clear_buffer();
    }

    pub fn clear_saved_pdfs(&mut self) {
        self.saved_pdfs.clear();
    }

    pub fn get_printer_state(&self) -> &PrinterState {
        &self.printer_state
    }

    pub fn set_paper_width(&mut self, width_mm: u32) {
        let paper_width = match width_mm {
            50 => PaperWidth::Width50mm,
            78 => PaperWidth::Width78mm,
            80 => PaperWidth::Width80mm,
            _ => PaperWidth::Width80mm,
        };
        self.printer_state.set_paper_width(paper_width);
    }

    pub fn set_mode(&mut self, mode: AppMode) {
        self.mode = mode;
    }

    pub fn set_pdf_save_path(&mut self, path: PathBuf) {
        self.pdf_save_path = path;
    }
}
