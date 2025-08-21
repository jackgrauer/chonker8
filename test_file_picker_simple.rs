#!/usr/bin/env rust-script

//! Simple test to check if the integrated file picker is working
//! 
//! ```cargo
//! [dependencies]
//! chonker8 = { path = "." }
//! anyhow = "1.0"
//! crossterm = "0.27"
//! ```

use anyhow::Result;
use chonker8::integrated_file_picker::IntegratedFilePicker;
use crossterm::{
    execute,
    terminal::{self, Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen},
    cursor::Hide,
    event::{self, Event, KeyCode},
};
use std::io::stdout;
use std::time::Duration;

fn main() -> Result<()> {
    println!("🧪 Testing IntegratedFilePicker directly...");

    // Initialize the file picker
    let mut file_picker = match IntegratedFilePicker::new() {
        Ok(picker) => {
            println!("✅ File picker initialized successfully");
            picker
        },
        Err(e) => {
            println!("❌ Failed to initialize file picker: {}", e);
            return Ok(());
        }
    };

    // Setup terminal
    terminal::enable_raw_mode()?;
    execute!(stdout(), EnterAlternateScreen, Hide)?;

    println!("🎮 Test controls:");
    println!("  - Type to search");
    println!("  - Up/Down to navigate");
    println!("  - Enter to select");
    println!("  - Esc to exit");
    println!("  - Press any key to start...");

    // Wait for user to press a key
    loop {
        if let Event::Key(_) = event::read()? {
            break;
        }
    }

    // Main test loop
    loop {
        // Get terminal size
        let (width, height) = terminal::size()?;

        // Render the file picker
        file_picker.render(width, height)?;

        // Handle input
        if event::poll(Duration::from_millis(50))? {
            match event::read()? {
                Event::Key(key) => {
                    match key.code {
                        KeyCode::Char(c) => {
                            file_picker.handle_char(c)?;
                        }
                        KeyCode::Backspace => {
                            file_picker.handle_backspace()?;
                        }
                        KeyCode::Up => {
                            file_picker.handle_up()?;
                        }
                        KeyCode::Down => {
                            file_picker.handle_down()?;
                        }
                        KeyCode::Enter => {
                            if let Some(selected_file) = file_picker.get_selected_file() {
                                execute!(stdout(), Clear(ClearType::All))?;
                                println!("🎯 Selected file: {:?}", selected_file);
                                std::thread::sleep(Duration::from_millis(2000));
                                break;
                            }
                        }
                        KeyCode::Esc => {
                            break;
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
        }
    }

    // Cleanup terminal
    execute!(stdout(), LeaveAlternateScreen)?;
    terminal::disable_raw_mode()?;

    println!("✅ File picker test completed!");
    Ok(())
}