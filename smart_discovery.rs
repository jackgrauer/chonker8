#!/usr/bin/env rust-script
//! ```cargo
//! [dependencies]
//! anyhow = "1.0"
//! ```

use anyhow::Result;
use std::process::Command;

fn main() -> Result<()> {
    println!("🧠 SMART DISCOVERY SYSTEM - Finding and fixing extraction issues\n");
    
    let test_pdf = "/Users/jack/Desktop/Testing_the_waters_for_floating_class_7.5M___Philadelphia_Daily_News_PA___February_17_2025__pX10.pdf";
    let expected = "Testing the waters";
    
    let mut iterations = 0;
    let mut fixed = false;
    
    while iterations < 5 && !fixed {
        iterations += 1;
        println!("ITERATION {}", iterations);
        println!("{}", "=".repeat(60));
        
        // Test current extraction
        let output = Command::new("chonker8")
            .args(&["extract", test_pdf, "--page", "1", "--raw"])
            .output()?;
        
        let text = String::from_utf8_lossy(&output.stdout);
        let first_line = text.lines().find(|l| !l.trim().is_empty()).unwrap_or("");
        
        println!("Current first line: '{}'", first_line.trim());
        
        if first_line.contains(expected) {
            println!("✅ EXTRACTION IS CORRECT!");
            fixed = true;
            break;
        }
        
        if first_line.contains("overtheclassroom") {
            println!("❌ Wrong - showing end of article (green roof paragraph)");
            println!("   This suggests Y sorting is inverted\n");
            
            // Try to fix it
            println!("Attempting fix #{}", iterations);
            
            let improved_path = "src/pdf_extraction/improved.rs";
            let current_code = std::fs::read_to_string(improved_path)?;
            
            // Try different fixes based on iteration
            let fixed_code = match iterations {
                1 => {
                    println!("Fix 1: Reverse Y sorting (a.2 before b.2)");
                    current_code.replace(
                        "let y_cmp = b.2.partial_cmp(&a.2)",
                        "let y_cmp = a.2.partial_cmp(&b.2)"
                    )
                },
                2 => {
                    println!("Fix 2: Reverse back but invert grid placement");
                    let code = current_code.replace(
                        "let y_cmp = a.2.partial_cmp(&b.2)",
                        "let y_cmp = b.2.partial_cmp(&a.2)"
                    );
                    code.replace(
                        "let grid_y = ((y / page_height) * height as f32).round() as usize;",
                        "let grid_y = height - 1 - ((y / page_height) * height as f32).round() as usize;"
                    )
                },
                3 => {
                    println!("Fix 3: Use bottom Y instead of top Y");
                    current_code.replace(
                        "bounds.top().value,",
                        "bounds.bottom().value,"
                    )
                },
                4 => {
                    println!("Fix 4: Group by lines first");
                    // This would be more complex - for now just try original sorting
                    current_code.replace(
                        "// Sort by y position",
                        "// Sort by y position (trying original order)"
                    )
                },
                _ => current_code
            };
            
            // Apply the fix
            std::fs::write(improved_path, fixed_code)?;
            
            // Rebuild
            println!("Rebuilding...");
            let output = Command::new("cargo")
                .args(&["build", "--release", "--quiet"])
                .env("DYLD_LIBRARY_PATH", "./lib")
                .output()?;
            
            if !output.status.success() {
                println!("Build failed: {}", String::from_utf8_lossy(&output.stderr));
                break;
            }
            
            println!("Build complete, testing fix...\n");
        } else {
            println!("Unexpected output: '{}'", first_line);
            break;
        }
    }
    
    if fixed {
        println!("\n🎉 SUCCESS! Extraction now shows correct text order!");
        
        // Show comparison
        println!("\nFinal test with --compare mode:");
        let output = Command::new("chonker8")
            .args(&["extract", test_pdf, "--page", "1", "--compare"])
            .stderr(std::process::Stdio::null())
            .output()?;
        
        let comparison = String::from_utf8_lossy(&output.stdout);
        for line in comparison.lines().skip(5).take(10) {
            println!("{}", line);
        }
    } else {
        println!("\n❌ Could not fix extraction order after {} attempts", iterations);
        println!("Manual investigation needed");
    }
    
    Ok(())
}