use rexpect::spawn;

#[test]
fn test_ocr_extraction_with_fallback() {
    // Test that OCR extraction works (falls back to pdftotext when models missing)
    let mut cmd = spawn(
        "DYLD_LIBRARY_PATH=/Users/jack/chonker8/lib /Users/jack/chonker8/target/release/chonker8 extract /Users/jack/Documents/test.pdf --page 1 --mode ocr --format text",
        Some(10000)
    ).expect("Failed to spawn chonker8");
    
    // Should show extraction message
    cmd.exp_string("Extracting page 1 with OCR")
        .expect("Should show extraction message");
    
    // Should attempt OAR-OCR
    cmd.exp_string("Using OAR-OCR with Metal acceleration")
        .expect("Should attempt OAR-OCR");
    
    // Should fall back to pdftotext when models are missing
    cmd.exp_string("trying pdftotext")
        .expect("Should fall back to pdftotext");
    
    // Should extract the chromium table data
    cmd.exp_string("Chromium")
        .expect("Should extract chromium data");
    
    cmd.exp_string("SAMPLE ID")
        .expect("Should extract table headers");
    
    // Should complete successfully
    cmd.exp_eof().expect("Should complete");
}

#[test]
fn test_database_storage() {
    use std::fs;
    
    // Clean up test database
    let _ = fs::remove_file("test_storage.db");
    
    // Store a document
    let mut cmd = spawn(
        "DYLD_LIBRARY_PATH=/Users/jack/chonker8/lib /Users/jack/chonker8/target/release/chonker8 --db test_storage.db extract /Users/jack/Documents/test.pdf --page 1 --mode ocr --store",
        Some(10000)
    ).expect("Failed to spawn chonker8");
    
    cmd.exp_string("Stored in database")
        .expect("Should store in database");
    
    cmd.exp_eof().expect("Should complete");
    
    // Now search for content
    let mut search_cmd = spawn(
        "DYLD_LIBRARY_PATH=/Users/jack/chonker8/lib /Users/jack/chonker8/target/release/chonker8 --db test_storage.db search chromium",
        Some(5000)
    ).expect("Failed to spawn search");
    
    search_cmd.exp_string("test.pdf")
        .expect("Should find the stored document");
    
    search_cmd.exp_eof().expect("Search should complete");
    
    // Check stats
    let mut stats_cmd = spawn(
        "DYLD_LIBRARY_PATH=/Users/jack/chonker8/lib /Users/jack/chonker8/target/release/chonker8 --db test_storage.db stats",
        Some(5000)
    ).expect("Failed to spawn stats");
    
    stats_cmd.exp_string("Documents: 1")
        .expect("Should show 1 document");
    
    stats_cmd.exp_eof().expect("Stats should complete");
    
    // Clean up
    let _ = fs::remove_file("test_storage.db");
}

#[test]
fn test_help_command() {
    let mut cmd = spawn(
        "DYLD_LIBRARY_PATH=/Users/jack/chonker8/lib /Users/jack/chonker8/target/release/chonker8 --help",
        Some(5000)
    ).expect("Failed to spawn chonker8");
    
    cmd.exp_string("chonker8 v8.8.0")
        .expect("Should show version");
    
    cmd.exp_string("PDF text extraction")
        .expect("Should show description");
    
    cmd.exp_string("extract")
        .expect("Should show extract command");
    
    cmd.exp_string("search")
        .expect("Should show search command");
    
    cmd.exp_eof().expect("Should complete");
}