use anyhow::Result;
use chonker8::pdf_extraction::{
    DocumentAnalyzer, PageFingerprint,
    ExtractionRouter, ExtractionMethod, ExtractionResult,
};
use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "test-extraction")]
#[command(about = "Test document-agnostic PDF extraction")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Analyze a PDF document
    Analyze {
        /// Path to PDF file
        pdf: PathBuf,
        /// Page number (0-indexed)
        #[arg(short, long, default_value_t = 0)]
        page: usize,
    },
    /// Extract text using automatic routing
    Extract {
        /// Path to PDF file
        pdf: PathBuf,
        /// Page number (0-indexed)
        #[arg(short, long, default_value_t = 0)]
        page: usize,
        /// Show detailed information
        #[arg(short, long)]
        verbose: bool,
    },
    /// Test fallback chain
    Fallback {
        /// Path to PDF file
        pdf: PathBuf,
        /// Page number (0-indexed)
        #[arg(short, long, default_value_t = 0)]
        page: usize,
    },
    /// Run full pipeline test
    Pipeline {
        /// Path to PDF file
        pdf: PathBuf,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    
    match cli.command {
        Commands::Analyze { pdf, page } => {
            analyze_document(&pdf, page).await?;
        }
        Commands::Extract { pdf, page, verbose } => {
            extract_with_routing(&pdf, page, verbose).await?;
        }
        Commands::Fallback { pdf, page } => {
            test_fallback_chain(&pdf, page).await?;
        }
        Commands::Pipeline { pdf } => {
            run_full_pipeline(&pdf).await?;
        }
    }
    
    Ok(())
}

async fn analyze_document(pdf_path: &PathBuf, page: usize) -> Result<()> {
    println!("Analyzing PDF: {}", pdf_path.display());
    println!("Page: {}", page);
    println!("{}", "=".repeat(50));
    
    let analyzer = DocumentAnalyzer::new()?;
    let fingerprint = analyzer.analyze_page(pdf_path, page)?;
    
    print_fingerprint(&fingerprint);
    
    let strategy = ExtractionRouter::determine_strategy(&fingerprint);
    println!("\nRecommended extraction method: {:?}", strategy);
    
    Ok(())
}

async fn extract_with_routing(pdf_path: &PathBuf, page: usize, verbose: bool) -> Result<()> {
    println!("Extracting from PDF: {}", pdf_path.display());
    println!("Page: {}", page);
    println!("{}", "=".repeat(50));
    
    // Analyze page
    let analyzer = DocumentAnalyzer::new()?;
    let fingerprint = analyzer.analyze_page(pdf_path, page)?;
    
    if verbose {
        print_fingerprint(&fingerprint);
    }
    
    // Determine strategy
    let strategy = ExtractionRouter::determine_strategy(&fingerprint);
    println!("Selected method: {:?}", strategy);
    
    // Extract with fallback
    let result = ExtractionRouter::extract_with_fallback(pdf_path, page, &fingerprint).await?;
    
    println!("\nExtraction Results:");
    println!("{}", "-".repeat(30));
    println!("Method used: {:?}", result.method);
    println!("Quality score: {:.2}", result.quality_score);
    println!("Extraction time: {}ms", result.extraction_time_ms);
    println!("\nExtracted text (first 500 chars):");
    println!("{}", &result.text[..result.text.len().min(500)]);
    
    Ok(())
}

async fn test_fallback_chain(pdf_path: &PathBuf, page: usize) -> Result<()> {
    println!("Testing fallback chain for: {}", pdf_path.display());
    println!("Page: {}", page);
    println!("{}", "=".repeat(50));
    
    let analyzer = DocumentAnalyzer::new()?;
    let fingerprint = analyzer.analyze_page(pdf_path, page)?;
    
    let primary = ExtractionRouter::determine_strategy(&fingerprint);
    let fallback_chain = ExtractionRouter::get_fallback_chain(&primary);
    
    println!("Primary method: {:?}", primary);
    println!("Fallback chain: {:?}", fallback_chain);
    
    // Try each method
    println!("\nTrying each method:");
    println!("{}", "-".repeat(30));
    
    for method in [primary.clone()].iter().chain(fallback_chain.iter()) {
        println!("\nTrying method: {:?}", method);
        
        match try_extraction_method(pdf_path, page, method).await {
            Ok(result) => {
                println!("  Quality: {:.2}", result.quality_score);
                println!("  Time: {}ms", result.extraction_time_ms);
                println!("  Text preview: {}", 
                    &result.text[..result.text.len().min(50)]);
            }
            Err(e) => {
                println!("  Failed: {}", e);
            }
        }
    }
    
    Ok(())
}

async fn run_full_pipeline(pdf_path: &PathBuf) -> Result<()> {
    println!("Running full pipeline for: {}", pdf_path.display());
    println!("{}", "=".repeat(50));
    
    let analyzer = DocumentAnalyzer::new()?;
    
    // Get page count
    let page_count = chonker8::pdf_extraction::basic::get_page_count(pdf_path)?;
    println!("Document has {} pages", page_count);
    
    // Analyze all pages
    println!("\nAnalyzing all pages:");
    println!("{}", "-".repeat(30));
    
    for page in 0..page_count.min(5) {  // Limit to first 5 pages for testing
        println!("\nPage {}:", page);
        
        let fingerprint = analyzer.analyze_page(pdf_path, page)?;
        let strategy = ExtractionRouter::determine_strategy(&fingerprint);
        
        println!("  Text coverage: {:.1}%", fingerprint.text_coverage * 100.0);
        println!("  Image coverage: {:.1}%", fingerprint.image_coverage * 100.0);
        println!("  Has tables: {}", fingerprint.has_tables);
        println!("  Text quality: {:.2}", fingerprint.text_quality);
        println!("  Recommended: {:?}", strategy);
        
        // Extract
        let result = ExtractionRouter::extract_with_fallback(pdf_path, page, &fingerprint).await?;
        println!("  Extraction quality: {:.2}", result.quality_score);
    }
    
    Ok(())
}

fn print_fingerprint(fingerprint: &PageFingerprint) {
    println!("\nPage Fingerprint:");
    println!("{}", "-".repeat(30));
    println!("Text coverage: {:.1}%", fingerprint.text_coverage * 100.0);
    println!("Image coverage: {:.1}%", fingerprint.image_coverage * 100.0);
    println!("Character count: {}", fingerprint.char_count);
    println!("Has tables: {}", fingerprint.has_tables);
    println!("Text quality: {:.2}", fingerprint.text_quality);
    println!("Analysis time: {}ms", fingerprint.extraction_time_ms);
}

async fn try_extraction_method(
    pdf_path: &PathBuf,
    page: usize,
    method: &ExtractionMethod,
) -> Result<ExtractionResult> {
    use std::time::Instant;
    let start = Instant::now();
    
    let text = match method {
        ExtractionMethod::NativeText => {
            chonker8::pdf_extraction::basic::extract_with_pdfium(pdf_path, page).await?
        }
        ExtractionMethod::FastText => {
            let matrix = chonker8::pdf_extraction::pdftotext_extraction::extract_with_pdftotext(
                pdf_path,
                page,
                80,
                40,
            ).await?;
            // Convert matrix to string
            matrix.iter()
                .map(|row| row.iter().collect::<String>())
                .collect::<Vec<_>>()
                .join("\n")
        }
        ExtractionMethod::OCR | ExtractionMethod::LayoutAnalysis => {
            // For now, fallback to native
            chonker8::pdf_extraction::basic::extract_with_pdfium(pdf_path, page).await?
        }
    };
    
    let mut result = ExtractionResult::new(text, method.clone());
    result.extraction_time_ms = start.elapsed().as_millis() as u64;
    
    Ok(result)
}