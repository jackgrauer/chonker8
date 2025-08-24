#!/bin/bash

echo "Testing lopdf-vello-kitty pipeline for chonker8-hot..."

# Build the project
echo "Building chonker8-hot with Vello renderer..."
DYLD_LIBRARY_PATH=./lib cargo build --release --bin chonker8-hot --quiet

# Test rendering a PDF page
echo "Testing PDF rendering with Vello..."
cat > test_vello_direct.rs << 'EOF'
use anyhow::Result;
use chonker8::pdf_renderer;
use std::path::Path;

fn main() -> Result<()> {
    println!("Testing Vello PDF renderer in chonker8...");
    
    let pdf_path = Path::new("real_test.pdf");
    
    if !pdf_path.exists() {
        eprintln!("Test PDF not found. Please ensure real_test.pdf exists.");
        return Ok(());
    }
    
    // Test page count
    println!("Getting page count...");
    let page_count = pdf_renderer::get_pdf_page_count(pdf_path)?;
    println!("PDF has {} pages", page_count);
    
    // Test rendering
    println!("Rendering page 1 with Vello...");
    let image = pdf_renderer::render_pdf_page(pdf_path, 0, 800, 1000)?;
    println!("Successfully rendered page!");
    println!("Image dimensions: {}x{}", image.width(), image.height());
    
    // Save test image
    image.save("vello_ui_test.png")?;
    println!("Saved test image to vello_ui_test.png");
    
    println!("✅ lopdf-vello pipeline test successful!");
    
    Ok(())
}
EOF

# Compile and run the test
echo "Compiling test..."
DYLD_LIBRARY_PATH=./lib rustc --edition 2021 test_vello_direct.rs \
    -L target/release/deps \
    -L target/release \
    --extern chonker8=target/release/libchonker8.rlib \
    --extern anyhow=target/release/deps/libanyhow*.rlib \
    -o test_vello_direct

echo "Running test..."
DYLD_LIBRARY_PATH=./lib ./test_vello_direct

echo "Done!"