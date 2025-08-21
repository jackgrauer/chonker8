// Hot-reloadable PDF processor binary
use anyhow::Result;
use std::{
    env,
    path::Path,
    io::{self, BufRead},
};
use chonker8::file_picker;

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
    // HOT-RELOADABLE: This is where you can change PDF processing logic
    // and see changes immediately without restarting the main TUI
    
    let mut result = vec![vec![' '; 80]; 24];
    
    // Version 1: Simple text output
    let text = format!("📄 Processing: {} (Page {})", 
                      pdf_path.file_name().unwrap_or_default().to_string_lossy(), 
                      page);
    
    // Add some dynamic content that changes with hot-reload
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    
    let version_line = format!("║  \x1b[42m\x1b[30mPDF Processor v{}\x1b[0m{:<} ║", 
                              get_version(), 
                              " ".repeat(15_usize.saturating_sub(get_version().len() + 16)));
    let file_line = format!("║  File: {:<30}║", &text[0..20.min(text.len())]);
    let page_line = format!("║  Page: {:<30}║", page);
    let timestamp_line = format!("║  Timestamp: {:<25}║", timestamp);
    
    let lines = vec![
        "╔══════════════════════════════════════╗",
        &version_line,
        "╠══════════════════════════════════════╣",
        "║                                      ║",
        &file_line,
        &page_line,
        &timestamp_line,
        "║                                      ║",
        "║  🚀 HOT-RELOAD IS WORKING!           ║",
        "║                                      ║",
        "║  Claude edited this file for you!    ║",
        "║  Watch it update automatically!      ║",
        "║                                      ║",
        "║  Edit src/bin/pdf_processor.rs       ║",
        "║  and see changes instantly!          ║",
        "║                                      ║",
        "║  🔥 Press Ctrl+F for file picker!    ║",
        "║                                      ║",
        "╚══════════════════════════════════════╝",
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
    
    Ok(result)
}

fn get_page_count(pdf_path: &Path) -> Result<usize> {
    // HOT-RELOADABLE: Page counting logic
    
    if !pdf_path.exists() {
        return Ok(1); // Default for demo
    }
    
    // For demo, return a dynamic count
    Ok(3) // Always 3 pages for demo
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