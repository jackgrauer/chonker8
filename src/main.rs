// Hot-reload TUI for chonker8
// Main entry point that sets up the PDF viewer with hot-reload

// Use the library modules
use chonker8::core::config as ui_config;
use chonker8::display::terminal_ui as ui_renderer;
use chonker8::core::hot_reload as hot_reload_manager;

use anyhow::Result;
use clap::Parser;
use crossterm::{
    cursor::{Hide, MoveTo, Show},
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers, MouseEvent, MouseEventKind, EnableMouseCapture, DisableMouseCapture},
    execute,
    terminal::{self, Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen},
};
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use std::{
    io::{stdout, Write},
    path::{Path, PathBuf},
    sync::mpsc::{channel, Receiver},
    time::Duration,
};
use ui_config::UIConfig;
use ui_renderer::{UIRenderer, Screen};
use hot_reload_manager::HotReloadManager;
// Removed unused imports
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
    
    /// Run interactive test to verify arrow keys and text display work correctly
    #[arg(long)]
    test_ui: bool,
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
    last_render_time: std::time::Instant,
}

impl App {
    fn write_debug(message: &str) {
        // Always overwrite the same debug file (not append, for cleaner output)
        if let Ok(mut file) = std::fs::File::create("/tmp/chonker8_debug.txt") {
            use std::io::Write;
            writeln!(file, "========== CHONKER8 DEBUG LOG ==========").ok();
            writeln!(file, "Timestamp: {}", chrono::Local::now()).ok();
            writeln!(file, "\n{}", message).ok();
            writeln!(file, "=========================================").ok();
        }
    }
    
    fn new() -> Result<Self> {
        // Load initial config
        let config = UIConfig::load()?;
        let renderer = UIRenderer::new(config.clone());
        
        // Setup file watcher for ui.toml (only if it exists)
        let (tx, rx) = channel();
        let mut watcher = notify::recommended_watcher(tx)?;
        if Path::new("ui.toml").exists() {
            watcher.watch(Path::new("ui.toml"), RecursiveMode::NonRecursive)?;
        }
        
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
            last_render_time: std::time::Instant::now(),
        })
    }
    
    fn load_pdf(&mut self, path: &str) -> Result<()> {
        // eprintln!("[DEBUG] load_pdf called with: {}", path);
        // eprintln!("[DEBUG] Command line loading PDF: {}", path);
        self.pdf_path = Some(path.to_string());
        
        // Load PDF synchronously to avoid runtime issues
        let pdf_path = PathBuf::from(path);
        // eprintln!("[DEBUG] Checking if path exists: {}", pdf_path.exists());
        // eprintln!("[DEBUG] PDF path exists: {}", pdf_path.exists());
        // eprintln!("[DEBUG] Full path: {:?}", pdf_path);
        
        if pdf_path.exists() {
            // eprintln!("[DEBUG] Path exists, calling renderer.load_pdf");
            // eprintln!("[DEBUG] Path exists, calling renderer.load_pdf");
            // Load synchronously without async runtime
            match self.renderer.load_pdf(pdf_path) {
                Ok(()) => {
                    // eprintln!("[DEBUG] ✅ PDF loaded successfully: {}", path);
                    // Switch to PDF viewer screen when loading from command line
                    self.renderer.set_screen(Screen::PdfViewer);
                    self.needs_redraw = true;
                    // eprintln!("[DEBUG] Switched to PDF viewer screen");
                }
                Err(e) => {
                    // Silenced: eprintln!("[ERROR] ❌ Failed to load PDF: {}", e);
                    return Err(e);
                }
            }
        } else {
            // Silenced: eprintln!("[ERROR] ❌ PDF file does not exist: {}", path);
            return Err(anyhow::anyhow!("PDF file not found: {}", path));
        }
        
        self.needs_redraw = true;
        // eprintln!("[DEBUG] load_pdf complete, needs_redraw set");
        Ok(())
    }
    
    fn run(&mut self) -> Result<()> {
        // Setup terminal - make it resilient to non-TTY environments
        let is_tty = atty::is(atty::Stream::Stdout);
        
        if is_tty {
            terminal::enable_raw_mode()?;
            execute!(stdout(), EnterAlternateScreen, Hide, EnableMouseCapture)?;
        } else {
            // eprintln!("[DEBUG] Not a TTY, running in non-interactive mode");
        }
        
        // Initial render - only render once
        // eprintln!("[DEBUG] Initial render call");
        self.renderer.render()?;
        self.needs_redraw = false;
        
        {
            // eprintln!("[DEBUG] needs_redraw=false, no second render");
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
                        
                        // Silenced: println!("🔄 Main app rebuilt - hot-reloading...");
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
            
            // Render if needed with debouncing
            if self.needs_redraw {
                // Debounce rapid renders - only render if at least 16ms have passed (60 FPS)
                let now = std::time::Instant::now();
                let time_since_last_render = now.duration_since(self.last_render_time);
                
                if time_since_last_render >= Duration::from_millis(16) {
                    // Show cursor when in PDF viewer (for text editing), hide otherwise
                    if *self.renderer.current_screen() == Screen::PdfViewer {
                        execute!(stdout(), Show)?;
                    } else {
                        execute!(stdout(), Hide)?;
                    }
                    
                    self.renderer.render()?;
                    self.needs_redraw = false;
                    self.last_render_time = now;
                } else {
                    // Keep needs_redraw true to render on next iteration
                }
            }
            
            // Handle input only if we're in a TTY
            if is_tty {
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
            } else {
                // In non-TTY mode, just sleep briefly to avoid busy-waiting
                std::thread::sleep(Duration::from_millis(50));
                
                // Auto-exit after displaying the image for a moment
                static mut NON_TTY_COUNTER: u32 = 0;
                unsafe {
                    NON_TTY_COUNTER += 1;
                    if NON_TTY_COUNTER > 40 { // 2 seconds
                        // Silenced: eprintln!("[DEBUG] Non-TTY mode timeout, exiting gracefully");
                        self.running = false;
                    }
                }
            }
        }
        
        // Cleanup - only if we're in a TTY
        if is_tty {
            execute!(stdout(), Show, LeaveAlternateScreen, DisableMouseCapture)?;
            terminal::disable_raw_mode()?;
        }
        
        Ok(())
    }
    
    fn handle_key(&mut self, key: KeyEvent) -> Result<()> {
        // Debug screen removed
        
        // Check if we're on the PDF viewer screen and handle scrolling/editing
        let screen = self.renderer.current_screen();
        if *screen == Screen::PdfViewer {
            // Check for navigation keys FIRST (Tab, Esc)
            match key.code {
                KeyCode::Tab => {
                    // Tab always switches screens
                    self.renderer.next_screen();
                    self.needs_redraw = true;
                    return Ok(());
                }
                KeyCode::Esc => {
                    self.running = false;
                    return Ok(());
                }
                _ => {}
            }
            
            // Check for special control key that we handle directly (Save)
            if key.code == KeyCode::Char('s') && key.modifiers.contains(KeyModifiers::CONTROL) {
                // Save edited text
                if let Some(pdf_path) = self.renderer.current_pdf_path.clone() {
                    let txt_path = pdf_path.with_extension("edited.txt");
                    if let Err(e) = self.renderer.save_edited_text(&txt_path) {
                        // Silenced: eprintln!("Failed to save: {}", e);
                    } else {
                        // Silenced: eprintln!("Saved edited text to {:?}", txt_path);
                    }
                }
                return Ok(());
            }
            
            // Pass all keyboard input to Excel grid for advanced editing
            let shift_held = key.modifiers.contains(KeyModifiers::SHIFT);
            let ctrl_held = key.modifiers.contains(KeyModifiers::CONTROL);
            let alt_held = key.modifiers.contains(KeyModifiers::ALT);
            
            // Only redraw for keys that actually change the display
            match key.code {
                // Ctrl+F for search needs special handling
                KeyCode::Char('f') if ctrl_held => {
                    self.renderer.handle_grid_input_with_modifiers(key.code, shift_held, ctrl_held, alt_held);
                    self.needs_redraw = true;
                }
                // These keys always need redraw
                KeyCode::Char(_) | KeyCode::Backspace | KeyCode::Delete | KeyCode::Enter => {
                    self.renderer.handle_grid_input_with_modifiers(key.code, shift_held, ctrl_held, alt_held);
                    self.needs_redraw = true;
                }
                // F3 for search
                KeyCode::F(3) => {
                    self.renderer.handle_grid_input_with_modifiers(key.code, shift_held, ctrl_held, alt_held);
                    self.needs_redraw = true;
                }
                // Arrow keys and selection only redraw if something changes
                KeyCode::Up | KeyCode::Down | KeyCode::Left | KeyCode::Right |
                KeyCode::Home | KeyCode::End | KeyCode::PageUp | KeyCode::PageDown => {
                    let old_cursor = self.renderer.get_grid_cursor();
                    let old_selecting = self.renderer.is_selecting();
                    
                    self.renderer.handle_grid_input_with_modifiers(key.code, shift_held, ctrl_held, alt_held);
                    
                    let new_cursor = self.renderer.get_grid_cursor();
                    let new_selecting = self.renderer.is_selecting();
                    
                    // Only redraw if cursor actually moved or selection changed
                    if old_cursor != new_cursor || old_selecting != new_selecting {
                        self.needs_redraw = true;
                    }
                }
                // Escape only redraws if there was a selection or searching
                KeyCode::Esc => {
                    let was_selecting = self.renderer.is_selecting();
                    let was_searching = self.renderer.is_searching();
                    self.renderer.handle_grid_input_with_modifiers(key.code, shift_held, ctrl_held, alt_held);
                    if was_selecting || was_searching {
                        self.needs_redraw = true;
                    }
                }
                // Ignore other keys
                _ => {
                    self.renderer.handle_grid_input_with_modifiers(key.code, shift_held, ctrl_held, alt_held);
                    // Don't set needs_redraw for other keys
                }
            }
            return Ok(());
        }
        
        // Check if we're on the file picker screen and handle file picker input
        if *self.renderer.current_screen() == Screen::FilePicker {
            // Try to handle file picker input
            if let Some(selected_file) = self.renderer.handle_file_picker_input(key)? {
                // Load the selected PDF and switch to PDF viewer
                // Try to load the PDF
                if let Err(e) = self.load_pdf(&selected_file) {
                    // Clear screen and show error in a clean way
                    use crossterm::{execute, terminal::{Clear, ClearType}, cursor::MoveTo};
                    use std::io::stdout;
                    
                    // Temporarily disable raw mode for clean output
                    crossterm::terminal::disable_raw_mode().ok();
                    execute!(stdout(), Clear(ClearType::All), MoveTo(0, 0)).ok();
                    
                    println!("\n========== PDF LOAD ERROR ==========\n");
                    println!("File: {}\n", selected_file);
                    println!("Error: {}\n", e);
                    println!("========================================");
                    
                    // Write debug info to the single debug file
                    let debug_msg = format!(
                        "PDF LOAD ERROR\nFile: {}\n\nError Details:\n{}",
                        selected_file, e
                    );
                    Self::write_debug(&debug_msg);
                    println!("\nDebug info written to: /tmp/chonker8_debug.txt");
                    
                    println!("\nPress Enter to return to file picker...");
                    
                    // Flush to ensure output appears
                    std::io::Write::flush(&mut stdout()).ok();
                    
                    // Wait for Enter key
                    let mut input = String::new();
                    std::io::stdin().read_line(&mut input).ok();
                    
                    // Re-enable raw mode and force redraw
                    crossterm::terminal::enable_raw_mode().ok();
                    execute!(stdout(), Clear(ClearType::All)).ok();
                    self.needs_redraw = true;
                    
                    // Stay on file picker screen
                    return Ok(());
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
        // Use enhanced mouse handling with zone detection
        let screen = self.renderer.current_screen();
        if *screen == Screen::PdfViewer {
            // Use the new enhanced mouse handler
            let needs_redraw = self.renderer.handle_mouse_enhanced(mouse);
            if needs_redraw {
                self.needs_redraw = true;
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
    
    fn call_pdf_processor(&self, _pdf_path: &str, _page: usize) -> Result<Vec<Vec<char>>> {
        // Just return fallback content for now since pdf-processor doesn't exist
        Ok(self.get_fallback_content())
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
    // Parse command line arguments using clap
    let args = Args::parse();
    
    // Handle test mode
    if args.test_kitty {
        eprintln!("Testing Kitty graphics protocol...");
        if std::env::var("KITTY_WINDOW_ID").is_ok() {
            eprintln!("✅ Kitty graphics protocol detected");
            eprintln!("  KITTY_WINDOW_ID={}", std::env::var("KITTY_WINDOW_ID").unwrap());
        } else {
            eprintln!("❌ Kitty graphics protocol not detected");
            eprintln!("  Run this in a Kitty terminal for graphics support");
        }
        return Ok(());
    }
    
    // Handle UI test mode
    if args.test_ui {
        eprintln!("╔══════════════════════════════════════════════════════════╗");
        eprintln!("║          CHONKER8-HOT UI TEST MODE                        ║");
        eprintln!("╠══════════════════════════════════════════════════════════╣");
        eprintln!("║ Testing the following fixes:                              ║");
        eprintln!("║ 1. PDF image should NOT disappear when arrow keys pressed ║");
        eprintln!("║ 2. Text extraction should display on the right side       ║");
        eprintln!("║ 3. Text should be editable with Excel-style grid          ║");
        eprintln!("╠══════════════════════════════════════════════════════════╣");
        eprintln!("║ Controls to test:                                         ║");
        eprintln!("║ • Arrow Keys: Move cursor (image should stay visible)     ║");
        eprintln!("║ • Shift+Arrows: Select text blocks                        ║");
        eprintln!("║ • Type: Edit text at cursor position                      ║");
        eprintln!("║ • Ctrl+S: Save edited text                                ║");
        eprintln!("║ • Tab: Switch between screens                             ║");
        eprintln!("║ • Esc: Exit                                               ║");
        eprintln!("╠══════════════════════════════════════════════════════════╣");
        eprintln!("║ Expected behavior:                                        ║");
        eprintln!("║ ✓ Left panel shows PDF image (dark mode filtered)         ║");
        eprintln!("║ ✓ Right panel shows extracted text                        ║");
        eprintln!("║ ✓ Arrow keys move cursor without hiding PDF              ║");
        eprintln!("║ ✓ Text can be edited and selected                        ║");
        eprintln!("╚══════════════════════════════════════════════════════════╝");
        eprintln!();
        eprintln!("Starting in 2 seconds...");
        std::thread::sleep(Duration::from_secs(2));
    }
    
    
    // Create app
    let mut app = App::new()?;
    
    // Handle test mode PDF loading
    let pdf_to_load = if args.test_ui {
        // In test mode, try to find a test PDF
        if PathBuf::from("test.pdf").exists() {
            eprintln!("✓ Loading test.pdf for UI testing");
            Some(PathBuf::from("test.pdf"))
        } else if PathBuf::from("real_test.pdf").exists() {
            eprintln!("✓ Loading real_test.pdf for UI testing");
            Some(PathBuf::from("real_test.pdf"))
        } else if let Some(ref pdf) = args.pdf_file {
            eprintln!("✓ Loading {} for UI testing", pdf.display());
            Some(pdf.clone())
        } else {
            eprintln!("⚠ No test PDF found, will use file picker");
            eprintln!("  Tip: Create test.pdf or pass a PDF file as argument");
            None
        }
    } else {
        args.pdf_file
    };
    
    // Load PDF if provided, or use default test PDF
    if let Some(pdf_path) = pdf_to_load {
        if !args.test_ui {
            // eprintln!("[INFO] A/B Comparison Mode:");
            // eprintln!("[INFO] Left pane: lopdf-kitty rendering");
            // eprintln!("[INFO] Right pane: pdftotext extraction");
        }
        app.load_pdf(&pdf_path.to_string_lossy())?;
    } else {
        // Auto-load the test PDF for easier development
        let test_pdf = PathBuf::from("/Users/jack/Documents/chonker_test.pdf");
        if test_pdf.exists() {
            // eprintln!("[INFO] Auto-loading test PDF: {:?}", test_pdf);
            // eprintln!("[INFO] A/B Comparison Mode:");
            // eprintln!("[INFO] Left pane: lopdf-kitty rendering");
            // eprintln!("[INFO] Right pane: pdftotext extraction");
            app.load_pdf(&test_pdf.to_string_lossy())?;
        } else {
        // Silenced usage help - just start quietly
        // println!("Usage: chonker8-hot [pdf_file]");
        // println!("       chonker8-hot --help");
        // println!("       chonker8-hot --test-kitty");
        // println!("       chonker8-hot --test-vello [pdf_file]");
        // println!("\nStarting in demo mode...");
        
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
    }
    
    // Run the app
    app.run()?;
    
    // Show test report if in test mode
    if args.test_ui {
        eprintln!();
        eprintln!("╔══════════════════════════════════════════════════════════╗");
        eprintln!("║               TEST COMPLETE                               ║");
        eprintln!("╠══════════════════════════════════════════════════════════╣");
        eprintln!("║ Please verify the following worked correctly:             ║");
        eprintln!("║                                                            ║");
        eprintln!("║ ✓ PDF image remained visible when using arrow keys?       ║");
        eprintln!("║ ✓ Text extraction displayed on the right panel?           ║");
        eprintln!("║ ✓ Text cursor moved with arrow keys?                      ║");
        eprintln!("║ ✓ Text could be edited by typing?                         ║");
        eprintln!("║ ✓ Shift+arrows selected text blocks?                      ║");
        eprintln!("║                                                            ║");
        eprintln!("║ If any of these didn't work, please report the issue.     ║");
        eprintln!("╚══════════════════════════════════════════════════════════╝");
    } else {
        // Silenced: println!("Thanks for using Chonker8!");
    }
    Ok(())
}