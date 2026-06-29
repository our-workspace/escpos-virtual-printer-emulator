use crate::escpos::printer::{PaperWidth, PrinterState, ReceiptLine};
use anyhow::Result;
use printpdf::{
    BuiltinFont, Image, ImageTransform, Mm, PdfDocument,
};
use std::fs::File;
use std::io::BufWriter;
use std::path::Path;

const TEXT_LINE_HEIGHT_MM: f64 = 3.5;
const MARGIN_MM: f64 = 3.0;
const FONT_SIZE_PT: f64 = 8.0;
const MM_PER_DOT: f64 = 0.141; // 180 DPI ≈ 0.141mm per dot

/// Save a receipt buffer as a single-page PDF
pub fn save_receipt_pdf(
    buffer: &[ReceiptLine],
    output_path: &Path,
    paper_width: &PaperWidth,
) -> Result<()> {
    let paper_width_mm = match paper_width {
        PaperWidth::Width50mm => 50.0_f64,
        PaperWidth::Width78mm => 78.0,
        PaperWidth::Width80mm => 80.0,
    };

    // Calculate total page height
    let content_height = calculate_height_mm(buffer);
    let total_height_mm = content_height + MARGIN_MM * 2.0;

    // Create PDF document
    let (doc, page1, layer1) = PdfDocument::new(
        "Receipt",
        Mm(paper_width_mm),
        Mm(total_height_mm),
        "Layer 1",
    );

    let current_layer = doc.get_page(page1).get_layer(layer1);
    let font = doc.add_builtin_font(BuiltinFont::Courier)?;
    let font_bold = doc.add_builtin_font(BuiltinFont::CourierBold)?;

    let mut y_pos = total_height_mm - MARGIN_MM;

    for line in buffer {
        match line {
            ReceiptLine::Text(text_line) => {
                if !text_line.content.is_empty() {
                    let used_font = if text_line.emphasis { &font_bold } else { &font };

                    // Calculate x position based on justification
                    let x_pos = match text_line.justification {
                        crate::escpos::commands::Justification::Left => MARGIN_MM,
                        crate::escpos::commands::Justification::Center => {
                            let char_width = FONT_SIZE_PT * 0.6 / 2.8346; // approximate char width in mm
                            let text_width = text_line.content.len() as f64 * char_width;
                            ((paper_width_mm - text_width) / 2.0).max(MARGIN_MM)
                        }
                        crate::escpos::commands::Justification::Right => {
                            let char_width = FONT_SIZE_PT * 0.6 / 2.8346;
                            let text_width = text_line.content.len() as f64 * char_width;
                            (paper_width_mm - MARGIN_MM - text_width).max(MARGIN_MM)
                        }
                    };

                    current_layer.use_text(
                        &text_line.content,
                        FONT_SIZE_PT,
                        Mm(x_pos),
                        Mm(y_pos),
                        used_font,
                    );
                }
                y_pos -= TEXT_LINE_HEIGHT_MM;
            }
            ReceiptLine::Bitmap { width_px, height_px, data } => {
                // Convert 1bpp bitmap to RGB for PDF embedding
                let rgb_image = PrinterState::bitmap_to_rgb(*width_px, *height_px, data);
                // Use the image crate directly (not shadowed by printpdf)
                let dynamic_img: image::DynamicImage = rgb_image.into();

                // Calculate display dimensions
                let raw_width_mm = *width_px as f64 * MM_PER_DOT;
                let raw_height_mm = *height_px as f64 * MM_PER_DOT;
                let max_width = paper_width_mm - MARGIN_MM * 2.0;
                let scale = (max_width / raw_width_mm).min(1.0);
                let display_height_mm = raw_height_mm * scale;

                let pdf_image = Image::from_dynamic_image(&dynamic_img);
                let transform = ImageTransform {
                    translate_x: Some(Mm(MARGIN_MM)),
                    translate_y: Some(Mm(y_pos - display_height_mm)),
                    scale_x: Some(scale),
                    scale_y: Some(scale),
                    ..Default::default()
                };
                pdf_image.add_to_layer(current_layer.clone(), transform);

                y_pos -= display_height_mm;
            }
            ReceiptLine::Separator => {
                // Draw a dashed separator line as text
                let dash_count = ((paper_width_mm - MARGIN_MM * 2.0) / (FONT_SIZE_PT * 0.6 / 2.8346)) as usize;
                let sep_text = "-".repeat(dash_count);
                current_layer.use_text(
                    &sep_text,
                    FONT_SIZE_PT,
                    Mm(MARGIN_MM),
                    Mm(y_pos),
                    &font,
                );
                y_pos -= TEXT_LINE_HEIGHT_MM;
            }
        }
    }

    // Save to file
    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    doc.save(&mut BufWriter::new(File::create(output_path)?))?;

    Ok(())
}

/// Calculate total content height in mm
fn calculate_height_mm(buffer: &[ReceiptLine]) -> f64 {
    let mut height = 0.0_f64;
    for line in buffer {
        match line {
            ReceiptLine::Text(_) => height += TEXT_LINE_HEIGHT_MM,
            ReceiptLine::Bitmap { height_px, .. } => {
                height += *height_px as f64 * MM_PER_DOT;
            }
            ReceiptLine::Separator => height += TEXT_LINE_HEIGHT_MM,
        }
    }
    height.max(10.0)
}
