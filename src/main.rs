// Chonker8 - CLI tool with file browser and PDF text editor
use anyhow::Result;
use chonker8::app_state::AppState;
use chonker8::display::file_browser::{FileBrowser, FileBrowserResult};
use chonker8::display::text_editor::TextEditor;
use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "chonker8")]
#[command(version = "8.9.0")]
#[command(about = "File browser and processing tool", long_about = None)]
struct Args {
    /// Input file (if not provided, opens file browser)
    input_file: Option<String>,
    
    /// Verbose output
    #[arg(short, long)]
    verbose: bool,
    
    /// Skip file browser and process file directly
    #[arg(long)]
    no_browser: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();
    
    if args.verbose {
        println!("Chonker8 v8.9.0 - File browser and processing tool");
    }
    
    let file_path = if let Some(input_file) = args.input_file {
        if args.no_browser {
            // Process file directly without browser
            process_file(&input_file, args.verbose)?;
            return Ok(());
        } else {
            // File provided but still show browser for selection
            Some(input_file)
        }
    } else {
        None
    };
    
    // Launch application with state management
    if args.verbose {
        println!("Starting Chonker8 with Tab key navigation...");
    }
    
    let mut app_state = AppState::new()?;
    app_state.run()?;
    
    Ok(())
}

fn process_file(file_path: &str, verbose: bool) -> Result<()> {
    if verbose {
        println!("Processing file: {}", file_path);
    }
    
    // Check if file exists
    if !std::path::Path::new(file_path).exists() {
        eprintln!("Error: File does not exist: {}", file_path);
        return Ok(());
    }
    
    // Get file info
    let metadata = std::fs::metadata(file_path)?;
    let file_size = metadata.len();
    let is_dir = metadata.is_dir();
    
    println!("Selected: {}", file_path);
    
    if is_dir {
        println!("Type: Directory");
    } else {
        println!("Type: File");
        println!("Size: {} bytes", file_size);
        
        // Show file extension
        if let Some(extension) = std::path::Path::new(file_path).extension() {
            println!("Extension: {}", extension.to_string_lossy());
        }
    }
    
    // Add your custom file processing logic here
    println!("✓ File processed successfully");
    
    Ok(())
}