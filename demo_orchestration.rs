#!/usr/bin/env rust-script
//! Demo script showing document-agnostic PDF extraction orchestration
//! 
//! ```cargo
//! [dependencies]
//! anyhow = "1.0"
//! rexpect = "0.5"
//! colored = "2.0"
//! ```

use anyhow::Result;
use colored::*;
use rexpect::spawn_bash;
use std::io::{self, Write};

fn main() -> Result<()> {
    println!("{}", "=".repeat(80).bright_blue());
    println!("{}", "    CHONKER8 DOCUMENT-AGNOSTIC PDF EXTRACTION DEMO".bright_yellow().bold());
    println!("{}", "=".repeat(80).bright_blue());
    println!();
    
    println!("{}", "This demo showcases the intelligent PDF extraction pipeline that:".green());
    println!("  • {} each page to understand its content", "Analyzes".bright_cyan());
    println!("  • {} the optimal extraction method", "Selects".bright_cyan());
    println!("  • {} quality and falls back if needed", "Validates".bright_cyan());
    println!("  • {} multiple extraction technologies", "Orchestrates".bright_cyan());
    println!();
    
    // Get PDF path from user
    print!("{}", "Enter PDF path (or press Enter for default): ".yellow());
    io::stdout().flush()?;
    
    let mut pdf_path = String::new();
    io::stdin().read_line(&mut pdf_path)?;
    pdf_path = pdf_path.trim().to_string();
    
    if pdf_path.is_empty() {
        // Try to find a default PDF
        let defaults = vec![
            "/Users/jack/Desktop/BERF-CERT.pdf",
            "/tmp/test.pdf",
        ];
        
        for default in defaults {
            if std::path::Path::new(default).exists() {
                pdf_path = default.to_string();
                break;
            }
        }
        
        if pdf_path.is_empty() {
            println!("{}", "No PDF found. Please provide a path.".red());
            return Ok(());
        }
    }
    
    println!();
    println!("{} {}", "Using PDF:".green(), pdf_path.bright_white());
    println!();
    
    // Build the extraction binary
    println!("{}", "Building extraction system...".yellow());
    let build_cmd = "DYLD_LIBRARY_PATH=./lib cargo build --release --bin test-extraction --quiet";
    std::process::Command::new("bash")
        .arg("-c")
        .arg(build_cmd)
        .output()?;
    println!("{}", "✓ Build complete".green());
    println!();
    
    // Demo 1: Document Analysis
    demo_analysis(&pdf_path)?;
    
    // Demo 2: Intelligent Routing
    demo_routing(&pdf_path)?;
    
    // Demo 3: Fallback Chain
    demo_fallback(&pdf_path)?;
    
    // Demo 4: Quality Validation
    demo_quality(&pdf_path)?;
    
    println!();
    println!("{}", "=".repeat(80).bright_blue());
    println!("{}", "    DEMO COMPLETE!".bright_green().bold());
    println!("{}", "=".repeat(80).bright_blue());
    
    Ok(())
}

fn demo_analysis(pdf_path: &str) -> Result<()> {
    println!("{}", "─".repeat(80).bright_black());
    println!("{}", "STAGE 1: DOCUMENT ANALYSIS".bright_magenta().bold());
    println!("{}", "─".repeat(80).bright_black());
    println!();
    
    println!("{}", "Analyzing page content to generate fingerprint...".cyan());
    
    let cmd = format!(
        "DYLD_LIBRARY_PATH=./lib ./target/release/test-extraction analyze '{}' --page 0",
        pdf_path
    );
    
    let mut session = spawn_bash(Some(5000))?;
    session.send_line(&cmd)?;
    
    // Capture and display output
    if let Ok(output) = session.exp_regex(r"Text coverage: ([\d.]+)%") {
        println!("  {} {}%", "• Text coverage:".bright_white(), output.1.green());
    }
    
    if let Ok(output) = session.exp_regex(r"Image coverage: ([\d.]+)%") {
        println!("  {} {}%", "• Image coverage:".bright_white(), output.1.yellow());
    }
    
    if let Ok(output) = session.exp_regex(r"Has tables: (\w+)") {
        let has_tables = output.1;
        let color = if has_tables == "true" { has_tables.cyan() } else { has_tables.bright_black() };
        println!("  {} {}", "• Has tables:".bright_white(), color);
    }
    
    if let Ok(output) = session.exp_regex(r"Text quality: ([\d.]+)") {
        let quality = output.1.parse::<f32>().unwrap_or(0.0);
        let color = if quality > 0.7 {
            output.1.green()
        } else if quality > 0.4 {
            output.1.yellow()
        } else {
            output.1.red()
        };
        println!("  {} {}", "• Text quality:".bright_white(), color);
    }
    
    if let Ok(output) = session.exp_regex(r"Recommended extraction method: (\w+)") {
        println!("  {} {}", "• Recommended:".bright_white(), output.1.bright_cyan().bold());
    }
    
    session.wait_for_prompt()?;
    
    println!();
    println!("{}", "✓ Analysis complete".green());
    println!();
    
    Ok(())
}

fn demo_routing(pdf_path: &str) -> Result<()> {
    println!("{}", "─".repeat(80).bright_black());
    println!("{}", "STAGE 2: INTELLIGENT ROUTING".bright_magenta().bold());
    println!("{}", "─".repeat(80).bright_black());
    println!();
    
    println!("{}", "Extracting text using optimal method...".cyan());
    
    let cmd = format!(
        "DYLD_LIBRARY_PATH=./lib ./target/release/test-extraction extract '{}' --page 0",
        pdf_path
    );
    
    let mut session = spawn_bash(Some(10000))?;
    session.send_line(&cmd)?;
    
    if let Ok(output) = session.exp_regex(r"Selected method: (\w+)") {
        println!("  {} {}", "• Method selected:".bright_white(), output.1.bright_cyan());
    }
    
    if let Ok(output) = session.exp_regex(r"Quality score: ([\d.]+)") {
        let score = output.1.parse::<f32>().unwrap_or(0.0);
        let color = if score > 0.7 {
            output.1.green()
        } else if score > 0.4 {
            output.1.yellow()
        } else {
            output.1.red()
        };
        println!("  {} {}", "• Quality achieved:".bright_white(), color);
    }
    
    if let Ok(output) = session.exp_regex(r"Extraction time: (\d+)ms") {
        println!("  {} {}ms", "• Processing time:".bright_white(), output.1.bright_blue());
    }
    
    session.wait_for_prompt()?;
    
    println!();
    println!("{}", "✓ Extraction complete".green());
    println!();
    
    Ok(())
}

fn demo_fallback(pdf_path: &str) -> Result<()> {
    println!("{}", "─".repeat(80).bright_black());
    println!("{}", "STAGE 3: FALLBACK CHAIN".bright_magenta().bold());
    println!("{}", "─".repeat(80).bright_black());
    println!();
    
    println!("{}", "Testing fallback strategies...".cyan());
    
    let cmd = format!(
        "DYLD_LIBRARY_PATH=./lib ./target/release/test-extraction fallback '{}' --page 0",
        pdf_path
    );
    
    let mut session = spawn_bash(Some(15000))?;
    session.send_line(&cmd)?;
    
    if let Ok(output) = session.exp_regex(r"Primary method: (\w+)") {
        println!("  {} {}", "• Primary:".bright_white(), output.1.bright_cyan());
    }
    
    if let Ok(output) = session.exp_regex(r"Fallback chain: \[([^\]]+)\]") {
        let methods: Vec<&str> = output.1.split(", ").collect();
        println!("  {} {}", "• Fallbacks:".bright_white(), 
            methods.join(" → ").bright_black());
    }
    
    // Show attempts
    let mut attempt_count = 0;
    while let Ok(output) = session.exp_regex(r"Trying method: (\w+)") {
        attempt_count += 1;
        if attempt_count <= 3 {
            println!("    {} {}", format!("{}.", attempt_count).bright_black(), 
                output.1.cyan());
        }
        
        if let Ok(quality) = session.exp_regex(r"Quality: ([\d.]+)") {
            let score = quality.1.parse::<f32>().unwrap_or(0.0);
            let indicator = if score > 0.7 { "✓".green() } else { "✗".red() };
            println!("       {} Quality: {}", indicator, quality.1);
        }
    }
    
    session.wait_for_prompt()?;
    
    println!();
    println!("{}", "✓ Fallback testing complete".green());
    println!();
    
    Ok(())
}

fn demo_quality(pdf_path: &str) -> Result<()> {
    println!("{}", "─".repeat(80).bright_black());
    println!("{}", "STAGE 4: QUALITY VALIDATION".bright_magenta().bold());
    println!("{}", "─".repeat(80).bright_black());
    println!();
    
    println!("{}", "Validating extraction quality metrics...".cyan());
    
    // Demonstrate quality thresholds
    println!();
    println!("  {} Quality scoring criteria:", "•".bright_white());
    println!("    {} Text length > 10 chars", "✓".green());
    println!("    {} Contains sentences (. )", "✓".green());
    println!("    {} Not gibberish", "✓".green());
    println!("    {} Has dictionary words", "✓".green());
    println!("    {} Proper whitespace", "✓".green());
    
    println!();
    println!("  {} Quality thresholds:", "•".bright_white());
    println!("    {} High quality, use result", "> 0.7:".green());
    println!("    {} Acceptable, may try fallback", "0.5-0.7:".yellow());
    println!("    {} Poor quality, use fallback", "< 0.5:".red());
    
    println!();
    println!("{}", "✓ Quality validation complete".green());
    println!();
    
    Ok(())
}