// Split-screen file picker with live PDF thumbnail preview
use anyhow::Result;
use crossterm::{
    cursor::MoveTo,
    event::{self, Event, KeyCode, MouseButton, MouseEventKind},
    execute,
    style::{Color, Print, ResetColor, SetBackgroundColor, SetForegroundColor},
    terminal::{self, Clear, ClearType},
};
use std::io::{self, Write};
use std::path::PathBuf;
use std::process::Command;
use image::DynamicImage;

use crate::{
    theme_const::*,
    pdf_renderer,
    viuer_display,
    debug_log,
};

use std::collections::HashMap;

#[derive(Debug)]
pub enum FilePickerResult {
    None,           // No action
    Redraw,         // Selection changed, needs redraw
    Selected(PathBuf), // File selected
}

pub struct FilePickerScreen {
    files: Vec<PathBuf>,
    filtered_files: Vec<usize>,  // Indices into files vec
    selected_index: usize,
    scroll_offset: usize,
    query: String,
    current_thumbnail: Option<DynamicImage>,
    last_thumbnail_path: Option<PathBuf>,
    thumbnail_cache: HashMap<PathBuf, Option<DynamicImage>>,  // Cache to prevent re-loading
    last_render_time: std::time::Instant,
    pending_thumbnail: Option<PathBuf>,  // Track if we're loading a thumbnail
}

impl FilePickerScreen {
    pub fn new() -> Result<Self> {
        let files = find_pdf_files()?;
        let filtered_files: Vec<usize> = (0..files.len()).collect();
        
        Ok(Self {
            files,
            filtered_files,
            selected_index: 0,
            scroll_offset: 0,
            query: String::new(),
            current_thumbnail: None,
            last_thumbnail_path: None,
            thumbnail_cache: HashMap::new(),
            last_render_time: std::time::Instant::now(),
            pending_thumbnail: None,
        })
    }
    
    pub fn render(&mut self, term_width: u16, term_height: u16) -> Result<()> {
        let mut stdout = io::stdout();
        
        // Calculate split position - 60% for file list, 40% for preview
        let split_x = (term_width * 3) / 5;
        
        // Draw soft pink header bar (full width)
        execute!(stdout, MoveTo(0, 0))?;
        execute!(stdout, SetBackgroundColor(HEADER_PDF))?;
        execute!(stdout, SetForegroundColor(Color::Black))?;
        write!(stdout, "{:^width$}", " ● FILE PICKER ", width = term_width as usize)?;
        execute!(stdout, ResetColor)?;
        
        // Draw search box on left side
        execute!(
            stdout,
            MoveTo(2, 2),
            SetForegroundColor(ACCENT_TEXT),
            Print("Search: "),
            SetForegroundColor(TEXT_PRIMARY),
            Print(&self.query),
            SetForegroundColor(TEXT_DIM),
            Print("_"),
            ResetColor
        )?;
        
        // Draw divider line
        for y in 1..term_height - 1 {
            execute!(
                stdout,
                MoveTo(split_x, y),
                SetForegroundColor(BORDER),
                Print("│"),
                ResetColor
            )?;
        }
        
        // Render file list on left
        self.render_file_list(2, 4, split_x - 3, term_height - 6)?;
        
        // Render thumbnail on right
        self.render_thumbnail(split_x + 2, 2, term_width - split_x - 3, term_height - 4)?;
        
        // Help text at bottom
        execute!(
            stdout,
            MoveTo(2, term_height - 2),
            SetForegroundColor(TEXT_DIM),
            Print("↑/↓ Navigate • Enter Select • Tab Switch Screen • Esc Cancel"),
            ResetColor
        )?;
        
        stdout.flush()?;
        Ok(())
    }
    
    fn render_file_list(&mut self, x: u16, y: u16, width: u16, height: u16) -> Result<()> {
        let mut stdout = io::stdout();
        let max_items = height as usize;
        
        // Update scroll to keep selection visible
        if self.selected_index >= self.scroll_offset + max_items {
            self.scroll_offset = self.selected_index.saturating_sub(max_items - 1);
        } else if self.selected_index < self.scroll_offset {
            self.scroll_offset = self.selected_index;
        }
        
        // Get visible files
        let visible_files: Vec<usize> = self.filtered_files
            .iter()
            .skip(self.scroll_offset)
            .take(max_items)
            .copied()
            .collect();
        
        for (i, file_idx) in visible_files.iter().enumerate() {
            let actual_index = self.scroll_offset + i;
            let file_path: &PathBuf = &self.files[*file_idx];
            
            // Clean up path for display
            let display_name = file_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown.pdf");
            
            // Truncate if needed
            let display_str: String = if display_name.len() > width as usize - 4 {
                format!("{}...", &display_name[..width as usize - 7])
            } else {
                display_name.to_string()
            };
            
            execute!(stdout, MoveTo(x, y + i as u16))?;
            
            if actual_index == self.selected_index {
                execute!(
                    stdout,
                    SetBackgroundColor(SELECTION_BG),
                    SetForegroundColor(SELECTION_FG),
                    Print(format!(" ▶ {:<width$}", display_str, width = width as usize - 3)),
                    ResetColor
                )?;
            } else {
                execute!(
                    stdout,
                    Print(format!("   {:<width$}", display_str, width = width as usize - 3))
                )?;
            }
        }
        
        // Clear remaining lines
        for i in visible_files.len()..max_items {
            execute!(
                stdout,
                MoveTo(x, y + i as u16),
                Clear(ClearType::UntilNewLine)
            )?;
        }
        
        // Scroll indicator
        if self.files.len() > max_items {
            let indicator = format!("{}/{}", 
                self.selected_index + 1, 
                self.filtered_files.len()
            );
            execute!(
                stdout,
                MoveTo(x + width - indicator.len() as u16 - 2, y + height - 1),
                SetForegroundColor(TEXT_DIM),
                Print(&indicator),
                ResetColor
            )?;
        }
        
        Ok(())
    }
    
    fn render_thumbnail(&mut self, x: u16, y: u16, width: u16, height: u16) -> Result<()> {
        let mut stdout = io::stdout();
        
        // Get currently selected file
        if self.filtered_files.is_empty() {
            execute!(
                stdout,
                MoveTo(x + width/2 - 6, y + height/2),
                SetForegroundColor(TEXT_DIM),
                Print("No PDF selected"),
                ResetColor
            )?;
            return Ok(());
        }
        
        let file_idx = self.filtered_files[self.selected_index];
        let file_path = &self.files[file_idx];
        
        // Check if we need to load a new thumbnail
        let needs_load = self.last_thumbnail_path.as_ref() != Some(file_path) &&
                        self.pending_thumbnail.as_ref() != Some(file_path);
        
        if needs_load {
            // Check cache first
            if let Some(cached) = self.thumbnail_cache.get(file_path) {
                debug_log(format!("Using cached thumbnail for: {:?}", file_path));
                self.current_thumbnail = cached.clone();
                self.last_thumbnail_path = Some(file_path.clone());
            } else {
                // Load immediately - no debouncing for better responsiveness
                debug_log(format!("Loading new thumbnail for: {:?}", file_path));
                self.pending_thumbnail = Some(file_path.clone());
                
                // Try to render first page as thumbnail
                match pdf_renderer::render_pdf_page(file_path, 0, 400, 600) {
                    Ok(image) => {
                        // Clear old graphics only when new one is ready
                        let _ = viuer_display::clear_graphics();
                        
                        // Cache the result
                        self.thumbnail_cache.insert(file_path.clone(), Some(image.clone()));
                        self.current_thumbnail = Some(image);
                        self.last_thumbnail_path = Some(file_path.clone());
                        self.pending_thumbnail = None;
                        debug_log("Thumbnail loaded and cached successfully");
                    }
                    Err(e) => {
                        debug_log(format!("Failed to load thumbnail: {}", e));
                        // Cache the failure to avoid retrying
                        self.thumbnail_cache.insert(file_path.clone(), None);
                        self.current_thumbnail = None;
                        self.last_thumbnail_path = Some(file_path.clone());
                        self.pending_thumbnail = None;
                    }
                }
            }
        }
        
        // Display thumbnail or error message
        if let Some(image) = &self.current_thumbnail {
            // Display the thumbnail image
            let _ = viuer_display::display_pdf_image(
                image,
                x,
                y + 1,
                width,
                height - 2,
                true  // dark mode
            );
            
            // Show filename below
            let filename = file_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown.pdf");
            
            execute!(
                stdout,
                MoveTo(x, y + height - 1),
                SetForegroundColor(TEXT_SECONDARY),
                Print(format!("{:^width$}", filename, width = width as usize)),
                ResetColor
            )?;
        } else {
            // Show placeholder/error with file info
            let filename = file_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown.pdf");
            
            // Draw a simple PDF icon placeholder
            let mid_y = y + height / 2;
            execute!(
                stdout,
                MoveTo(x + width/2 - 4, mid_y - 2),
                SetForegroundColor(TEXT_DIM),
                Print("┌─────┐"),
                MoveTo(x + width/2 - 4, mid_y - 1),
                Print("│ PDF │"),
                MoveTo(x + width/2 - 4, mid_y),
                Print("│     │"),
                MoveTo(x + width/2 - 4, mid_y + 1),
                Print("└─────┘"),
                ResetColor
            )?;
            
            // Show filename
            execute!(
                stdout,
                MoveTo(x, y + height - 3),
                SetForegroundColor(TEXT_SECONDARY),
                Print(format!("{:^width$}", filename, width = width as usize)),
                ResetColor
            )?;
            
            // Show loading or error status
            let status = if self.pending_thumbnail.as_ref() == Some(file_path) {
                "Loading..."
            } else {
                "Preview unavailable"
            };
            
            execute!(
                stdout,
                MoveTo(x, y + height - 1),
                SetForegroundColor(TEXT_DIM),
                Print(format!("{:^width$}", status, width = width as usize)),
                ResetColor
            )?;
        }
        
        Ok(())
    }
    
    pub fn handle_input(&mut self, event: Event) -> FilePickerResult {
        match event {
            Event::Key(key) => {
                match key.code {
                    KeyCode::Enter => {
                        if !self.filtered_files.is_empty() {
                            let file_idx = self.filtered_files[self.selected_index];
                            return FilePickerResult::Selected(self.files[file_idx].clone());
                        }
                    }
                    KeyCode::Up => {
                        if self.selected_index > 0 {
                            self.selected_index -= 1;
                            return FilePickerResult::Redraw;
                        }
                    }
                    KeyCode::Down => {
                        if self.selected_index < self.filtered_files.len().saturating_sub(1) {
                            self.selected_index += 1;
                            return FilePickerResult::Redraw;
                        }
                    }
                    KeyCode::PageUp => {
                        self.selected_index = self.selected_index.saturating_sub(10);
                        return FilePickerResult::Redraw;
                    }
                    KeyCode::PageDown => {
                        let max = self.filtered_files.len().saturating_sub(1);
                        self.selected_index = (self.selected_index + 10).min(max);
                        return FilePickerResult::Redraw;
                    }
                    KeyCode::Home => {
                        self.selected_index = 0;
                        return FilePickerResult::Redraw;
                    }
                    KeyCode::End => {
                        self.selected_index = self.filtered_files.len().saturating_sub(1);
                        return FilePickerResult::Redraw;
                    }
                    KeyCode::Backspace => {
                        self.query.pop();
                        self.update_filter();
                        return FilePickerResult::Redraw;
                    }
                    KeyCode::Char(c) => {
                        self.query.push(c);
                        self.update_filter();
                        return FilePickerResult::Redraw;
                    }
                    _ => {}
                }
            }
            Event::Mouse(mouse) => {
                match mouse.kind {
                    MouseEventKind::ScrollUp => {
                        if self.selected_index > 0 {
                            self.selected_index = self.selected_index.saturating_sub(3);
                            return FilePickerResult::Redraw;
                        }
                    }
                    MouseEventKind::ScrollDown => {
                        let max = self.filtered_files.len().saturating_sub(1);
                        self.selected_index = (self.selected_index + 3).min(max);
                        return FilePickerResult::Redraw;
                    }
                    _ => {}
                }
            }
            _ => {}
        }
        FilePickerResult::None
    }
    
    fn update_filter(&mut self) {
        if self.query.is_empty() {
            self.filtered_files = (0..self.files.len()).collect();
        } else {
            let query_lower = self.query.to_lowercase();
            self.filtered_files = self.files
                .iter()
                .enumerate()
                .filter(|(_, path)| {
                    path.to_string_lossy()
                        .to_lowercase()
                        .contains(&query_lower)
                })
                .map(|(i, _)| i)
                .collect();
        }
        
        self.selected_index = 0;
        self.scroll_offset = 0;
        // Don't force reload - cache handles it
    }
}

fn find_pdf_files() -> Result<Vec<PathBuf>> {
    let search_dir = "/Users/jack/Documents";
    
    // Try using fd first (faster), fallback to find
    let output = if command_exists("fd") {
        Command::new("fd")
            .args(&["-e", "pdf", "-t", "f", ".", search_dir])
            .output()?
    } else {
        Command::new("find")
            .args(&[search_dir, "-name", "*.pdf", "-type", "f"])
            .output()?
    };
    
    if !output.status.success() {
        return Ok(Vec::new());
    }
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    let files: Vec<PathBuf> = stdout
        .lines()
        .map(PathBuf::from)
        .filter(|p| p.exists())
        .collect();
    
    Ok(files)
}

fn command_exists(cmd: &str) -> bool {
    Command::new("which")
        .arg(cmd)
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}