// Edit panel renderer for text editor view
use anyhow::Result;
use crossterm::{
    cursor::MoveTo,
    execute,
    style::{Color, Print, ResetColor, SetBackgroundColor, SetForegroundColor},
};
use std::io::{self, Write};

use crate::theme_const::*;

pub struct EditPanelRenderer {
    width: u16,
    height: u16,
    buffer: Vec<Vec<char>>,
    scroll_x: u16,
    scroll_y: u16,
}

impl EditPanelRenderer {
    pub fn new(width: u16, height: u16) -> Self {
        Self {
            width,
            height,
            buffer: vec![vec![' '; width as usize]; height as usize],
            scroll_x: 0,
            scroll_y: 0,
        }
    }
    
    pub fn update_buffer(&mut self, data: &Vec<Vec<char>>) {
        self.buffer = data.clone();
    }
    
    pub fn get_scroll(&self) -> (u16, u16) {
        (self.scroll_x, self.scroll_y)
    }
    
    pub fn render_with_cursor_and_selection(
        &self,
        x: u16,
        y: u16,
        width: u16,
        height: u16,
        cursor: (usize, usize),
        sel_start: Option<(usize, usize)>,
        sel_end: Option<(usize, usize)>,
    ) -> Result<()> {
        let mut stdout = io::stdout();
        
        // Determine visible range
        let visible_rows = height as usize;
        let visible_cols = width as usize;
        
        // Auto-scroll to keep cursor visible
        let scroll_y = if cursor.1 >= self.scroll_y as usize + visible_rows {
            (cursor.1 - visible_rows + 1) as u16
        } else if cursor.1 < self.scroll_y as usize {
            cursor.1 as u16
        } else {
            self.scroll_y
        };
        
        let scroll_x = if cursor.0 >= self.scroll_x as usize + visible_cols {
            (cursor.0 - visible_cols + 1) as u16
        } else if cursor.0 < self.scroll_x as usize {
            cursor.0 as u16
        } else {
            self.scroll_x
        };
        
        // Check if we have a selection
        let has_selection = sel_start.is_some() && sel_end.is_some();
        let (sel_start_pos, sel_end_pos) = if has_selection {
            let start = sel_start.unwrap();
            let end = sel_end.unwrap();
            
            // Normalize selection (start should be before end)
            if start.1 < end.1 || (start.1 == end.1 && start.0 <= end.0) {
                (start, end)
            } else {
                (end, start)
            }
        } else {
            ((0, 0), (0, 0))
        };
        
        // Render the visible portion of the buffer
        for row in 0..visible_rows {
            let buffer_row = row + scroll_y as usize;
            if buffer_row >= self.buffer.len() {
                break;
            }
            
            execute!(stdout, MoveTo(x, y + row as u16))?;
            
            for col in 0..visible_cols {
                let buffer_col = col + scroll_x as usize;
                if buffer_col >= self.buffer[buffer_row].len() {
                    write!(stdout, " ")?;
                    continue;
                }
                
                let ch = self.buffer[buffer_row][buffer_col];
                let is_cursor = buffer_row == cursor.1 && buffer_col == cursor.0;
                let is_selected = has_selection && is_in_selection(
                    buffer_row, buffer_col,
                    sel_start_pos, sel_end_pos
                );
                
                if is_cursor {
                    execute!(
                        stdout,
                        SetBackgroundColor(CURSOR_BG),
                        SetForegroundColor(CURSOR_FG),
                        Print(ch),
                        ResetColor
                    )?;
                } else if is_selected {
                    execute!(
                        stdout,
                        SetBackgroundColor(SELECTION_BG),
                        SetForegroundColor(SELECTION_FG),
                        Print(ch),
                        ResetColor
                    )?;
                } else {
                    write!(stdout, "{}", ch)?;
                }
            }
        }
        
        Ok(())
    }
}

fn is_in_selection(
    row: usize, col: usize,
    start: (usize, usize), end: (usize, usize)
) -> bool {
    if row < start.1 || row > end.1 {
        return false;
    }
    
    if row == start.1 && row == end.1 {
        // Selection on single line
        col >= start.0 && col <= end.0
    } else if row == start.1 {
        // First line of multi-line selection
        col >= start.0
    } else if row == end.1 {
        // Last line of multi-line selection
        col <= end.0
    } else {
        // Middle lines are fully selected
        true
    }
}