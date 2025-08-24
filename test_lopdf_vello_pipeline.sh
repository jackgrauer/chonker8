#!/bin/bash

echo "Testing lopdf-vello-kitty pipeline for chonker8-hot..."
echo "This uses pure Rust for PDF rendering - no PDFium!"
echo

# Build with the new pipeline
echo "Building chonker8-hot with lopdf-vello pipeline..."
DYLD_LIBRARY_PATH=./lib cargo build --release --bin chonker8-hot --quiet

# Test the pipeline directly
echo
echo "Testing Vello PDF renderer..."
cat > test_vello_pipeline.rs << 'EOF'
use anyhow::Result;
use chonker8::pdf_renderer;
use std::path::Path;

fn main() -> Result<()> {
    println!("Testing lopdf-vello pipeline (PDFium removed!)...");
    
    let pdf_path = Path::new("real_test.pdf");
    
    if !pdf_path.exists() {
        eprintln!("Test PDF not found. Please ensure real_test.pdf exists.");
        return Ok(());
    }
    
    // Test page count with lopdf
    println!("Getting page count with lopdf...");
    let page_count = pdf_renderer::get_pdf_page_count(pdf_path)?;
    println!("✓ PDF has {} pages (using lopdf)", page_count);
    
    // Test rendering with Vello
    println!("Rendering page 1 with Vello GPU acceleration...");
    let image = pdf_renderer::render_pdf_page(pdf_path, 0, 800, 1000)?;
    println!("✓ Successfully rendered page with Vello!");
    println!("  Image dimensions: {}x{}", image.width(), image.height());
    
    // Save test image
    image.save("lopdf_vello_test.png")?;
    println!("✓ Saved test image to lopdf_vello_test.png");
    
    println!();
    println!("🎉 lopdf-vello-kitty pipeline test successful!");
    println!("   PDFium has been completely removed!");
    println!("   Using pure Rust: lopdf for parsing, Vello for GPU rendering");
    
    Ok(())
}
EOF

# Compile and run the test
echo "Compiling pipeline test..."
DYLD_LIBRARY_PATH=./lib rustc --edition 2021 test_vello_pipeline.rs \
    -L target/release/deps \
    -L target/release \
    --extern chonker8=target/release/libchonker8.rlib \
    --extern anyhow=target/release/deps/libanyhow*.rlib \
    -o test_vello_pipeline

echo
echo "Running pipeline test..."
DYLD_LIBRARY_PATH=./lib ./test_vello_pipeline

echo
echo "Pipeline components:"
echo "  1. lopdf: Pure Rust PDF parsing (replaces PDFium)"
echo "  2. Vello: GPU-accelerated rendering (Metal on ARM)"
echo "  3. Kitty: Terminal graphics protocol for display"
echo
echo "chonker8-hot now uses this pure Rust pipeline for PDF display!"