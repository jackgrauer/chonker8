// Grid editor for PDF text extraction
// Provides spreadsheet-like block selection and editing

use crossterm::event::KeyCode;
use arboard::Clipboard;
use std::cmp::{min, max};
use std::sync::{Arc, Mutex};
use nucleo::{Nucleo, Config as NucleoConfig};

// Threshold for recommending rope backend (characters) 
pub const ROPE_THRESHOLD: usize = 100_000;

pub struct Grid {
    pub cells: Vec<Vec<char>>,
    pub cursor: (usize, usize),  // (col, row)
    pub selecting: bool,
    pub anchor: (usize, usize),   // selection start point
    pub width: usize,
    pub height: usize,
    clipboard: Option<Arc<Mutex<Clipboard>>>,
    status_message: Option<String>,
    // Search functionality
    pub search_query: String,
    search_results: Vec<(usize, usize)>,  // (col, row) positions
    search_current_index: usize,
    pub searching: bool,
    // Scrolling offsets
    pub scroll_x: usize,
    pub scroll_y: usize,
}

impl Clone for Grid {
    fn clone(&self) -> Self {
        // Create a new clipboard instance for the clone
        let clipboard = Clipboard::new().ok().map(|c| Arc::new(Mutex::new(c)));
        Self {
            cells: self.cells.clone(),
            cursor: self.cursor,
            selecting: self.selecting,
            anchor: self.anchor,
            width: self.width,
            height: self.height,
            clipboard,
            status_message: self.status_message.clone(),
            search_query: self.search_query.clone(),
            search_results: self.search_results.clone(),
            search_current_index: self.search_current_index,
            searching: self.searching,
            scroll_x: self.scroll_x,
            scroll_y: self.scroll_y,
        }
    }
}

impl std::fmt::Debug for Grid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Grid")
            .field("cursor", &self.cursor)
            .field("selecting", &self.selecting)
            .field("anchor", &self.anchor)
            .field("width", &self.width)
            .field("height", &self.height)
            .field("status_message", &self.status_message)
            .finish()
    }
}

impl Default for Grid {
    fn default() -> Self {
        Self::new(80, 24)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SelectionMode {
    Character,
    Word,
    Line,
    Block,
    All,
}

impl Grid {
    /// Check if text is large enough to benefit from rope backend
    pub fn should_use_rope(text: &str) -> bool {
        text.len() > ROPE_THRESHOLD
    }

    pub fn new(width: usize, height: usize) -> Self {
        let cells = vec![vec![' '; width]; height];
        let clipboard = Clipboard::new().ok().map(|c| Arc::new(Mutex::new(c)));
        Self {
            cells,
            cursor: (0, 0),
            selecting: false,
            anchor: (0, 0),
            width,
            height,
            clipboard,
            status_message: None,
            search_query: String::new(),
            search_results: Vec::new(),
            search_current_index: 0,
            searching: false,
            scroll_x: 0,
            scroll_y: 0,
        }
    }
    
    /// Set viewport dimensions (for ensuring cursor stays visible)
    pub fn set_viewport_size(&mut self, viewport_width: usize, viewport_height: usize) {
        self.ensure_cursor_visible(viewport_width, viewport_height);
    }
    
    /// Create grid from pdftotext output
    pub fn from_pdftext(text: &str, min_width: usize) -> Self {
        let lines: Vec<String> = text.lines().map(|s| s.to_string()).collect();
        let height = lines.len().max(24);
        
        // Find the maximum line length to determine grid width
        let max_line_length = lines.iter()
            .map(|line| line.chars().count())
            .max()
            .unwrap_or(0);
        
        // Use the larger of min_width or the longest line
        let actual_width = max_line_length.max(min_width);
        
        let mut cells = Vec::with_capacity(height);
        for line in lines.iter() {
            let mut row: Vec<char> = line.chars().collect();
            // Pad with spaces if shorter than grid width
            row.resize(actual_width, ' ');
            cells.push(row);
        }
        
        // Ensure minimum height
        while cells.len() < height {
            cells.push(vec![' '; actual_width]);
        }
        
        let grid_height = cells.len();
        let clipboard = Clipboard::new().ok().map(|c| Arc::new(Mutex::new(c)));
        
        Self {
            cells,
            cursor: (0, 0),
            selecting: false,
            anchor: (0, 0),
            width: actual_width,
            height: grid_height,
            clipboard,
            status_message: None,
            search_query: String::new(),
            search_results: Vec::new(),
            search_current_index: 0,
            searching: false,
            scroll_x: 0,
            scroll_y: 0,
        }
    }
    
    /// Ensure cursor is visible in the viewport
    pub fn ensure_cursor_visible(&mut self, viewport_width: usize, viewport_height: usize) {
        // Horizontal scrolling to keep cursor in view
        if self.cursor.0 < self.scroll_x {
            self.scroll_x = self.cursor.0;
        } else if self.cursor.0 >= self.scroll_x + viewport_width {
            self.scroll_x = self.cursor.0.saturating_sub(viewport_width - 1);
        }
        
        // Vertical scrolling to keep cursor in view
        if self.cursor.1 < self.scroll_y {
            self.scroll_y = self.cursor.1;
        } else if self.cursor.1 >= self.scroll_y + viewport_height {
            self.scroll_y = self.cursor.1.saturating_sub(viewport_height - 1);
        }
    }
    
    /// Handle keyboard input with modifiers (with viewport tracking)
    pub fn handle_key_with_modifiers_with_viewport(&mut self, key: KeyCode, shift: bool, ctrl: bool, _alt: bool, viewport_width: usize, viewport_height: usize) {
        self.handle_key_with_modifiers(key, shift, ctrl, _alt);
        // Ensure cursor stays visible after any key input
        self.ensure_cursor_visible(viewport_width, viewport_height);
    }
    
    /// Handle keyboard input with modifiers
    pub fn handle_key_with_modifiers(&mut self, key: KeyCode, shift: bool, ctrl: bool, _alt: bool) {
        // Handle search mode input first
        if self.searching {
            match key {
                KeyCode::Esc => {
                    self.exit_search();
                }
                KeyCode::Enter => {
                    if shift {
                        self.find_previous();
                    } else {
                        self.find_next();
                    }
                }
                KeyCode::Backspace => {
                    self.search_query.pop();
                    self.perform_search();
                }
                KeyCode::Char('n') if ctrl && shift => {
                    self.find_previous();
                }
                KeyCode::Char('n') if ctrl => {
                    self.find_next();
                }
                KeyCode::Char(c) if !ctrl => {
                    self.search_query.push(c);
                    self.perform_search();
                }
                _ => {}
            }
            return;
        }
        
        match key {
            // Search functionality
            KeyCode::Char('f') if ctrl => {
                self.start_search();
            }
            
            KeyCode::F(3) => {
                if !self.searching {
                    self.start_search();
                } else {
                    self.find_next();
                }
            }
            
            KeyCode::Char('n') if ctrl => {
                self.find_next();
            }
            
            // Basic cut/copy/paste only
            KeyCode::Char('c') if ctrl => {
                self.copy_to_clipboard();
            }
            
            KeyCode::Char('x') if ctrl => {
                self.cut_to_clipboard();
            }
            
            KeyCode::Char('v') if ctrl => {
                self.paste_from_clipboard();
            }
            
            // Simple arrow key movement
            KeyCode::Up => {
                if shift && !self.selecting {
                    self.selecting = true;
                    self.anchor = self.cursor;
                    self.status_message = Some(format!("Selection started at ({},{})", self.anchor.0, self.anchor.1));
                }
                self.cursor.1 = self.cursor.1.saturating_sub(1);
                if shift && self.selecting {
                    let (x1, y1, x2, y2) = self.get_selection_bounds();
                    self.status_message = Some(format!("Selecting ({},{}) to ({},{})", x1, y1, x2, y2));
                } else if !shift {
                    self.selecting = false;
                }
            }
            KeyCode::Down => {
                if shift && !self.selecting {
                    self.selecting = true;
                    self.anchor = self.cursor;
                    self.status_message = Some(format!("Selection started at ({},{})", self.anchor.0, self.anchor.1));
                }
                self.cursor.1 = (self.cursor.1 + 1).min(self.height - 1);
                if shift && self.selecting {
                    let (x1, y1, x2, y2) = self.get_selection_bounds();
                    self.status_message = Some(format!("Selecting ({},{}) to ({},{})", x1, y1, x2, y2));
                } else if !shift {
                    self.selecting = false;
                }
            }
            KeyCode::Left => {
                if shift && !self.selecting {
                    self.selecting = true;
                    self.anchor = self.cursor;
                    self.status_message = Some(format!("Selection started at ({},{})", self.anchor.0, self.anchor.1));
                }
                self.cursor.0 = self.cursor.0.saturating_sub(1);
                if shift && self.selecting {
                    let (x1, y1, x2, y2) = self.get_selection_bounds();
                    self.status_message = Some(format!("Selecting ({},{}) to ({},{})", x1, y1, x2, y2));
                } else if !shift {
                    self.selecting = false;
                }
            }
            KeyCode::Right => {
                if shift && !self.selecting {
                    self.selecting = true;
                    self.anchor = self.cursor;
                    self.status_message = Some(format!("Selection started at ({},{})", self.anchor.0, self.anchor.1));
                }
                self.cursor.0 = (self.cursor.0 + 1).min(self.width - 1);
                if shift && self.selecting {
                    let (x1, y1, x2, y2) = self.get_selection_bounds();
                    self.status_message = Some(format!("Selecting ({},{}) to ({},{})", x1, y1, x2, y2));
                } else if !shift {
                    self.selecting = false;
                }
            }
            
            // Escape cancels selection
            KeyCode::Esc => {
                self.selecting = false;
                self.status_message = Some("Selection cancelled".to_string());
            }
            
            // Delete/Backspace
            KeyCode::Delete => {
                if self.selecting {
                    self.clear_selection();
                } else {
                    self.cells[self.cursor.1][self.cursor.0] = ' ';
                }
            }
            KeyCode::Backspace => {
                if self.selecting {
                    self.clear_selection();
                } else if self.cursor.0 > 0 {
                    self.cursor.0 -= 1;
                    self.cells[self.cursor.1][self.cursor.0] = ' ';
                }
            }
            
            // Character input - handle both ctrl shortcuts and regular typing
            KeyCode::Char(c) if !ctrl => {
                // Regular typing (not a ctrl shortcut)
                if self.selecting {
                    // Replace all characters in selected block
                    self.for_each_in_selection(|cell| *cell = c);
                    self.selecting = false;
                    self.status_message = Some(format!("Replaced selection with '{}'", c));
                } else {
                    // Type at cursor
                    if self.cursor.1 < self.cells.len() && self.cursor.0 < self.cells[self.cursor.1].len() {
                        self.cells[self.cursor.1][self.cursor.0] = c;
                        // Move cursor right after typing
                        self.cursor.0 = (self.cursor.0 + 1).min(self.width - 1);
                    }
                }
            }
            
            // Home/End for line navigation
            KeyCode::Home => {
                self.cursor.0 = 0;
            }
            KeyCode::End => {
                // Find last non-space character in current line
                if self.cursor.1 < self.cells.len() {
                    let line = &self.cells[self.cursor.1];
                    self.cursor.0 = line.iter()
                        .rposition(|&c| c != ' ')
                        .map(|i| (i + 1).min(self.width - 1))
                        .unwrap_or(0);
                }
            }
            
            // Enter key - move to next line
            KeyCode::Enter => {
                self.cursor.1 = (self.cursor.1 + 1).min(self.height - 1);
                self.cursor.0 = 0;
            }
            
            // Page Up/Down
            KeyCode::PageUp => {
                self.cursor.1 = self.cursor.1.saturating_sub(10);
            }
            KeyCode::PageDown => {
                self.cursor.1 = (self.cursor.1 + 10).min(self.height - 1);
            }
            
            _ => {}
        }
    }
    
    /// Apply a function to each cell in the current selection
    fn for_each_in_selection<F>(&mut self, f: F) 
    where F: Fn(&mut char)
    {
        let (x1, y1, x2, y2) = self.get_selection_bounds();
        for y in y1..=y2.min(self.cells.len().saturating_sub(1)) {
            for x in x1..=x2.min(self.cells.get(y).map(|row| row.len().saturating_sub(1)).unwrap_or(0)) {
                f(&mut self.cells[y][x]);
            }
        }
    }
    
    /// Clear the selected block
    pub fn clear_selection(&mut self) {
        self.for_each_in_selection(|c| *c = ' ');
        self.selecting = false;
    }
    
    /// Get normalized selection bounds (x1, y1, x2, y2)
    pub fn get_selection_bounds(&self) -> (usize, usize, usize, usize) {
        (
            min(self.anchor.0, self.cursor.0),
            min(self.anchor.1, self.cursor.1),
            max(self.anchor.0, self.cursor.0),
            max(self.anchor.1, self.cursor.1),
        )
    }
    
    /// Check if a position is within the selection
    pub fn is_selected(&self, x: usize, y: usize) -> bool {
        if !self.selecting {
            return false;
        }
        let (x1, y1, x2, y2) = self.get_selection_bounds();
        let selected = x >= x1 && x <= x2 && y >= y1 && y <= y2;
        selected
    }
    
    /// Get the content as a string
    pub fn to_string(&self) -> String {
        self.cells
            .iter()
            .map(|row| row.iter().collect::<String>().trim_end().to_string())
            .collect::<Vec<_>>()
            .join("\n")
    }
    
    /// Handle keyboard input (backward compatibility wrapper)
    pub fn handle_key(&mut self, key: KeyCode, shift_held: bool) {
        self.handle_key_with_modifiers(key, shift_held, false, false);
    }
    
    /// Helper to execute clipboard operations with proper error handling
    fn with_clipboard<F, T>(&mut self, operation: &str, f: F) -> Option<T>
    where 
        F: FnOnce(&mut Clipboard) -> Result<T, Box<dyn std::error::Error>>
    {
        match &self.clipboard {
            Some(clipboard) => match clipboard.lock() {
                Ok(mut clip) => match f(&mut *clip) {
                    Ok(result) => Some(result),
                    Err(e) => {
                        self.status_message = Some(format!("{} FAILED: {}", operation, e));
                        None
                    }
                },
                Err(_) => {
                    self.status_message = Some("ERROR: Clipboard locked".to_string());
                    None
                }
            },
            None => {
                self.status_message = Some("ERROR: No clipboard".to_string());
                None
            }
        }
    }
    
    /// Copy selection to system clipboard
    pub fn copy_to_clipboard(&mut self) {
        if !self.selecting {
            self.status_message = Some("ERROR: No selection to copy".to_string());
            return;
        }
        
        let text = self.copy_selection();
        let text_len = text.len();
        
        self.with_clipboard("COPY", |clip| {
            clip.set_text(&text)
                .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
        }).map(|_| {
            self.status_message = Some(format!("COPIED {} chars", text_len));
        });
    }
    
    /// Cut selection to clipboard
    pub fn cut_to_clipboard(&mut self) {
        if !self.selecting {
            self.status_message = Some("ERROR: No selection to cut".to_string());
            return;
        }
        
        let text = self.copy_selection();
        let chars_count = text.len();
        
        self.with_clipboard("CUT", |clip| {
            clip.set_text(&text)
                .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
        }).map(|_| {
            self.clear_selection();
            self.status_message = Some(format!("CUT: {} chars removed", chars_count));
        });
    }
    
    /// Paste from system clipboard
    pub fn paste_from_clipboard(&mut self) {
        self.with_clipboard("PASTE", |clip| {
            clip.get_text()
                .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
        }).map(|text| {
            let text_len = text.len();
            self.paste_text(&text);
            self.status_message = Some(format!("PASTED {} chars", text_len));
        }).or_else(|| {
            self.status_message = Some("CLIPBOARD EMPTY".to_string());
            None
        });
    }
    
    /// Get current status message
    pub fn get_status_message(&self) -> Option<&str> {
        self.status_message.as_deref()
    }
    
    /// Handle mouse events for selection
    pub fn handle_mouse_down(&mut self, col: usize, row: usize) {
        // Start selection at clicked position
        self.cursor = (col.min(self.width - 1), row.min(self.height - 1));
        self.anchor = self.cursor;
        self.selecting = false;  // Will become true on drag
        self.status_message = Some(format!("POS: {},{}", col + 1, row + 1));
    }
    
    /// Handle mouse down with viewport adjustment
    pub fn handle_mouse_down_with_viewport(&mut self, col: usize, row: usize, viewport_width: usize, viewport_height: usize) {
        self.handle_mouse_down(col, row);
        self.ensure_cursor_visible(viewport_width, viewport_height);
    }
    
    /// Handle mouse drag for selection
    pub fn handle_mouse_drag(&mut self, col: usize, row: usize) {
        // Update cursor position and enable selection
        self.cursor = (col.min(self.width - 1), row.min(self.height - 1));
        if !self.selecting && (self.cursor != self.anchor) {
            self.selecting = true;
        }
        
        if self.selecting {
            let (x1, y1, x2, y2) = self.get_selection_bounds();
            self.status_message = Some(format!("SELECTING: {},{} to {},{}", x1 + 1, y1 + 1, x2 + 1, y2 + 1));
        }
    }
    
    /// Handle mouse release
    pub fn handle_mouse_up(&mut self, col: usize, row: usize) {
        self.cursor = (col.min(self.width - 1), row.min(self.height - 1));
        if self.selecting {
            let (x1, y1, x2, y2) = self.get_selection_bounds();
            self.status_message = Some(format!("SELECTED: {},{} to {},{}", x1 + 1, y1 + 1, x2 + 1, y2 + 1));
        }
    }
    
    /// Handle mouse up with viewport adjustment
    pub fn handle_mouse_up_with_viewport(&mut self, col: usize, row: usize, viewport_width: usize, viewport_height: usize) {
        self.handle_mouse_up(col, row);
        self.ensure_cursor_visible(viewport_width, viewport_height);
    }
    
    /// Clear status message
    pub fn clear_status_message(&mut self) {
        self.status_message = None;
    }
    
    // Search functionality methods
    
    /// Start search mode
    pub fn start_search(&mut self) {
        self.searching = true;
        self.search_query.clear();
        self.search_results.clear();
        self.search_current_index = 0;
        self.status_message = Some("🔍 SEARCH: Type to search, Enter/Shift+Enter to navigate, ESC to exit".to_string());
    }
    
    /// Exit search mode
    pub fn exit_search(&mut self) {
        self.searching = false;
        self.search_results.clear();
        self.status_message = Some("Search cancelled".to_string());
    }
    
    /// Perform search and find all matches (with fuzzy search support)
    pub fn perform_search(&mut self) {
        if self.search_query.is_empty() {
            self.search_results.clear();
            self.status_message = Some("SEARCH: Type to search".to_string());
            return;
        }
        
        self.search_results.clear();
        
        // Try exact match first for better performance on simple searches
        let query_lower = self.search_query.to_lowercase();
        let mut found_exact = false;
        
        // Search through all cells for exact matches
        for (row_idx, row) in self.cells.iter().enumerate() {
            let row_text: String = row.iter().collect::<String>().to_lowercase();
            
            // Find all occurrences in this row
            let mut start = 0;
            while let Some(pos) = row_text[start..].find(&query_lower) {
                let actual_pos = start + pos;
                self.search_results.push((actual_pos, row_idx));
                found_exact = true;
                start = actual_pos + 1;
            }
        }
        
        // If no exact matches, try fuzzy search with Nucleo
        if !found_exact && self.search_query.len() >= 2 {
            // Create a temporary Nucleo matcher for fuzzy search
            let mut nucleo = Nucleo::<(usize, usize, String)>::new(
                NucleoConfig::DEFAULT,
                Arc::new(|| {}),
                None,
                1,
            );
            
            let injector = nucleo.injector();
            
            // Add all text positions as searchable items
            for (row_idx, row) in self.cells.iter().enumerate() {
                let row_text: String = row.iter().collect();
                // Break row into words for better fuzzy matching
                for word in row_text.split_whitespace() {
                    if let Some(word_start) = row_text.find(word) {
                        injector.push((word_start, row_idx, word.to_string()), |_, cols| {
                            cols[0] = word.clone().into();
                        });
                    }
                }
            }
            
            // Run the fuzzy search
            nucleo.tick(10);
            nucleo.pattern.reparse(
                0,
                &self.search_query,  // Pass the query string directly
                nucleo::pattern::CaseMatching::Ignore,
                nucleo::pattern::Normalization::Smart,
                false,
            );
            nucleo.tick(10);
            
            // Get fuzzy matches
            let snapshot = nucleo.snapshot();
            for item in snapshot.matched_items(..).take(20) {
                let (col, row, _) = &item.data;
                self.search_results.push((*col, *row));
            }
        }
        
        if !self.search_results.is_empty() {
            self.search_current_index = 0;
            let (col, row) = self.search_results[0];
            self.cursor = (col, row);
            let match_type = if found_exact { "exact" } else { "fuzzy" };
            self.status_message = Some(format!("SEARCH: '{}' - {} {} matches (Enter for next)", 
                                               self.search_query,
                                               self.search_results.len(),
                                               match_type));
        } else {
            self.status_message = Some(format!("SEARCH: '{}' - No matches found", self.search_query));
        }
    }
    
    /// Find next search result
    pub fn find_next(&mut self) {
        if self.search_results.is_empty() {
            self.status_message = Some("No search results".to_string());
            return;
        }
        
        self.search_current_index = (self.search_current_index + 1) % self.search_results.len();
        let (col, row) = self.search_results[self.search_current_index];
        self.cursor = (col, row);
        self.status_message = Some(format!("SEARCH: Match {}/{}", 
                                           self.search_current_index + 1,
                                           self.search_results.len()));
    }
    
    /// Find previous search result
    pub fn find_previous(&mut self) {
        if self.search_results.is_empty() {
            self.status_message = Some("No search results".to_string());
            return;
        }
        
        if self.search_current_index == 0 {
            self.search_current_index = self.search_results.len() - 1;
        } else {
            self.search_current_index -= 1;
        }
        
        let (col, row) = self.search_results[self.search_current_index];
        self.cursor = (col, row);
        self.status_message = Some(format!("SEARCH: Match {}/{}", 
                                           self.search_current_index + 1,
                                           self.search_results.len()));
    }
    
    /// Check if a position matches current search
    pub fn is_search_match(&self, col: usize, row: usize) -> bool {
        if !self.searching || self.search_query.is_empty() {
            return false;
        }
        
        // Check if this position is part of any search result
        for &(result_col, result_row) in &self.search_results {
            if result_row == row && 
               col >= result_col && 
               col < result_col + self.search_query.len() {
                return true;
            }
        }
        false
    }
    
    /// Insert text at cursor position
    pub fn insert_text(&mut self, text: &str) {
        for ch in text.chars() {
            if ch == '\n' {
                // Move to next line
                self.cursor.1 = (self.cursor.1 + 1).min(self.height - 1);
                self.cursor.0 = 0;
            } else if ch == '\t' {
                // Tab moves 4 spaces
                self.cursor.0 = (self.cursor.0 + 4).min(self.width - 1);
            } else {
                // Insert character
                if self.cursor.1 < self.cells.len() && self.cursor.0 < self.cells[self.cursor.1].len() {
                    self.cells[self.cursor.1][self.cursor.0] = ch;
                    self.cursor.0 = (self.cursor.0 + 1).min(self.width - 1);
                }
            }
        }
    }
    
    /// Copy selected text to string
    pub fn copy_selection(&self) -> String {
        if !self.selecting {
            return String::new();
        }
        
        let (x1, y1, x2, y2) = self.get_selection_bounds();
        let mut result = Vec::new();
        
        for y in y1..=y2 {
            let mut line = String::new();
            for x in x1..=x2 {
                if y < self.cells.len() && x < self.cells[y].len() {
                    line.push(self.cells[y][x]);
                }
            }
            // Don't trim if it's all spaces - might be intentional
            result.push(line);
        }
        
        result.join("\n")
    }
    
    /// Replace selection with text
    pub fn paste_text(&mut self, text: &str) {
        if self.selecting {
            self.clear_selection();
        }
        self.insert_text(text);
    }
}