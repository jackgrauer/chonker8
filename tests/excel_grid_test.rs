// Rexpect integration tests for Excel-style grid editor
use rexpect::spawn;
use std::time::Duration;
use std::thread;

#[test]
fn test_excel_grid_interactive() {
    // Create a test binary that uses ExcelGrid
    let mut p = spawn("cargo run --bin excel-grid-demo", Some(5000))
        .expect("Failed to spawn excel-grid-demo");
    
    // Wait for initialization
    thread::sleep(Duration::from_millis(500));
    
    // Test basic typing
    p.send("H").expect("Failed to send H");
    p.send("e").expect("Failed to send e");
    p.send("l").expect("Failed to send l");
    p.send("l").expect("Failed to send l");
    p.send("o").expect("Failed to send o");
    
    // Verify output shows typed text
    p.exp_string("Hello").expect("Should see Hello");
    
    // Test cursor movement
    p.send("\x1b[D").expect("Failed to send Left arrow"); // Left arrow
    p.send("\x1b[D").expect("Failed to send Left arrow");
    p.send("X").expect("Failed to send X");
    
    // Should now show "HelXo"
    p.exp_string("HelXo").expect("Should see HelXo");
    
    // Test block selection with Ctrl+V
    p.send("\x16").expect("Failed to send Ctrl+V"); // Ctrl+V
    p.send("\x1b[C").expect("Failed to send Right arrow"); // Right arrow
    p.send("\x1b[C").expect("Failed to send Right arrow");
    p.send("\x1b[B").expect("Failed to send Down arrow"); // Down arrow
    
    // Type to replace block
    p.send("Z").expect("Failed to send Z");
    
    // Exit
    p.send("q").expect("Failed to send quit");
}

#[test]
fn test_excel_grid_with_pdf_text() {
    // Test with actual PDF text extraction
    let mut p = spawn("cargo run --bin excel-grid-demo -- --pdf", Some(5000))
        .expect("Failed to spawn with PDF mode");
    
    thread::sleep(Duration::from_millis(500));
    
    // Should load some sample PDF text
    p.exp_string("Page").expect("Should see PDF content");
    
    // Test editing PDF text
    p.send("\x1b[B").expect("Failed to send Down arrow"); // Move down
    p.send("\x1b[B").expect("Failed to send Down arrow");
    p.send("EDITED").expect("Failed to type EDITED");
    
    p.exp_string("EDITED").expect("Should see edited text");
    
    // Test block selection and clear
    p.send("\x16").expect("Failed to send Ctrl+V"); // Start selection
    for _ in 0..5 {
        p.send("\x1b[C").expect("Failed to send Right arrow");
    }
    for _ in 0..3 {
        p.send("\x1b[B").expect("Failed to send Down arrow");
    }
    
    p.send("\x7f").expect("Failed to send Delete"); // Delete selection
    
    // Exit
    p.send("q").expect("Failed to send quit");
}

#[test]
fn test_shift_selection() {
    let mut p = spawn("cargo run --bin excel-grid-demo", Some(5000))
        .expect("Failed to spawn");
    
    thread::sleep(Duration::from_millis(500));
    
    // Type some initial text
    p.send("ABCDEF").expect("Failed to type ABCDEF");
    p.send("\r").expect("Failed to send Enter");
    p.send("123456").expect("Failed to type 123456");
    
    // Move back to start
    p.send("\x1b[H").expect("Failed to send Home"); // Home key
    p.send("\x1b[A").expect("Failed to send Up arrow");
    
    // Shift+Right to select (this would need terminal to support shift modifiers)
    // For now we'll use Ctrl+V mode
    p.send("\x16").expect("Failed to send Ctrl+V");
    p.send("\x1b[C").expect("Failed to send Right");
    p.send("\x1b[C").expect("Failed to send Right");
    p.send("\x1b[B").expect("Failed to send Down");
    
    // Replace selection
    p.send("X").expect("Failed to send X");
    
    p.exp_string("XXC").expect("Should see replaced text");
    
    p.send("q").expect("Failed to quit");
}

#[test]
fn test_copy_paste_operations() {
    let mut p = spawn("cargo run --bin excel-grid-demo", Some(5000))
        .expect("Failed to spawn");
    
    thread::sleep(Duration::from_millis(500));
    
    // Type some text
    p.send("Original Text").expect("Failed to type");
    
    // Select it (Ctrl+V then move)
    p.send("\x1b[H").expect("Home");
    p.send("\x16").expect("Ctrl+V");
    for _ in 0..8 {
        p.send("\x1b[C").expect("Right");
    }
    
    // Copy (would be Ctrl+C in real app)
    p.send("\x03").expect("Ctrl+C");
    
    // Move elsewhere
    p.send("\x1b").expect("Esc"); // Cancel selection
    p.send("\x1b[B").expect("Down");
    p.send("\x1b[B").expect("Down");
    
    // Paste (would be Ctrl+V in real app)
    p.send("\x16").expect("Ctrl+V for paste");
    
    p.send("q").expect("Quit");
}