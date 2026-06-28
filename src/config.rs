use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Application operating mode
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AppMode {
    /// Display receipts in GUI without saving
    PrintAndView,
    /// Save each receipt as PDF, organized by device IP
    SaveAsPdf,
}

impl Default for AppMode {
    fn default() -> Self {
        AppMode::PrintAndView
    }
}

/// Persistent application configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub mode: AppMode,
    pub server_port: u16,
    pub bind_address: String,
    pub paper_width_mm: u32,
    pub pdf_save_path: PathBuf,
    pub max_history_size: usize,
}

impl Default for AppConfig {
    fn default() -> Self {
        let default_save_path = dirs::document_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("ESC_POS_Receipts");

        Self {
            mode: AppMode::PrintAndView,
            server_port: 9100,
            bind_address: "0.0.0.0".to_string(),
            paper_width_mm: 80,
            pdf_save_path: default_save_path,
            max_history_size: 1000,
        }
    }
}

impl AppConfig {
    pub fn config_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("escpos_emulator")
            .join("config.json")
    }

    pub fn load() -> Self {
        let path = Self::config_path();
        if path.exists() {
            if let Ok(data) = std::fs::read_to_string(&path) {
                if let Ok(config) = serde_json::from_str(&data) {
                    return config;
                }
            }
        }
        Self::default()
    }

    pub fn save(&self) -> anyhow::Result<()> {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let data = serde_json::to_string_pretty(self)?;
        std::fs::write(&path, data)?;
        Ok(())
    }
}
