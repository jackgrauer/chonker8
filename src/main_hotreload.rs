// Hot-reload TUI for chonker8
mod ui_config;
mod ui_renderer;
mod pdf_extraction;
mod config;
mod hot_reload_manager;
mod build_system;
mod output_capture;

use anyhow::Result;
use clap::Parser;
use crossterm::{
    cursor::{Hide, MoveTo, Show},
    event::{self, Event, KeyCode, KeyEvent, MouseEvent, MouseEventKind, EnableMouseCapture, DisableMouseCapture},
    execute,
    terminal::{self, Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen},
};
use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher};
use std::{
    io::{stdout, Write},
    path::{Path, PathBuf},
    sync::mpsc::{channel, Receiver},
    time::Duration,
};
use ui_config::UIConfig;
use ui_renderer::{UIRenderer, Screen};
use hot_reload_manager::HotReloadManager;
use std::process::{Command, Stdio};
// use chonker8::integrated_file_picker::IntegratedFilePicker; // Unused import

#[derive(Parser, Debug)]
#[command(name = "chonker8-hot")]
#[command(version = "8.8.0")]
#[command(about = "A/B PDF comparison viewer - Visual quality assessment tool", long_about = None)]
struct Args {
    /// PDF file to display for A/B comparison (left: rendered PDF, right: pdftotext extraction)
    pdf_file: Option<PathBuf>,
    
    /// Test Kitty graphics protocol detection
    #[arg(long)]
    test_kitty: bool,
}

struct App {
    config: UIConfig,
    renderer: UIRenderer,
    config_watcher: RecommendedWatcher,
    config_rx: Receiver<notify::Result<notify::Event>>,
    hot_reload_manager: HotReloadManager,
    needs_redraw: bool,
    pdf_path: Option<String>,
    running: bool,
    last_processor_version: String,
}

impl App {
    fn new() -> Result<Self> {
        // Load initial config
        let config = UIConfig::load()?;
        let renderer = UIRenderer::new(config.clone());
        
        // Setup file watcher for ui.toml
        let (tx, rx) = channel();
        let mut watcher = notify::recommended_watcher(tx)?;
        watcher.watch(Path::new("ui.toml"), RecursiveMode::NonRecursive)?;
        
        // Setup hot-reload manager for Rust code
        let hot_reload_manager = HotReloadManager::new()?;
        
        Ok(Self {
            config,
            renderer,
            config_watcher: watcher,
            config_rx: rx,
            hot_reload_manager,
            needs_redraw: true,
            pdf_path: None,
            running: true,
            last_processor_version: String::new(),
        })
    }
    
    fn load_pdf(&mut self, path: &str) -> Result<()> {
        capture_debug!("load_pdf called with: {}", path);
        eprintln!("[DEBUG] Command line loading PDF: {}", path);
        self.pdf_path = Some(path.to_string());
        
        // Load PDF synchronously to avoid runtime issues
        let pdf_path = PathBuf::from(path);
        capture_debug!("Checking if path exists: {}", pdf_path.exists());
        eprintln!("[DEBUG] PDF path exists: {}", pdf_path.exists());
        eprintln!("[DEBUG] Full path: {:?}", pdf_path);
        
        if pdf_path.exists() {
            capture_debug!("Path exists, calling renderer.load_pdf");
            eprintln!("[DEBUG] Path exists, calling renderer.load_pdf");
            // Load synchronously without async runtime
            match self.renderer.load_pdf(pdf_path) {
                Ok(()) => {
                    eprintln!("[DEBUG] ✅ PDF loaded successfully: {}", path);
                    // Switch to PDF viewer screen when loading from command line
                    self.renderer.set_screen(Screen::PdfViewer);
                    self.needs_redraw = true;
                    eprintln!("[DEBUG] Switched to PDF viewer screen");
                }
                Err(e) => {
                    eprintln!("[ERROR] ❌ Failed to load PDF: {}", e);
                    return Err(e);
                }
            }
        } else {
            eprintln!("[ERROR] ❌ PDF file does not exist: {}", path);
            return Err(anyhow::anyhow!("PDF file not found: {}", path));
        }
        
        self.needs_redraw = true;
        eprintln!("[DEBUG] load_pdf complete, needs_redraw set");
        Ok(())
    }
    
    fn run(&mut self) -> Result<()> {
        // Setup terminal - make it resilient to non-TTY environments
        let is_tty = atty::is(atty::Stream::Stdout);
        
        if is_tty {
            terminal::enable_raw_mode()?;
            execute!(stdout(), EnterAlternateScreen, Hide, EnableMouseCapture)?;
        } else {
            eprintln!("[DEBUG] Not a TTY, skipping terminal setup");
        }
        
        // Initial render
        eprintln!("[DEBUG] Initial render call");
        self.renderer.render()?;
        
        // If PDF was loaded before run, render again with correct screen
        if self.needs_redraw {
            eprintln!("[DEBUG] needs_redraw=true, rendering again");
            self.renderer.render()?;
            self.needs_redraw = false;
        } else {
            eprintln!("[DEBUG] needs_redraw=false, no second render");
        }
        
        // Main loop
        while self.running {
            // Check for config file changes (hot-reload)
            if let Ok(Ok(event)) = self.config_rx.try_recv() {
                if matches!(event.kind, notify::EventKind::Modify(_)) {
                    // Reload config
                    if let Ok(new_config) = UIConfig::load() {
                        self.config = new_config.clone();
                        self.renderer.update_config(new_config);
                        self.needs_redraw = true;
                        
                        // Flash a message to show reload happened
                        execute!(
                            stdout(),
                            crossterm::cursor::MoveTo(0, 0),
                            crossterm::style::Print("✨ Config reloaded!")
                        )?;
                    }
                }
            }
            
            // Check for Rust code changes (automatic hot-reload)
            if let Ok(Some(build_result)) = self.hot_reload_manager.check_for_changes() {
                if build_result.success {
                    if build_result.should_restart {
                        // Main app needs restart - clean up terminal first
                        execute!(stdout(), Show, LeaveAlternateScreen)?;
                        terminal::disable_raw_mode()?;
                        
                        println!("🔄 Main app rebuilt - hot-reloading...");
                        std::thread::sleep(Duration::from_millis(100)); // Brief pause
                        
                        // Restart the application
                        HotReloadManager::restart_app();
                    } else {
                        // Just external processor reload
                        if let Some(pdf_path) = &self.pdf_path.clone() {
                            self.reload_pdf_content(pdf_path)?;
                        } else {
                            self.update_demo_content()?;
                        }
                        
                        // Show brief success message
                        execute!(
                            stdout(),
                            crossterm::cursor::MoveTo(0, 1),
                            crossterm::style::Print(&format!("🔥 Auto-reloaded! ({:?})", build_result.build_time))
                        )?;
                        self.needs_redraw = true;
                        
                        // Clear the message after a moment
                        std::thread::sleep(Duration::from_millis(500));
                        execute!(
                            stdout(),
                            crossterm::cursor::MoveTo(0, 1),
                            crossterm::style::Print(" ".repeat(50))
                        )?;
                    }
                } else {
                    execute!(
                        stdout(),
                        crossterm::cursor::MoveTo(0, 1),
                        crossterm::style::Print("❌ Build failed - check terminal for errors")
                    )?;
                }
            }
            
            // Render if needed
            if self.needs_redraw {
                // Pass the file picker reference to the renderer for file picker screen
                // The renderer now has its own integrated file picker
                self.renderer.render()?;
                self.needs_redraw = false;
            }
            
            // Handle input (non-blocking with timeout)
            if event::poll(Duration::from_millis(50))? {
                match event::read()? {
                    Event::Key(key) => self.handle_key(key)?,
                    Event::Mouse(mouse) => self.handle_mouse(mouse)?,
                    Event::Resize(_, _) => {
                        // Complete screen reset on resize
                        execute!(
                            stdout(), 
                            Clear(ClearType::All),
                            Clear(ClearType::Purge),
                            MoveTo(0, 0)
                        )?;
                        stdout().flush()?;
                        // Small delay to let terminal catch up
                        std::thread::sleep(Duration::from_millis(10));
                        self.needs_redraw = true;
                    },
                    _ => {}
                }
            }
        }
        
        // Cleanup
        execute!(stdout(), Show, LeaveAlternateScreen, DisableMouseCapture)?;
        terminal::disable_raw_mode()?;
        
        Ok(())
    }
    
    fn handle_key(&mut self, key: KeyEvent) -> Result<()> {
        // Check if we're on the DEBUG screen and handle scrolling
        if *self.renderer.current_screen() == Screen::Debug {
            match key.code {
                KeyCode::Up => {
                    self.renderer.scroll_debug_up();
                    self.needs_redraw = true;
                    return Ok(());
                }
                KeyCode::Down => {
                    self.renderer.scroll_debug_down();
                    self.needs_redraw = true;
                    return Ok(());
                }
                KeyCode::PageUp => {
                    self.renderer.scroll_debug_page_up();
                    self.needs_redraw = true;
                    return Ok(());
                }
                KeyCode::PageDown => {
                    self.renderer.scroll_debug_page_down();
                    self.needs_redraw = true;
                    return Ok(());
                }
                KeyCode::Home => {
                    self.renderer.scroll_debug_to_top();
                    self.needs_redraw = true;
                    return Ok(());
                }
                KeyCode::End => {
                    self.renderer.scroll_debug_to_bottom();
                    self.needs_redraw = true;
                    return Ok(());
                }
                _ => {}
            }
        }
        
        // Check if we're on the PDF viewer screen OR Demo screen (which also shows PDF) and handle scrolling
        let screen = self.renderer.current_screen();
        if *screen == Screen::PdfViewer || *screen == Screen::Demo {
            match key.code {
                KeyCode::Up => {
                    self.renderer.scroll_up();
                    self.needs_redraw = true;
                    return Ok(());
                }
                KeyCode::Down => {
                    self.renderer.scroll_down();
                    self.needs_redraw = true;
                    return Ok(());
                }
                KeyCode::PageUp => {
                    self.renderer.prev_page();
                    self.needs_redraw = true;
                    return Ok(());
                }
                KeyCode::PageDown => {
                    self.renderer.next_page();
                    self.needs_redraw = true;
                    return Ok(());
                }
                _ => {}
            }
        }
        
        // Check if we're on the file picker screen and handle file picker input
        if *self.renderer.current_screen() == Screen::FilePicker {
            // Try to handle file picker input
            if let Some(selected_file) = self.renderer.handle_file_picker_input(key)? {
                // Load the selected PDF and switch to PDF viewer
                if let Err(e) = self.load_pdf(&selected_file) {
                    eprintln!("Failed to load PDF: {}", e);
                }
                self.renderer.set_screen(Screen::PdfViewer);
                self.needs_redraw = true;
                return Ok(());
            }
            
            // Check for navigation keys even on file picker screen
            match key.code {
                KeyCode::Tab => {
                    self.renderer.next_screen();
                    self.needs_redraw = true;
                    return Ok(());
                }
                KeyCode::Esc => {
                    self.running = false;
                    return Ok(());
                }
                _ => {
                    // File picker handled the input, need redraw
                    self.needs_redraw = true;
                    return Ok(());
                }
            }
        }
        
        // Handle global navigation keys
        match key.code {
            KeyCode::Tab => {
                self.renderer.next_screen();
                self.needs_redraw = true;
            }
            KeyCode::Esc => {
                self.running = false;
            }
            _ => {}
        }
        
        Ok(())
    }
    
    fn handle_mouse(&mut self, mouse: MouseEvent) -> Result<()> {
        // Handle mouse wheel scrolling on DEBUG screen
        if *self.renderer.current_screen() == Screen::Debug {
            match mouse.kind {
                MouseEventKind::ScrollUp => {
                    self.renderer.scroll_debug_up();
                    self.needs_redraw = true;
                }
                MouseEventKind::ScrollDown => {
                    self.renderer.scroll_debug_down();
                    self.needs_redraw = true;
                }
                // Explicitly ignore all other mouse events to prevent terminal corruption
                MouseEventKind::Moved => {
                    // Ignore mouse movement
                }
                MouseEventKind::Down(_) => {
                    // Ignore mouse button presses
                }
                MouseEventKind::Up(_) => {
                    // Ignore mouse button releases
                }
                MouseEventKind::Drag(_) => {
                    // Ignore mouse drag
                }
                _ => {
                    // Ignore any other mouse events
                }
            }
        }
        
        // Handle mouse wheel scrolling on PDF viewer screen OR Demo screen
        let screen = self.renderer.current_screen();
        if *screen == Screen::PdfViewer || *screen == Screen::Demo {
            match mouse.kind {
                MouseEventKind::ScrollUp => {
                    self.renderer.scroll_up();
                    self.needs_redraw = true;
                }
                MouseEventKind::ScrollDown => {
                    self.renderer.scroll_down();
                    self.needs_redraw = true;
                }
                // Explicitly ignore all other mouse events to prevent terminal corruption
                MouseEventKind::Moved => {
                    // Ignore mouse movement
                }
                MouseEventKind::Down(_) => {
                    // Ignore mouse button presses
                }
                MouseEventKind::Up(_) => {
                    // Ignore mouse button releases
                }
                MouseEventKind::Drag(_) => {
                    // Ignore mouse drag
                }
                _ => {
                    // Ignore any other mouse events
                }
            }
        }
        
        Ok(())
    }
    
    fn reload_pdf_content(&mut self, pdf_path: &str) -> Result<()> {
        // Use the hot-reloaded PDF processor
        let content = self.call_pdf_processor(pdf_path, 1)?;
        self.renderer.set_pdf_content(content);
        Ok(())
    }
    
    fn update_demo_content(&mut self) -> Result<()> {
        // Get fresh demo content from the hot-reloaded processor
        let content = self.call_pdf_processor("demo.pdf", 1)?;
        self.renderer.set_pdf_content(content);
        Ok(())
    }
    
    fn call_pdf_processor(&self, pdf_path: &str, page: usize) -> Result<Vec<Vec<char>>> {
        // Call the external PDF processor binary
        let output = Command::new("./target/release/pdf-processor")
            .args(&["process", pdf_path, &page.to_string()])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output();
        
        match output {
            Ok(output) if output.status.success() => {
                // Parse the output back into a character grid
                let stdout = String::from_utf8_lossy(&output.stdout);
                let lines: Vec<&str> = stdout.lines().collect();
                
                let mut grid = vec![vec![' '; 80]; 24];
                for (i, line) in lines.iter().enumerate() {
                    if i < grid.len() {
                        for (j, ch) in line.chars().enumerate() {
                            if j < grid[i].len() {
                                grid[i][j] = ch;
                            }
                        }
                    }
                }
                Ok(grid)
            },
            _ => {
                // Fallback to original demo content if processor fails
                Ok(self.get_fallback_content())
            }
        }
    }
    
    fn get_fallback_content(&self) -> Vec<Vec<char>> {
        let mut content = vec![vec![' '; 80]; 24];
        let lines = vec![
            "╔══════════════════════════════════════╗",
            "║  Chonker8.1 - PDF Processor Offline ║",
            "╠══════════════════════════════════════╣",
            "║                                      ║",
            "║  Hot-reload system is starting...    ║",
            "║                                      ║",
            "║  The PDF processor is being built.   ║",
            "║  This content will update once the   ║",
            "║  build completes successfully.       ║",
            "║                                      ║",
            "╚══════════════════════════════════════╝",
        ];
        
        for (i, line) in lines.iter().enumerate() {
            for (j, ch) in line.chars().enumerate() {
                if j < 80 {
                    content[i][j] = ch;
                }
            }
        }
        content
    }
    
    // Removed old file picker launch methods - file picker is now integrated as a screen
}

fn main() -> Result<()> {
    // Initialize output capture system for rexpect testing
    output_capture::initialize_output_capture();
    
    // Parse command line arguments using clap
    let args = Args::parse();
    
    // Handle test mode
    if args.test_kitty {
        capture_info!("Testing Kitty graphics protocol...");
        if std::env::var("KITTY_WINDOW_ID").is_ok() {
            capture_info!("✅ Kitty graphics protocol detected");
            capture_info!("  KITTY_WINDOW_ID={}", std::env::var("KITTY_WINDOW_ID").unwrap());
        } else {
            capture_warning!("❌ Kitty graphics protocol not detected");
            capture_warning!("  Run this in a Kitty terminal for graphics support");
        }
        return Ok(());
    }
    
    // Create app
    let mut app = App::new()?;
    
    // Load PDF if provided
    if let Some(pdf_path) = args.pdf_file {
        eprintln!("[INFO] A/B Comparison Mode:");
        eprintln!("[INFO] Left pane: lopdf-vello-kitty rendering");
        eprintln!("[INFO] Right pane: pdftotext extraction");
        app.load_pdf(&pdf_path.to_string_lossy())?;
    } else {
        // Show usage
        println!("Usage: chonker8-hot [pdf_file]");
        println!("       chonker8-hot --help");
        println!("       chonker8-hot --test-kitty");
        println!("\nStarting in demo mode...");
        
        // Create demo content for page 1
        let mut demo_content = vec![vec![' '; 80]; 24];
        let lines = vec![
            "╔══════════════════════════════════════╗",
            "║  Chonker8.1 Hot-Reload TUI Demo     ║",
            "╠══════════════════════════════════════╣",
            "║                                      ║",
            "║  Three-screen hot-reload TUI         ║",
            "║                                      ║",
            "║  🎮 Controls:                        ║",
            "║    Tab - Cycle screens               ║",
            "║    Esc - Exit                        ║",
            "║                                      ║",
            "║  🖥️  Available screens:               ║",
            "║    1. Demo (current)                 ║",
            "║    2. File Picker                    ║",
            "║    3. PDF Viewer                     ║",
            "║                                      ║",
            "║  Ready for chonker7 UI integration! ║",
            "║                                      ║",
            "║  ▶️ Press Tab to continue             ║",
            "║                                      ║",
            "╚══════════════════════════════════════╝",
        ];
        
        for (i, line) in lines.iter().enumerate() {
            for (j, ch) in line.chars().enumerate() {
                if j < 80 {
                    demo_content[i][j] = ch;
                }
            }
        }
        
        app.renderer.set_pdf_content(demo_content);
        app.renderer.set_total_pages(2);
    }
    
    // Run the app
    app.run()?;
    
    println!("Thanks for using Chonker8!");
    Ok(())
}