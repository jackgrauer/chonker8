// Status bar rendering
use crossterm::{
    cursor::MoveTo,
    execute,
    style::{Print, SetBackgroundColor, SetForegroundColor, ResetColor},
};
use std::io::{self, Write};
use crate::screen_mode::ScreenMode;
use crate::theme::colors;

pub fn render(stdout: &mut io::Stdout, screen_mode: ScreenMode, current_page: usize, total_pages: usize, status_message: &str, width: u16, height: u16) -> io::Result<()> {
    execute!(stdout, MoveTo(0, height - 1))?;
    execute!(stdout, SetBackgroundColor(colors::BG_STATUS))?;
    execute!(stdout, SetForegroundColor(colors::FG_STATUS))?;
    
    let screen_name = match screen_mode {
        ScreenMode::FilePicker => "FILE PICKER",
        ScreenMode::Editor => "EDITOR",
        ScreenMode::Debug => "DEBUG",
    };
    
    let status = format!(
        " {} | Page {}/{} | {} | Tab: Switch | Ctrl+C: Copy | Ctrl+Q: Quit ",
        screen_name,
        current_page + 1,
        total_pages,
        if status_message.is_empty() { "Ready" } else { status_message }
    );
    
    let status_len = status.len();
    execute!(stdout, Print(status))?;
    execute!(stdout, Print(" ".repeat((width as usize).saturating_sub(status_len))))?;
    execute!(stdout, ResetColor)?;
    
    Ok(())
}