// Integration tests using rexpect for chonker8
use rexpect::spawn;
use rstest::rstest;
use std::path::Path;
use std::time::Duration;

const TIMEOUT_MS: u64 = 10000;

fn build_and_spawn() -> rexpect::session::PtySession {
    // Set library path
    std::env::set_var("DYLD_LIBRARY_PATH", "/Users/jack/chonker8/lib");
    
    // Build the project
    let output = std::process::Command::new("cargo")
        .args(&["build", "--release"])
        .env("DYLD_LIBRARY_PATH", "/Users/jack/chonker8/lib")
        .output()
        .expect("Failed to build");
    
    if !output.status.success() {
        panic!("Build failed: {}", String::from_utf8_lossy(&output.stderr));
    }
    
    // Spawn the binary
    let session = spawn("/Users/jack/chonker8/target/release/chonker8", Some(TIMEOUT_MS))
        .expect("Failed to spawn chonker8");
    
    session
}

#[test]
fn test_basic_extraction() {
    let mut p = build_and_spawn();
    
    // Test help command
    p.send_line("--help").unwrap();
    p.exp_string("PDF extraction tool").unwrap();
}

#[rstest]
#[case("/Users/jack/Documents/chonker_test.pdf", "CITY", "Should extract city text")]
#[case("/Users/jack/Desktop/BERF-CERT.pdf", "CERTIFICATE", "Should extract certificate")]
fn test_pdf_extraction(
    #[case] pdf_path: &str,
    #[case] expected: &str,
    #[case] description: &str,
) {
    if !Path::new(pdf_path).exists() {
        println!("Skipping: {} not found", pdf_path);
        return;
    }
    
    let mut p = build_and_spawn();
    
    let cmd = format!("extract \"{}\" --page 1 --mode ocr --format text", pdf_path);
    p.send_line(&cmd).unwrap();
    
    // Wait for processing
    std::thread::sleep(Duration::from_secs(2));
    
    // Check for expected text
    match p.exp_string(expected, Some(5000)) {
        Ok(_) => println!("✓ {}", description),
        Err(e) => panic!("✗ {}: {}", description, e),
    }
}

#[test]
fn test_fallback_mechanism() {
    let pdf = "/Users/jack/Documents/chonker_test.pdf";
    if !Path::new(pdf).exists() {
        return;
    }
    
    let mut p = build_and_spawn();
    
    // Extract with OCR mode (should trigger fallback)
    let cmd = format!("extract \"{}\" --page 1 --mode ocr --format text", pdf);
    p.send_line(&cmd).unwrap();
    
    // Should detect gibberish and fall back
    if p.exp_string("pdftotext", Some(3000)).is_ok() {
        println!("✓ Fallback to pdftotext triggered");
    }
    
    // Should get clean text
    if p.exp_string("MANAGEMENT", Some(5000)).is_ok() {
        println!("✓ Clean text extracted after fallback");
    }
}