use anyhow::Result;
use rexpect::spawn;
use std::time::Duration;

/// Test runner that captures ALL output including stderr and visual feedback
pub struct TestRunner {
    session: rexpect::session::PtySession,
    captured_output: Vec<String>,
}

impl TestRunner {
    /// Start a new test session with the app
    pub fn new(pdf_path: &str) -> Result<Self> {
        // Set environment for Kitty
        std::env::set_var("KITTY_WINDOW_ID", "1");
        std::env::set_var("TERM", "xterm-kitty");
        
        // Spawn the app with PTY to capture everything
        let mut session = spawn(
            &format!("./target/release/chonker8-hot {}", pdf_path),
            Some(Duration::from_secs(30))
        )?;
        
        Ok(Self {
            session,
            captured_output: Vec::new(),
        })
    }
    
    /// Capture all current output
    pub fn capture_output(&mut self) -> Result<String> {
        // Read everything available
        let mut output = String::new();
        
        // Try to read with timeout
        match self.session.try_read() {
            Ok(data) => {
                output.push_str(&data);
                self.captured_output.push(data.clone());
            }
            Err(_) => {
                // No more data available
            }
        }
        
        Ok(output)
    }
    
    /// Wait for specific pattern in output
    pub fn wait_for(&mut self, pattern: &str) -> Result<bool> {
        match self.session.exp_string(pattern) {
            Ok(_) => {
                println!("[TEST] Found pattern: {}", pattern);
                Ok(true)
            }
            Err(e) => {
                println!("[TEST] Pattern not found: {} ({})", pattern, e);
                Ok(false)
            }
        }
    }
    
    /// Send a key press
    pub fn send_key(&mut self, key: &str) -> Result<()> {
        self.session.send(key)?;
        Ok(())
    }
    
    /// Get all captured output
    pub fn get_all_output(&self) -> String {
        self.captured_output.join("\n")
    }
    
    /// Check if Kitty graphics are working
    pub fn check_kitty_graphics(&mut self) -> Result<bool> {
        // Look for Kitty escape sequences in output
        let output = self.capture_output()?;
        
        // Check for Kitty graphics protocol markers
        let has_kitty_clear = output.contains("\x1b_Ga=d");
        let has_kitty_transmit = output.contains("\x1b_Ga=T");
        let has_image_data = output.contains("f=100");
        
        println!("[TEST] Kitty graphics check:");
        println!("  - Clear command: {}", has_kitty_clear);
        println!("  - Transmit command: {}", has_kitty_transmit);
        println!("  - Image format: {}", has_image_data);
        
        Ok(has_kitty_clear && has_kitty_transmit)
    }
    
    /// Dump the current screen content
    pub fn dump_screen(&mut self) -> Result<()> {
        let output = self.capture_output()?;
        
        println!("=== SCREEN DUMP ===");
        println!("{}", output);
        println!("=== END DUMP ===");
        
        // Also show hex dump of first 200 bytes to see escape codes
        println!("=== HEX DUMP (first 200 bytes) ===");
        for (i, byte) in output.bytes().take(200).enumerate() {
            if i % 16 == 0 {
                println!();
                print!("{:04x}: ", i);
            }
            print!("{:02x} ", byte);
        }
        println!("\n=== END HEX DUMP ===");
        
        Ok(())
    }
    
    /// Run automated test sequence
    pub fn run_test_sequence(&mut self) -> Result<()> {
        println!("[TEST] Starting test sequence...");
        
        // Wait for initial load
        std::thread::sleep(Duration::from_millis(500));
        
        // Capture initial state
        println!("[TEST] Initial output:");
        self.dump_screen()?;
        
        // Check for Kitty graphics
        if self.check_kitty_graphics()? {
            println!("[TEST] ✅ Kitty graphics detected!");
        } else {
            println!("[TEST] ❌ No Kitty graphics detected!");
        }
        
        // Check for debug messages
        let output = self.get_all_output();
        if output.contains("[DEBUG]") {
            println!("[TEST] ✅ Debug messages present");
        }
        if output.contains("[KITTY]") {
            println!("[TEST] ✅ Kitty protocol messages present");
        }
        if output.contains("SIMPLE_KITTY") {
            println!("[TEST] ✅ Simple Kitty messages present");
        }
        
        // Try to switch screens with Tab
        println!("[TEST] Sending Tab key...");
        self.send_key("\t")?;
        std::thread::sleep(Duration::from_millis(500));
        self.dump_screen()?;
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_pdf_display() {
        let mut runner = TestRunner::new("/Users/jack/Desktop/BERF-CERT.pdf").unwrap();
        runner.run_test_sequence().unwrap();
    }
}