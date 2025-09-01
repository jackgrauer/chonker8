// Simple file browser with PDF highlighting
use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    execute,
    style::{Color, Print, ResetColor, SetBackgroundColor, SetForegroundColor},
    terminal::{self, Clear, ClearType},
};
use std::{
    fs,
    io::{stdout, Write},
    path::PathBuf,
};

pub struct FileBrowser {
    files: Vec<String>,
    current_dir: PathBuf,
    query: String,
    selected_index: usize,
    max_visible: usize,
    scroll_offset: usize,
}

pub enum FileBrowserResult {
    Exit,
    FileSelected(PathBuf),
    SwitchToEditor,
    HotReload,
}

impl FileBrowser {
    pub fn new() -> Result<Self> {
        let current_dir = PathBuf::from("/Users/jack/Documents");
        let mut browser = Self {
            files: Vec::new(),
            current_dir,
            query: String::new(),
            selected_index: 0,
            max_visible: 20,
            scroll_offset: 0,
        };
        
        browser.scan_directory()?;
        Ok(browser)
    }
    
    fn get_file_color(&self, filename: &str) -> Color {
        if filename.ends_with('/') {
            if filename == "../" {
                Color::DarkGrey  // Parent directory
            } else {
                Color::Cyan      // Regular directory
            }
        } else if filename.to_lowercase().ends_with(".pdf") {
            Color::Green         // PDF files - highlighted in green
        } else {
            Color::White         // Other files
        }
    }
    
    fn scan_directory(&mut self) -> Result<()> {
        self.files.clear();
        
        // Add parent directory if not at root
        if self.current_dir.parent().is_some() {
            self.files.push("../".to_string());
        }
        
        // Read directory entries
        if let Ok(entries) = fs::read_dir(&self.current_dir) {
            let mut dirs = Vec::new();
            let mut files = Vec::new();
            
            for entry in entries.flatten() {
                let file_name = entry.file_name();
                let file_str = file_name.to_string_lossy().to_string();
                
                // Skip hidden files
                if file_str.starts_with('.') && file_str != ".." {
                    continue;
                }
                
                if entry.path().is_dir() {
                    dirs.push(format!("{}/", file_str));
                } else if file_str.to_lowercase().ends_with(".pdf") {
                    files.push(file_str);
                }
            }
            
            // Sort and combine
            dirs.sort();
            files.sort();
            self.files.extend(dirs);
            self.files.extend(files);
        }
        
        self.selected_index = 0;
        self.scroll_offset = 0;
        Ok(())
    }
    
    pub fn run(&mut self) -> Result<FileBrowserResult> {
        self.render()?;
        
        if let Event::Key(key) = event::read()? {
            match self.handle_key(key)? {
                KeyResult::Continue => Ok(FileBrowserResult::FileSelected(PathBuf::new())), // Will loop back
                KeyResult::Exit => Ok(FileBrowserResult::Exit),
                KeyResult::Select(path) => Ok(FileBrowserResult::FileSelected(path)),
                KeyResult::SwitchToEditor => Ok(FileBrowserResult::SwitchToEditor),
                KeyResult::HotReload => Ok(FileBrowserResult::HotReload),
            }
        } else {
            Ok(FileBrowserResult::FileSelected(PathBuf::new())) // Will loop back
        }
    }
    
    fn handle_key(&mut self, key: KeyEvent) -> Result<KeyResult> {
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => {
                return Ok(KeyResult::Exit);
            }
            KeyCode::Tab => {
                return Ok(KeyResult::SwitchToEditor);
            }
            KeyCode::Enter => {
                if let Some(selected_file) = self.get_selected_file() {
                    let full_path = self.current_dir.join(&selected_file);
                    
                    if selected_file == "../" {
                        // Go to parent directory
                        if let Some(parent) = self.current_dir.parent() {
                            self.current_dir = parent.to_path_buf();
                            self.scan_directory()?;
                        }
                    } else if selected_file.ends_with('/') {
                        // Enter directory
                        let dir_name = &selected_file[..selected_file.len()-1];
                        self.current_dir = self.current_dir.join(dir_name);
                        self.scan_directory()?;
                    } else {
                        // Select file
                        return Ok(KeyResult::Select(full_path));
                    }
                }
            }
            KeyCode::Up => {
                if self.selected_index > 0 {
                    self.selected_index -= 1;
                    self.update_scroll();
                }
            }
            KeyCode::Down => {
                let visible_count = self.get_visible_files().len();
                if self.selected_index + 1 < visible_count {
                    self.selected_index += 1;
                    self.update_scroll();
                }
            }
            KeyCode::Char(c) => {
                if key.modifiers.contains(KeyModifiers::CONTROL) {
                    match c {
                        'c' => return Ok(KeyResult::Exit),
                        'u' => {
                            // Debug: write to a file to confirm key is detected
                            std::fs::write("/tmp/chonker8_debug.txt", "Ctrl+U detected in file browser").ok();
                            return Ok(KeyResult::HotReload);
                        }
                        _ => {}
                    }
                } else {
                    self.query.push(c);
                    self.update_selection();
                }
            }
            KeyCode::Backspace => {
                if !self.query.is_empty() {
                    self.query.pop();
                    self.update_selection();
                }
            }
            _ => {}
        }
        
        Ok(KeyResult::Continue)
    }
    
    fn update_selection(&mut self) {
        self.selected_index = 0;
        self.scroll_offset = 0;
    }
    
    fn get_visible_files(&self) -> Vec<String> {
        if self.query.is_empty() {
            // Show all files
            self.files.clone()
        } else {
            // Simple string matching (case insensitive)
            let query_lower = self.query.to_lowercase();
            self.files
                .iter()
                .filter(|f| f.to_lowercase().contains(&query_lower))
                .cloned()
                .collect()
        }
    }
    
    fn get_selected_file(&self) -> Option<String> {
        let visible = self.get_visible_files();
        visible.get(self.selected_index).cloned()
    }
    
    fn update_scroll(&mut self) {
        if self.selected_index < self.scroll_offset {
            self.scroll_offset = self.selected_index;
        } else if self.selected_index >= self.scroll_offset + self.max_visible {
            self.scroll_offset = self.selected_index - self.max_visible + 1;
        }
    }
    
    fn render(&mut self) -> Result<()> {
        let (width, height) = terminal::size()?;
        self.max_visible = (height as usize).saturating_sub(1); // Leave room for search line
        
        execute!(stdout(), Clear(ClearType::All))?;
        
        // Search input line at top with blinking cursor
        execute!(
            stdout(),
            crossterm::cursor::MoveTo(0, 0),
            Print(format!("{}█", self.query)) // Add blinking cursor block
        )?;
        
        // File list starting from line 1
        let visible_files = self.get_visible_files();
        
        for (i, file) in visible_files
            .iter()
            .skip(self.scroll_offset)
            .take(self.max_visible)
            .enumerate()
        {
            let file_index = self.scroll_offset + i;
            let y = (i + 1) as u16; // Start from line 1
            
            let file_color = self.get_file_color(file);
            
            if file_index == self.selected_index {
                execute!(
                    stdout(),
                    crossterm::cursor::MoveTo(0, y),
                    SetBackgroundColor(Color::Blue),
                    SetForegroundColor(Color::White),
                    Print(format!("{:<width$}", file, width = width as usize)),
                    ResetColor
                )?;
            } else {
                execute!(
                    stdout(),
                    crossterm::cursor::MoveTo(0, y),
                    SetForegroundColor(file_color),
                    Print(file),
                    ResetColor
                )?;
            }
        }
        
        stdout().flush()?;
        Ok(())
    }
}

enum KeyResult {
    Continue,
    Exit,
    Select(PathBuf),
    SwitchToEditor,
    HotReload,
}

// Re-import cursor for the render function
use crossterm::cursor;