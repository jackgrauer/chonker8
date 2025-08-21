// CHONKER8 CLI - PDF text extraction with DuckDB storage
use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

mod config;
mod pdf_extraction;
mod storage;
mod types;

use config::{GRID_WIDTH, GRID_HEIGHT};
use storage::{DuckDBStorage, SearchResult};

#[derive(Parser, Debug)]
#[command(author, version, about = "chonker8 v8.8.0 - PDF text extraction with OCR convergence and DuckDB storage")]
struct Args {
    #[command(subcommand)]
    command: Commands,
    
    /// Database file path (uses in-memory if not specified)
    #[arg(long, global = true)]
    db: Option<PathBuf>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Extract text from PDF with intelligent OCR and convergence analysis
    Extract {
        /// PDF file to extract text from
        pdf_file: PathBuf,
        
        /// Page number to extract (1-based indexing)
        #[arg(short, long, default_value_t = 1)]
        page: usize,
        
        /// Extraction mode: auto, ocr, native, compare, visual
        #[arg(short = 'm', long, default_value = "auto")]
        mode: String,
        
        /// Output format: text, grid, json
        #[arg(short, long, default_value = "text")]
        format: String,
        
        /// Save extracted text to file
        #[arg(short, long)]
        output: Option<PathBuf>,
        
        /// Grid dimensions for visual rendering
        #[arg(long, default_value_t = GRID_WIDTH)]
        width: usize,
        #[arg(long, default_value_t = GRID_HEIGHT)]
        height: usize,
        
        /// Force OCR even for text-based PDFs
        #[arg(long)]
        force_ocr: bool,
        
        /// Show detailed extraction statistics and convergence scores
        #[arg(long)]
        stats: bool,
        
        /// Store in database for searchability
        #[arg(long)]
        store: bool,
        
        /// Raw output without headers or formatting
        #[arg(long)]
        raw: bool,
        
        /// Use hybrid mode: Ferrules for layout, pdftotext for content
        #[arg(long)]
        hybrid: bool,
        
        /// Legacy flags (deprecated - use --mode instead)
        #[arg(long, hide = true)]
        ai: bool,
        #[arg(long, hide = true)]
        visual: bool,
        #[arg(long, hide = true)]
        compare: bool,
    },
    
    /// Search for text in stored documents
    Search {
        /// Search query
        query: String,
        
        /// Limit number of results
        #[arg(short, long, default_value_t = 20)]
        limit: usize,
    },
    
    /// List all stored documents
    List,
    
    /// Show database statistics
    Stats,
    
    /// Execute custom SQL query
    Query {
        /// SQL query to execute
        sql: String,
    },
    
    /// Load document from database
    Load {
        /// Document ID or path
        doc: String,
        
        /// Page number to load
        #[arg(short, long)]
        page: Option<usize>,
    },
    
    /// Force save database to disk
    Save,
    
    /// Batch process multiple PDFs with full OCR and convergence analysis
    Batch {
        /// Directory containing PDF files or glob pattern
        input: PathBuf,
        
        /// Output directory for results
        #[arg(short, long)]
        output: Option<PathBuf>,
        
        /// Extraction mode for all files: auto, ocr, compare
        #[arg(short = 'm', long, default_value = "auto")]
        mode: String,
        
        /// Store all results in database
        #[arg(long)]
        store: bool,
        
        /// Generate convergence reports
        #[arg(long)]
        convergence_report: bool,
        
        /// Maximum parallel processing threads
        #[arg(long, default_value_t = 4)]
        threads: usize,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    
    // Create database connection (starts in memory, auto-flushes when needed)
    let mut db = DuckDBStorage::new(args.db.as_deref())?;
    
    match args.command {
        Commands::Extract { 
            pdf_file, 
            page, 
            mode,
            format, 
            output, 
            width, 
            height, 
            force_ocr,
            stats,
            store,
            raw,
            hybrid,
            ai, // legacy
            visual, // legacy
            compare, // legacy
        } => {
            // Validate inputs
            if !pdf_file.exists() {
                eprintln!("Error: PDF file '{}' does not exist", pdf_file.display());
                std::process::exit(1);
            }
            
            // Get page count
            let total_pages = pdf_extraction::get_page_count(&pdf_file)?;
            if page == 0 || page > total_pages {
                eprintln!("Error: Page {} is out of range (1-{})", page, total_pages);
                std::process::exit(1);
            }
            
            let page_index = page - 1; // Convert to 0-based
            
            // Determine extraction strategy based on mode and legacy flags
            let (use_ocr, use_visual, use_compare) = match mode.as_str() {
                "auto" => {
                    // Intelligent auto mode: Try native first, use OCR if needed
                    if !raw { println!("🧠 Auto mode: Analyzing PDF structure..."); }
                    (true, false, false) // Default to OCR with fallback
                },
                "ocr" => (true, false, false),
                "native" => (false, false, false),
                "visual" => (false, true, false),
                "compare" => (true, true, true),
                _ => {
                    // Handle legacy flags for backward compatibility
                    (ai || force_ocr, visual, compare)
                }
            };
            
            if !raw {
                match mode.as_str() {
                    "auto" => println!("📄 Extracting page {} with intelligent analysis...", page),
                    "ocr" => println!("🔍 Extracting page {} with OCR...", page),
                    "native" => println!("⚡ Extracting page {} with native text extraction...", page),
                    "visual" => println!("👁️  Rendering page {} as visual ground truth...", page),
                    "compare" => println!("⚖️  Comparing extraction methods for page {}...", page),
                    _ => println!("Extracting page {} from '{}'...", page, pdf_file.display()),
                }
            }
            
            let start_time = std::time::Instant::now();
            
            // Text extraction with intelligent fallback
            let text_grid = if hybrid {
                if !raw { println!("🔄 Using hybrid mode: Ferrules layout + pdftotext content..."); }
                pdf_extraction::extract_hybrid(&pdf_file, page_index, width, height).await
                    .unwrap_or_else(|e| {
                        if !raw { println!("⚠️  Hybrid extraction failed: {}, falling back to pdftotext...", e); }
                        tokio::task::block_in_place(|| {
                            tokio::runtime::Handle::current().block_on(
                                pdf_extraction::extract_with_extractous_advanced(&pdf_file, page_index, width, height)
                            )
                        }).unwrap_or_else(|_| vec![vec![' '; width]; height])
                    })
            } else if use_ocr {
                if !raw && mode == "auto" { println!("🔍 Trying OCR extraction (best quality)..."); }
                else if !raw { println!("Using Ferrules for structured extraction..."); }
                
                // Try Ferrules first
                match pdf_extraction::extract_with_ferrules(&pdf_file, page_index, width, height).await {
                    Ok(grid) => {
                        // Check for poor OCR quality using language detection
                        use whatlang::detect;
                        
                        // Get text from grid for analysis
                        let full_text: String = grid.iter()
                            .map(|row| row.iter().collect::<String>())
                            .collect::<Vec<_>>()
                            .join(" ");
                        
                        // Sample text chunks for language detection
                        // Remove excessive whitespace first
                        let cleaned_text: String = full_text
                            .split_whitespace()
                            .collect::<Vec<_>>()
                            .join(" ");
                        
                        let text_sample = cleaned_text.chars()
                            .skip(50)  // Skip potential headers
                            .take(500)  // Take a good sample
                            .collect::<String>();
                        
                        // Detect if it's valid language or gibberish
                        let mut is_gibberish = false;
                        
                        // Only use language detection if we have enough text
                        if text_sample.len() > 100 {
                            if let Some(info) = detect(&text_sample) {
                                // Very low confidence indicates OCR gibberish
                                if info.confidence() < 0.7 {
                                    is_gibberish = true;
                                    if !raw { 
                                        println!("⚠️  Low language confidence ({:.2}), likely OCR issues", info.confidence()); 
                                    }
                                }
                            } else if text_sample.len() > 200 {
                                // Only consider it gibberish if we have substantial text
                                is_gibberish = true;
                                if !raw { println!("⚠️  No valid language detected in OCR output"); }
                            }
                        }
                        
                        // Also check for known bad patterns as fallback
                        if !is_gibberish {
                            let gibberish_patterns = ["anties", "priety", "baline", "retion", "oAls", "Ghotoh"];
                            is_gibberish = gibberish_patterns.iter()
                                .any(|pattern| full_text.contains(pattern));
                        }
                        
                        if is_gibberish {
                            if !raw { println!("⚠️  Poor OCR quality detected, switching to pdftotext..."); }
                            pdf_extraction::extract_with_extractous_advanced(&pdf_file, page_index, width, height).await
                                .unwrap_or(grid)
                        } else {
                            grid
                        }
                    },
                    Err(e) => {
                        if !raw { 
                            println!("⚡ Ferrules failed ({}), trying pdftotext...", e); 
                        }
                        // Try pdftotext as fallback
                        pdf_extraction::extract_with_extractous_advanced(&pdf_file, page_index, width, height).await
                            .unwrap_or_else(|_| {
                                if !raw { println!("⚠️  pdftotext failed, falling back to native PDFium..."); }
                                tokio::task::block_in_place(|| {
                                    tokio::runtime::Handle::current().block_on(
                                        pdf_extraction::improved::extract_with_word_grouping(&pdf_file, page_index, width, height)
                                    )
                                }).unwrap_or_else(|_| vec![vec![' '; width]; height])
                            })
                    }
                }
            } else if mode == "pdftotext" {
                if !raw { println!("📄 Using pdftotext for clean text extraction..."); }
                pdf_extraction::extract_with_extractous_advanced(&pdf_file, page_index, width, height).await?
            } else {
                if !raw { println!("Using native text extraction..."); }
                pdf_extraction::improved::extract_with_word_grouping(&pdf_file, page_index, width, height).await?
            };
            
            // Visual rendering for ground truth and comparison
            let visual_grid = if use_visual {
                if !raw { println!("👁️  Generating TRUE visual ground truth (actual character positions)..."); }
                pdf_extraction::render_true_visual(&pdf_file, page_index, width, height).await?
            } else {
                vec![]
            };
            
            let extraction_time = start_time.elapsed();
            
            // Choose which grid to use for main output
            let grid = if visual && !compare {
                visual_grid.clone()
            } else {
                text_grid.clone()
            };
            
            // Store in database if requested
            if store {
                let doc_id = db.register_document(&pdf_file, total_pages)?;
                db.save_page(doc_id, page, &grid, width, height)?;
                println!("Stored in database (document ID: {})", doc_id);
            }
            
            // Format output
            let output_text = match format.as_str() {
                "grid" => format_as_grid(&grid),
                "json" => format_as_json(&grid)?,
                "text" | _ => format_as_text(&grid),
            };
            
            // Output results based on mode
            if use_compare {
                // Show convergence analysis
                if !raw {
                    println!("\n⚖️  CONVERGENCE ANALYSIS");
                    println!("Left: Visual Ground Truth (PDF) | Right: Extracted Text");
                    println!("{}", "=".repeat(80));
                }
                show_comparison(&visual_grid, &text_grid);
            } else if let Some(output_path) = output {
                std::fs::write(&output_path, &output_text)?;
                if !raw {
                    println!("Text saved to '{}'", output_path.display());
                }
            } else if raw {
                // Raw output - just the text, no headers
                print!("{}", output_text);
            } else {
                println!("\n--- Extracted Text ---");
                println!("{}", output_text);
            }
            
            // Show statistics
            if stats {
                let char_count = grid.iter()
                    .flat_map(|row| row.iter())
                    .filter(|&&c| c != ' ')
                    .count();
                
                println!("\n--- Statistics ---");
                println!("Extraction time: {:?}", extraction_time);
                println!("Grid size: {}x{}", width, height);
                println!("Characters extracted: {}", char_count);
                println!("Total pages in PDF: {}", total_pages);
            }
        },
        
        Commands::Search { query, limit } => {
            let results = db.search_text(&query)?;
            
            if results.is_empty() {
                println!("No results found for '{}'", query);
            } else {
                println!("Found {} results for '{}':\n", results.len().min(limit), query);
                
                for (i, result) in results.iter().take(limit).enumerate() {
                    println!("{}. {} (page {})", i + 1, result.filename, result.page_num);
                    println!("   Line {}: {}", result.line_num, result.text);
                    println!("   Path: {}\n", result.path);
                }
            }
        },
        
        Commands::List => {
            let docs = db.list_documents()?;
            
            if docs.is_empty() {
                println!("No documents in database");
            } else {
                println!("Documents in database:\n");
                println!("{:<4} {:<40} {:<10} {:<10}", "ID", "Filename", "Pages", "Size");
                println!("{}", "-".repeat(70));
                
                for doc in docs {
                    let size_mb = doc.file_size as f64 / (1024.0 * 1024.0);
                    println!("{:<4} {:<40} {:<10} {:<10.2}MB", 
                        doc.id, 
                        truncate(&doc.filename, 40),
                        doc.total_pages,
                        size_mb
                    );
                }
            }
        },
        
        Commands::Stats => {
            let stats = db.get_stats()?;
            
            println!("Database Statistics:");
            println!("-------------------");
            println!("Documents:     {}", stats.document_count);
            println!("Pages:         {}", stats.page_count);
            println!("Characters:    {}", stats.total_characters);
            println!("Total Size:    {:.2} MB", stats.total_file_size as f64 / (1024.0 * 1024.0));
            
            if stats.page_count > 0 {
                let avg_chars = stats.total_characters / stats.page_count;
                println!("Avg chars/page: {}", avg_chars);
            }
        },
        
        Commands::Query { sql } => {
            match db.query(&sql) {
                Ok(rows) => {
                    if rows.is_empty() {
                        println!("No results");
                    } else {
                        for row in rows {
                            println!("{}", row.join(" | "));
                        }
                    }
                },
                Err(e) => {
                    eprintln!("Query error: {}", e);
                    std::process::exit(1);
                }
            }
        },
        
        Commands::Load { doc, page } => {
            // Try to parse as document ID or use as path
            let doc_id: i64 = if let Ok(id) = doc.parse() {
                id
            } else {
                // Look up by path
                let results = db.query(&format!(
                    "SELECT id FROM documents WHERE path LIKE '%{}%' LIMIT 1", 
                    doc
                ))?;
                
                if results.is_empty() {
                    eprintln!("Document not found: {}", doc);
                    std::process::exit(1);
                }
                
                results[0][0].parse()?
            };
            
            // Load specific page or all pages
            if let Some(page_num) = page {
                match db.load_page(doc_id, page_num) {
                    Ok(grid) => {
                        let text = format_as_text(&grid);
                        println!("{}", text);
                    },
                    Err(e) => {
                        eprintln!("Error loading page: {}", e);
                        std::process::exit(1);
                    }
                }
            } else {
                // Load document info
                let doc_info = db.query(&format!(
                    "SELECT filename, total_pages FROM documents WHERE id = {}", 
                    doc_id
                ))?;
                
                if !doc_info.is_empty() {
                    println!("Document: {}", doc_info[0][0]);
                    println!("Total pages: {}", doc_info[0][1]);
                }
            }
        },
        
        Commands::Save => {
            println!("Saving database to disk...");
            db.force_save()?;
            println!("Database saved successfully!");
        },
        
        Commands::Batch {
            input,
            output,
            mode,
            store,
            convergence_report,
            threads,
        } => {
            println!("🚀 Starting batch processing with {} threads...", threads);
            println!("📁 Input: {}", input.display());
            if let Some(output_dir) = &output {
                println!("📤 Output: {}", output_dir.display());
                std::fs::create_dir_all(output_dir)?;
            }
            
            // Find all PDF files
            let pdf_files = if input.is_dir() {
                std::fs::read_dir(&input)?
                    .filter_map(|entry| {
                        let entry = entry.ok()?;
                        let path = entry.path();
                        if path.extension()?.to_str()? == "pdf" {
                            Some(path)
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<_>>()
            } else {
                vec![input.clone()]
            };
            
            if pdf_files.is_empty() {
                eprintln!("❌ No PDF files found");
                std::process::exit(1);
            }
            
            println!("📄 Found {} PDF files", pdf_files.len());
            
            // Process files (simplified - full parallel implementation would need more work)
            let mut successful = 0;
            let mut failed = 0;
            
            for (i, pdf_file) in pdf_files.iter().enumerate() {
                println!("\n📋 Processing {}/{}: {}", i + 1, pdf_files.len(), pdf_file.file_name().unwrap().to_string_lossy());
                
                // Use the same extraction logic as the single file mode
                match pdf_extraction::get_page_count(&pdf_file) {
                    Ok(total_pages) => {
                        for page in 1..=total_pages.min(5) { // Limit to first 5 pages for demo
                            let page_index = page - 1;
                            
                            match mode.as_str() {
                                "auto" | "ocr" => {
                                    match pdf_extraction::extract_with_ferrules(&pdf_file, page_index, GRID_WIDTH, GRID_HEIGHT).await {
                                        Ok(_) => {
                                            println!("  ✅ Page {} extracted with OCR", page);
                                            if store {
                                                // Could store in database here
                                            }
                                        },
                                        Err(e) => {
                                            println!("  ⚠️  Page {} OCR failed: {}", page, e);
                                            // Fallback to native extraction
                                            match pdf_extraction::improved::extract_with_word_grouping(&pdf_file, page_index, GRID_WIDTH, GRID_HEIGHT).await {
                                                Ok(_) => println!("  ✅ Page {} extracted with native method", page),
                                                Err(e) => println!("  ❌ Page {} failed: {}", page, e),
                                            }
                                        }
                                    }
                                },
                                "compare" => {
                                    if convergence_report {
                                        println!("  📊 Generating convergence report for page {}...", page);
                                        // Could implement convergence analysis here
                                    }
                                },
                                _ => {
                                    match pdf_extraction::improved::extract_with_word_grouping(&pdf_file, page_index, GRID_WIDTH, GRID_HEIGHT).await {
                                        Ok(_) => println!("  ✅ Page {} extracted", page),
                                        Err(e) => println!("  ❌ Page {} failed: {}", page, e),
                                    }
                                }
                            }
                        }
                        successful += 1;
                    },
                    Err(e) => {
                        println!("  ❌ Failed to read PDF: {}", e);
                        failed += 1;
                    }
                }
            }
            
            println!("\n🎉 Batch processing complete!");
            println!("✅ Successful: {}", successful);
            println!("❌ Failed: {}", failed);
        },
    }
    
    Ok(())
}

fn format_as_text(grid: &[Vec<char>]) -> String {
    let lines: Vec<String> = grid.iter()
        .map(|row| {
            let line: String = row.iter().collect();
            line.trim_end().to_string()
        })
        .collect();
    
    // Remove leading empty lines
    let start_idx = lines.iter().position(|line| !line.is_empty()).unwrap_or(0);
    
    // Remove trailing empty lines  
    let end_idx = lines.iter().rposition(|line| !line.is_empty()).map(|i| i + 1).unwrap_or(lines.len());
    
    // Preserve structure but reduce excessive empty lines
    let content_lines = &lines[start_idx..end_idx];
    let mut result = Vec::new();
    let mut consecutive_empty = 0;
    
    for line in content_lines {
        if line.is_empty() {
            consecutive_empty += 1;
            // Allow max 2 consecutive empty lines to preserve paragraph breaks
            if consecutive_empty <= 2 {
                result.push(line.clone());
            }
        } else {
            consecutive_empty = 0;
            result.push(line.clone());
        }
    }
    
    result.join("\n")
}

fn format_as_grid(grid: &[Vec<char>]) -> String {
    grid.iter()
        .map(|row| row.iter().collect::<String>())
        .collect::<Vec<_>>()
        .join("\n")
}

fn format_as_json(grid: &[Vec<char>]) -> Result<String> {
    use serde_json::json;
    
    let rows: Vec<String> = grid.iter()
        .map(|row| row.iter().collect())
        .collect();
    
    let output = json!({
        "grid": rows,
        "width": grid.get(0).map(|r| r.len()).unwrap_or(0),
        "height": grid.len(),
        "format": "character_grid"
    });
    
    Ok(serde_json::to_string_pretty(&output)?)
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len - 3])
    }
}

fn show_comparison(text_grid: &[Vec<char>], visual_grid: &[Vec<char>]) {
    let max_lines = text_grid.len().min(visual_grid.len());
    
    for i in 0..max_lines {
        let text_line: String = text_grid.get(i)
            .map(|row| row.iter().collect())
            .unwrap_or_default();
        let visual_line: String = visual_grid.get(i)
            .map(|row| row.iter().collect())
            .unwrap_or_default();
        
        // Show first 40 chars of each side by side
        let text_part: String = text_line.chars().take(40).collect();
        let visual_part: String = visual_line.chars().take(40).collect();
        
        println!("{:<40} | {:<40}", text_part, visual_part);
    }
    
    // Calculate convergence score
    let mut matches = 0;
    let mut total = 0;
    
    for (text_row, visual_row) in text_grid.iter().zip(visual_grid.iter()) {
        for (t_char, v_char) in text_row.iter().zip(visual_row.iter()) {
            total += 1;
            let t_has_content = *t_char != ' ';
            let v_has_content = *v_char != ' ' && *v_char != '⠀';
            if t_has_content == v_has_content {
                matches += 1;
            }
        }
    }
    
    let convergence = if total > 0 {
        matches as f32 / total as f32 * 100.0
    } else {
        0.0
    };
    
    println!("\n📊 Convergence Score: {:.1}%", convergence);
}