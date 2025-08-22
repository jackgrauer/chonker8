#!/usr/bin/env rust-script

use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Testing extraction router with different PDFs...\n");
    
    // Test PDFs
    let test_files = vec![
        // Scanned PDF (should trigger OCR)
        "/Users/jack/Downloads/righttoknowrequestresultsfortieriidataon4southwes/split_pages/page_0061.pdf",
        // Regular PDF
        "/Users/jack/Desktop/BERF-CERT.pdf",
        // Another test
        "/Users/jack/Desktop/17.pdf",
    ];
    
    for pdf_path in test_files {
        if Path::new(pdf_path).exists() {
            println!("Testing: {}", pdf_path);
            
            // Run chonker8-hot with the PDF
            let output = std::process::Command::new("./target/release/chonker8-hot")
                .env("DYLD_LIBRARY_PATH", "./lib")
                .arg(pdf_path)
                .output()?;
                
            // Check stderr for debug output
            let stderr = String::from_utf8_lossy(&output.stderr);
            
            // Extract relevant debug lines
            for line in stderr.lines() {
                if line.contains("[DEBUG]") && (
                    line.contains("Analyzing page") ||
                    line.contains("Page fingerprint") ||
                    line.contains("Primary extraction method") ||
                    line.contains("Using") ||
                    line.contains("Extraction complete")
                ) {
                    println!("  {}", line);
                }
            }
            
            println!();
        }
    }
    
    Ok(())
}