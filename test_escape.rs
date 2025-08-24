fn main() {
    // Test what Rust actually outputs
    print!("\x1b_Ga=d\x1b\\");
    println!(" <- should be ESC _ G a = d ESC \\");
    
    // Check the bytes
    let s = "\x1b_Ga=d\x1b\\";
    println!("Bytes: {:?}", s.as_bytes());
    
    // Expected: [27, 95, 71, 97, 61, 100, 27, 92]
    //           ESC _   G   a   =   d   ESC \
}