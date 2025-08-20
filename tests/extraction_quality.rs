// tests/extraction_quality.rs
use rstest::rstest;
use rexpect::spawn;

/// Test different extraction modes to see which gives best word separation
#[rstest]
#[case("auto", "🧠 Auto mode")]
#[case("ocr", "🔍 Trying OCR extraction")]
#[case("native", "📄 Using native PDF text")]
fn test_extraction_modes(
    #[case] mode: &str,
    #[case] expected_indicator: &str,
) {
    let mut p = spawn(
        &format!("chonker8 extract /Users/jack/Desktop/BERF-CERT.pdf --format text --mode {}", mode),
        Some(10000)
    ).unwrap();
    
    // Verify mode selection
    p.exp_string(expected_indicator).unwrap();
    
    // Look for word separation success (no concatenated titles)
    if p.exp_string("CITYCASHMANAGEMENT").is_ok() {
        panic!("Found concatenated words in {} mode - word separation failed", mode);
    }
    
    // Should see properly separated words
    p.exp_string("CASH").unwrap();
    p.exp_string("MANAGEMENT").unwrap();
    
    p.exp_eof().unwrap();
}

/// Test that OCR extraction preserves text when layout detection fails
#[test]
fn test_ocr_fallback_preserves_text() {
    let mut p = spawn(
        "chonker8 extract /Users/jack/Desktop/BERF-CERT.pdf --mode ocr --format text",
        Some(10000)
    ).unwrap();
    
    // Should get OCR text even if no layout boxes
    p.exp_string("🔍 Trying OCR extraction").unwrap();
    
    // Key test: text should appear even with challenging layout
    p.exp_string("VOID WITHOUT WATERMARK").unwrap();
    p.exp_string("CERTIFICATE").unwrap();
    p.exp_string("FERRANTE").unwrap();
    
    p.exp_eof().unwrap();
}

/// Test different PDFs to ensure extraction works across document types
#[rstest]
#[case("/Users/jack/Desktop/BERF-CERT.pdf", &["CERTIFICATE", "FERRANTE", "VIRGINIA"])]
#[case("/Users/jack/Desktop/Testing_the_waters_for_floating_class_7.5M___Philadelphia_Daily_News_PA___February_17_2025__pX10.pdf", &["floating", "classroom", "Philadelphia"])]
fn test_pdf_text_extraction(
    #[case] pdf_path: &str,
    #[case] expected_words: &[&str],
) {
    let mut p = spawn(
        &format!("chonker8 extract '{}' --format text", pdf_path),
        Some(15000)
    ).unwrap();
    
    // Should see auto mode selection
    p.exp_string("🧠 Auto mode").unwrap();
    
    // Check that expected words appear (not concatenated)
    for word in expected_words {
        p.exp_string(word).unwrap();
    }
    
    p.exp_eof().unwrap();
}

/// Test that word boundaries are preserved in different extraction scenarios
#[test]
fn test_word_boundary_preservation() {
    let mut p = spawn(
        "chonker8 extract /Users/jack/Documents/chonker_test.pdf --format text",
        Some(10000)
    ).unwrap();
    
    // Should see the text extraction
    p.exp_string("--- Extracted Text ---").unwrap();
    
    // Key test: words should be separated, not concatenated
    // If we see very long strings, word separation is failing
    let output = p.exp_eof().unwrap();
    
    // Parse the text output and check for word separation quality
    let lines: Vec<&str> = output.lines().collect();
    let mut found_long_concatenated = false;
    
    for line in lines {
        let words: Vec<&str> = line.split_whitespace().collect();
        for word in words {
            // Flag potential concatenation issues (very long words that aren't URLs/codes)
            if word.len() > 25 && !word.contains("http") && !word.contains("://") {
                println!("Warning: Potentially concatenated word found: {}", word);
                if word.len() > 40 {
                    found_long_concatenated = true;
                }
            }
        }
    }
    
    if found_long_concatenated {
        panic!("Found severely concatenated words - extraction quality needs improvement");
    }
}

/// Test consistency - same PDF should give same results
#[test] 
fn test_extraction_consistency() {
    let pdf_path = "/Users/jack/Desktop/BERF-CERT.pdf";
    
    // Run extraction twice
    let mut p1 = spawn(
        &format!("chonker8 extract '{}' --format text", pdf_path),
        Some(10000)
    ).unwrap();
    
    p1.exp_string("--- Extracted Text ---").unwrap();
    let output1 = p1.exp_eof().unwrap();
    
    let mut p2 = spawn(
        &format!("chonker8 extract '{}' --format text", pdf_path),
        Some(10000)
    ).unwrap();
    
    p2.exp_string("--- Extracted Text ---").unwrap();
    let output2 = p2.exp_eof().unwrap();
    
    // Extract just the text portions for comparison
    let text1 = extract_text_content(&output1);
    let text2 = extract_text_content(&output2);
    
    assert_eq!(text1, text2, "Extraction should be consistent between runs");
}

fn extract_text_content(full_output: &str) -> String {
    if let Some(pos) = full_output.find("--- Extracted Text ---") {
        full_output[pos..].to_string()
    } else {
        full_output.to_string()
    }
}