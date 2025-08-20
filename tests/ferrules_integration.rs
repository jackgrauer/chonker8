// tests/ferrules_integration.rs
use rstest::rstest;
use rexpect::spawn;

/// Test that ferrules gets the right page range and outputs structured data
#[rstest]
#[case(1, "1")] // Test page 1
#[case(2, "2")] // Test page 2 if it exists
fn test_ferrules_page_selection(
    #[case] page_num: usize,
    #[case] expected_page_arg: &str,
) {
    // Use a PDF we know has multiple pages or just test page 1
    let mut p = spawn(
        &format!("chonker8 extract /Users/jack/Desktop/BERF-CERT.pdf --page {} --format text", page_num),
        Some(15000)
    ).unwrap();
    
    p.exp_string(&format!("📄 Extracting page {} with intelligent analysis", page_num)).unwrap();
    
    // Should complete successfully
    p.exp_eof().unwrap();
}

/// Test that ferrules OCR integration works when layout detection needs it
#[test]
fn test_ferrules_ocr_activation() {
    let mut p = spawn(
        "chonker8 extract /Users/jack/Desktop/BERF-CERT.pdf --mode auto --format text",
        Some(15000)
    ).unwrap();
    
    // Auto mode should analyze the document
    p.exp_string("🧠 Auto mode: Analyzing PDF structure").unwrap();
    
    // Should decide on extraction method
    if p.exp_string("🔍 Trying OCR extraction").is_ok() {
        println!("Document required OCR - good detection");
    } else if p.exp_string("📄 Using native PDF text").is_ok() {
        println!("Document has native text - good detection");  
    } else {
        panic!("No clear extraction method detected");
    }
    
    // Should produce text output
    p.exp_string("--- Extracted Text ---").unwrap();
    
    p.exp_eof().unwrap();
}

/// Test that we're feeding ferrules the right coordinate information
#[test]
fn test_coordinate_precision() {
    let mut p = spawn(
        "chonker8 extract /Users/jack/Desktop/BERF-CERT.pdf --format text",
        Some(15000)
    ).unwrap();
    
    p.exp_string("--- Extracted Text ---").unwrap();
    let output = p.exp_eof().unwrap();
    
    // Check that text positioning looks reasonable
    let lines: Vec<&str> = output.lines().collect();
    
    // Look for properly positioned text (not all crammed to left margin)
    let mut has_indented_content = false;
    let mut has_centered_content = false;
    
    for line in lines {
        // Skip empty lines
        if line.trim().is_empty() {
            continue;
        }
        
        // Check for indentation (spaces at start)
        let leading_spaces = line.len() - line.trim_start().len();
        if leading_spaces > 10 {
            has_indented_content = true;
        }
        
        // Check for centered content (significant leading spaces)
        if leading_spaces > 30 {
            has_centered_content = true;
        }
    }
    
    assert!(has_indented_content || has_centered_content, 
            "Text positioning seems flat - coordinate precision may be poor");
}

/// Test different grid sizes to see what gives best layout preservation
#[rstest]
#[case(100, 60)]   // Small grid
#[case(150, 90)]   // Medium grid  
#[case(200, 120)]  // Large grid
fn test_grid_size_impact(
    #[case] width: usize,
    #[case] height: usize,
) {
    let mut p = spawn(
        &format!("chonker8 extract /Users/jack/Desktop/BERF-CERT.pdf --width {} --height {} --format text", 
                width, height),
        Some(15000)
    ).unwrap();
    
    p.exp_string("--- Extracted Text ---").unwrap();
    let output = p.exp_eof().unwrap();
    
    // Larger grids should preserve more layout detail
    let text_lines: Vec<&str> = output.lines()
        .filter(|line| !line.trim().is_empty())
        .collect();
    
    // Should have reasonable amount of text
    assert!(text_lines.len() > 5, 
            "Grid size {}x{} produced too few text lines", width, height);
    
    println!("Grid {}x{} produced {} text lines", width, height, text_lines.len());
}

/// Test that ferrules preserves document structure
#[test]
fn test_document_structure_preservation() {
    let mut p = spawn(
        "chonker8 extract /Users/jack/Desktop/BERF-CERT.pdf --format text",
        Some(15000)
    ).unwrap();
    
    p.exp_string("--- Extracted Text ---").unwrap();
    let output = p.exp_eof().unwrap();
    
    // Should preserve key structural elements
    let text = output.to_uppercase();
    
    // Birth certificate should have key fields in reasonable positions
    assert!(text.contains("CERTIFICATE"), "Missing certificate title");
    assert!(text.contains("NAME"), "Missing name field");
    assert!(text.contains("DATE"), "Missing date field");
    
    // Structure should be preserved - title near top
    let lines: Vec<&str> = output.lines().collect();
    let mut certificate_line = None;
    
    for (i, line) in lines.iter().enumerate() {
        if line.to_uppercase().contains("CERTIFICATE") {
            certificate_line = Some(i);
            break;
        }
    }
    
    if let Some(cert_line) = certificate_line {
        // Certificate title should be in first half of document
        assert!(cert_line < lines.len() / 2, 
                "Certificate title found too low in document - structure not preserved");
    }
}