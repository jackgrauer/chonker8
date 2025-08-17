use ferrules_core::{
    FerrulesParser,
    FerrulesParseConfig,
    layout::model::{ORTConfig, OrtExecutionProvider},
};
use std::path::Path;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("Testing ferrules...");
    
    // Configure ferrules with CPU execution
    let ort_config = ORTConfig {
        execution_providers: vec![OrtExecutionProvider::CPU],
        intra_threads: 16,
        inter_threads: 4,
        opt_level: None,
    };
    
    println!("Creating parser...");
    let parser = FerrulesParser::new(ort_config);
    println!("Parser created successfully!");
    
    // Test with a simple PDF
    let pdf_path = Path::new("/Users/jack/Documents/chonker_test.pdf");
    if !pdf_path.exists() {
        println!("Test PDF not found at {:?}", pdf_path);
        return Ok(());
    }
    
    println!("Reading PDF...");
    let doc_bytes = std::fs::read(pdf_path)?;
    println!("PDF size: {} bytes", doc_bytes.len());
    
    let config = FerrulesParseConfig {
        password: None,
        flatten_pdf: true,
        page_range: None,
        debug_dir: Some("/tmp/ferrules_test".into()),
    };
    
    println!("Parsing document...");
    let parsed_doc = parser.parse_document(
        &doc_bytes,
        "test.pdf".to_string(),
        config,
        Some(|page_id| println!("Processed page {}", page_id)),
    ).await?;
    
    println!("\nResults:");
    println!("  Pages: {}", parsed_doc.pages.len());
    println!("  Blocks: {}", parsed_doc.blocks.len());
    
    for (i, page) in parsed_doc.pages.iter().enumerate() {
        println!("  Page {}: id={}, size={}x{}", i, page.id, page.width, page.height);
    }
    
    for (i, block) in parsed_doc.blocks.iter().take(5).enumerate() {
        println!("  Block {}: pages={:?}, type={:?}", 
            i, block.pages_id, std::mem::discriminant(&block.kind));
    }
    
    Ok(())
}