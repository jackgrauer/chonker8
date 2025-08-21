// Test ORT 2.0 tensor creation
use ort::{Session, Value};

fn main() {
    // Create a simple tensor
    let data = vec![1.0f32, 2.0, 3.0, 4.0];
    let shape = vec![2, 2];
    
    // Try to create a tensor
    let tensor = Value::from_array((shape.as_slice(), data.as_slice()));
    println!("Created tensor");
}
