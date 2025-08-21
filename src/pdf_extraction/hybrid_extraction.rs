// Hybrid extraction: Ferrules for layout, pdftotext for accurate text
use anyhow::Result;
use std::path::Path;
use std::process::Command;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Deserialize, Serialize)]
struct FerrulesOutput {
    doc_name: String,
    pages: Vec<FerrulesPage>,
    blocks: Vec<FerrulesBlock>,
    metadata: serde_json::Value,
}

#[derive(Debug, Deserialize, Serialize)]
struct FerrulesPage {
    id: usize,
    width: f32,
    height: f32,
    need_ocr: bool,
}

#[derive(Debug, Deserialize, Serialize)]
struct FerrulesBlockKind {
    block_type: String,
    text: String,  // We'll ignore this gibberish
}

#[derive(Debug, Deserialize, Serialize, Clone, Copy)]
struct FerrulesBBox {
    x0: f32,
    y0: f32,
    x1: f32,
    y1: f32,
}

#[derive(Debug, Deserialize, Serialize)]
struct FerrulesBlock {
    id: usize,
    kind: FerrulesBlockKind,
    pages_id: Vec<usize>,
    bbox: FerrulesBBox,
}

#[derive(Debug, Clone)]
struct TextLine {
    text: String,
    y_position: f32,
    x_start: f32,
    x_end: f32,
}

/// Extract using Ferrules for layout and pdftotext for accurate text
pub async fn extract_hybrid(
    pdf_path: &Path,
    page_index: usize,
    width: usize,
    height: usize,
) -> Result<Vec<Vec<char>>> {
    // Step 1: Get layout from Ferrules (bounding boxes only)
    let ferrules_layout = get_ferrules_layout(pdf_path, page_index).await?;
    
    // Step 2: Get clean text from pdftotext
    let clean_text = extract_with_pdftotext(pdf_path, page_index).await?;
    
    // Step 3: Map clean text to Ferrules layout
    let mapped_blocks = map_text_to_layout(ferrules_layout, clean_text)?;
    
    // Step 4: Reconstruct grid with proper layout and clean text
    reconstruct_grid(mapped_blocks, width, height)
}

/// Get layout information from Ferrules (ignore the OCR text)
async fn get_ferrules_layout(pdf_path: &Path, page_index: usize) -> Result<FerrulesOutput> {
    let ferrules_path = "/Users/jack/chonker8/ferrules/target/release/ferrules";
    
    if !std::path::Path::new(ferrules_path).exists() {
        anyhow::bail!("Ferrules not found at {}", ferrules_path);
    }
    
    let temp_dir = std::env::temp_dir().join(format!("ferrules_hybrid_{}", std::process::id()));
    std::fs::create_dir_all(&temp_dir)?;
    
    // Run Ferrules for layout only
    let output = Command::new(ferrules_path)
        .env("DYLD_LIBRARY_PATH", "/Users/jack/chonker8/lib")
        .args(&[
            pdf_path.to_str().unwrap(),
            "-r", &format!("{}", page_index + 1),
            "-o", temp_dir.to_str().unwrap(),
        ])
        .output()?;
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Ferrules failed: {}", stderr);
    }
    
    // Find and parse the JSON output
    let mut json_file = None;
    for entry in std::fs::read_dir(&temp_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            for sub_entry in std::fs::read_dir(&path)? {
                let sub_entry = sub_entry?;
                if sub_entry.path().extension().and_then(|ext| ext.to_str()) == Some("json") {
                    json_file = Some(sub_entry.path());
                    break;
                }
            }
        }
    }
    
    let json_path = json_file.ok_or_else(|| anyhow::anyhow!("No JSON output from Ferrules"))?;
    let json_content = std::fs::read_to_string(&json_path)?;
    let ferrules_data: FerrulesOutput = serde_json::from_str(&json_content)?;
    
    // Cleanup
    let _ = std::fs::remove_dir_all(&temp_dir);
    
    Ok(ferrules_data)
}

/// Extract clean text using pdftotext
async fn extract_with_pdftotext(pdf_path: &Path, page_index: usize) -> Result<Vec<TextLine>> {
    // Use regular layout mode (bbox-layout outputs XML which we don't want here)
    let output = Command::new("pdftotext")
        .args(&[
            "-f", &(page_index + 1).to_string(),
            "-l", &(page_index + 1).to_string(),
            "-layout",  // Maintain layout without XML
            pdf_path.to_str().unwrap(),
            "-"
        ])
        .output()?;
    
    if !output.status.success() {
        anyhow::bail!("pdftotext failed");
    }
    
    let text = String::from_utf8_lossy(&output.stdout);
    
    // Parse text lines with approximate positions
    let mut text_lines = Vec::new();
    for (line_idx, line) in text.lines().enumerate() {
        if !line.trim().is_empty() {
            // Simple position estimation (can be improved with actual bbox data)
            let y_position = line_idx as f32 * 12.0;  // Approximate line height
            let x_start = line.len() - line.trim_start().len();
            
            text_lines.push(TextLine {
                text: line.to_string(),
                y_position,
                x_start: x_start as f32 * 7.0,  // Approximate char width
                x_end: line.len() as f32 * 7.0,
            });
        }
    }
    
    Ok(text_lines)
}

/// Map clean pdftotext content to Ferrules layout blocks
fn map_text_to_layout(
    ferrules_data: FerrulesOutput,
    text_lines: Vec<TextLine>,
) -> Result<Vec<MappedBlock>> {
    let mut mapped_blocks = Vec::new();
    
    // Get page info
    let page = ferrules_data.pages.first()
        .ok_or_else(|| anyhow::anyhow!("No page data"))?;
    
    let page_height = page.height;
    
    // For each Ferrules block, find the corresponding text
    for block in ferrules_data.blocks {
        let bbox = block.bbox;
        
        // Find text lines that fall within this bounding box
        let mut block_text = String::new();
        
        for text_line in &text_lines {
            // Check if this line falls within the block's y-range
            // Note: This is simplified - real implementation would need better mapping
            let line_y = text_line.y_position;
            
            // Rough approximation - can be improved with actual coordinate mapping
            if line_y >= bbox.y0 - 5.0 && line_y <= bbox.y1 + 5.0 {
                // Check x-range overlap
                if text_line.x_start <= bbox.x1 && text_line.x_end >= bbox.x0 {
                    if !block_text.is_empty() {
                        block_text.push(' ');
                    }
                    
                    // Extract the portion of text within x-range
                    let text = &text_line.text;
                    let start_col = ((bbox.x0 - text_line.x_start) / 7.0).max(0.0) as usize;
                    let end_col = ((bbox.x1 - text_line.x_start) / 7.0).min(text.len() as f32) as usize;
                    
                    if start_col < text.len() && end_col > start_col {
                        block_text.push_str(&text[start_col..end_col.min(text.len())]);
                    } else {
                        block_text.push_str(text.trim());
                    }
                }
            }
        }
        
        // If we didn't find text, use a more lenient search
        if block_text.trim().is_empty() {
            // Find the closest text line
            if let Some(closest_line) = find_closest_text(&bbox, &text_lines) {
                block_text = closest_line.text.clone();
            }
        }
        
        mapped_blocks.push(MappedBlock {
            bbox,
            text: block_text.trim().to_string(),
            block_type: block.kind.block_type,
        });
    }
    
    Ok(mapped_blocks)
}

#[derive(Debug)]
struct MappedBlock {
    bbox: FerrulesBBox,
    text: String,
    block_type: String,
}

/// Find the closest text line to a bounding box
fn find_closest_text<'a>(bbox: &FerrulesBBox, text_lines: &'a [TextLine]) -> Option<&'a TextLine> {
    let bbox_center_y = (bbox.y0 + bbox.y1) / 2.0;
    let bbox_center_x = (bbox.x0 + bbox.x1) / 2.0;
    
    text_lines.iter()
        .min_by_key(|line| {
            let y_dist = (line.y_position - bbox_center_y).abs();
            let x_dist = if line.x_start > bbox.x1 {
                line.x_start - bbox.x1
            } else if line.x_end < bbox.x0 {
                bbox.x0 - line.x_end
            } else {
                0.0
            };
            
            ((y_dist + x_dist) * 100.0) as i32
        })
}

/// Reconstruct the character grid with mapped blocks
fn reconstruct_grid(
    mapped_blocks: Vec<MappedBlock>,
    requested_width: usize,
    requested_height: usize,
) -> Result<Vec<Vec<char>>> {
    // Calculate required dimensions
    let max_x = mapped_blocks.iter()
        .map(|b| (b.bbox.x1 + b.text.len() as f32 * 0.5) as usize)
        .max()
        .unwrap_or(requested_width);
    
    let max_y = mapped_blocks.iter()
        .map(|b| b.bbox.y1 as usize)
        .max()
        .unwrap_or(requested_height);
    
    let width = max_x.max(requested_width).max(200);
    let height = (max_y / 12).max(requested_height).max(100);  // Approximate line height
    
    let mut grid = vec![vec![' '; width]; height];
    
    // Place each block's text at its proper position
    for block in mapped_blocks {
        let row = (block.bbox.y0 / 12.0) as usize;  // Convert to grid row
        let col = (block.bbox.x0 / 7.0) as usize;   // Convert to grid column
        
        if row < height {
            for (i, ch) in block.text.chars().enumerate() {
                if col + i < width {
                    grid[row][col + i] = ch;
                }
            }
        }
    }
    
    Ok(grid)
}