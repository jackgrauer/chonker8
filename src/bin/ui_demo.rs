use anyhow::Result;
use image::DynamicImage;

// Use the crate name from Cargo.toml
extern crate chonker8;
use chonker8::pdf_extraction::{DocumentAIService, UIRequest};

#[tokio::main]
async fn main() -> Result<()> {
    println!("🎨 Document AI UI Demo");
    println!("{}", "═".repeat(50));
    
    // Initialize service
    println!("\n📦 Initializing Document AI Service...");
    let service = DocumentAIService::new()?;
    
    // Check status
    println!("\n📊 Checking service status...");
    let status_request = UIRequest {
        action: "get_status".to_string(),
        page_number: None,
        options: None,
    };
    
    let status_response = service.process_request(status_request, None).await;
    if status_response.success {
        println!("✅ Service Status:");
        if let Some(data) = status_response.data {
            println!("{}", serde_json::to_string_pretty(&data)?);
        }
    }
    
    // Process a test image
    println!("\n🖼️ Processing test image...");
    let test_image = DynamicImage::new_rgb8(224, 224);
    
    let process_request = UIRequest {
        action: "process_page".to_string(),
        page_number: Some(0),
        options: None,
    };
    
    let process_response = service.process_request(process_request, Some(test_image)).await;
    
    if process_response.success {
        println!("✅ Processing successful!");
        println!("   Time: {}ms", process_response.processing_time_ms);
        if let Some(data) = process_response.data {
            println!("   Results preview:");
            let json_str = serde_json::to_string_pretty(&data)?;
            for line in json_str.lines().take(10) {
                println!("   {}", line);
            }
        }
    } else {
        println!("❌ Processing failed: {:?}", process_response.error);
    }
    
    println!("\n✨ UI Demo Complete!");
    Ok(())
}
