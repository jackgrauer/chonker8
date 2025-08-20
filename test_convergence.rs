#!/usr/bin/env rust-script
//! Test convergence quality with rexpect
//! ```cargo
//! [dependencies]
//! rexpect = "0.5"
//! ```

use rexpect::spawn;
use std::time::Duration;

fn main() {
    println!("Testing Chonker8 Convergence Quality");
    println!("=====================================\n");
    
    std::env::set_var("DYLD_LIBRARY_PATH", "/Users/jack/chonker8/lib");
    
    let test_cases = vec![
        (
            "/Users/jack/Documents/chonker_test.pdf",
            vec!["CITY CASH MANAGEMENT", "INVESTMENT POLICIES"],
            vec!["CITYCASHMANAGEMENT", "anties", "priety"],
        ),
        (
            "/Users/jack/Desktop/BERF-CERT.pdf",
            vec!["CERTIFICATE", "VITAL RECORD"],
            vec!["anties", "priety", "baline"],
        ),
    ];
    
    let mut passed = 0;
    let mut failed = 0;
    
    for (pdf, expected, forbidden) in test_cases {
        println!("Testing: {}", pdf);
        
        let mut session = spawn(
            "/Users/jack/chonker8/target/release/chonker8",
            Some(10000)
        ).expect("Failed to spawn");
        
        // Send extraction command
        let cmd = format!("extract \"{}\" --page 1 --mode ocr --format text", pdf);
        session.send_line(&cmd).unwrap();
        
        // Wait for extraction
        std::thread::sleep(Duration::from_secs(2));
        
        // Get full output
        let output = session.exp_eof().unwrap_or_default();
        
        // Check expected patterns
        let mut test_passed = true;
        for pattern in &expected {
            if !output.contains(pattern) {
                println!("  ❌ Missing expected: '{}'", pattern);
                test_passed = false;
            } else {
                println!("  ✅ Found: '{}'", pattern);
            }
        }
        
        // Check forbidden patterns (gibberish)
        for pattern in &forbidden {
            if output.contains(pattern) {
                println!("  ❌ Found forbidden gibberish: '{}'", pattern);
                test_passed = false;
            }
        }
        
        if test_passed {
            println!("  ✅ PASSED\n");
            passed += 1;
        } else {
            println!("  ❌ FAILED\n");
            failed += 1;
        }
    }
    
    println!("\n=====================================");
    println!("Results: {} passed, {} failed", passed, failed);
    
    if failed == 0 {
        println!("🎉 All tests passed! Convergence quality verified.");
    } else {
        println!("⚠️  Some tests failed. Check extraction quality.");
        std::process::exit(1);
    }
}