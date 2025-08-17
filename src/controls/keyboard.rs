// MINIMAL KEYBOARD HANDLING
use crate::{App, MOD_KEY};
use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

pub async fn handle_input(app: &mut App, key: KeyEvent) -> Result<bool> {
    use crate::screen_mode::ScreenMode;
    use crate::types::AppFlags;
    
    // Tab - Switch screens
    if key.code == KeyCode::Tab {
        app.screen_mode = app.screen_mode.next();
        app.flags.insert(AppFlags::REDRAW);
        crate::debug_log(format!("Switched to {:?} screen", app.screen_mode));
        return Ok(true);
    }
    
    // Screen-specific handling
    match app.screen_mode {
        ScreenMode::Debug => {
            // Debug panel controls
            match key.code {
                KeyCode::Char('c') if key.modifiers.contains(MOD_KEY) => {
                    if let Err(e) = app.debug_panel.copy_to_clipboard() {
                        app.status_message = format!("Copy failed: {}", e);
                    } else {
                        app.status_message = "Debug logs copied".to_string();
                    }
                    return Ok(true);
                }
                KeyCode::Char('a') if key.modifiers.contains(MOD_KEY) => {
                    app.debug_panel.select_all();
                    return Ok(true);
                }
                KeyCode::Up => {
                    app.debug_panel.scroll_up(1);
                    return Ok(true);
                }
                KeyCode::Down => {
                    app.debug_panel.scroll_down(1);
                    return Ok(true);
                }
                KeyCode::PageUp => {
                    app.debug_panel.scroll_up(10);
                    return Ok(true);
                }
                KeyCode::PageDown => {
                    app.debug_panel.scroll_down(10);
                    return Ok(true);
                }
                _ => {}
            }
        }
        ScreenMode::Editor => {
            // Editor screen - handle text editing
        }
        ScreenMode::FilePicker => {
            // File picker - handle file navigation
        }
    }
    
    // Cmd+C - Copy (for editor screen)
    if key.code == KeyCode::Char('c') && key.modifiers.contains(MOD_KEY) && app.screen_mode == ScreenMode::Editor {
        if let Some(text) = extract_selection_text(app) {
            copy_to_clipboard(&text)?;
            app.status_message = "Copied".to_string();
        }
        return Ok(true);
    }
    
    // Cmd+V - Paste
    if key.code == KeyCode::Char('v') && key.modifiers.contains(MOD_KEY) {
        if let Ok(text) = paste_from_clipboard() {
            paste_at_cursor(app, &text);
            app.status_message = "Pasted".to_string();
        }
        return Ok(true);
    }
    
    // Cmd+A - Select All
    if key.code == KeyCode::Char('a') && key.modifiers.contains(MOD_KEY) {
        if let Some(data) = &app.edit_data {
            app.selection_start = Some((0, 0));
            let last_y = data.len().saturating_sub(1);
            let last_x = data[last_y].len().saturating_sub(1);
            app.selection_end = Some((last_x, last_y));
        }
        return Ok(true);
    }
    
    match key.code {
        KeyCode::Char('q') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.exit_requested = true;
        }
        
        KeyCode::Char('o') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.open_file_picker = true;
        }
        
        KeyCode::Char('e') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.extract_current_page().await?;
        }
        
        KeyCode::Char('n') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.next_page();
            if app.current_page_image.is_none() {
                app.load_pdf_page().await?;
            }
        }
        KeyCode::Char('p') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.prev_page();
            if app.current_page_image.is_none() {
                app.load_pdf_page().await?;
            }
        }
        
        // Arrow keys for cursor movement (with shift for selection)
        KeyCode::Up => {
            if app.cursor.1 > 0 {
                if key.modifiers.contains(KeyModifiers::SHIFT) {
                    if app.selection_start.is_none() {
                        app.selection_start = Some(app.cursor);
                    }
                    app.cursor.1 -= 1;
                    app.selection_end = Some(app.cursor);
                } else {
                    app.cursor.1 -= 1;
                    app.selection_start = None;
                    app.selection_end = None;
                }
            }
        }
        KeyCode::Down => {
            if let Some(data) = &app.edit_data {
                if app.cursor.1 < data.len() - 1 {
                    if key.modifiers.contains(KeyModifiers::SHIFT) {
                        if app.selection_start.is_none() {
                            app.selection_start = Some(app.cursor);
                        }
                        app.cursor.1 += 1;
                        app.selection_end = Some(app.cursor);
                    } else {
                        app.cursor.1 += 1;
                        app.selection_start = None;
                        app.selection_end = None;
                    }
                }
            }
        }
        KeyCode::Left => {
            if app.cursor.0 > 0 {
                if key.modifiers.contains(KeyModifiers::SHIFT) {
                    if app.selection_start.is_none() {
                        app.selection_start = Some(app.cursor);
                    }
                    app.cursor.0 -= 1;
                    app.selection_end = Some(app.cursor);
                } else {
                    app.cursor.0 -= 1;
                    app.selection_start = None;
                    app.selection_end = None;
                }
            }
        }
        KeyCode::Right => {
            if let Some(data) = &app.edit_data {
                if app.cursor.1 < data.len() && app.cursor.0 < data[app.cursor.1].len() {
                    if key.modifiers.contains(KeyModifiers::SHIFT) {
                        if app.selection_start.is_none() {
                            app.selection_start = Some(app.cursor);
                        }
                        app.cursor.0 += 1;
                        app.selection_end = Some(app.cursor);
                    } else {
                        app.cursor.0 += 1;
                        app.selection_start = None;
                        app.selection_end = None;
                    }
                }
            }
        }
        
        // Text editing
        KeyCode::Backspace => {
            if let Some(data) = &mut app.edit_data {
                if app.cursor.0 > 0 && app.cursor.1 < data.len() {
                    data[app.cursor.1].remove(app.cursor.0 - 1);
                    app.cursor.0 -= 1;
                    if let Some(renderer) = &mut app.edit_display {
                        renderer.update_buffer(data);
                    }
                }
            }
        }
        
        KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
            if let Some(data) = &mut app.edit_data {
                while data.len() <= app.cursor.1 {
                    data.push(vec![]);
                }
                let row_len = data[app.cursor.1].len();
                data[app.cursor.1].insert(app.cursor.0.min(row_len), c);
                app.cursor.0 += 1;
                if let Some(renderer) = &mut app.edit_display {
                    renderer.update_buffer(data);
                }
            }
        }
        
        _ => {}
    }
    
    Ok(true)
}

fn extract_selection_text(app: &App) -> Option<String> {
    let (start, end) = match (app.selection_start, app.selection_end) {
        (Some(s), Some(e)) => if s.1 < e.1 || (s.1 == e.1 && s.0 < e.0) { (s, e) } else { (e, s) },
        _ => return None,
    };
    
    if let Some(data) = &app.edit_data {
        let mut text = String::new();
        for y in start.1..=end.1 {
            if let Some(row) = data.get(y) {
                let start_x = if y == start.1 { start.0 } else { 0 };
                let end_x = if y == end.1 { end.0 } else { row.len() - 1 };
                for x in start_x..=end_x.min(row.len() - 1) {
                    text.push(row[x]);
                }
                if y < end.1 { text.push('\n'); }
            }
        }
        Some(text)
    } else {
        None
    }
}

fn paste_at_cursor(app: &mut App, text: &str) {
    if let Some(data) = &mut app.edit_data {
        while data.len() <= app.cursor.1 {
            data.push(vec![]);
        }
        
        for ch in text.chars() {
            if ch == '\n' {
                app.cursor.1 += 1;
                app.cursor.0 = 0;
                if app.cursor.1 >= data.len() {
                    data.push(vec![]);
                }
            } else {
                let row_len = data[app.cursor.1].len();
                data[app.cursor.1].insert(app.cursor.0.min(row_len), ch);
                app.cursor.0 += 1;
            }
        }
        
        if let Some(renderer) = &mut app.edit_display {
            renderer.update_buffer(data);
        }
    }
}

fn copy_to_clipboard(text: &str) -> Result<()> {
    use copypasta::{ClipboardContext, ClipboardProvider};
    let mut ctx = ClipboardContext::new().map_err(|e| anyhow::anyhow!("Clipboard error: {}", e))?;
    ctx.set_contents(text.to_owned()).map_err(|e| anyhow::anyhow!("Clipboard error: {}", e))?;
    Ok(())
}

fn paste_from_clipboard() -> Result<String> {
    use copypasta::{ClipboardContext, ClipboardProvider};
    let mut ctx = ClipboardContext::new().map_err(|e| anyhow::anyhow!("Clipboard error: {}", e))?;
    ctx.get_contents().map_err(|e| anyhow::anyhow!("Clipboard error: {}", e))
}