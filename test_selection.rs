#!/usr/bin/env rust-script

//! Test the selection visibility in chonker8
//! 
//! ```cargo
//! [dependencies]
//! crossterm = "0.27"
//! ```

use crossterm::{
    execute,
    cursor::{MoveTo, Show, Hide},
    style::{SetBackgroundColor, SetForegroundColor, Color, ResetColor, Print},
    terminal::{Clear, ClearType, enable_raw_mode, disable_raw_mode},
    event::{self, Event, KeyCode, KeyModifiers},
};
use std::io::{stdout, Result};

fn main() -> Result<()> {
    enable_raw_mode()?;
    
    // Test selection colors
    execute!(
        stdout(),
        Clear(ClearType::All),
        MoveTo(0, 0),
        Print("Testing selection visibility:\n\n"),
    )?;
    
    // Test 1: Normal text
    execute!(
        stdout(),
        Print("Normal text: "),
        Print("Hello World"),
        Print("\n\n"),
    )?;
    
    // Test 2: Yellow cursor (no selection)
    execute!(
        stdout(),
        Print("Yellow cursor: "),
        SetBackgroundColor(Color::Yellow),
        SetForegroundColor(Color::Black),
        Print("X"),
        ResetColor,
        Print("ello World"),
        Print("\n\n"),
    )?;
    
    // Test 3: Cyan cursor (during selection)
    execute!(
        stdout(),
        Print("Cyan cursor (selecting): "),
        SetBackgroundColor(Color::Cyan),
        SetForegroundColor(Color::Black),
        Print("X"),
        ResetColor,
        Print("ello World"),
        Print("\n\n"),
    )?;
    
    // Test 4: Blue selected text
    execute!(
        stdout(),
        Print("Blue selected text: "),
        SetBackgroundColor(Color::Blue),
        SetForegroundColor(Color::White),
        Print("Hello"),
        ResetColor,
        Print(" World"),
        Print("\n\n"),
    )?;
    
    // Test 5: Mixed - Cyan cursor with blue selection
    execute!(
        stdout(),
        Print("Selection with cursor: "),
        SetBackgroundColor(Color::Blue),
        SetForegroundColor(Color::White),
        Print("Hell"),
        SetBackgroundColor(Color::Cyan),
        SetForegroundColor(Color::Black),
        Print("o"),
        SetBackgroundColor(Color::Blue),
        SetForegroundColor(Color::White),
        Print(" Wor"),
        ResetColor,
        Print("ld"),
        Print("\n\n"),
    )?;
    
    execute!(
        stdout(),
        Print("\nPress any key to exit..."),
    )?;
    
    // Wait for keypress
    loop {
        if let Event::Key(_) = event::read()? {
            break;
        }
    }
    
    disable_raw_mode()?;
    execute!(stdout(), Clear(ClearType::All), MoveTo(0, 0))?;
    
    Ok(())
}