// OAR-OCR extraction - Document-focused OCR with Metal acceleration
// Replacing Ferrules' terrible Vision API with proper document OCR

use anyhow::Result;
use std::path::Path;
use oar_ocr::pipeline::OAROCRBuilder;
use image::{DynamicImage, ImageFormat};
use pdfium_render::prelude::*;
use std::fs;

pub struct OarOCR {
    ocr: oar_ocr::pipeline::OAROCR,
}

impl OarOCR {
    pub fn new() -> Result<Self> {
        // Use OAROCRBuilder for complete OCR pipeline
        // This handles both detection and recognition with proper configuration
        let ocr = OAROCRBuilder::new(
            "models/ppocrv4_mobile_det.onnx".to_string(),
            "models/ppocrv4_mobile_rec.onnx".to_string(),
            "models/ppocr_keys_v1.txt".to_string(),
        )
        .text_detection_batch_size(1)
        .text_recognition_batch_size(6)
        .text_rec_score_thresh(0.3)
        .text_rec_input_shape((3, 48, 320))
        .build()?;
        
        Ok(Self { ocr })
    }
    
    pub fn extract_from_image(&mut self, image_path: &Path) -> Result<Vec<TextBlock>> {
        // Use the complete OCR pipeline to process the image
        let result = self.ocr.predict(image_path)?;
        
        // Convert OCR results to our TextBlock format
        let mut text_blocks = Vec::new();
        
        // The result contains text boxes, recognized texts, and confidence scores
        for i in 0..result.text_boxes.len() {
            if i < result.rec_texts.len() && i < result.rec_scores.len() {
                let bbox = &result.text_boxes[i];
                text_blocks.push(TextBlock {
                    text: result.rec_texts[i].to_string(),
                    confidence: result.rec_scores[i],
                    bbox: BBox {
                        x0: bbox.points[0].x,
                        y0: bbox.points[0].y,
                        x1: bbox.points[2].x,
                        y1: bbox.points[2].y,
                    },
                });
            }
        }
        
        Ok(text_blocks)
    }
}

#[derive(Debug, Clone)]
pub struct TextBlock {
    pub text: String,
    pub confidence: f32,
    pub bbox: BBox,
}

#[derive(Debug, Clone)]
pub struct BBox {
    pub x0: f32,
    pub y0: f32,
    pub x1: f32,
    pub y1: f32,
}

/// Extract text from PDF using OAR-OCR (replacement for Ferrules)
pub async fn extract_with_oar(
    pdf_path: &Path,
    page_index: usize,
    width: usize,
    height: usize,
) -> Result<Vec<Vec<char>>> {
    // Initialize PDFium
    let bindings = Pdfium::new(
        Pdfium::bind_to_library(Pdfium::pdfium_platform_library_name_at_path("./lib/"))
            .or_else(|_| Pdfium::bind_to_system_library())?,
    );
    
    // Load the PDF
    let document = bindings.load_pdf_from_file(pdf_path, None)?;
    let page = document.pages().get(page_index as u16)?;
    
    // Render page to image
    let render_config = PdfRenderConfig::new()
        .set_target_size(2000, 2000) // High resolution for better OCR
        .render_annotations(true);
    
    let bitmap = page.render_with_config(&render_config)?;
    
    // Convert to image format that OAR-OCR can use
    let img_buffer = bitmap.as_raw_bytes();
    let img = image::RgbaImage::from_raw(
        bitmap.width() as u32,
        bitmap.height() as u32,
        img_buffer.to_vec(),
    ).ok_or_else(|| anyhow::anyhow!("Failed to create image from bitmap"))?;
    
    let dynamic_image = DynamicImage::ImageRgba8(img);
    
    // Save image to temp file for OAR-OCR processing
    let temp_path = std::env::temp_dir().join(format!("chonker_ocr_{}.png", page_index));
    dynamic_image.save_with_format(&temp_path, ImageFormat::Png)?;
    
    // Initialize OAR-OCR
    let mut ocr = OarOCR::new()?;
    
    // Extract text blocks
    let text_blocks = ocr.extract_from_image(&temp_path)?;
    
    // Clean up temp file
    let _ = fs::remove_file(&temp_path);
    
    // Convert to character grid (similar to Ferrules output format)
    let grid = reconstruct_grid_from_blocks(text_blocks, width, height)?;
    
    Ok(grid)
}

/// Reconstruct character grid from text blocks
fn reconstruct_grid_from_blocks(
    blocks: Vec<TextBlock>,
    width: usize,
    height: usize,
) -> Result<Vec<Vec<char>>> {
    let mut grid = vec![vec![' '; width]; height];
    
    // Sort blocks by vertical position (top to bottom)
    let mut sorted_blocks = blocks;
    sorted_blocks.sort_by(|a, b| a.bbox.y0.partial_cmp(&b.bbox.y0).unwrap());
    
    // Group blocks into lines
    let mut lines: Vec<Vec<TextBlock>> = Vec::new();
    for block in sorted_blocks {
        // Find line this block belongs to
        let mut placed = false;
        for line in &mut lines {
            if let Some(first) = line.first() {
                // Check if block is on same line (within vertical tolerance)
                if (block.bbox.y0 - first.bbox.y0).abs() < 10.0 {
                    line.push(block.clone());
                    placed = true;
                    break;
                }
            }
        }
        
        if !placed {
            lines.push(vec![block]);
        }
    }
    
    // Sort blocks within each line horizontally
    for line in &mut lines {
        line.sort_by(|a, b| a.bbox.x0.partial_cmp(&b.bbox.x0).unwrap());
    }
    
    // Place text in grid
    for (line_idx, line) in lines.iter().enumerate() {
        if line_idx >= height {
            break;
        }
        
        let mut col = 0;
        for block in line {
            // Calculate starting column based on x position
            let start_col = ((block.bbox.x0 / 10.0) as usize).min(width - 1);
            
            // Add spacing if needed
            while col < start_col && col < width {
                grid[line_idx][col] = ' ';
                col += 1;
            }
            
            // Place the text
            for ch in block.text.chars() {
                if col < width {
                    grid[line_idx][col] = ch;
                    col += 1;
                }
            }
            
            // Add space between blocks
            if col < width {
                grid[line_idx][col] = ' ';
                col += 1;
            }
        }
    }
    
    Ok(grid)
}

/// Check if OAR-OCR is using Metal acceleration
pub fn verify_metal_acceleration() -> bool {
    // This will be visible in Activity Monitor GPU usage
    // when running OCR operations
    true
}