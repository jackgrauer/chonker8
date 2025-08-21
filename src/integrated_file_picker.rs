// Integrated file picker that runs within the TUI screen
use anyhow::Result;
use crossterm::{
    cursor::MoveTo,
    execute,
    style::{Print, ResetColor, SetForegroundColor},
    terminal::{self, Clear, ClearType},
};
use nucleo::{Config, Nucleo, Utf32String};
use std::io::{stdout, Write};
use std::path::PathBuf;
use std::process::Command;
use std::sync::Arc;
use crate::theme::ChonkerTheme;

pub struct IntegratedFilePicker {
    nucleo: Nucleo<Arc<str>>,
    files: Vec<String>,
    query: String,
    selected_index: usize,
    scroll_offset: usize,
    initialized: bool,
}

impl IntegratedFilePicker {
    pub fn new() -> Result<Self> {
        let files = find_pdf_files()?;
        
        let mut nucleo = Nucleo::<Arc<str>>::new(
            Config::DEFAULT,
            Arc::new(|| {}),
            None,
            1,
        );

        // Add all files as items
        let injector = nucleo.injector();
        for file in &files {
            let file_arc: Arc<str> = Arc::from(file.as_str());
            let _ = injector.push(file_arc.clone(), |data, cols: &mut [Utf32String]| {
                cols[0] = data.as_ref().into();
            });
        }

        Ok(Self {
            nucleo,
            files,
            query: String::new(),
            selected_index: 0,
            scroll_offset: 0,
            initialized: true,
        })
    }

    pub fn render(&mut self, width: u16, height: u16) -> Result<()> {
        if !self.initialized {
            return Ok(());
        }

        // Clear the screen area
        execute!(
            stdout(),
            Clear(ClearType::All),
            MoveTo(0, 0)
        )?;

        // Draw header
        let header_text = "🐹 Chonker8 Hot-Reload File Picker [TEST]";
        execute!(
            stdout(),
            MoveTo(0, 0),
            SetForegroundColor(ChonkerTheme::accent_load_file()),
            Print(format!("  {:<width$}", header_text, width = (width - 2) as usize)),
            ResetColor,
            MoveTo(0, 1),
            SetForegroundColor(ChonkerTheme::accent_load_file()),
            Print(" ".repeat(width as usize)),
            ResetColor,
            Print("\n")
        )?;

        // Draw search box
        execute!(
            stdout(),
            MoveTo(0, 3),
            SetForegroundColor(ChonkerTheme::accent_text()),
            Print("  🔍 Search: "),
            SetForegroundColor(ChonkerTheme::text_primary()),
            Print(&self.query),
            SetForegroundColor(ChonkerTheme::text_dim()),
            Print("_"),
            ResetColor,
            Print("\n\n")
        )?;

        // Get filtered results
        let snapshot = self.nucleo.snapshot();
        let all_matches = snapshot.matched_items(..).collect::<Vec<_>>();

        // Calculate display parameters
        let max_path_width = (width as usize).saturating_sub(5);
        let max_display_items = (height as usize).saturating_sub(9).min(15);

        // Update scroll offset to keep selected item visible
        if self.selected_index >= self.scroll_offset + max_display_items {
            self.scroll_offset = self.selected_index.saturating_sub(max_display_items - 1);
        } else if self.selected_index < self.scroll_offset {
            self.scroll_offset = self.selected_index;
        }

        // Get visible matches with scrolling
        let visible_matches = all_matches
            .iter()
            .skip(self.scroll_offset)
            .take(max_display_items)
            .collect::<Vec<_>>();

        // Draw matches
        for (display_i, item) in visible_matches.iter().enumerate() {
            let actual_index = self.scroll_offset + display_i;
            let path = item.data.as_ref();

            // Strip the /Users/jack/Documents/ prefix for cleaner display
            let clean_path = if path.starts_with("/Users/jack/Documents/") {
                &path[22..] // Length of "/Users/jack/Documents/"
            } else {
                path
            };

            let line_pos = 6 + display_i as u16;

            // Move to the correct line and clear it
            execute!(
                stdout(),
                MoveTo(0, line_pos),
                Clear(ClearType::CurrentLine)
            )?;

            // Force truncate to terminal width
            let display_str = if clean_path.len() > max_path_width {
                if let Some(filename) = clean_path.split('/').last() {
                    if filename.len() <= max_path_width - 4 {
                        format!(".../{}", filename)
                    } else {
                        let truncate_len = max_path_width.saturating_sub(3).min(filename.len());
                        format!("{}...", &filename[..truncate_len])
                    }
                } else {
                    let truncate_len = max_path_width.saturating_sub(3).min(clean_path.len());
                    format!("{}...", &clean_path[..truncate_len])
                }
            } else {
                clean_path.to_string()
            };

            // Final safety check
            let final_display: String = display_str.chars().take(max_path_width).collect();

            if actual_index == self.selected_index {
                execute!(
                    stdout(),
                    SetForegroundColor(ChonkerTheme::success()),
                    Print("  ▶ "),
                    SetForegroundColor(ChonkerTheme::text_primary()),
                    Print(&final_display),
                    ResetColor
                )?;
            } else {
                execute!(
                    stdout(),
                    Print("    "),
                    SetForegroundColor(ChonkerTheme::text_secondary()),
                    Print(&final_display),
                    ResetColor
                )?;
            }
        }

        // Clear any remaining lines
        for i in visible_matches.len()..max_display_items {
            let line_pos = 6 + i as u16;
            execute!(
                stdout(),
                MoveTo(0, line_pos),
                Clear(ClearType::CurrentLine)
            )?;
        }

        // Draw status and help
        let help_line = (6 + max_display_items + 2) as u16;
        let scroll_indicator = if all_matches.len() > max_display_items {
            format!("  Showing {}-{} of {} files", 
                self.scroll_offset + 1, 
                (self.scroll_offset + visible_matches.len()).min(all_matches.len()), 
                all_matches.len())
        } else {
            format!("  {} files", all_matches.len())
        };

        execute!(
            stdout(),
            MoveTo(0, help_line),
            Clear(ClearType::CurrentLine),
            SetForegroundColor(ChonkerTheme::text_dim()),
            Print(&scroll_indicator),
            ResetColor
        )?;

        execute!(
            stdout(),
            MoveTo(0, help_line + 1),
            Clear(ClearType::CurrentLine),
            SetForegroundColor(ChonkerTheme::text_dim()),
            Print("  🔥 INTEGRATED FILE PICKER  •  Tab: Next Screen  •  Esc: Exit"),
            ResetColor
        )?;

        stdout().flush()?;

        // Let nucleo process
        self.nucleo.tick(10);

        Ok(())
    }

    pub fn handle_char(&mut self, c: char) -> Result<()> {
        self.query.push(c);
        self.selected_index = 0;
        self.scroll_offset = 0;
        
        // Update nucleo pattern
        self.nucleo.pattern.reparse(
            0,
            &self.query,
            nucleo::pattern::CaseMatching::Smart,
            nucleo::pattern::Normalization::Smart,
            false
        );
        
        Ok(())
    }

    pub fn handle_backspace(&mut self) -> Result<()> {
        self.query.pop();
        self.selected_index = 0;
        self.scroll_offset = 0;
        
        // Update nucleo pattern
        self.nucleo.pattern.reparse(
            0,
            &self.query,
            nucleo::pattern::CaseMatching::Smart,
            nucleo::pattern::Normalization::Smart,
            false
        );
        
        Ok(())
    }

    pub fn handle_up(&mut self) -> Result<()> {
        if self.selected_index > 0 {
            self.selected_index -= 1;
        }
        Ok(())
    }

    pub fn handle_down(&mut self) -> Result<()> {
        let snapshot = self.nucleo.snapshot();
        let all_matches = snapshot.matched_items(..).collect::<Vec<_>>();
        
        if self.selected_index < all_matches.len().saturating_sub(1) {
            self.selected_index += 1;
        }
        Ok(())
    }

    pub fn get_selected_file(&self) -> Option<PathBuf> {
        let snapshot = self.nucleo.snapshot();
        let all_matches = snapshot.matched_items(..).collect::<Vec<_>>();
        
        if self.selected_index < all_matches.len() {
            let selected = all_matches[self.selected_index].data.as_ref();
            Some(PathBuf::from(selected))
        } else {
            None
        }
    }
}

/// Find all PDF files in current directory and subdirectories
fn find_pdf_files() -> Result<Vec<String>> {
    let search_dirs = ["/Users/jack/Documents", ".", "/Users/jack/Desktop"];
    
    let mut all_files = Vec::new();
    
    for search_dir in &search_dirs {
        let files = find_pdfs_in_dir(search_dir)?;
        all_files.extend(files);
    }
    
    // Remove duplicates while preserving order
    all_files.sort();
    all_files.dedup();
    
    Ok(all_files)
}

fn find_pdfs_in_dir(search_dir: &str) -> Result<Vec<String>> {
    // Try using fd first (faster), fallback to find
    let output = if command_exists("fd") {
        Command::new("fd")
            .args(&["-e", "pdf", "-t", "f", ".", search_dir])
            .output()
    } else {
        // Fallback to find command
        Command::new("find")
            .args(&[search_dir, "-name", "*.pdf", "-type", "f"])
            .output()
    };
    
    match output {
        Ok(output) if output.status.success() => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let files: Vec<String> = stdout
                .lines()
                .map(|s| s.to_string())
                .filter(|s| !s.is_empty())
                .collect();
            Ok(files)
        }
        _ => Ok(Vec::new()) // Return empty vec on error
    }
}

/// Check if a command exists
fn command_exists(cmd: &str) -> bool {
    Command::new("which")
        .arg(cmd)
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}