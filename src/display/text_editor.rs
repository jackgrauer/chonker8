// Rope-based text editor for PDF content
use anyhow::Result;
use crossterm::{
    cursor::{Hide, Show},
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    execute,
    style::{Color, Print, ResetColor, SetBackgroundColor, SetForegroundColor},
    terminal::{self, Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen},
};
use ropey::Rope;
use std::{
    io::{stdout, Write},
    path::PathBuf,
    process::Command,
};

pub struct TextEditor {
    rope: Rope,
    pdf_path: PathBuf,
    scroll_offset: usize,
    max_visible: usize,
    cursor_line: usize,
    cursor_column: usize,
}

impl TextEditor {
    pub fn new(pdf_path: PathBuf) -> Result<Self> {
        let mut editor = Self {
            rope: Rope::new(),
            pdf_path,
            scroll_offset: 0,
            max_visible: 20,
            cursor_line: 0,
            cursor_column: 0,
        };
        
        editor.extract_pdf_text()?;
        Ok(editor)
    }
    
    fn extract_pdf_text(&mut self) -> Result<()> {
        // Use pdftotext to extract text from PDF
        let output = Command::new("pdftotext")
            .args(&[
                "-layout",  // Preserve layout
                "-nopgbrk", // No page breaks
                self.pdf_path.to_str().unwrap(),
                "-"  // Output to stdout
            ])
            .output();
        
        let text = match output {
            Ok(output) if output.status.success() => {
                String::from_utf8_lossy(&output.stdout).to_string()
            }
            _ => {
                // Fallback if pdftotext fails
                format!(
                    "Failed to extract text from: {}\n\n\
                    Make sure 'pdftotext' is installed:\n\
                      brew install poppler\n\n\
                    Or try a different PDF file.\n",
                    self.pdf_path.display()
                )
            }
        };
        
        // Load text into rope
        self.rope = Rope::from_str(&text);
        self.cursor_line = 0;
        self.cursor_column = 0;
        self.scroll_offset = 0;
        
        Ok(())
    }
    
    pub fn run(&mut self) -> Result<()> {
        // Enter alternate screen
        execute!(stdout(), EnterAlternateScreen, Hide)?;
        terminal::enable_raw_mode()?;
        
        let result = self.run_loop();
        
        // Cleanup
        terminal::disable_raw_mode()?;
        execute!(stdout(), Show, LeaveAlternateScreen)?;
        
        result
    }
    
    pub fn run_once(&mut self) -> Result<crate::app_state::StateTransition> {
        self.render()?;
        
        if let Event::Key(key) = event::read()? {
            match self.handle_key(key)? {
                EditorResult::Continue => Ok(crate::app_state::StateTransition::Continue),
                EditorResult::Exit => Ok(crate::app_state::StateTransition::Exit),
                EditorResult::SwitchToBrowser => Ok(crate::app_state::StateTransition::SwitchTo(crate::app_state::AppMode::FileBrowser)),
                EditorResult::HotReload => {
                    // Need to exit to terminal first
                    Ok(crate::app_state::StateTransition::HotReload)
                }
            }
        } else {
            Ok(crate::app_state::StateTransition::Continue)
        }
    }
    
    fn run_loop(&mut self) -> Result<()> {
        loop {
            self.render()?;
            
            if let Event::Key(key) = event::read()? {
                match self.handle_key(key)? {
                    EditorResult::Continue => continue,
                    EditorResult::Exit => break,
                    EditorResult::SwitchToBrowser => break, // For standalone mode, just exit
                    EditorResult::HotReload => break, // For standalone mode, just exit
                }
            }
        }
        Ok(())
    }
    
    fn handle_key(&mut self, key: KeyEvent) -> Result<EditorResult> {
        let line_count = self.rope.len_lines();
        
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => {
                return Ok(EditorResult::Exit);
            }
            KeyCode::Tab => {
                return Ok(EditorResult::SwitchToBrowser);
            }
            KeyCode::Up => {
                if self.cursor_line > 0 {
                    self.cursor_line -= 1;
                    self.clamp_cursor_column();
                    self.update_scroll();
                }
            }
            KeyCode::Down => {
                if self.cursor_line + 1 < line_count {
                    self.cursor_line += 1;
                    self.clamp_cursor_column();
                    self.update_scroll();
                }
            }
            KeyCode::Left => {
                if self.cursor_column > 0 {
                    self.cursor_column -= 1;
                } else if self.cursor_line > 0 {
                    // Move to end of previous line
                    self.cursor_line -= 1;
                    self.cursor_column = self.get_line_len(self.cursor_line);
                    self.update_scroll();
                }
            }
            KeyCode::Right => {
                let line_len = self.get_line_len(self.cursor_line);
                if self.cursor_column < line_len {
                    self.cursor_column += 1;
                } else if self.cursor_line + 1 < line_count {
                    // Move to start of next line
                    self.cursor_line += 1;
                    self.cursor_column = 0;
                    self.update_scroll();
                }
            }
            KeyCode::PageUp => {
                self.cursor_line = self.cursor_line.saturating_sub(self.max_visible);
                self.clamp_cursor_column();
                self.update_scroll();
            }
            KeyCode::PageDown => {
                self.cursor_line = (self.cursor_line + self.max_visible).min(line_count.saturating_sub(1));
                self.clamp_cursor_column();
                self.update_scroll();
            }
            KeyCode::Home => {
                if key.modifiers.contains(KeyModifiers::CONTROL) {
                    // Go to start of document
                    self.cursor_line = 0;
                    self.cursor_column = 0;
                    self.scroll_offset = 0;
                } else {
                    // Go to start of line
                    self.cursor_column = 0;
                }
            }
            KeyCode::End => {
                if key.modifiers.contains(KeyModifiers::CONTROL) {
                    // Go to end of document
                    self.cursor_line = line_count.saturating_sub(1);
                    self.cursor_column = self.get_line_len(self.cursor_line);
                    self.update_scroll();
                } else {
                    // Go to end of line
                    self.cursor_column = self.get_line_len(self.cursor_line);
                }
            }
            KeyCode::Char(c) => {
                if key.modifiers.contains(KeyModifiers::CONTROL) {
                    match c {
                        'c' => return Ok(EditorResult::Exit),
                        'u' => {
                            // Debug: write to a file to confirm key is detected
                            std::fs::write("/tmp/chonker8_debug.txt", "Ctrl+U detected in text editor").ok();
                            return Ok(EditorResult::HotReload);
                        }
                        'r' => {
                            // Reload PDF content
                            self.extract_pdf_text()?;
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
        
        Ok(EditorResult::Continue)
    }
    
    fn get_line_len(&self, line_idx: usize) -> usize {
        if line_idx < self.rope.len_lines() {
            let line = self.rope.line(line_idx);
            // Subtract 1 for newline character, but handle empty lines
            line.len_chars().saturating_sub(1)
        } else {
            0
        }
    }
    
    fn clamp_cursor_column(&mut self) {
        let line_len = self.get_line_len(self.cursor_line);
        self.cursor_column = self.cursor_column.min(line_len);
    }
    
    fn update_scroll(&mut self) {
        if self.cursor_line < self.scroll_offset {
            self.scroll_offset = self.cursor_line;
        } else if self.cursor_line >= self.scroll_offset + self.max_visible {
            self.scroll_offset = self.cursor_line - self.max_visible + 1;
        }
    }
    
    fn render(&mut self) -> Result<()> {
        let (width, height) = terminal::size()?;
        self.max_visible = (height as usize).saturating_sub(4);
        
        execute!(stdout(), Clear(ClearType::All))?;
        
        // Title bar
        execute!(
            stdout(),
            crossterm::cursor::MoveTo(0, 0),
            SetBackgroundColor(Color::DarkBlue),
            SetForegroundColor(Color::White),
            Print(format!(" 📄 {} ", self.pdf_path.file_name().unwrap_or_default().to_string_lossy())),
            ResetColor
        )?;
        
        // Content
        let start_y = 1;
        let line_count = self.rope.len_lines();
        let end_line = (self.scroll_offset + self.max_visible).min(line_count);
        
        for line_idx in self.scroll_offset..end_line {
            let y = start_y + (line_idx - self.scroll_offset) as u16;
            let line_content = if line_idx < line_count {
                self.rope.line(line_idx).to_string()
            } else {
                String::new()
            };
            
            // Remove trailing newline for display
            let display_line = line_content.trim_end_matches('\n');
            
            // Highlight current line
            if line_idx == self.cursor_line {
                execute!(
                    stdout(),
                    crossterm::cursor::MoveTo(0, y),
                    SetBackgroundColor(Color::DarkGrey),
                    Print(format!("{:4} {:<width$}", line_idx + 1, display_line, width = width as usize - 5)),
                    ResetColor
                )?;
            } else {
                execute!(
                    stdout(),
                    crossterm::cursor::MoveTo(0, y),
                    SetForegroundColor(Color::DarkGrey),
                    Print(format!("{:4}", line_idx + 1)),
                    ResetColor,
                    Print(format!(" {}", display_line))
                )?;
            }
        }
        
        // Status bar
        let status_y = height - 1;
        let total_lines = line_count;
        let current_line = self.cursor_line + 1;
        let current_column = self.cursor_column + 1;
        
        let status_text = format!(
            " Ln {}/{}, Col {} | ←→↑↓: Navigate | PgUp/PgDn: Page | Tab: Switch to Browser | Ctrl+Home/End: Document | Ctrl-R: Reload | Esc/q: Exit ",
            current_line, total_lines, current_column
        );
        
        execute!(
            stdout(),
            crossterm::cursor::MoveTo(0, status_y),
            SetBackgroundColor(Color::DarkBlue),
            SetForegroundColor(Color::White),
            Print(format!("{:<width$}", status_text, width = width as usize)),
            ResetColor
        )?;
        
        stdout().flush()?;
        Ok(())
    }
}

pub enum EditorResult {
    Continue,
    Exit,
    SwitchToBrowser,
    HotReload,
}