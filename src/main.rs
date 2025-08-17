// MINIMAL CHONKER - Just PDF text extraction to editable grid
use anyhow::Result;
use crossterm::{
    cursor::{Hide, MoveTo, Show},
    event::{self, Event, KeyCode, KeyModifiers, EnableMouseCapture, DisableMouseCapture, MouseEvent, MouseEventKind, MouseButton},
    execute,
    style::{Color, Print, ResetColor, SetBackgroundColor, SetForegroundColor},
    terminal::{self, Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use std::{io::{self, Write}, path::PathBuf, time::Duration};
use image::DynamicImage;
use clap::Parser;

mod content_extractor;
mod edit_renderer;
mod pdf_renderer;
mod pdf_extraction;
mod file_picker;
mod theme;
mod theme_const;
mod types;
mod screen_mode;
mod viuer_display;
mod keyboard;
mod debug_panel;
mod storage;
mod migration_tool;

use edit_renderer::EditPanelRenderer;
use debug_panel::DebugPanel;
use theme::ChonkerTheme;
use types::{Pos, Selection, AppFlags, GRID_WIDTH, GRID_HEIGHT};
use screen_mode::ScreenMode;
use storage::StorageBackend;
use std::sync::Mutex;

// Global debug logger
lazy_static::lazy_static! {
    static ref DEBUG_LOGS: Mutex<Vec<String>> = Mutex::new(Vec::new());
}

pub fn debug_log<S: Into<String>>(msg: S) {
    let msg = msg.into();
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis();
    
    let formatted_msg = format!("[{}] {}", timestamp % 100000, msg);
    
    eprintln!("{}", formatted_msg);  // Still print to stderr for debugging
    if let Ok(mut logs) = DEBUG_LOGS.lock() {
        logs.push(formatted_msg);
        if logs.len() > 1000 {
            logs.remove(0);
        }
    }
}

// Enhanced debug logging macros
#[macro_export]
macro_rules! debug_trace {
    ($($arg:tt)*) => {
        crate::debug_log(format!("TRACE: {}", format!($($arg)*)));
    };
}

#[macro_export]
macro_rules! debug_error {
    ($($arg:tt)*) => {
        crate::debug_log(format!("ERROR: {}", format!($($arg)*)));
    };
}

#[macro_export]
macro_rules! debug_timing {
    ($name:expr, $start:expr) => {
        crate::debug_log(format!("TIMING: {} took {:?}", $name, $start.elapsed()));
    };
}

#[cfg(target_os = "macos")]
const MOD_KEY: KeyModifiers = KeyModifiers::SUPER;
#[cfg(not(target_os = "macos"))]
const MOD_KEY: KeyModifiers = KeyModifiers::CONTROL;

#[derive(Parser, Debug)]
#[command(author, version, about)]
struct Args {
    pdf_file: Option<PathBuf>,
    #[arg(short, long, default_value_t = 1)]
    page: usize,
}

pub struct App {
    pub pdf_path: PathBuf,
    pub current_page: usize,
    pub total_pages: usize,
    // NEW: Lance/Legacy storage backend
    pub storage: StorageBackend,
    pub current_grid: Vec<Vec<char>>,  // Working copy for editing
    // DELETE: pub edit_data: Option<Vec<Vec<char>>>,
    pub edit_display: Option<EditPanelRenderer>,
    pub current_page_image: Option<DynamicImage>,
    pub cursor: Pos,
    pub selection: Selection,
    pub status_message: String,
    pub flags: AppFlags,
    pub open_file_picker: bool,
    pub screen_mode: ScreenMode,
    pub debug_panel: DebugPanel,
    pub last_rendered_screen: Option<ScreenMode>,
}

impl App {
    pub fn new(pdf_path: PathBuf, start_page: usize) -> Result<Self> {
        let total_pages = content_extractor::get_page_count(&pdf_path)?;
        let flags = AppFlags::DARK_MODE | AppFlags::REDRAW;
        
        // Initialize storage backend (try Lance first if available)
        let prefer_lance = cfg!(feature = "lance-storage");
        let storage = StorageBackend::new(&pdf_path, prefer_lance)?;
        
        debug_log(format!("🚀 Chonker8 v8.30.0 - {} storage backend", storage.storage_type()));
        
        Ok(Self {
            pdf_path,
            current_page: start_page.saturating_sub(1),
            total_pages,
            storage,
            current_grid: vec![vec![' '; GRID_WIDTH]; GRID_HEIGHT],
            edit_display: None,
            current_page_image: None,
            cursor: Pos::default(),
            selection: Selection::None,
            status_message: String::new(),
            flags,
            open_file_picker: false,
            screen_mode: ScreenMode::Editor,  // Start with editor
            debug_panel: DebugPanel::new(),
            last_rendered_screen: None,
        })
    }

    pub async fn load_pdf_page(&mut self) -> Result<()> {
        debug_log(format!("=== Loading PDF page {} from {} ===", self.current_page, self.storage.storage_type()));
        
        // Try to load from storage first
        self.current_grid = self.storage.load_page(self.current_page)?;
        
        // Check if it's empty (not yet extracted)
        let is_empty = self.current_grid.iter()
            .all(|row| row.iter().all(|&c| c == ' '));
        
        if is_empty {
            debug_log("Page not in storage, extracting...");
            
            // Render PDF image for display
            debug_log("Rendering PDF page as image...");
            let start = std::time::Instant::now();
            match pdf_renderer::render_pdf_page(&self.pdf_path, self.current_page, 800, 1000) {
                Ok(image) => {
                    debug_log(format!("PDF rendered successfully: {}x{} pixels", image.width(), image.height()));
                    self.current_page_image = Some(image);
                    debug_timing!("PDF rendering", start);
                }
                Err(e) => {
                    debug_error!("Failed to render PDF: {}", e);
                    return Err(e);
                }
            }
            
            // Extract text to grid
            debug_log(format!("Extracting text to {}x{} grid...", GRID_WIDTH, GRID_HEIGHT));
            let start = std::time::Instant::now();
            match pdf_extraction::extract_to_matrix(&self.pdf_path, self.current_page, GRID_WIDTH, GRID_HEIGHT).await {
                Ok(data) => {
                    debug_log("Text extraction successful");
                    self.current_grid = data;
                    debug_timing!("Text extraction", start);
                    
                    // Save to storage for next time
                    let version = self.storage.save_page(self.current_page, self.current_grid.clone())?;
                    debug_log(format!("Saved to storage version {}", version));
                }
                Err(e) => {
                    debug_error!("Failed to extract text: {}", e);
                    return Err(e);
                }
            }
        } else {
            debug_log(format!("Loaded page {} from storage (v{})", self.current_page, self.storage.current_version()));
        }
        
        // Update display renderer
        let non_empty = self.current_grid.iter()
            .flat_map(|row| row.iter())
            .filter(|&&c| c != ' ')
            .count();
        debug_log(format!("Grid populated: {} non-space characters", non_empty));
        
        if let Some(renderer) = &mut self.edit_display {
            renderer.update_buffer(&self.current_grid);
        } else {
            let mut renderer = EditPanelRenderer::new(GRID_WIDTH as u16, GRID_HEIGHT as u16);
            renderer.update_buffer(&self.current_grid);
            self.edit_display = Some(renderer);
        }
        
        debug_log(format!("=== Page {} loaded successfully ===", self.current_page));
        Ok(())
    }

    pub async fn extract_current_page(&mut self) -> Result<()> {
        self.current_grid = pdf_extraction::extract_to_matrix(&self.pdf_path, self.current_page, GRID_WIDTH, GRID_HEIGHT).await?;
        
        // Save to storage
        let version = self.storage.save_page(self.current_page, self.current_grid.clone())?;
        debug_log(format!("Re-extracted page {} saved as version {}", self.current_page, version));
        
        if let Some(renderer) = &mut self.edit_display {
            renderer.update_buffer(&self.current_grid);
        } else {
            let mut renderer = EditPanelRenderer::new(GRID_WIDTH as u16, GRID_HEIGHT as u16);
            renderer.update_buffer(&self.current_grid);
            self.edit_display = Some(renderer);
        }
        
        self.status_message = "Text extracted".to_string();
        Ok(())
    }

    // Consolidated page change function
    pub fn change_page(&mut self, delta: i32) -> bool {
        let new_page = (self.current_page as i32 + delta)
            .clamp(0, self.total_pages as i32 - 1) as usize;
        
        if new_page != self.current_page {
            let _ = viuer_display::clear_graphics();
            self.current_page = new_page;
            self.clear_page_state();
            true
        } else {
            false
        }
    }
    
    fn clear_page_state(&mut self) {
        // Current grid is now managed by storage backend
        self.edit_display = None;
        self.current_page_image = None;
        self.cursor = Pos::default();
        self.selection = Selection::None;
        self.flags.insert(AppFlags::REDRAW);
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    
    let pdf_path = if let Some(path) = args.pdf_file {
        path
    } else {
        // Use file picker
        println!("🐹 Launching Chonker file picker...");
        if let Some(path) = file_picker::pick_pdf_file()? {
            println!("Selected: {}", path.display());
            path
        } else {
            println!("No file selected");
            return Ok(());
        }
    };

    let mut app = App::new(pdf_path, args.page)?;
    app.load_pdf_page().await?;
    
    setup_terminal()?;
    let result = run_app(&mut app).await;
    restore_terminal()?;
    
    result
}

fn setup_terminal() -> Result<()> {
    enable_raw_mode()?;
    execute!(io::stdout(), EnterAlternateScreen, Hide, EnableMouseCapture)?;
    Ok(())
}

fn restore_terminal() -> Result<()> {
    let _ = viuer_display::clear_graphics();
    execute!(io::stdout(), Clear(ClearType::All), MoveTo(0, 0))?;
    execute!(io::stdout(), Show, LeaveAlternateScreen, DisableMouseCapture)?;
    disable_raw_mode()?;
    Ok(())
}

async fn run_app(app: &mut App) -> Result<()> {
    let mut stdout = io::stdout();
    let mut last_term_size = (0, 0);
    
    // Initial render
    app.flags.insert(AppFlags::REDRAW);
    
    // Remove file picker screen mode initialization
    
    loop {
        let (term_width, term_height) = terminal::size()?;
        let panel_width = term_width / 3;
        
        // Check if terminal was resized or screen changed
        if (term_width, term_height) != last_term_size || 
           app.last_rendered_screen != Some(app.screen_mode) {
            app.flags.insert(AppFlags::REDRAW);
            last_term_size = (term_width, term_height);
            app.last_rendered_screen = Some(app.screen_mode);
        }
        
        // Check if we need to open file picker
        if app.open_file_picker {
            app.open_file_picker = false;
            restore_terminal()?;
            
            if let Some(new_path) = file_picker::pick_pdf_file()? {
                app.pdf_path = new_path;
                app.current_page = 0;
                app.total_pages = content_extractor::get_page_count(&app.pdf_path)?;
                app.load_pdf_page().await?;
                app.flags.insert(AppFlags::REDRAW);
            }
            
            setup_terminal()?;
            app.flags.insert(AppFlags::REDRAW);
        }
        
        // Only redraw when necessary
        if app.flags.contains(AppFlags::REDRAW) {
            execute!(stdout, Clear(ClearType::All), MoveTo(0, 0))?;
            
            match app.screen_mode {
                ScreenMode::Editor => {
                    // Add header for Editor mode
                    use theme_const::HEADER_PDF;
                    render_panel_header(&mut stdout, 0, 0, term_width, 
                        "PDF EDITOR", 
                        HEADER_PDF, true)?;
                    
                    // Split screen: PDF view (left) + Text editor (right)
                    let split_x = term_width / 2;
                    
                    // Render PDF image on left (below header)
                    if let Some(image) = &app.current_page_image {
                        let _ = viuer_display::display_pdf_image(
                            image, 0, 1, split_x - 1, term_height - 3, 
                            app.flags.contains(AppFlags::DARK_MODE)
                        );
                    }
                    
                    // Render text editor on right (no header, clean view)
                    if let Some(renderer) = &app.edit_display {
                        // Need to get back the old selection format temporarily
                        let sel_start = if let Selection::Active { start, .. } = app.selection {
                            Some((start.x, start.y))
                        } else { None };
                        let sel_end = if let Selection::Active { end, .. } = app.selection {
                            Some((end.x, end.y))
                        } else { None };
                        
                        renderer.render_with_cursor_and_selection(
                            split_x, 1, term_width - split_x, term_height - 3,
                            (app.cursor.x, app.cursor.y),
                            sel_start,
                            sel_end
                        )?;
                    }
                }
                
                ScreenMode::Debug => {
                    // Full screen debug output with soft green header
                    use theme_const::HEADER_DEBUG;
                    render_panel_header(&mut stdout, 0, 0, term_width, 
                        "DEBUG LOG - Ctrl+C to copy", 
                        HEADER_DEBUG, true)?;
                    
                    // Sync global logs to debug panel
                    if let Ok(logs) = DEBUG_LOGS.lock() {
                        app.debug_panel.logs = logs.clone();
                    }
                    
                    app.debug_panel.render(0, 1, term_width, term_height - 3)?;
                }
            }
            
            // Status bar
            render_status_bar(&mut stdout, app, term_width, term_height)?;
            
            stdout.flush()?;
            app.flags.remove(AppFlags::REDRAW);
        }
        
        // Handle input
        if event::poll(Duration::from_millis(50))? {
            match event::read()? {
                Event::Key(key) => {
                    let old_cursor = app.cursor;
                    let old_selection = app.selection.clone();
                    
                    if !keyboard::handle_input(app, key).await? {
                        break;
                    }
                    if app.flags.contains(AppFlags::EXIT) {
                        break;
                    }
                    
                    // Only redraw if something visual changed
                    // Don't redraw for simple cursor movements or selections
                    let cursor_or_selection_changed = app.cursor != old_cursor || 
                        app.selection != old_selection;
                    
                    if cursor_or_selection_changed && app.screen_mode == ScreenMode::Editor {
                        // Only update the text area, not the whole screen
                        if let Some(renderer) = &app.edit_display {
                            let split_x = term_width / 2;
                            let sel_start = if let Selection::Active { start, .. } = app.selection {
                                Some((start.x, start.y))
                            } else { None };
                            let sel_end = if let Selection::Active { end, .. } = app.selection {
                                Some((end.x, end.y))
                            } else { None };
                            
                            renderer.render_with_cursor_and_selection(
                                split_x, 1, term_width - split_x, term_height - 3,
                                (app.cursor.x, app.cursor.y),
                                sel_start,
                                sel_end
                            )?;
                            stdout.flush()?;
                        }
                    } else {
                        // For other changes, do full redraw
                        app.flags.insert(AppFlags::REDRAW);
                    }
                }
                Event::Mouse(mouse) => {
                    if app.screen_mode == ScreenMode::Debug {
                        // Handle debug panel mouse events
                        if app.debug_panel.handle_mouse(mouse) {
                            app.flags.insert(AppFlags::REDRAW);
                        }
                    } else if app.screen_mode == ScreenMode::Editor {
                        handle_mouse_event(app, mouse, term_width / 2, term_height)?;
                        
                        // Update display for selection changes
                        if let Some(renderer) = &app.edit_display {
                            let split_x = term_width / 2;
                            let sel_start = if let Selection::Active { start, .. } = app.selection {
                                Some((start.x, start.y))
                            } else { None };
                            let sel_end = if let Selection::Active { end, .. } = app.selection {
                                Some((end.x, end.y))
                            } else { None };
                            
                            renderer.render_with_cursor_and_selection(
                                split_x, 1, term_width - split_x, term_height - 3,
                                (app.cursor.x, app.cursor.y),
                                sel_start,
                                sel_end
                            )?;
                            stdout.flush()?;
                        }
                    }
                }
                _ => {}
            }
        }
    }
    
    Ok(())
}

fn render_panel_header(stdout: &mut io::Stdout, x: u16, y: u16, width: u16, title: &str, color: Color, is_active: bool) -> Result<()> {
    execute!(stdout, MoveTo(x, y))?;
    execute!(stdout, SetBackgroundColor(color))?;
    execute!(stdout, SetForegroundColor(Color::Black))?;
    
    let indicator = if is_active { "●" } else { "○" };
    let header_text = format!(" {} {} ", indicator, title);
    write!(stdout, "{:^width$}", header_text, width = width as usize)?;
    
    execute!(stdout, ResetColor)?;
    Ok(())
}

fn handle_mouse_event(app: &mut App, mouse: MouseEvent, split_x: u16, term_height: u16) -> Result<()> {
    // Only handle mouse in text panel (right side) and below header
    if mouse.column <= split_x || mouse.row == 0 || mouse.row >= term_height - 2 {
        return Ok(());
    }
    
    // Convert screen coordinates to text buffer coordinates (account for header)
    let text_x = (mouse.column - split_x) as usize;
    let text_y = (mouse.row - 1) as usize; // Subtract 1 for header
    
    if let Some(renderer) = &app.edit_display {
        let (scroll_x, scroll_y) = renderer.get_scroll();
        let buffer_x = text_x + scroll_x as usize;
        let buffer_y = text_y + scroll_y as usize;
        
        match mouse.kind {
            MouseEventKind::Down(MouseButton::Left) => {
                // Start selection
                app.cursor = Pos { x: buffer_x, y: buffer_y };
                app.selection = Selection::Active { 
                    start: app.cursor, 
                    end: app.cursor 
                };
                app.flags.insert(AppFlags::SELECTING);
            }
            MouseEventKind::Drag(MouseButton::Left) => {
                // Continue selection
                if app.flags.contains(AppFlags::SELECTING) {
                    app.cursor = Pos { x: buffer_x, y: buffer_y };
                    if let Selection::Active { start, .. } = app.selection {
                        app.selection = Selection::Active { start, end: app.cursor };
                    }
                }
            }
            MouseEventKind::Up(MouseButton::Left) => {
                // End selection
                if app.flags.contains(AppFlags::SELECTING) {
                    app.cursor = Pos { x: buffer_x, y: buffer_y };
                    if let Selection::Active { start, .. } = app.selection {
                        if start == app.cursor {
                            app.selection = Selection::None;
                        } else {
                            app.selection = Selection::Active { start, end: app.cursor };
                        }
                    }
                    app.flags.remove(AppFlags::SELECTING);
                }
            }
            _ => {}
        }
    }
    
    Ok(())
}

fn render_status_bar(stdout: &mut io::Stdout, app: &App, width: u16, height: u16) -> Result<()> {
    execute!(stdout, MoveTo(0, height - 1))?;
    execute!(stdout, SetBackgroundColor(ChonkerTheme::bg_status_dark()))?;
    execute!(stdout, SetForegroundColor(ChonkerTheme::text_status_dark()))?;
    
    let screen_name = match app.screen_mode {
        ScreenMode::Editor => "EDITOR",
        ScreenMode::Debug => "DEBUG",
    };
    
    let status = format!(
        " {} | Page {}/{} | {} | Tab: Switch | Ctrl+O: Open | Ctrl+C: Quit ",
        screen_name,
        app.current_page + 1,
        app.total_pages,
        if app.status_message.is_empty() { "Ready" } else { &app.status_message }
    );
    
    let status_len = status.len();
    execute!(stdout, Print(status))?;
    execute!(stdout, Print(" ".repeat((width as usize).saturating_sub(status_len))))?;
    execute!(stdout, ResetColor)?;
    
    Ok(())
}