use ort::{init, session::Session};

fn main() {
    println!("Testing model loading...\n");
    
    // Initialize ORT
    let _ = init();
    
    // Test TrOCR
    println!("Testing TrOCR model:");
    match Session::builder()
        .unwrap()
        .commit_from_file("models/trocr.onnx") {
        Ok(session) => {
            println!("  ✅ TrOCR loaded successfully");
            println!("  Model has {} inputs", session.inputs.len());
            for input in &session.inputs {
                println!("    - {}: {:?}", input.name, input.input_type);
            }
            println!("  Model has {} outputs", session.outputs.len());
            for output in &session.outputs {
                println!("    - {}: {:?}", output.name, output.output_type);
            }
        },
        Err(e) => println!("  ❌ TrOCR failed: {}", e),
    }
    
    // Test LayoutLM
    println!("\nTesting LayoutLM model:");
    match Session::builder()
        .unwrap()
        .commit_from_file("models/layoutlm.onnx") {
        Ok(session) => {
            println!("  ✅ LayoutLM loaded successfully");
            println!("  Model has {} inputs", session.inputs.len());
            for input in &session.inputs {
                println!("    - {}: {:?}", input.name, input.input_type);
            }
        },
        Err(e) => println!("  ❌ LayoutLM failed: {}", e),
    }
}