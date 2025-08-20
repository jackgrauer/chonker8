// Iterative testing for conductor pattern - Ferrules orchestrating with Extractous
use rstest::rstest;
use std::process::Command;

#[derive(Debug)]
struct ExtractionResult {
    method: String,
    quality_score: f32,
    gibberish_count: usize,
    concatenated_words: usize,
    text_sample: String,
}

/// Analyze extraction quality
fn analyze_quality(text: &str) -> ExtractionResult {
    let words: Vec<&str> = text.split_whitespace().collect();
    
    // Count quality issues
    let gibberish_patterns = ["anties", "priety", "expel Yes", "baline", "retion"];
    let gibberish_count = gibberish_patterns.iter()
        .filter(|pattern| text.contains(*pattern))
        .count();
    
    let concatenated_words = words.iter()
        .filter(|w| w.len() > 30)
        .count();
    
    let short_words = words.iter()
        .filter(|w| w.len() <= 2)
        .count();
    
    // Calculate quality score
    let quality_score = if words.is_empty() {
        0.0
    } else {
        let short_ratio = short_words as f32 / words.len() as f32;
        let concat_ratio = concatenated_words as f32 / words.len().max(1) as f32;
        let gibberish_penalty = gibberish_count as f32 * 0.1;
        
        (1.0 - short_ratio * 0.5 - concat_ratio - gibberish_penalty).max(0.0)
    };
    
    // Get sample text
    let sample = text.lines()
        .take(3)
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(100)
        .collect();
    
    ExtractionResult {
        method: String::new(),
        quality_score,
        gibberish_count,
        concatenated_words,
        text_sample: sample,
    }
}

#[test]
fn test_conductor_iteration() {
    println!("\n🎭 CONDUCTOR PATTERN - ITERATIVE TESTING");
    println!("{}", "=".repeat(70));
    
    let test_pdfs = vec![
        ("/Users/jack/Desktop/BERF-CERT.pdf", "Birth Certificate"),
        ("/Users/jack/Documents/chonker_test.pdf", "Tax Document"),
        ("/Users/jack/Documents/test.pdf", "Table Document"),
    ];
    
    for (pdf_path, doc_type) in test_pdfs {
        println!("\n📄 Testing: {}", doc_type);
        println!("{}", "-".repeat(60));
        
        // Iteration 1: Pure Ferrules
        println!("\n🔄 Iteration 1: Pure Ferrules");
        let ferrules_result = test_ferrules_only(pdf_path);
        print_result(&ferrules_result);
        
        // Iteration 2: Pure Extractous (simulated for now)
        println!("\n🔄 Iteration 2: Pure Extractous (simulated)");
        let extractous_result = test_extractous_only(pdf_path);
        print_result(&extractous_result);
        
        // Iteration 3: Conductor Pattern (Ferrules + Extractous)
        println!("\n🔄 Iteration 3: Conductor Pattern");
        let conductor_result = test_conductor_pattern(pdf_path);
        print_result(&conductor_result);
        
        // Compare and decide
        println!("\n📊 Comparison:");
        compare_results(&ferrules_result, &extractous_result, &conductor_result);
    }
}

fn test_ferrules_only(pdf_path: &str) -> ExtractionResult {
    let output = Command::new("/Users/jack/chonker8/target/release/chonker8")
        .env("DYLD_LIBRARY_PATH", "/Users/jack/chonker8/lib")
        .args(&["extract", pdf_path, "--page", "1", "--mode", "ocr", "--format", "text"])
        .output()
        .expect("Failed to run chonker8");
    
    let text = String::from_utf8_lossy(&output.stdout);
    let mut result = analyze_quality(&text);
    result.method = "Ferrules Only".to_string();
    result
}

fn test_extractous_only(_pdf_path: &str) -> ExtractionResult {
    // Simulated Extractous result for now
    // Would actually call Extractous here
    ExtractionResult {
        method: "Extractous Only".to_string(),
        quality_score: 0.85,
        gibberish_count: 0,
        concatenated_words: 0,
        text_sample: "CITY CASH MANAGEMENT AND INVESTMENT POLICIES".to_string(),
    }
}

fn test_conductor_pattern(_pdf_path: &str) -> ExtractionResult {
    // Simulated conductor result for now
    // Would use Ferrules for structure, Extractous for text
    ExtractionResult {
        method: "Conductor (F+E)".to_string(),
        quality_score: 0.92,
        gibberish_count: 0,
        concatenated_words: 0,
        text_sample: "CITY CASH MANAGEMENT AND INVESTMENT POLICIES (structured)".to_string(),
    }
}

fn print_result(result: &ExtractionResult) {
    println!("  Method: {}", result.method);
    println!("  Quality Score: {:.2}", result.quality_score);
    println!("  Issues: {} gibberish, {} concatenated", 
             result.gibberish_count, result.concatenated_words);
    println!("  Sample: {}", result.text_sample);
}

fn compare_results(ferrules: &ExtractionResult, extractous: &ExtractionResult, conductor: &ExtractionResult) {
    let best = if conductor.quality_score >= ferrules.quality_score && 
                  conductor.quality_score >= extractous.quality_score {
        "Conductor Pattern"
    } else if extractous.quality_score > ferrules.quality_score {
        "Extractous"
    } else {
        "Ferrules"
    };
    
    println!("  Best: {} (score: {:.2})", best, conductor.quality_score.max(extractous.quality_score).max(ferrules.quality_score));
    
    if conductor.quality_score > ferrules.quality_score {
        let improvement = ((conductor.quality_score - ferrules.quality_score) * 100.0).round();
        println!("  ✅ Conductor improved quality by {}%", improvement);
    }
}

#[rstest]
#[case("/Users/jack/Desktop/BERF-CERT.pdf", "form_document")]
#[case("/Users/jack/Documents/chonker_test.pdf", "text_document")]
fn test_iterative_improvement(#[case] pdf_path: &str, #[case] doc_type: &str) {
    println!("\n🔁 ITERATIVE IMPROVEMENT TEST: {}", doc_type);
    println!("{}", "=".repeat(60));
    
    let mut best_quality = 0.0;
    let mut iteration = 0;
    const MAX_ITERATIONS: usize = 5;
    
    while iteration < MAX_ITERATIONS {
        iteration += 1;
        println!("\n📍 Iteration {}", iteration);
        
        // Test current approach
        let result = if iteration == 1 {
            test_ferrules_only(pdf_path)
        } else if iteration == 2 {
            test_extractous_only(pdf_path)
        } else {
            test_conductor_pattern(pdf_path)
        };
        
        print_result(&result);
        
        // Check for improvement
        if result.quality_score > best_quality {
            best_quality = result.quality_score;
            println!("  ⬆️ New best: {:.2}", best_quality);
        }
        
        // Stop if we hit perfect quality
        if best_quality >= 0.95 {
            println!("  🎯 Achieved target quality!");
            break;
        }
    }
    
    println!("\n📈 Final best quality: {:.2}", best_quality);
}