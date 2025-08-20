// Better text extraction using pdftotext (same as Extractous uses internally)
use anyhow::Result;
use std::path::Path;

/// Extract text using pdftotext for clean extraction without scrambling  
pub async fn extract_with_extractous(
    pdf_path: &Path,
    page_index: usize,
    width: usize,
    height: usize,
) -> Result<Vec<Vec<char>>> {
    use std::process::Command;
    
    // Use pdftotext for clean extraction
    let output = Command::new("pdftotext")
        .args(&[
            "-f", &(page_index + 1).to_string(),  // First page (1-indexed)
            "-l", &(page_index + 1).to_string(),  // Last page (same page)
            "-layout",  // Maintain layout
            pdf_path.to_str().unwrap(),
            "-"  // Output to stdout
        ])
        .output()?;
    
    if !output.status.success() {
        anyhow::bail!("pdftotext failed");
    }
    
    let text = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = text.lines().collect();
    
    // Create grid with dynamic sizing
    let actual_height = lines.len().max(height);
    let actual_width = lines.iter().map(|l| l.len()).max().unwrap_or(0).max(width);
    
    let mut grid = vec![vec![' '; actual_width]; actual_height];
    
    // Fill the grid preserving layout
    for (row, line) in lines.iter().enumerate() {
        for (col, ch) in line.chars().enumerate() {
            if row < actual_height && col < actual_width {
                grid[row][col] = ch;
            }
        }
    }
    
    Ok(grid)
}

/// Extract with better page handling
pub async fn extract_with_extractous_advanced(
    pdf_path: &Path,
    page_index: usize,
    width: usize,
    height: usize,
) -> Result<Vec<Vec<char>>> {
    use std::process::Command;
    
    // For now, use pdftotext which Extractous uses internally
    // This gives us better control over page extraction
    let output = Command::new("pdftotext")
        .args(&[
            "-f", &(page_index + 1).to_string(),  // First page (1-indexed)
            "-l", &(page_index + 1).to_string(),  // Last page (same page)
            "-layout",  // Maintain layout
            pdf_path.to_str().unwrap(),
            "-"  // Output to stdout
        ])
        .output()?;
    
    if !output.status.success() {
        // Fallback to full extraction
        return extract_with_extractous(pdf_path, page_index, width, height).await;
    }
    
    let text = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = text.lines().collect();
    
    // Create grid with dynamic sizing
    let actual_height = lines.len().max(height);
    let actual_width = lines.iter().map(|l| l.len()).max().unwrap_or(0).max(width);
    
    let mut grid = vec![vec![' '; actual_width]; actual_height];
    
    // Fill the grid preserving layout
    for (row, line) in lines.iter().enumerate() {
        for (col, ch) in line.chars().enumerate() {
            if row < actual_height && col < actual_width {
                grid[row][col] = ch;
            }
        }
    }
    
    Ok(grid)
}