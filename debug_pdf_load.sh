#!/bin/bash

echo "Testing PDF loading directly..."

# Create a simple test that loads a PDF
cat > test_direct_load.rs << 'EOF'
use chonker8::pdf_extraction::{DocumentAnalyzer, basic};
use std::path::Path;

fn main() {
    let pdf_path = Path::new("/Users/jack/Desktop/BERF-CERT.pdf");
    
    if !pdf_path.exists() {
        println!("PDF not found at: {:?}", pdf_path);
        return;
    }
    
    println!("Testing PDF load for: {:?}", pdf_path);
    
    // Test 1: Basic extraction
    println!("1. Testing basic extraction...");
    match basic::extract_with_pdfium_sync(pdf_path, 0) {
        Ok(text) => println!("   ✓ Got {} chars", text.len()),
        Err(e) => println!("   ✗ Error: {}", e),
    }
    
    // Test 2: Document analyzer
    println!("2. Testing document analyzer...");
    match DocumentAnalyzer::new() {
        Ok(analyzer) => {
            match analyzer.analyze_page(pdf_path, 0) {
                Ok(fp) => println!("   ✓ Text: {:.1}%, Images: {:.1}%", 
                    fp.text_coverage * 100.0, fp.image_coverage * 100.0),
                Err(e) => println!("   ✗ Error: {}", e),
            }
        }
        Err(e) => println!("   ✗ Failed to create analyzer: {}", e),
    }
    
    println!("Direct load test complete!");
}
EOF

# Compile and run
echo "Compiling test..."
DYLD_LIBRARY_PATH=./lib rustc test_direct_load.rs \
    -L target/release/deps \
    --extern chonker8=target/release/libchonker8.rlib \
    --extern anyhow=target/release/deps/libanyhow*.rlib \
    --extern pdfium_render=target/release/deps/libpdfium_render*.rlib \
    --edition 2021 \
    -o test_direct_load 2>/dev/null

if [ $? -eq 0 ]; then
    echo "Running test..."
    DYLD_LIBRARY_PATH=./lib ./test_direct_load
else
    echo "Compilation failed, trying with cargo..."
    # Fallback to using cargo
    DYLD_LIBRARY_PATH=./lib cargo run --release --bin test-extraction -- analyze /Users/jack/Desktop/BERF-CERT.pdf --page 0
fi

# Clean up
rm -f test_direct_load test_direct_load.rs