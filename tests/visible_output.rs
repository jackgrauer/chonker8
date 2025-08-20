// tests/visible_output.rs
use rexpect::spawn;

#[test]
fn show_current_extraction() {
    println!("\n🔍 CURRENT EXTRACTION OUTPUT:");
    println!("{}", "=".repeat(60));
    
    let mut p = spawn(
        "chonker8 extract /Users/jack/Desktop/BERF-CERT.pdf --format text",
        Some(10000)
    ).unwrap();
    
    // Capture and display the full output
    let output = p.exp_eof().unwrap();
    
    // Split and show just the extracted text part
    if let Some(pos) = output.find("--- Extracted Text ---") {
        let text_part = &output[pos..];
        println!("{}", text_part);
    } else {
        println!("Full output:\n{}", output);
    }
    
    println!("\n{}", "=".repeat(60));
}

#[test]
fn compare_extraction_modes() {
    let pdf_path = "/Users/jack/Desktop/BERF-CERT.pdf";
    let modes = ["auto", "ocr", "native"];
    
    println!("\n🔄 COMPARING EXTRACTION MODES:");
    
    for mode in &modes {
        println!("\n📋 MODE: {}", mode.to_uppercase());
        println!("{}", "-".repeat(40));
        
        let mut p = spawn(
            &format!("chonker8 extract '{}' --mode {} --format text", pdf_path, mode),
            Some(15000)
        ).unwrap();
        
        let output = p.exp_eof().unwrap();
        
        // Show first few lines of extracted text
        if let Some(pos) = output.find("--- Extracted Text ---") {
            let text_part = &output[pos + 23..]; // Skip the header
            let lines: Vec<&str> = text_part.lines().take(8).collect();
            for (i, line) in lines.iter().enumerate() {
                let preview = if line.len() > 70 { &line[..70] } else { line };
                println!("  {:2}: {}", i+1, preview);
            }
        }
    }
}

#[test]
fn show_word_separation_quality() {
    println!("\n🔤 WORD SEPARATION ANALYSIS:");
    println!("{}", "=".repeat(50));
    
    let mut p = spawn(
        "chonker8 extract /Users/jack/Documents/chonker_test.pdf --format text",
        Some(10000)
    ).unwrap();
    
    let output = p.exp_eof().unwrap();
    
    if let Some(pos) = output.find("--- Extracted Text ---") {
        let text_part = &output[pos + 23..];
        let words: Vec<&str> = text_part.split_whitespace().collect();
        
        println!("📊 Total words: {}", words.len());
        
        // Analyze word lengths
        let long_words: Vec<&str> = words.iter().filter(|w| w.len() > 20).cloned().collect();
        let very_long_words: Vec<&str> = words.iter().filter(|w| w.len() > 30).cloned().collect();
        
        println!("📏 Long words (>20 chars): {}", long_words.len());
        println!("📏 Very long words (>30 chars): {}", very_long_words.len());
        
        if !long_words.is_empty() {
            println!("\n⚠️  Long words found (potential concatenation):");
            for word in long_words.iter().take(5) {
                println!("   • {} ({} chars)", word, word.len());
            }
        }
        
        if very_long_words.is_empty() {
            println!("✅ No severely concatenated words found!");
        }
        
        // Show average word length
        let avg_length: f64 = words.iter().map(|w| w.len()).sum::<usize>() as f64 / words.len() as f64;
        println!("📐 Average word length: {:.1} chars", avg_length);
    }
}