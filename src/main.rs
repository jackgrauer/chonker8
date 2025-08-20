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
#[command(author, version, about = "PDF text extraction with DuckDB storage")]
struct Args {
    #[command(subcommand)]
    command: Commands,
    
    /// Database file path (uses in-memory if not specified)
    #[arg(long, global = true)]
    db: Option<PathBuf>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Extract text from a PDF file
    Extract {
        /// PDF file to extract text from
        pdf_file: PathBuf,
        
        /// Page number to extract (1-based indexing)
        #[arg(short, long, default_value_t = 1)]
        page: usize,
        
        /// Output format: grid, text, json
        #[arg(short, long, default_value = "text")]
        format: String,
        
        /// Save extracted text to file
        #[arg(short, long)]
        output: Option<PathBuf>,
        
        /// Grid width for extraction
        #[arg(long, default_value_t = GRID_WIDTH)]
        width: usize,
        
        /// Grid height for extraction  
        #[arg(long, default_value_t = GRID_HEIGHT)]
        height: usize,
        
        /// Use AI-powered extraction (slower but better)
        #[arg(long)]
        ai: bool,
        
        /// Show extraction statistics
        #[arg(long)]
        stats: bool,
        
        /// Store in database
        #[arg(long)]
        store: bool,
        
        /// Raw output without headers
        #[arg(long)]
        raw: bool,
        
        /// Show visual rendering as dots/braille
        #[arg(long)]
        visual: bool,
        
        /// Compare text extraction vs visual rendering
        #[arg(long)]
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
            format, 
            output, 
            width, 
            height, 
            ai, 
            stats,
            store,
            raw,
            visual,
            compare,
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
            
            if !raw {
                println!("Extracting page {} from '{}'...", page, pdf_file.display());
            }
            
            // Extract text and/or visual
            let start_time = std::time::Instant::now();
            
            // Always do text extraction
            let text_grid = if ai {
                if !raw { println!("Using Ferrules for structured extraction..."); }
                pdf_extraction::extract_with_ferrules(&pdf_file, page_index, width, height).await
                    .unwrap_or_else(|e| {
                        if !raw { println!("Ferrules failed ({}), falling back to PDFium...", e); }
                        tokio::task::block_in_place(|| {
                            tokio::runtime::Handle::current().block_on(
                                pdf_extraction::improved::extract_with_word_grouping(&pdf_file, page_index, width, height)
                            )
                        }).unwrap_or_else(|_| vec![vec![' '; width]; height])
                    })
            } else {
                pdf_extraction::improved::extract_with_word_grouping(&pdf_file, page_index, width, height).await?
            };
            
            // Optionally do visual rendering (TRUE ground truth showing actual character positions)
            let visual_grid = if visual || compare {
                if !raw { println!("Generating TRUE visual ground truth (actual character positions)..."); }
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
            
            // Output results
            if compare {
                // Show both extractions side by side
                if !raw {
                    println!("\n--- CONVERGENCE COMPARISON ---");
                    println!("Left: Visual Ground Truth (PDF) | Right: PDFium Text Extraction");
                    println!("{}", "=".repeat(80));
                }
                show_comparison(&visual_grid, &text_grid);  // Swapped order!
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
    }
    
    Ok(())
}

fn format_as_text(grid: &[Vec<char>]) -> String {
    grid.iter()
        .map(|row| {
            let line: String = row.iter().collect();
            line.trim_end().to_string()
        })
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
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