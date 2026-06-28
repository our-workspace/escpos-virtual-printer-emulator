use crate::config::{AppConfig, AppMode};
use crate::emulator::EmulatorState;
use egui::{Color32, RichText, Ui};
use std::path::PathBuf;

pub struct SettingsPanel {
    status_message: Option<(String, bool)>, // (message, is_success)
    pdf_path_input: String,
}

impl Default for SettingsPanel {
    fn default() -> Self {
        let config = AppConfig::load();
        Self {
            status_message: None,
            pdf_path_input: config.pdf_save_path.to_string_lossy().to_string(),
        }
    }
}

impl SettingsPanel {
    pub fn show(&mut self, ui: &mut Ui, state: &mut EmulatorState) {
        ui.heading("⚙️ Settings");
        ui.separator();

        // ── Mode Selection ──
        ui.group(|ui| {
            ui.label(RichText::new("🔄 Operating Mode").strong());
            ui.add_space(4.0);

            let mut mode = state.mode.clone();
            ui.horizontal(|ui| {
                ui.radio_value(&mut mode, AppMode::PrintAndView, "🖨️ Print & View");
                ui.radio_value(&mut mode, AppMode::SaveAsPdf, "📄 Save as PDF");
            });

            match &mode {
                AppMode::PrintAndView => {
                    ui.label(RichText::new("Display receipts in the viewer without saving")
                        .size(11.0).color(Color32::GRAY));
                }
                AppMode::SaveAsPdf => {
                    ui.label(RichText::new("Save each receipt as PDF — organized by device IP")
                        .size(11.0).color(Color32::GRAY));
                }
            }

            if mode != state.mode {
                state.set_mode(mode);
            }
        });

        ui.add_space(6.0);

        // ── PDF Save Path (only in SaveAsPdf mode) ──
        if state.mode == AppMode::SaveAsPdf {
            ui.group(|ui| {
                ui.label(RichText::new("📁 PDF Save Location").strong());
                ui.add_space(4.0);

                ui.horizontal(|ui| {
                    ui.text_edit_singleline(&mut self.pdf_path_input);
                    if ui.button("📂 Browse").clicked() {
                        // Use the current path as starting point
                        if let Some(path) = rfd_pick_folder(&self.pdf_path_input) {
                            self.pdf_path_input = path;
                        }
                    }
                    if ui.button("✅ Apply").clicked() {
                        let path = PathBuf::from(&self.pdf_path_input);
                        state.set_pdf_save_path(path);
                        self.status_message = Some(("PDF save path updated".to_string(), true));
                    }
                });

                ui.label(RichText::new("Receipts will be saved to: {device_ip}/receipt_{timestamp}.pdf")
                    .size(10.0).color(Color32::GRAY));
            });

            ui.add_space(6.0);
        }

        // ── Paper Width ──
        ui.group(|ui| {
            ui.label(RichText::new("📄 Paper Width").strong());
            ui.add_space(4.0);

            let current_width = match &state.printer_state.paper_width {
                crate::escpos::printer::PaperWidth::Width50mm => 50u32,
                crate::escpos::printer::PaperWidth::Width78mm => 78,
                crate::escpos::printer::PaperWidth::Width80mm => 80,
            };

            let mut selected = current_width;
            ui.horizontal(|ui| {
                ui.radio_value(&mut selected, 50, "50mm");
                ui.radio_value(&mut selected, 78, "78mm");
                ui.radio_value(&mut selected, 80, "80mm");
            });

            if selected != current_width {
                state.set_paper_width(selected);
            }
        });

        ui.add_space(6.0);

        // ── Virtual Printer Management ──
        ui.group(|ui| {
            ui.label(RichText::new("🖨️ Virtual Printer Management").strong());
            ui.add_space(4.0);

            ui.horizontal(|ui| {
                if ui.button("📥 Install Windows Printer").clicked() {
                    self.install_windows_printer();
                }
                if ui.button("🗑️ Uninstall Printer").clicked() {
                    self.uninstall_printer();
                }
                if ui.button("🔍 Check Status").clicked() {
                    self.check_printer_status();
                }
            });

            ui.label(RichText::new("Note: Requires administrator privileges")
                .size(10.0).color(Color32::GRAY));
        });

        ui.add_space(6.0);

        // ── Network Info ──
        ui.group(|ui| {
            ui.label(RichText::new("📡 Network Information").strong());
            ui.add_space(4.0);

            ui.horizontal(|ui| {
                ui.label("TCP Port:");
                ui.label(RichText::new("9100").monospace().strong());
                ui.label("  |  Bind Address:");
                ui.label(RichText::new("0.0.0.0 (all interfaces)").monospace().strong());
            });

            // Show local IP addresses
            ui.horizontal(|ui| {
                ui.label("Local IP:");
                if let Ok(hostname) = hostname::get() {
                    ui.label(RichText::new(hostname.to_string_lossy().to_string())
                        .monospace().color(Color32::from_rgb(130, 170, 255)));
                }
            });

            if ui.button("📡 Test Connection").clicked() {
                self.test_connection();
            }
        });

        ui.add_space(6.0);

        // ── Status Message ──
        if let Some((msg, is_success)) = &self.status_message {
            let color = if *is_success {
                Color32::from_rgb(100, 200, 100)
            } else {
                Color32::from_rgb(200, 100, 100)
            };
            ui.label(RichText::new(msg).color(color));
        }

        // ── Save Config ──
        ui.add_space(6.0);
        if ui.button("💾 Save Settings").clicked() {
            let config = AppConfig {
                mode: state.mode.clone(),
                server_port: 9100,
                bind_address: "0.0.0.0".to_string(),
                paper_width_mm: match &state.printer_state.paper_width {
                    crate::escpos::printer::PaperWidth::Width50mm => 50,
                    crate::escpos::printer::PaperWidth::Width78mm => 78,
                    crate::escpos::printer::PaperWidth::Width80mm => 80,
                },
                pdf_save_path: state.pdf_save_path.clone(),
                max_history_size: state.max_history_size,
            };
            match config.save() {
                Ok(()) => {
                    self.status_message = Some(("✅ Settings saved successfully".to_string(), true));
                }
                Err(e) => {
                    self.status_message = Some((format!("❌ Failed to save: {}", e), false));
                }
            }
        }
    }

    fn install_windows_printer(&mut self) {
        let output = std::process::Command::new("powershell")
            .args([
                "-Command",
                "Add-PrinterPort -Name '0.0.0.0:9100' -PrinterHostAddress '127.0.0.1' -PortNumber 9100; \
                 $driver = (Get-PrinterDriver | Where-Object { $_.Name -like '*Microsoft*' } | Select-Object -First 1).Name; \
                 Add-Printer -Name 'ESC_POS_Virtual_Printer' -DriverName $driver -PortName '0.0.0.0:9100'; \
                 Write-Host 'OK'"
            ])
            .output();

        match output {
            Ok(o) if o.status.success() => {
                self.status_message = Some(("✅ Printer installed successfully".to_string(), true));
            }
            Ok(o) => {
                let err = String::from_utf8_lossy(&o.stderr).to_string();
                self.status_message = Some((format!("❌ {}", err), false));
            }
            Err(e) => {
                self.status_message = Some((format!("❌ {}", e), false));
            }
        }
    }

    fn uninstall_printer(&mut self) {
        let output = std::process::Command::new("powershell")
            .args([
                "-Command",
                "Remove-Printer -Name 'ESC_POS_Virtual_Printer' -Confirm:$false -ErrorAction SilentlyContinue; \
                 Remove-PrinterPort -Name '0.0.0.0:9100' -ErrorAction SilentlyContinue; \
                 Write-Host 'OK'"
            ])
            .output();

        match output {
            Ok(o) if o.status.success() => {
                self.status_message = Some(("✅ Printer uninstalled".to_string(), true));
            }
            Ok(o) => {
                let err = String::from_utf8_lossy(&o.stderr).to_string();
                self.status_message = Some((format!("❌ {}", err), false));
            }
            Err(e) => {
                self.status_message = Some((format!("❌ {}", e), false));
            }
        }
    }

    fn check_printer_status(&mut self) {
        let output = std::process::Command::new("powershell")
            .args([
                "-Command",
                "Get-Printer -Name 'ESC_POS_Virtual_Printer' -ErrorAction SilentlyContinue | Format-List Name, PortName, DriverName, PrinterStatus"
            ])
            .output();

        match output {
            Ok(o) if o.status.success() => {
                let stdout = String::from_utf8_lossy(&o.stdout).trim().to_string();
                if stdout.is_empty() {
                    self.status_message = Some(("ℹ️ Virtual printer not installed".to_string(), false));
                } else {
                    self.status_message = Some((format!("✅ Installed: {}", stdout), true));
                }
            }
            _ => {
                self.status_message = Some(("ℹ️ Virtual printer not installed".to_string(), false));
            }
        }
    }

    fn test_connection(&mut self) {
        let output = std::process::Command::new("powershell")
            .args([
                "-Command",
                "(Test-NetConnection -ComputerName 127.0.0.1 -Port 9100 -WarningAction SilentlyContinue).TcpTestSucceeded"
            ])
            .output();

        match output {
            Ok(o) if o.status.success() => {
                let stdout = String::from_utf8_lossy(&o.stdout).trim().to_string();
                if stdout.contains("True") {
                    self.status_message = Some(("✅ Connection to port 9100 successful".to_string(), true));
                } else {
                    self.status_message = Some(("❌ Connection to port 9100 failed".to_string(), false));
                }
            }
            _ => {
                self.status_message = Some(("❌ Cannot test connection".to_string(), false));
            }
        }
    }
}

/// Simple folder picker fallback (returns None if no GUI picker available)
fn rfd_pick_folder(start: &str) -> Option<String> {
    // Open explorer to select folder via PowerShell
    let output = std::process::Command::new("powershell")
        .args([
            "-Command",
            &format!(
                "Add-Type -AssemblyName System.Windows.Forms; \
                 $f = New-Object System.Windows.Forms.FolderBrowserDialog; \
                 $f.SelectedPath = '{}'; \
                 if ($f.ShowDialog() -eq 'OK') {{ Write-Host $f.SelectedPath }}",
                start
            )
        ])
        .output()
        .ok()?;

    let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if path.is_empty() { None } else { Some(path) }
}
