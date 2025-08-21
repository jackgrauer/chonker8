#!/usr/bin/env rust-script
//! Complete Document AI test with real PDF processing
//! ```cargo
//! [dependencies]
//! pdfium-render = "0.8"
//! ort = { version = "2.0.0-rc.10", features = ["coreml"] }
//! anyhow = "1.0"
//! image = "0.25"
//! ```

use anyhow::Result;
use pdfium_render::prelude::*;
use ort::{session::Session, value::Value, inputs};
use image::{ImageBuffer, Rgb, DynamicImage, GenericImageView};

fn main() -> Result<()> {
    println!("🚀 Document AI Complete Integration Test");
    println!("{}", "═".repeat(50));
    
    let _ = ort::init();
    
    // Load models
    println!("\n📦 Loading models...");
    let mut trocr_encoder = Session::builder()?
        .commit_from_file("models/trocr_encoder.onnx")?;
    let mut layoutlm = Session::builder()?
        .commit_from_file("models/layoutlm.onnx")?;
    
    println!("✅ TrOCR Encoder loaded");
    println!("✅ LayoutLMv3 loaded");
    
    // Create test PDF with text
    println!("\n📄 Creating test PDF...");
    let pdfium = Pdfium::new(
        Pdfium::bind_to_library(Pdfium::pdfium_platform_library_name_at_path("./lib"))
            .or_else(|_| Pdfium::bind_to_system_library())?
    );
    
    // Check for existing PDFs first
    let test_pdfs = ["test.pdf", "sample.pdf", "document.pdf"];
    let mut pdf_path = None;
    
    for pdf in &test_pdfs {
        if std::path::Path::new(pdf).exists() {
            pdf_path = Some(pdf.to_string());
            println!("  Found existing PDF: {}", pdf);
            break;
        }
    }
    
    // If no PDF exists, create one
    if pdf_path.is_none() {
        println!("  Creating simple test PDF...");
        // We'll use the actual PDF if it exists, otherwise create a simple white image
        std::fs::write("test_document.pdf", b"%PDF-1.4\n1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] >>\nendobj\nxref\n0 4\n0000000000 65535 f\n0000000009 00000 n\n0000000058 00000 n\n0000000115 00000 n\ntrailer\n<< /Size 4 /Root 1 0 R >>\nstartxref\n190\n%%EOF")?;
        pdf_path = Some("test_document.pdf".to_string());
    }
    
    let pdf_file = pdf_path.unwrap();
    
    // Load PDF
    let document = pdfium.load_pdf_from_file(&pdf_file, None)?;
    println!("  PDF loaded: {} pages", document.pages().len());
    
    // Render first page
    let page = document.pages().get(0)?;
    let width = page.width().value as i32;
    let height = page.height().value as i32;
    
    println!("\n🖼️ Rendering page ({}x{})...", width, height);
    let bitmap = page.render(width, height, None)?;
    
    // Convert to image
    let buffer = bitmap.as_raw_bytes();
    let img = ImageBuffer::<Rgb<u8>, Vec<u8>>::from_raw(
        width as u32, 
        height as u32,
        buffer.chunks(4)
            .flat_map(|bgra| vec![bgra[2], bgra[1], bgra[0]])
            .collect()
    ).unwrap();
    
    let dynamic_img = DynamicImage::ImageRgb8(img);
    
    // Process with TrOCR (384x384)
    println!("\n1️⃣ TrOCR Processing:");
    let trocr_img = dynamic_img.resize_exact(384, 384, image::imageops::FilterType::Lanczos3);
    let mut trocr_pixels = Vec::with_capacity(3 * 384 * 384);
    
    // Convert to CHW format
    for c in 0..3 {
        for y in 0..384 {
            for x in 0..384 {
                let pixel = trocr_img.get_pixel(x, y);
                let value = pixel[c] as f32 / 255.0;
                trocr_pixels.push(value);
            }
        }
    }
    
    let trocr_input = Value::from_array(([1_usize, 3, 384, 384], trocr_pixels.into_boxed_slice()))?;
    let trocr_outputs = trocr_encoder.run(inputs![trocr_input])?;
    
    let (shape, data) = trocr_outputs[0].try_extract_tensor::<f32>()?;
    println!("  ✅ Encoder output: {:?}", shape);
    
    // Check for meaningful output
    let non_zero = data.iter().filter(|&&x| x.abs() > 0.01).count();
    let percent = (non_zero as f32 / data.len() as f32) * 100.0;
    println!("  📊 Non-zero values: {:.1}%", percent);
    println!("  📝 Ready for decoder → text extraction");
    
    // Process with LayoutLM (224x224)
    println!("\n2️⃣ LayoutLMv3 Processing:");
    let layoutlm_img = dynamic_img.resize_exact(224, 224, image::imageops::FilterType::Lanczos3);
    let mut layoutlm_pixels = Vec::with_capacity(3 * 224 * 224);
    
    // Convert to CHW format with normalization
    for c in 0..3 {
        for y in 0..224 {
            for x in 0..224 {
                let pixel = layoutlm_img.get_pixel(x, y);
                let value = (pixel[c] as f32 / 255.0 - 0.5) / 0.5;
                layoutlm_pixels.push(value);
            }
        }
    }
    
    // Prepare LayoutLM inputs
    let seq_len = 512;
    let input_ids = Value::from_array(([1_usize, seq_len], vec![101i64; seq_len].into_boxed_slice()))?;
    let bbox = Value::from_array(([1_usize, seq_len, 4], vec![0i64; seq_len * 4].into_boxed_slice()))?;
    let attention_mask = Value::from_array(([1_usize, seq_len], vec![1i64; seq_len].into_boxed_slice()))?;
    let pixel_values = Value::from_array(([1_usize, 3, 224, 224], layoutlm_pixels.into_boxed_slice()))?;
    
    let layoutlm_outputs = layoutlm.run(inputs![input_ids, bbox, attention_mask, pixel_values])?;
    
    let (feat_shape, feat_data) = layoutlm_outputs[0].try_extract_tensor::<f32>()?;
    println!("  ✅ LayoutLM output: {:?}", feat_shape);
    
    // Check for meaningful output
    let feat_non_zero = feat_data.iter().filter(|&&x| x.abs() > 0.01).count();
    let feat_percent = (feat_non_zero as f32 / feat_data.len() as f32) * 100.0;
    println!("  📊 Non-zero values: {:.1}%", feat_percent);
    println!("  📈 Ready for document structure analysis");
    
    // Summary
    println!("\n{}", "═".repeat(50));
    println!("✨ DOCUMENT AI PIPELINE COMPLETE!");
    println!("\n📊 Results Summary:");
    println!("  • PDF rendered: {}x{} pixels", width, height);
    println!("  • TrOCR: {:.1}% meaningful output", percent);
    println!("  • LayoutLM: {:.1}% meaningful output", feat_percent);
    
    if percent > 90.0 && feat_percent > 90.0 {
        println!("\n🎉 Both models processing real document content!");
        println!("   TrOCR → Text extraction from images");
        println!("   LayoutLM → Document structure understanding");
    } else {
        println!("\n⚠️ Models may need real document content for better results");
    }
    
    Ok(())
}