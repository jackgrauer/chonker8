// Advanced rope-based text editor with selection and editing
use anyhow::Result;
use arboard::Clipboard;
use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers, MouseEvent, MouseEventKind, EnableMouseCapture, DisableMouseCapture},
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

#[derive(Debug, Clone, PartialEq)]
pub enum SelectionMode {
    None,
    Normal,    // Line-based selection
    Block,     // Rectangular block selection
}

#[derive(Debug, Clone)]
pub struct Selection {
    pub start_line: usize,
    pub start_col: usize,
    pub end_line: usize,
    pub end_col: usize,
    pub mode: SelectionMode,
}

impl Selection {
    pub fn new(line: usize, col: usize, mode: SelectionMode) -> Self {
        Self {
            start_line: line,
            start_col: col,
            end_line: line,
            end_col: col,
            mode,
        }
    }
    
    pub fn update_end(&mut self, line: usize, col: usize) {
        self.end_line = line;
        self.end_col = col;
    }
    
    pub fn is_active(&self) -> bool {
        self.mode != SelectionMode::None && 
        (self.start_line != self.end_line || self.start_col != self.end_col)
    }
    
    pub fn get_bounds(&self) -> (usize, usize, usize, usize) {
        let start_line = self.start_line.min(self.end_line);
        let end_line = self.start_line.max(self.end_line);
        let start_col = if start_line == end_line {
            self.start_col.min(self.end_col)
        } else if self.start_line < self.end_line {
            self.start_col
        } else {
            self.end_col
        };
        let end_col = if start_line == end_line {
            self.start_col.max(self.end_col)
        } else if self.start_line < self.end_line {
            self.end_col
        } else {
            self.start_col
        };
        (start_line, start_col, end_line, end_col)
    }
}

pub struct TextEditor {
    rope: Rope,
    pdf_path: PathBuf,
    scroll_offset: usize,
    max_visible: usize,
    cursor_line: usize,
    cursor_column: usize,
    selection: Selection,
    clipboard: Option<Clipboard>,
    modified: bool,
    mouse_down: bool,
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
            selection: Selection::new(0, 0, SelectionMode::None),
            clipboard: Clipboard::new().ok(),
            modified: false,
            mouse_down: false,
        };
        
        editor.extract_pdf_text()?;
        Ok(editor)
    }
    
    fn extract_pdf_text(&mut self) -> Result<()> {
        // Simple, reliable pdftotext extraction
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
                format!(
                    "Failed to extract text from: {}\n\
                    Make sure 'pdftotext' is installed: brew install poppler",
                    self.pdf_path.display()
                )
            }
        };
        
        // Load text into rope
        self.rope = Rope::from_str(&text);
        self.cursor_line = 0;
        self.cursor_column = 0;
        self.scroll_offset = 0;
        self.selection = Selection::new(0, 0, SelectionMode::None);
        self.modified = false;
        
        Ok(())
    }
    
    pub fn run(&mut self) -> Result<()> {
        // Enter alternate screen with mouse support
        execute!(stdout(), EnterAlternateScreen, EnableMouseCapture)?;
        terminal::enable_raw_mode()?;
        
        let result = self.run_loop();
        
        // Cleanup
        terminal::disable_raw_mode()?;
        execute!(stdout(), DisableMouseCapture, LeaveAlternateScreen)?;
        
        result
    }
    
    pub fn run_once(&mut self) -> Result<crate::app_state::StateTransition> {
        self.render()?;
        
        match event::read()? {
            Event::Key(key) => {
                match self.handle_key(key)? {
                    EditorResult::Continue => Ok(crate::app_state::StateTransition::Continue),
                    EditorResult::Exit => Ok(crate::app_state::StateTransition::Exit),
                    EditorResult::SwitchToBrowser => Ok(crate::app_state::StateTransition::SwitchTo(crate::app_state::AppMode::FileBrowser)),
                    EditorResult::HotReload => {
                        Ok(crate::app_state::StateTransition::HotReload)
                    }
                }
            }
            Event::Mouse(mouse) => {
                self.handle_mouse(mouse)?;
                Ok(crate::app_state::StateTransition::Continue)
            }
            _ => Ok(crate::app_state::StateTransition::Continue)
        }
    }
    
    fn run_loop(&mut self) -> Result<()> {
        loop {
            self.render()?;
            
            match event::read()? {
                Event::Key(key) => {
                    match self.handle_key(key)? {
                        EditorResult::Continue => continue,
                        EditorResult::Exit => break,
                        EditorResult::SwitchToBrowser => break,
                        EditorResult::HotReload => break,
                    }
                }
                Event::Mouse(mouse) => {
                    self.handle_mouse(mouse)?;
                }
                _ => {}
            }
        }
        Ok(())
    }
    
    fn handle_key(&mut self, key: KeyEvent) -> Result<EditorResult> {
        let line_count = self.rope.len_lines();
        let shift_held = key.modifiers.contains(KeyModifiers::SHIFT);
        let ctrl_held = key.modifiers.contains(KeyModifiers::CONTROL);
        
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => {
                return Ok(EditorResult::Exit);
            }
            KeyCode::Tab => {
                return Ok(EditorResult::SwitchToBrowser);
            }
            KeyCode::Up => {
                if shift_held {
                    self.extend_selection();
                }
                if self.cursor_line > 0 {
                    self.cursor_line -= 1;
                    self.clamp_cursor_column();
                    self.update_scroll();
                }
                if shift_held {
                    self.update_selection_end();
                } else {
                    self.clear_selection();
                }
            }
            KeyCode::Down => {
                if shift_held {
                    self.extend_selection();
                }
                if self.cursor_line + 1 < line_count {
                    self.cursor_line += 1;
                    self.clamp_cursor_column();
                    self.update_scroll();
                }
                if shift_held {
                    self.update_selection_end();
                } else {
                    self.clear_selection();
                }
            }
            KeyCode::Left => {
                if shift_held {
                    self.extend_selection();
                }
                if self.cursor_column > 0 {
                    self.cursor_column -= 1;
                } else if self.cursor_line > 0 {
                    self.cursor_line -= 1;
                    self.cursor_column = self.get_line_len(self.cursor_line);
                    self.update_scroll();
                }
                if shift_held {
                    self.update_selection_end();
                } else {
                    self.clear_selection();
                }
            }
            KeyCode::Right => {
                if shift_held {
                    self.extend_selection();
                }
                let line_len = self.get_line_len(self.cursor_line);
                if self.cursor_column < line_len {
                    self.cursor_column += 1;
                } else if self.cursor_line + 1 < line_count {
                    self.cursor_line += 1;
                    self.cursor_column = 0;
                    self.update_scroll();
                }
                if shift_held {
                    self.update_selection_end();
                } else {
                    self.clear_selection();
                }
            }
            KeyCode::PageUp => {
                if shift_held {
                    self.extend_selection();
                }
                self.cursor_line = self.cursor_line.saturating_sub(self.max_visible);
                self.clamp_cursor_column();
                self.update_scroll();
                if shift_held {
                    self.update_selection_end();
                } else {
                    self.clear_selection();
                }
            }
            KeyCode::PageDown => {
                if shift_held {
                    self.extend_selection();
                }
                self.cursor_line = (self.cursor_line + self.max_visible).min(line_count.saturating_sub(1));
                self.clamp_cursor_column();
                self.update_scroll();
                if shift_held {
                    self.update_selection_end();
                } else {
                    self.clear_selection();
                }
            }
            KeyCode::Home => {
                if shift_held {
                    self.extend_selection();
                }
                if ctrl_held {
                    // Go to start of document
                    self.cursor_line = 0;
                    self.cursor_column = 0;
                    self.scroll_offset = 0;
                } else {
                    // Go to start of line
                    self.cursor_column = 0;
                }
                if shift_held {
                    self.update_selection_end();
                } else {
                    self.clear_selection();
                }
            }
            KeyCode::End => {
                if shift_held {
                    self.extend_selection();
                }
                if ctrl_held {
                    // Go to end of document
                    self.cursor_line = line_count.saturating_sub(1);
                    self.cursor_column = self.get_line_len(self.cursor_line);
                    self.update_scroll();
                } else {
                    // Go to end of line
                    self.cursor_column = self.get_line_len(self.cursor_line);
                }
                if shift_held {
                    self.update_selection_end();
                } else {
                    self.clear_selection();
                }
            }
            KeyCode::Char(c) => {
                if ctrl_held {
                    match c {
                        'c' if self.selection.is_active() => {
                            // Copy selection
                            self.copy_selection()?;
                        }
                        'x' if self.selection.is_active() => {
                            // Cut selection
                            self.cut_selection()?;
                        }
                        'v' => {
                            // Paste
                            self.paste()?;
                        }
                        'a' => {
                            // Select all
                            self.select_all();
                        }
                        'b' => {
                            // Toggle block selection mode
                            self.toggle_block_selection();
                        }
                        'c' => return Ok(EditorResult::Exit),
                        'u' => {
                            std::fs::write("/tmp/chonker8_debug.txt", "Ctrl+U detected in text editor").ok();
                            return Ok(EditorResult::HotReload);
                        }
                        'r' => {
                            // Reload PDF content
                            self.extract_pdf_text()?;
                        }
                        _ => {}
                    }
                } else {
                    // Regular character input
                    self.insert_char(c)?;
                }
            }
            KeyCode::Enter => {
                self.insert_newline()?;
            }
            KeyCode::Backspace => {
                self.backspace()?;
            }
            KeyCode::Delete => {
                self.delete()?;
            }
            _ => {}
        }
        
        Ok(EditorResult::Continue)
    }
    
    fn handle_mouse(&mut self, mouse: MouseEvent) -> Result<()> {
        match mouse.kind {
            MouseEventKind::Down(button) => {
                if button == crossterm::event::MouseButton::Left {
                    // Click to position cursor
                    let click_line = (mouse.row as usize) + self.scroll_offset;
                    let click_col = mouse.column as usize;
                    
                    // Clamp to valid positions
                    let line_count = self.rope.len_lines();
                    if click_line < line_count {
                        let line_len = self.get_line_len(click_line);
                        self.cursor_line = click_line;
                        self.cursor_column = click_col.min(line_len);
                        
                        // Start selection if shift held
                        if mouse.modifiers.contains(KeyModifiers::SHIFT) {
                            self.extend_selection();
                            self.update_selection_end();
                        } else {
                            self.clear_selection();
                            self.mouse_down = true;
                        }
                    }
                }
            }
            MouseEventKind::Drag(button) => {
                if button == crossterm::event::MouseButton::Left && self.mouse_down {
                    // Drag to extend selection
                    if self.selection.mode == SelectionMode::None {
                        self.selection = Selection::new(self.cursor_line, self.cursor_column, SelectionMode::Normal);
                    }
                    
                    let drag_line = (mouse.row as usize) + self.scroll_offset;
                    let drag_col = mouse.column as usize;
                    
                    let line_count = self.rope.len_lines();
                    if drag_line < line_count {
                        let line_len = self.get_line_len(drag_line);
                        self.cursor_line = drag_line;
                        self.cursor_column = drag_col.min(line_len);
                        self.update_selection_end();
                    }
                }
            }
            MouseEventKind::Up(button) => {
                if button == crossterm::event::MouseButton::Left {
                    self.mouse_down = false;
                }
            }
            MouseEventKind::ScrollUp => {
                if self.scroll_offset > 0 {
                    self.scroll_offset -= 3; // Scroll 3 lines
                }
            }
            MouseEventKind::ScrollDown => {
                let line_count = self.rope.len_lines();
                let max_scroll = line_count.saturating_sub(self.max_visible);
                if self.scroll_offset < max_scroll {
                    self.scroll_offset += 3; // Scroll 3 lines
                }
            }
            _ => {}
        }
        
        Ok(())
    }
    
    fn extend_selection(&mut self) {
        if self.selection.mode == SelectionMode::None {
            self.selection = Selection::new(self.cursor_line, self.cursor_column, SelectionMode::Normal);
        }
    }
    
    fn update_selection_end(&mut self) {
        self.selection.update_end(self.cursor_line, self.cursor_column);
    }
    
    fn clear_selection(&mut self) {
        self.selection.mode = SelectionMode::None;
    }
    
    fn toggle_block_selection(&mut self) {
        if self.selection.mode == SelectionMode::Block {
            self.selection.mode = SelectionMode::Normal;
        } else {
            self.selection = Selection::new(self.cursor_line, self.cursor_column, SelectionMode::Block);
        }
    }
    
    fn select_all(&mut self) {
        let line_count = self.rope.len_lines();
        let last_line = line_count.saturating_sub(1);
        let last_col = self.get_line_len(last_line);
        
        self.selection = Selection {
            start_line: 0,
            start_col: 0,
            end_line: last_line,
            end_col: last_col,
            mode: SelectionMode::Normal,
        };
    }
    
    fn copy_selection(&mut self) -> Result<()> {
        if !self.selection.is_active() {
            return Ok(());
        }
        
        let text = self.get_selected_text();
        if let Some(ref mut clipboard) = self.clipboard {
            clipboard.set_text(text).ok();
        }
        Ok(())
    }
    
    fn cut_selection(&mut self) -> Result<()> {
        if !self.selection.is_active() {
            return Ok(());
        }
        
        let text = self.get_selected_text();
        if let Some(ref mut clipboard) = self.clipboard {
            clipboard.set_text(text).ok();
        }
        
        self.delete_selection()?;
        Ok(())
    }
    
    fn paste(&mut self) -> Result<()> {
        if let Some(ref mut clipboard) = self.clipboard {
            if let Ok(text) = clipboard.get_text() {
                if self.selection.is_active() {
                    self.delete_selection()?;
                }
                self.insert_text(&text)?;
            }
        }
        Ok(())
    }
    
    fn get_selected_text(&self) -> String {
        if !self.selection.is_active() {
            return String::new();
        }
        
        let (start_line, start_col, end_line, end_col) = self.selection.get_bounds();
        
        match self.selection.mode {
            SelectionMode::Normal => {
                if start_line == end_line {
                    // Single line selection
                    let line = self.rope.line(start_line);
                    let line_str = line.to_string();
                    line_str.chars().skip(start_col).take(end_col - start_col).collect()
                } else {
                    // Multi-line selection
                    let mut result = String::new();
                    for line_idx in start_line..=end_line {
                        let line = self.rope.line(line_idx);
                        let line_str = line.to_string();
                        
                        if line_idx == start_line {
                            result.push_str(&line_str.chars().skip(start_col).collect::<String>());
                        } else if line_idx == end_line {
                            result.push_str(&line_str.chars().take(end_col).collect::<String>());
                        } else {
                            result.push_str(&line_str);
                        }
                    }
                    result
                }
            }
            SelectionMode::Block => {
                // Block selection - rectangular area
                let mut result = String::new();
                let left_col = start_col.min(end_col);
                let right_col = start_col.max(end_col);
                
                for line_idx in start_line..=end_line {
                    if line_idx > start_line {
                        result.push('\n');
                    }
                    let line = self.rope.line(line_idx);
                    let line_str = line.to_string();
                    let block_text: String = line_str.chars()
                        .skip(left_col)
                        .take(right_col - left_col)
                        .collect();
                    result.push_str(&block_text);
                }
                result
            }
            SelectionMode::None => String::new(),
        }
    }
    
    fn delete_selection(&mut self) -> Result<()> {
        if !self.selection.is_active() {
            return Ok(());
        }
        
        let (start_line, start_col, end_line, end_col) = self.selection.get_bounds();
        
        match self.selection.mode {
            SelectionMode::Normal => {
                if start_line == end_line {
                    // Single line deletion
                    let start_char = self.rope.line_to_char(start_line) + start_col;
                    let end_char = self.rope.line_to_char(end_line) + end_col;
                    self.rope.remove(start_char..end_char);
                } else {
                    // Multi-line deletion
                    let start_char = self.rope.line_to_char(start_line) + start_col;
                    let end_char = self.rope.line_to_char(end_line) + end_col;
                    self.rope.remove(start_char..end_char);
                }
                
                self.cursor_line = start_line;
                self.cursor_column = start_col;
            }
            SelectionMode::Block => {
                // Block deletion - remove rectangular area
                let left_col = start_col.min(end_col);
                let right_col = start_col.max(end_col);
                
                // Delete from bottom to top to maintain line indices
                for line_idx in (start_line..=end_line).rev() {
                    let line_start = self.rope.line_to_char(line_idx);
                    let line_len = self.get_line_len(line_idx);
                    let actual_left = left_col.min(line_len);
                    let actual_right = right_col.min(line_len);
                    
                    if actual_left < actual_right {
                        let start_char = line_start + actual_left;
                        let end_char = line_start + actual_right;
                        self.rope.remove(start_char..end_char);
                    }
                }
                
                self.cursor_line = start_line;
                self.cursor_column = left_col;
            }
            SelectionMode::None => {}
        }
        
        self.clear_selection();
        self.modified = true;
        self.clamp_cursor_column();
        
        Ok(())
    }
    
    fn insert_char(&mut self, c: char) -> Result<()> {
        if self.selection.is_active() {
            self.delete_selection()?;
        }
        
        let char_idx = self.rope.line_to_char(self.cursor_line) + self.cursor_column;
        self.rope.insert_char(char_idx, c);
        self.cursor_column += 1;
        self.modified = true;
        
        Ok(())
    }
    
    fn insert_text(&mut self, text: &str) -> Result<()> {
        if self.selection.is_active() {
            self.delete_selection()?;
        }
        
        let char_idx = self.rope.line_to_char(self.cursor_line) + self.cursor_column;
        self.rope.insert(char_idx, text);
        
        // Update cursor position based on inserted text
        let newline_count = text.matches('\n').count();
        if newline_count > 0 {
            self.cursor_line += newline_count;
            let last_line = text.lines().last().unwrap_or("");
            self.cursor_column = last_line.len();
        } else {
            self.cursor_column += text.len();
        }
        
        self.modified = true;
        self.update_scroll();
        
        Ok(())
    }
    
    fn insert_newline(&mut self) -> Result<()> {
        if self.selection.is_active() {
            self.delete_selection()?;
        }
        
        let char_idx = self.rope.line_to_char(self.cursor_line) + self.cursor_column;
        self.rope.insert_char(char_idx, '\n');
        self.cursor_line += 1;
        self.cursor_column = 0;
        self.modified = true;
        self.update_scroll();
        
        Ok(())
    }
    
    fn backspace(&mut self) -> Result<()> {
        if self.selection.is_active() {
            self.delete_selection()?;
            return Ok(());
        }
        
        if self.cursor_column > 0 {
            // Delete character before cursor
            self.cursor_column -= 1;
            let char_idx = self.rope.line_to_char(self.cursor_line) + self.cursor_column;
            self.rope.remove(char_idx..char_idx + 1);
            self.modified = true;
        } else if self.cursor_line > 0 {
            // Join with previous line
            let prev_line_len = self.get_line_len(self.cursor_line - 1);
            let char_idx = self.rope.line_to_char(self.cursor_line) - 1; // Remove newline
            self.rope.remove(char_idx..char_idx + 1);
            self.cursor_line -= 1;
            self.cursor_column = prev_line_len;
            self.modified = true;
            self.update_scroll();
        }
        
        Ok(())
    }
    
    fn delete(&mut self) -> Result<()> {
        if self.selection.is_active() {
            self.delete_selection()?;
            return Ok(());
        }
        
        let line_len = self.get_line_len(self.cursor_line);
        let line_count = self.rope.len_lines();
        
        if self.cursor_column < line_len {
            // Delete character at cursor
            let char_idx = self.rope.line_to_char(self.cursor_line) + self.cursor_column;
            self.rope.remove(char_idx..char_idx + 1);
            self.modified = true;
        } else if self.cursor_line + 1 < line_count {
            // Join with next line
            let char_idx = self.rope.line_to_char(self.cursor_line + 1) - 1; // Remove newline
            self.rope.remove(char_idx..char_idx + 1);
            self.modified = true;
        }
        
        Ok(())
    }
    
    fn get_line_len(&self, line_idx: usize) -> usize {
        if line_idx < self.rope.len_lines() {
            let line = self.rope.line(line_idx);
            line.len_chars().saturating_sub(1) // Subtract newline
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
    
    fn is_position_selected(&self, line: usize, col: usize) -> bool {
        if !self.selection.is_active() {
            return false;
        }
        
        let (start_line, start_col, end_line, end_col) = self.selection.get_bounds();
        
        match self.selection.mode {
            SelectionMode::Normal => {
                if line < start_line || line > end_line {
                    false
                } else if line == start_line && line == end_line {
                    col >= start_col && col < end_col
                } else if line == start_line {
                    col >= start_col
                } else if line == end_line {
                    col < end_col
                } else {
                    true // Middle lines are fully selected
                }
            }
            SelectionMode::Block => {
                let left_col = start_col.min(end_col);
                let right_col = start_col.max(end_col);
                line >= start_line && line <= end_line && col >= left_col && col < right_col
            }
            SelectionMode::None => false,
        }
    }
    
    fn render(&mut self) -> Result<()> {
        let (width, height) = terminal::size()?;
        self.max_visible = height as usize; // Use full screen
        
        execute!(stdout(), Clear(ClearType::All))?;
        
        // Show block selection mode indicator in top-right corner
        if self.selection.mode == SelectionMode::Block {
            execute!(
                stdout(),
                crossterm::cursor::MoveTo(width.saturating_sub(6), 0),
                SetBackgroundColor(Color::DarkRed),
                SetForegroundColor(Color::White),
                Print("BLOCK"),
                ResetColor
            )?;
        }
        
        // Content without line numbers
        let line_count = self.rope.len_lines();
        let end_line = (self.scroll_offset + self.max_visible).min(line_count);
        
        for line_idx in self.scroll_offset..end_line {
            let y = (line_idx - self.scroll_offset) as u16;
            let line_content = if line_idx < line_count {
                self.rope.line(line_idx).to_string()
            } else {
                String::new()
            };
            
            let display_line = line_content.trim_end_matches('\n');
            
            // Render line with selection highlighting
            execute!(stdout(), crossterm::cursor::MoveTo(0, y))?;
            
            // Render line with selection highlighting and cursor
            for (col_idx, ch) in display_line.chars().enumerate() {
                let is_cursor_pos = line_idx == self.cursor_line && col_idx == self.cursor_column;
                let is_selected = self.selection.is_active() && self.is_position_selected(line_idx, col_idx);
                
                if is_cursor_pos && !is_selected {
                    // Glowing cursor - always show as bright block
                    execute!(
                        stdout(),
                        SetBackgroundColor(Color::Yellow),
                        SetForegroundColor(Color::Blue),
                        Print('█'), // Always show cursor as block regardless of character
                        ResetColor
                    )?;
                } else if is_selected {
                    // Selection highlighting
                    execute!(
                        stdout(),
                        SetBackgroundColor(Color::Blue),
                        SetForegroundColor(Color::White),
                        Print(ch),
                        ResetColor
                    )?;
                } else {
                    // Regular character
                    execute!(stdout(), Print(ch))?;
                }
            }
            
            // If cursor is at end of line, show cursor block
            if line_idx == self.cursor_line && self.cursor_column >= display_line.len() {
                execute!(
                    stdout(),
                    SetBackgroundColor(Color::Yellow),
                    SetForegroundColor(Color::Blue),
                    Print('█'),
                    ResetColor
                )?;
            }
        }
        
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