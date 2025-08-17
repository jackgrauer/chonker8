// Keyboard handling module
use crate::{App, MOD_KEY};
use crate::types::{AppFlags, Pos, Selection};
use crate::screen_mode::ScreenMode;
use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

pub async fn handle_input(app: &mut App, key: KeyEvent) -> Result<bool> {
    // Global commands that work from ANY screen
    
    // Ctrl+O - Open file picker (GLOBAL)
    if key.code == KeyCode::Char('o') && key.modifiers.contains(MOD_KEY) {
        app.open_file_picker = true;
        crate::debug_log("Global Ctrl+O: Opening file picker");
        return Ok(true);
    }
    
    // Tab - Switch screens (GLOBAL)
    if key.code == KeyCode::Tab {
        app.screen_mode = match app.screen_mode {
            ScreenMode::Editor => ScreenMode::Debug,
            ScreenMode::Debug => ScreenMode::Editor,
        };
        app.flags.insert(AppFlags::REDRAW);
        crate::debug_log(format!("Switched to {:?} screen", app.screen_mode));
        return Ok(true);
    }
    
    // Screen-specific handling
    match app.screen_mode {
        ScreenMode::Debug => {
            if !handle_debug_keys(app, key)? {
                return Ok(false); // Debug handler requested exit
            }
        }
        ScreenMode::Editor => {
            if !handle_editor_keys(app, key).await? {
                return Ok(false); // Editor handler requested exit
            }
        }
    }
    
    // Global Ctrl+C - Exit application (fallback if not handled by screen)
    if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
        app.flags.insert(AppFlags::EXIT);
        return Ok(false);
    }
    
    Ok(true)
}

fn handle_debug_keys(app: &mut App, key: KeyEvent) -> Result<bool> {
    match key.code {
        KeyCode::Char('c') if key.modifiers.contains(MOD_KEY) => {
            if let Err(e) = app.debug_panel.copy_to_clipboard() {
                app.status_message = format!("Copy failed: {}", e);
            } else {
                app.status_message = "Debug logs copied".to_string();
            }
        }
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            // In debug mode, Ctrl+C exits
            app.flags.insert(AppFlags::EXIT);
            return Ok(false);
        }
        KeyCode::Char('a') if key.modifiers.contains(MOD_KEY) => {
            app.debug_panel.select_all();
        }
        KeyCode::Up => {
            app.debug_panel.scroll_up(1);
        }
        KeyCode::Down => {
            app.debug_panel.scroll_down(1);
        }
        KeyCode::PageUp => {
            app.debug_panel.scroll_up(10);
        }
        KeyCode::PageDown => {
            app.debug_panel.scroll_down(10);
        }
        _ => {}
    }
    Ok(true)
}

async fn handle_editor_keys(app: &mut App, key: KeyEvent) -> Result<bool> {
    // Ctrl+C - Exit in editor mode
    if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
        app.flags.insert(AppFlags::EXIT);
        return Ok(false);
    }
    
    // Cmd+C - Copy
    if key.code == KeyCode::Char('c') && key.modifiers.contains(MOD_KEY) {
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
        {
            let data = &app.current_grid;
            let last_y = data.len().saturating_sub(1);
            let last_x = if last_y < data.len() {
                data[last_y].len().saturating_sub(1)
            } else {
                0
            };
            app.selection = Selection::Active {
                start: Pos { x: 0, y: 0 },
                end: Pos { x: last_x, y: last_y },
            };
        }
        return Ok(true);
    }
    
    // (Ctrl+O is handled globally)
    
    // Cmd+E - Extract current page
    if key.code == KeyCode::Char('e') && key.modifiers.contains(MOD_KEY) {
        app.extract_current_page().await?;
        return Ok(true);
    }
    
    // Cmd+N/P - Next/Previous page
    if key.code == KeyCode::Char('n') && key.modifiers.contains(MOD_KEY) {
        if app.change_page(1) {
            app.load_pdf_page().await?;
        }
        return Ok(true);
    }
    
    if key.code == KeyCode::Char('p') && key.modifiers.contains(MOD_KEY) {
        if app.change_page(-1) {
            app.load_pdf_page().await?;
        }
        return Ok(true);
    }
    
    // Arrow keys for cursor movement
    let _ = handle_arrow_keys(app, key)?;
    
    // Text editing
    match key.code {
        KeyCode::Backspace => {
            {
                let data = &mut app.current_grid;
                if app.cursor.x > 0 && app.cursor.y < data.len() {
                    data[app.cursor.y].remove(app.cursor.x - 1);
                    app.cursor.x -= 1;
                    if let Some(renderer) = &mut app.edit_display {
                        renderer.update_buffer(data);
                    }
                }
            }
        }
        
        KeyCode::Char(c) if !key.modifiers.contains(MOD_KEY) => {
            {
                let data = &mut app.current_grid;
                while data.len() <= app.cursor.y {
                    data.push(vec![]);
                }
                let row_len = data[app.cursor.y].len();
                data[app.cursor.y].insert(app.cursor.x.min(row_len), c);
                app.cursor.x += 1;
                if let Some(renderer) = &mut app.edit_display {
                    renderer.update_buffer(data);
                }
            }
        }
        
        _ => {}
    }
    
    Ok(true)
}

fn handle_arrow_keys(app: &mut App, key: KeyEvent) -> Result<bool> {
    let shift_held = key.modifiers.contains(KeyModifiers::SHIFT);
    
    // Start selection if shift is held and we don't have one
    if shift_held && !matches!(app.selection, Selection::Active { .. }) {
        app.selection = Selection::Active {
            start: app.cursor,
            end: app.cursor,
        };
    }
    
    match key.code {
        KeyCode::Up => {
            if app.cursor.y > 0 {
                app.cursor.y -= 1;
                if shift_held {
                    if let Selection::Active { start, .. } = app.selection {
                        app.selection = Selection::Active { start, end: app.cursor };
                    }
                } else {
                    app.selection = Selection::None;
                }
            }
        }
        KeyCode::Down => {
            {
            let data = &app.current_grid;
                if app.cursor.y < data.len() - 1 {
                    app.cursor.y += 1;
                    if shift_held {
                        if let Selection::Active { start, .. } = app.selection {
                            app.selection = Selection::Active { start, end: app.cursor };
                        }
                    } else {
                        app.selection = Selection::None;
                    }
                }
            }
        }
        KeyCode::Left => {
            if app.cursor.x > 0 {
                app.cursor.x -= 1;
                if shift_held {
                    if let Selection::Active { start, .. } = app.selection {
                        app.selection = Selection::Active { start, end: app.cursor };
                    }
                } else {
                    app.selection = Selection::None;
                }
            }
        }
        KeyCode::Right => {
            {
            let data = &app.current_grid;
                if app.cursor.y < data.len() && app.cursor.x < data[app.cursor.y].len() {
                    app.cursor.x += 1;
                    if shift_held {
                        if let Selection::Active { start, .. } = app.selection {
                            app.selection = Selection::Active { start, end: app.cursor };
                        }
                    } else {
                        app.selection = Selection::None;
                    }
                }
            }
        }
        _ => {}
    }
    
    Ok(true)
}

fn extract_selection_text(app: &App) -> Option<String> {
    if let Selection::Active { start, end } = app.selection {
        // Normalize selection
        let (start, end) = if start.y < end.y || (start.y == end.y && start.x <= end.x) {
            (start, end)
        } else {
            (end, start)
        };
        
        let data = &app.current_grid;
        let mut text = String::new();
        for y in start.y..=end.y {
            if let Some(row) = data.get(y) {
                let start_x = if y == start.y { start.x } else { 0 };
                let end_x = if y == end.y { end.x } else { row.len() - 1 };
                for x in start_x..=end_x.min(row.len() - 1) {
                    text.push(row[x]);
                }
                if y < end.y {
                    text.push('\n');
                }
            }
        }
        Some(text)
    } else {
        None
    }
}

fn paste_at_cursor(app: &mut App, text: &str) {
    {
        let data = &mut app.current_grid;
        while data.len() <= app.cursor.y {
            data.push(vec![]);
        }
        
        for ch in text.chars() {
            if ch == '\n' {
                app.cursor.y += 1;
                app.cursor.x = 0;
                if app.cursor.y >= data.len() {
                    data.push(vec![]);
                }
            } else {
                let row_len = data[app.cursor.y].len();
                data[app.cursor.y].insert(app.cursor.x.min(row_len), ch);
                app.cursor.x += 1;
            }
        }
        
        if let Some(renderer) = &mut app.edit_display {
            renderer.update_buffer(data);
        }
    }
}

fn copy_to_clipboard(text: &str) -> Result<()> {
    use copypasta::{ClipboardContext, ClipboardProvider};
    let mut ctx = ClipboardContext::new()
        .map_err(|e| anyhow::anyhow!("Clipboard error: {}", e))?;
    ctx.set_contents(text.to_owned())
        .map_err(|e| anyhow::anyhow!("Clipboard error: {}", e))?;
    Ok(())
}

fn paste_from_clipboard() -> Result<String> {
    use copypasta::{ClipboardContext, ClipboardProvider};
    let mut ctx = ClipboardContext::new()
        .map_err(|e| anyhow::anyhow!("Clipboard error: {}", e))?;
    ctx.get_contents()
        .map_err(|e| anyhow::anyhow!("Clipboard error: {}", e))
}