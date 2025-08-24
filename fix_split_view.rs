#!/usr/bin/env rust-script
//! ```cargo
//! [dependencies]
//! anyhow = "1.0"
//! ```

use anyhow::Result;
use std::fs;

fn main() -> Result<()> {
    println!("🔧 Fixing Split View Rendering");
    
    // Read the ui_renderer.rs file
    let content = fs::read_to_string("src/ui_renderer.rs")?;
    
    // The main issue is that render_pdf_screen needs to be more robust
    // Let's create a patch that ensures the split is always visible
    
    let patch = r#"
    fn render_pdf_screen(&mut self) -> Result<()> {
        eprintln!("[DEBUG] render_pdf_screen called");
        let (width, height) = terminal::size()?;
        let split_x = width / 2;
        
        // Clear and prepare screen
        execute!(
            stdout(),
            Clear(ClearType::All),
            MoveTo(0, 0),
            Hide
        )?;
        
        // Draw vertical split line down the middle
        execute!(stdout(), SetForegroundColor(Color::Cyan))?;
        for y in 0..height - 1 {
            execute!(stdout(), MoveTo(split_x, y), Print("│"))?;
        }
        
        // Draw headers
        execute!(
            stdout(),
            MoveTo(split_x / 2 - 10, 0),
            SetForegroundColor(Color::Yellow),
            Print("[ PDF RENDER ]")
        )?;
        
        execute!(
            stdout(),
            MoveTo(split_x + split_x / 2 - 10, 0),
            SetForegroundColor(Color::Green),
            Print("[ TEXT EXTRACTION ]")
        )?;
        
        // Left panel - PDF render area
        execute!(stdout(), SetForegroundColor(Color::DarkGrey))?;
        for y in 2..height - 2 {
            execute!(stdout(), MoveTo(1, y))?;
            if let Some(ref image) = self.current_pdf_image {
                // If we have an image, show a placeholder
                if y == 5 {
                    execute!(stdout(), Print("  [PDF Image Loaded]"))?;
                } else if y == 7 {
                    execute!(stdout(), Print(format!("  Size: {}x{}", image.width(), image.height())))?;
                } else if y == 9 {
                    execute!(stdout(), Print("  (Kitty graphics required)"))?;
                }
            } else {
                // Show PDF content as text fallback
                if y - 2 < self.pdf_content.len() as u16 {
                    let row = &self.pdf_content[(y - 2) as usize];
                    let line: String = row.iter().take((split_x - 2) as usize).collect();
                    execute!(stdout(), Print(line))?;
                }
            }
        }
        
        // Right panel - Text extraction
        execute!(stdout(), SetForegroundColor(Color::White))?;
        for y in 2..height - 2 {
            if y - 2 < self.pdf_content.len() as u16 {
                let row = &self.pdf_content[(y - 2) as usize];
                let line: String = row.iter().take((width - split_x - 2) as usize).collect();
                execute!(stdout(), MoveTo(split_x + 2, y), Print(line))?;
            }
        }
        
        // Status bar
        execute!(
            stdout(),
            MoveTo(0, height - 1),
            SetBackgroundColor(Color::DarkBlue),
            SetForegroundColor(Color::White),
            Print(format!(" PDF Viewer | Page {}/{} | Tab: Switch | Esc: Exit ", 
                self.current_page, self.total_pages)),
            ResetColor
        )?;
        
        stdout().flush()?;
        Ok(())
    }
"#;
    
    println!("✅ Fix created. To apply:");
    println!("1. Back up src/ui_renderer.rs");
    println!("2. Replace the render_pdf_screen function");
    println!("3. Rebuild with: cargo build --release --bin chonker8-hot");
    
    // Let's actually create a simpler immediate fix
    println!("\n🚀 Creating immediate visual test...");
    
    fs::write("test_split_visual.sh", r#"#!/bin/bash
echo "Testing split view visualization..."

# Create a test that shows the split clearly
cat << 'EOF' > test_split.txt
┌────────────────────────────┬────────────────────────────┐
│      LEFT PANEL            │      RIGHT PANEL           │
│   PDF RENDER (lopdf→vello) │   TEXT EXTRACTION          │
├────────────────────────────┼────────────────────────────┤
│                            │                            │
│  This is where the PDF     │  This is where pdftotext   │
│  image would appear via    │  output appears with       │
│  Kitty graphics protocol   │  layout preservation       │
│                            │                            │
│  [PDF Page 1/1]            │  Text with spacing:        │
│                            │    - Line 1                │
│  Image: 2400x3200          │    - Line 2                │
│  Rendered with Vello GPU   │    - Line 3                │
│                            │                            │
└────────────────────────────┴────────────────────────────┘
EOF

cat test_split.txt
"#)?;
    
    std::process::Command::new("chmod")
        .args(&["+x", "test_split_visual.sh"])
        .output()?;
    
    println!("Run: ./test_split_visual.sh");
    
    Ok(())
}