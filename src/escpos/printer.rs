use crate::escpos::commands::{EscPosCommand, Font, Justification};
use image::{ImageBuffer, Rgb, RgbImage};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PaperWidth {
    Width50mm,  // 384 dots
    Width78mm,  // 576 dots
    Width80mm,  // 640 dots
}

impl PaperWidth {
    pub fn get_width_dots(&self) -> u32 {
        match self {
            PaperWidth::Width50mm => 384,
            PaperWidth::Width78mm => 576,
            PaperWidth::Width80mm => 640,
        }
    }

    pub fn get_max_chars(&self, font_size: u32) -> u32 {
        let dots = self.get_width_dots();
        match font_size {
            0..=12 => dots / 8,
            13..=16 => dots / 10,
            17..=24 => dots / 12,
            _ => dots / 8,
        }
    }
}

/// Text line with per-line formatting snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextLine {
    pub content: String,
    pub font: Font,
    pub font_size: u32,
    pub justification: Justification,
    pub emphasis: bool,
    pub underline: bool,
    pub italic: bool,
    pub line_height: u32,
}

/// A single element in the receipt buffer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ReceiptLine {
    Text(TextLine),
    /// Monochrome bitmap: width in pixels, height in pixels, 1-bit-per-pixel packed data
    Bitmap { width_px: u32, height_px: u32, data: Vec<u8> },
    Separator,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrinterState {
    pub paper_width: PaperWidth,
    pub current_font: Font,
    pub justification: Justification,
    pub emphasis: bool,
    pub underline: bool,
    pub italic: bool,
    pub buffer: Vec<ReceiptLine>,
    pub line_height: u32,
    pub font_size: u32,
    pub dpi: u32,
    pub codepage: u8,
}

impl PrinterState {
    pub fn new() -> Self {
        Self {
            paper_width: PaperWidth::Width80mm,
            current_font: Font::FontA,
            justification: Justification::Left,
            emphasis: false,
            underline: false,
            italic: false,
            buffer: Vec::new(),
            line_height: 24,
            font_size: 12,
            dpi: 180,
            codepage: 0,
        }
    }

    /// Reset printer state to defaults (ESC @) — does NOT clear the buffer
    pub fn reset(&mut self) {
        self.current_font = Font::FontA;
        self.justification = Justification::Left;
        self.emphasis = false;
        self.underline = false;
        self.italic = false;
        self.line_height = 24;
        self.font_size = 12;
        self.codepage = 0;
    }

    /// Create a TextLine snapshot with current formatting state
    fn current_text_line(&self, content: String) -> TextLine {
        TextLine {
            content,
            font: self.current_font.clone(),
            font_size: self.font_size,
            justification: self.justification.clone(),
            emphasis: self.emphasis,
            underline: self.underline,
            italic: self.italic,
            line_height: self.line_height,
        }
    }

    pub fn process_command(&mut self, command: &EscPosCommand) {
        match command {
            EscPosCommand::Text(text) => {
                self.add_text(text);
            }
            EscPosCommand::NewLine => {
                self.add_new_line();
            }
            EscPosCommand::LineFeed => {
                self.add_new_line();
            }
            EscPosCommand::CarriageReturn => {
                // Carriage return — ignored (LF handles newline)
            }
            EscPosCommand::InitializePrinter => {
                self.reset();
            }
            EscPosCommand::SetFont(font) => {
                self.current_font = font.clone();
            }
            EscPosCommand::SetJustification(justification) => {
                self.justification = justification.clone();
            }
            EscPosCommand::SetEmphasis(enabled) => {
                self.emphasis = *enabled;
            }
            EscPosCommand::SetUnderline(enabled) => {
                self.underline = *enabled;
            }
            EscPosCommand::SetItalic(enabled) => {
                self.italic = *enabled;
            }
            EscPosCommand::SetLineHeight(height) => {
                self.line_height = *height;
            }
            EscPosCommand::SetFontSize(size) => {
                self.font_size = *size;
            }
            EscPosCommand::SetCodepage(cp) => {
                self.codepage = *cp;
            }
            EscPosCommand::CutPaper => {
                self.buffer.push(ReceiptLine::Separator);
            }
            EscPosCommand::PrintImage(image_data) => {
                // ESC * bit image — store as bitmap
                if !image_data.is_empty() {
                    self.buffer.push(ReceiptLine::Bitmap {
                        width_px: 8,
                        height_px: image_data.len() as u32,
                        data: image_data.clone(),
                    });
                }
            }
            EscPosCommand::PrintRasterImage { width_bytes, height, data } => {
                let width_px = *width_bytes as u32 * 8;
                let height_px = *height as u32;
                self.buffer.push(ReceiptLine::Bitmap {
                    width_px,
                    height_px,
                    data: data.clone(),
                });
            }
            EscPosCommand::Unknown(_) => {}
        }
    }

    fn add_text(&mut self, text: &str) {
        let max_chars = self.paper_width.get_max_chars(self.font_size) as usize;

        if let Some(ReceiptLine::Text(last_line)) = self.buffer.last_mut() {
            let current_length = last_line.content.chars().count();
            if current_length + text.chars().count() > max_chars {
                let text_line = self.current_text_line(text.to_string());
                self.buffer.push(ReceiptLine::Text(text_line));
            } else {
                last_line.content.push_str(text);
                // Update formatting to current state snapshot
                last_line.font = self.current_font.clone();
                last_line.font_size = self.font_size;
                last_line.justification = self.justification.clone();
                last_line.emphasis = self.emphasis;
                last_line.underline = self.underline;
                last_line.italic = self.italic;
                last_line.line_height = self.line_height;
            }
        } else {
            let text_line = self.current_text_line(text.to_string());
            self.buffer.push(ReceiptLine::Text(text_line));
        }
    }

    fn add_new_line(&mut self) {
        let text_line = self.current_text_line(String::new());
        self.buffer.push(ReceiptLine::Text(text_line));
    }

    pub fn clear_buffer(&mut self) {
        self.buffer.clear();
    }

    pub fn get_buffer(&self) -> &[ReceiptLine] {
        &self.buffer
    }

    /// Take ownership of the buffer, leaving it empty
    pub fn take_buffer(&mut self) -> Vec<ReceiptLine> {
        std::mem::take(&mut self.buffer)
    }

    pub fn get_paper_width_dots(&self) -> u32 {
        self.paper_width.get_width_dots()
    }

    /// Convert a monochrome 1bpp bitmap to an RGB image for display
    pub fn bitmap_to_rgb(width_px: u32, height_px: u32, data: &[u8]) -> RgbImage {
        let mut img = ImageBuffer::new(width_px, height_px);
        for pixel in img.pixels_mut() {
            *pixel = Rgb([255, 255, 255]);
        }
        let bytes_per_row = (width_px + 7) / 8;
        for y in 0..height_px {
            for x in 0..width_px {
                let byte_idx = (y * bytes_per_row + x / 8) as usize;
                let bit_idx = 7 - (x % 8);
                if byte_idx < data.len() && (data[byte_idx] >> bit_idx) & 1 == 1 {
                    img.put_pixel(x, y, Rgb([0, 0, 0]));
                }
            }
        }
        img
    }

    pub fn calculate_total_height(&self) -> u32 {
        let mut h = 0u32;
        for line in &self.buffer {
            match line {
                ReceiptLine::Text(tl) => h += tl.line_height,
                ReceiptLine::Bitmap { height_px, .. } => h += height_px,
                ReceiptLine::Separator => h += self.line_height,
            }
        }
        h.max(1)
    }

    pub fn set_paper_width(&mut self, width: PaperWidth) {
        self.paper_width = width;
    }

    pub fn set_line_height(&mut self, height: u32) {
        self.line_height = height;
    }

    pub fn set_font_size(&mut self, size: u32) {
        self.font_size = size;
    }
}
