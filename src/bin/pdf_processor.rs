// Hot-reloadable PDF processor binary with intelligent extraction
use anyhow::Result;
use std::{
    env,
    path::Path,
    io::{self, BufRead},
};
use chonker8::file_picker;
use chonker8::pdf_extraction::{DocumentAnalyzer, ExtractionRouter};

// This binary can be hot-reloaded independently of the main TUI
fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    
    if args.len() < 2 {
        eprintln!("Usage: pdf-processor <command> [args...]");
        eprintln!("Commands:");
        eprintln!("  process <pdf_path> <page> - Process a page");
        eprintln!("  count <pdf_path> - Get page count");
        eprintln!("  version - Get processor version");
        eprintln!("  interactive - Interactive mode");
        eprintln!("  filepicker - Launch hot-reload file picker");
        return Ok(());
    }
    
    match args[1].as_str() {
        "process" => {
            if args.len() < 4 {
                eprintln!("Usage: pdf-processor process <pdf_path> <page>");
                return Ok(());
            }
            let pdf_path = &args[2];
            let page: usize = args[3].parse()?;
            
            let result = process_page(Path::new(pdf_path), page)?;
            print_grid(&result);
        },
        "count" => {
            if args.len() < 3 {
                eprintln!("Usage: pdf-processor count <pdf_path>");
                return Ok(());
            }
            let pdf_path = &args[2];
            let count = get_page_count(Path::new(pdf_path))?;
            println!("{}", count);
        },
        "version" => {
            println!("pdf-processor v{}", get_version());
        },
        "interactive" => {
            run_interactive_mode()?;
        },
        "filepicker" => {
            launch_file_picker()?;
        },
        _ => {
            eprintln!("Unknown command: {}", args[1]);
        }
    }
    
    Ok(())
}

fn process_page(pdf_path: &Path, page: usize) -> Result<Vec<Vec<char>>> {
    // HOT-RELOADABLE: Now using intelligent document-agnostic extraction!
    
    // Initialize result grid
    let mut result = vec![vec![' '; 80]; 24];
    
    // If the file exists, use intelligent extraction
    if pdf_path.exists() {
        // Create runtime for async operations
        let rt = tokio::runtime::Runtime::new()?;
        
        // Analyze the page
        let analyzer = DocumentAnalyzer::new()?;
        let fingerprint = analyzer.analyze_page(pdf_path, page)?;
        
        // Extract with intelligent routing
        let extraction_result = rt.block_on(ExtractionRouter::extract_with_fallback(
            pdf_path,
            page,
            &fingerprint
        ))?;
        
        // Format the results for display
        let header = format!(
            "🔍 INTELLIGENT EXTRACTION - Page {} of {}",
            page + 1,
            get_page_count(pdf_path)?
        );
        
        let analysis = format!(
            "📊 Analysis: Text {:.0}% | Images {:.0}% | Quality {:.1}",
            fingerprint.text_coverage * 100.0,
            fingerprint.image_coverage * 100.0,
            fingerprint.text_quality
        );
        
        let method = format!(
            "⚙️  Method: {:?} | Score: {:.2} | Time: {}ms",
            extraction_result.method,
            extraction_result.quality_score,
            extraction_result.extraction_time_ms
        );
        
        // Display the extracted text
        let lines = vec![
            "╔══════════════════════════════════════════════════════════════════════════════╗",
            &format!("║ {:<76} ║", header),
            "╠══════════════════════════════════════════════════════════════════════════════╣",
            &format!("║ {:<76} ║", analysis),
            &format!("║ {:<76} ║", method),
            "╠══════════════════════════════════════════════════════════════════════════════╣",
            "║ EXTRACTED TEXT:                                                             ║",
            "╟──────────────────────────────────────────────────────────────────────────────╢",
        ];
        
        // Add header lines
        for (i, line) in lines.iter().enumerate() {
            if i < result.len() {
                for (j, ch) in line.chars().enumerate() {
                    if j < result[i].len() {
                        result[i][j] = ch;
                    }
                }
            }
        }
        
        // Add extracted text (word-wrapped)
        let text_start_row = lines.len();
        let text_lines: Vec<&str> = extraction_result.text.lines().collect();
        let max_text_rows = result.len() - text_start_row - 2;
        
        for (i, text_line) in text_lines.iter().take(max_text_rows).enumerate() {
            let row = text_start_row + i;
            if row < result.len() {
                result[row][0] = '║';
                result[row][1] = ' ';
                
                // Add text content (truncated to fit)
                let max_width = 76;
                let display_text = if text_line.len() > max_width {
                    &text_line[..max_width]
                } else {
                    text_line
                };
                
                for (j, ch) in display_text.chars().enumerate() {
                    if j + 2 < result[row].len() - 2 {
                        result[row][j + 2] = ch;
                    }
                }
                
                result[row][79] = '║';
            }
        }
        
        // Add bottom border
        let bottom_row = result.len() - 1;
        let bottom_line = "╚══════════════════════════════════════════════════════════════════════════════╝";
        for (j, ch) in bottom_line.chars().enumerate() {
            if j < result[bottom_row].len() {
                result[bottom_row][j] = ch;
            }
        }
        
    } else {
        // Fallback display for non-existent files
        let lines = vec![
            "╔══════════════════════════════════════════════════════════════════════════════╗",
            "║  📄 PDF PROCESSOR v4.0.0-HOTRELOAD WITH INTELLIGENT EXTRACTION              ║",
            "╠══════════════════════════════════════════════════════════════════════════════╣",
            "║                                                                              ║",
            "║  ⚠️  File not found or demo mode                                             ║",
            "║                                                                              ║",
            "║  This processor now features:                                               ║",
            "║  • 🔍 Automatic page analysis (text/image coverage)                         ║",
            "║  • 🎯 Intelligent extraction method selection                               ║",
            "║  • ✅ Quality validation with fallback                                      ║",
            "║  • ⚡ Optimized for each page type                                          ║",
            "║                                                                              ║",
            "║  Try loading a real PDF to see the magic!                                   ║",
            "║                                                                              ║",
            "╚══════════════════════════════════════════════════════════════════════════════╝",
        ];
        
        for (i, line) in lines.iter().enumerate() {
            if i < result.len() {
                for (j, ch) in line.chars().enumerate() {
                    if j < result[i].len() {
                        result[i][j] = ch;
                    }
                }
            }
        }
    }
    
    Ok(result)
}

fn get_page_count(pdf_path: &Path) -> Result<usize> {
    // HOT-RELOADABLE: Page counting logic
    
    if !pdf_path.exists() {
        return Ok(1); // Default for demo
    }
    
    // Use actual page count from PDF
    chonker8::pdf_extraction::basic::get_page_count(pdf_path)
}

fn get_version() -> String {
    // Change this and watch it update live!
    "4.0.0-HOTRELOAD".to_string()
}

fn print_grid(grid: &[Vec<char>]) {
    for row in grid {
        for ch in row {
            print!("{}", ch);
        }
        println!();
    }
}

fn run_interactive_mode() -> Result<()> {
    println!("🔥 PDF Processor Interactive Mode (Hot-Reload Enabled)");
    println!("Type commands or 'quit' to exit:");
    
    let stdin = io::stdin();
    for line in stdin.lock().lines() {
        let line = line?;
        let parts: Vec<&str> = line.trim().split_whitespace().collect();
        
        if parts.is_empty() {
            continue;
        }
        
        match parts[0] {
            "quit" | "exit" => break,
            "version" => println!("Version: {}", get_version()),
            "process" => {
                if parts.len() >= 3 {
                    if let Ok(page) = parts[2].parse::<usize>() {
                        match process_page(Path::new(parts[1]), page) {
                            Ok(grid) => print_grid(&grid),
                            Err(e) => eprintln!("Error: {}", e),
                        }
                    }
                } else {
                    println!("Usage: process <pdf_path> <page>");
                }
            },
            _ => println!("Unknown command: {}", parts[0]),
        }
        
        println!("\n> ");
    }
    
    Ok(())
}

fn launch_file_picker() -> Result<()> {
    println!("🚀 Launching Hot-Reload File Picker...");
    
    match file_picker::pick_pdf_file() {
        Ok(Some(path)) => {
            println!("✅ Selected: {}", path.display());
            
            // Show a preview of the selected file
            println!("\n🔍 Processing selected file...");
            let grid = process_page(&path, 1)?;
            print_grid(&grid);
        },
        Ok(None) => {
            println!("❌ No file selected or cancelled");
        },
        Err(e) => {
            eprintln!("💥 File picker error: {}", e);
        }
    }
    
    Ok(())
}