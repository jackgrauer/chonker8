// Output capture system for hot reload debugging and rexpect testing

use std::{
    io::{self, Write},
    sync::{Arc, Mutex},
    collections::VecDeque,
};

/// Thread-safe output buffer that captures all stdout and stderr
pub struct OutputCapture {
    buffer: Arc<Mutex<VecDeque<String>>>,
    max_lines: usize,
}

impl OutputCapture {
    pub fn new(max_lines: usize) -> Self {
        Self {
            buffer: Arc::new(Mutex::new(VecDeque::new())),
            max_lines,
        }
    }

    /// Capture a line of output
    pub fn capture_line(&self, line: String) {
        let mut buffer = self.buffer.lock().unwrap();
        
        // Add timestamp for debugging
        let timestamped = format!("[{}] {}", 
            chrono::Utc::now().format("%H:%M:%S%.3f"), 
            line
        );
        
        buffer.push_back(timestamped);
        
        // Keep buffer size manageable
        while buffer.len() > self.max_lines {
            buffer.pop_front();
        }
        
        // Also write to actual stdout for normal operation
        println!("{}", line);
        io::stdout().flush().unwrap();
    }

    /// Get all captured output as a single string
    pub fn get_all_output(&self) -> String {
        let buffer = self.buffer.lock().unwrap();
        buffer.iter().cloned().collect::<Vec<_>>().join("\n")
    }

    /// Get recent output (last N lines)
    pub fn get_recent_output(&self, lines: usize) -> String {
        let buffer = self.buffer.lock().unwrap();
        buffer.iter()
            .rev()
            .take(lines)
            .rev()
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Clear the buffer
    pub fn clear(&self) {
        let mut buffer = self.buffer.lock().unwrap();
        buffer.clear();
    }

    /// Get buffer size
    pub fn len(&self) -> usize {
        let buffer = self.buffer.lock().unwrap();
        buffer.len()
    }
}

/// Global output capture instance
pub static GLOBAL_CAPTURE: once_cell::sync::Lazy<OutputCapture> = 
    once_cell::sync::Lazy::new(|| OutputCapture::new(1000));

/// Macro for capturing debug output
#[macro_export]
macro_rules! capture_debug {
    ($($arg:tt)*) => {
        {
            let msg = format!($($arg)*);
            crate::output_capture::GLOBAL_CAPTURE.capture_line(format!("[DEBUG] {}", msg));
        }
    };
}

/// Macro for capturing info output
#[macro_export]
macro_rules! capture_info {
    ($($arg:tt)*) => {
        {
            let msg = format!($($arg)*);
            crate::output_capture::GLOBAL_CAPTURE.capture_line(format!("[INFO] {}", msg));
        }
    };
}

/// Macro for capturing warning output
#[macro_export]
macro_rules! capture_warning {
    ($($arg:tt)*) => {
        {
            let msg = format!($($arg)*);
            crate::output_capture::GLOBAL_CAPTURE.capture_line(format!("[WARNING] {}", msg));
        }
    };
}

/// Macro for capturing error output
#[macro_export]
macro_rules! capture_error {
    ($($arg:tt)*) => {
        {
            let msg = format!($($arg)*);
            crate::output_capture::GLOBAL_CAPTURE.capture_line(format!("[ERROR] {}", msg));
        }
    };
}

/// Get all captured output
pub fn get_all_captured_output() -> String {
    GLOBAL_CAPTURE.get_all_output()
}

/// Get recent captured output
pub fn get_recent_captured_output(lines: usize) -> String {
    GLOBAL_CAPTURE.get_recent_output(lines)
}

/// Clear captured output
pub fn clear_captured_output() {
    GLOBAL_CAPTURE.clear();
}

/// Output capture writer that implements Write trait
pub struct CaptureWriter;

impl Write for CaptureWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let s = String::from_utf8_lossy(buf);
        for line in s.lines() {
            if !line.trim().is_empty() {
                GLOBAL_CAPTURE.capture_line(line.to_string());
            }
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        io::stdout().flush()
    }
}

/// Initialize output capture system
pub fn initialize_output_capture() {
    capture_info!("Output capture system initialized");
    capture_debug!("Buffer size: {} lines", GLOBAL_CAPTURE.max_lines);
}