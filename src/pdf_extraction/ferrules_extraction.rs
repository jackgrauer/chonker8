// Ferrules-based extraction for better structured text with proper word boundaries
use anyhow::Result;
use std::path::Path;
use std::process::Command;
use serde::{Deserialize, Serialize};
use ferrules_core::*;

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
struct FerrulesBlock {
    #[serde(rename = "type")]
    block_type: String,
    text: Option<String>,
    bbox: Option<Vec<f32>>,
    page: usize,
}

/// Extract text using Ferrules for better structure and word boundaries
pub async fn extract_with_ferrules(
    pdf_path: &Path,
    page_index: usize,
    width: usize,
    height: usize,
) -> Result<Vec<Vec<char>>> {
    // First, try to run Ferrules as a subprocess
    let ferrules_path = "./ferrules/target/release/ferrules";
    
    // Check if Ferrules exists
    if !std::path::Path::new(ferrules_path).exists() {
        anyhow::bail!("Ferrules not found at {}", ferrules_path);
    }
    
    // Create a temp directory for output
    let temp_dir = std::env::temp_dir().join(format!("ferrules_{}", std::process::id()));
    std::fs::create_dir_all(&temp_dir)?;
    
    // Run Ferrules
    let output = Command::new(ferrules_path)
        .args(&[
            pdf_path.to_str().unwrap(),
            "-r", &format!("{}", page_index + 1), // Ferrules uses 1-based page numbers
            "-o", temp_dir.to_str().unwrap(),
        ])
        .output()?;
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        println!("Ferrules stderr: {}", stderr);
        println!("Ferrules stdout: {}", stdout);
        anyhow::bail!("Ferrules failed: {}", stderr);
    }
    
    // Debug: show what Ferrules actually output
    let stdout = String::from_utf8_lossy(&output.stdout);
    println!("Ferrules output: {}", stdout);
    
    // Debug: list everything in temp directory
    println!("Contents of temp directory {}:", temp_dir.display());
    if let Ok(entries) = std::fs::read_dir(&temp_dir) {
        for entry in entries.flatten() {
            println!("  - {}", entry.path().display());
        }
    }
    
    // Find the output JSON file - look recursively since Ferrules might create subdirs
    let mut json_files = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&temp_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                // Look in subdirectories
                if let Ok(sub_entries) = std::fs::read_dir(&path) {
                    for sub_entry in sub_entries.flatten() {
                        let sub_path = sub_entry.path();
                        if sub_path.extension().and_then(|ext| ext.to_str()) == Some("json") {
                            json_files.push(sub_entry);
                        }
                    }
                }
            } else if path.extension().and_then(|ext| ext.to_str()) == Some("json") {
                json_files.push(entry);
            }
        }
    }
    
    if json_files.is_empty() {
        // Cleanup
        let _ = std::fs::remove_dir_all(&temp_dir);
        anyhow::bail!("No JSON output from Ferrules");
    }
    
    // Read the JSON
    let json_content = std::fs::read_to_string(json_files[0].path())?;
    let ferrules_data: FerrulesOutput = serde_json::from_str(&json_content)?;
    
    // Cleanup temp directory
    let _ = std::fs::remove_dir_all(&temp_dir);
    
    // Create grid from Ferrules blocks
    let mut grid = vec![vec![' '; width]; height];
    
    // Get page dimensions
    let page = ferrules_data.pages.iter()
        .find(|p| p.id == page_index)
        .ok_or_else(|| anyhow::anyhow!("Page {} not found", page_index))?;
    
    let page_width = page.width;
    let page_height = page.height;
    
    // Debug output
    println!("Ferrules debug: Found {} pages, {} blocks", ferrules_data.pages.len(), ferrules_data.blocks.len());
    println!("Ferrules debug: Page {} - {}x{}, need_ocr: {}", page_index, page_width, page_height, page.need_ocr);
    
    // Process blocks for this page
    let page_blocks: Vec<_> = ferrules_data.blocks.iter().filter(|b| b.page == page_index).collect();
    println!("Ferrules debug: Found {} blocks for page {}", page_blocks.len(), page_index);
    
    for (i, block) in page_blocks.iter().enumerate() {
        println!("Ferrules debug: Block {}: type={:?}, text={:?}, bbox={:?}", 
                 i, block.block_type, block.text.as_deref().unwrap_or("None"), block.bbox);
        
        if let (Some(text), Some(bbox)) = (&block.text, &block.bbox) {
            if bbox.len() >= 4 {
                // bbox is [x0, y0, x1, y1]
                let x = bbox[0];
                let y = bbox[1];
                
                // Map to grid coordinates
                let grid_x = ((x / page_width) * width as f32).round() as usize;
                let grid_y = height - 1 - ((y / page_height) * height as f32).round() as usize;
                
                // Place text on grid with proper spacing
                let mut current_x = grid_x;
                for ch in text.chars() {
                    if current_x < width && grid_y < height {
                        grid[grid_y][current_x] = ch;
                        current_x += 1;
                    }
                }
            }
        }
    }
    
    // If no blocks found, this suggests Ferrules OCR failed or text extraction failed
    if ferrules_data.blocks.is_empty() {
        if page.need_ocr {
            anyhow::bail!("Ferrules detected OCR needed but returned no blocks - OCR may have failed");
        } else {
            anyhow::bail!("Ferrules returned no text blocks - document may be empty or extraction failed");
        }
    }
    
    Ok(grid)
}

