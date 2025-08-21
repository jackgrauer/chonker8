// Hot-reload TUI for chonker8
mod ui_config;
mod ui_renderer;
mod pdf_extraction;
mod config;
mod hot_reload_manager;
mod build_system;

use anyhow::Result;
use crossterm::{
    cursor::{Hide, MoveTo, Show},
    event::{self, Event, KeyCode, KeyEvent},
    execute,
    terminal::{self, Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen},
};
use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher};
use std::{
    io::{stdout, Write},
    path::Path,
    sync::mpsc::{channel, Receiver},
    time::Duration,
};
use ui_config::UIConfig;
use ui_renderer::UIRenderer;
use hot_reload_manager::HotReloadManager;
use std::process::{Command, Stdio};

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
        println!("Loading PDF: {}", path);
        self.pdf_path = Some(path.to_string());
        
        // Try to extract text using our PDF extraction
        let pdf_path = Path::new(path);
        if pdf_path.exists() {
            // For now, use a simple extraction
            let _page_count = pdf_extraction::basic::get_page_count(pdf_path)?;
            
            // Create a simple runtime for the async call
            let rt = tokio::runtime::Runtime::new()?;
            
            // Try to render the first page
            match rt.block_on(pdf_extraction::true_visual::render_true_visual(pdf_path, 0, 80, 24)) {
                Ok(grid) => {
                    self.renderer.set_pdf_content(grid);
                }
                Err(e) => {
                    eprintln!("Failed to render PDF: {}", e);
                    // Set some default content
                    let mut default_content = vec![vec![' '; 80]; 24];
                    let msg = format!("Error loading PDF: {}", e);
                    for (i, ch) in msg.chars().enumerate() {
                        if i < 80 {
                            default_content[0][i] = ch;
                        }
                    }
                    self.renderer.set_pdf_content(default_content);
                }
            }
        }
        
        self.needs_redraw = true;
        Ok(())
    }
    
    fn run(&mut self) -> Result<()> {
        // Setup terminal
        terminal::enable_raw_mode()?;
        execute!(stdout(), EnterAlternateScreen, Hide)?;
        
        // Initial render
        self.renderer.render()?;
        
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
                    // Automatically update content with new processor
                    if let Some(pdf_path) = &self.pdf_path.clone() {
                        self.reload_pdf_content(pdf_path)?;
                    } else {
                        // Update demo content with new processor
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
                self.renderer.render()?;
                self.needs_redraw = false;
            }
            
            // Handle input (non-blocking with timeout)
            if event::poll(Duration::from_millis(50))? {
                match event::read()? {
                    Event::Key(key) => self.handle_key(key)?,
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
        execute!(stdout(), Show, LeaveAlternateScreen)?;
        terminal::disable_raw_mode()?;
        
        Ok(())
    }
    
    fn handle_key(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Char(c) => {
                // Check against configurable hotkeys
                let c_str = c.to_string();
                
                if c_str == self.config.hotkeys.quit {
                    self.running = false;
                } else if c_str == self.config.hotkeys.next_page {
                    self.renderer.next_page();
                    self.needs_redraw = true;
                } else if c_str == self.config.hotkeys.prev_page {
                    self.renderer.prev_page();
                    self.needs_redraw = true;
                } else if c_str == self.config.hotkeys.toggle_wrap {
                    self.renderer.toggle_wrap();
                    self.needs_redraw = true;
                } else if c_str == self.config.hotkeys.toggle_mode {
                    self.renderer.toggle_mode();
                    self.needs_redraw = true;
                } else if c_str == self.config.hotkeys.reload_config {
                    // Manual reload
                    if let Ok(new_config) = UIConfig::load() {
                        self.config = new_config.clone();
                        self.renderer.update_config(new_config);
                        self.needs_redraw = true;
                    }
                } else if c_str == "R" {
                    // Manual PDF processor reload (uppercase R)
                    if let Ok(()) = self.update_demo_content() {
                        self.needs_redraw = true;
                        execute!(
                            stdout(),
                            crossterm::cursor::MoveTo(0, 1),
                            crossterm::style::Print("🔥 PDF Processor manually reloaded!")
                        )?;
                    }
                }
            }
            KeyCode::Up => {
                self.renderer.scroll_up();
                self.needs_redraw = true;
            }
            KeyCode::Down => {
                self.renderer.scroll_down();
                self.needs_redraw = true;
            }
            KeyCode::Tab => {
                self.renderer.next_page();
                self.needs_redraw = true;
            }
            KeyCode::Esc => {
                self.running = false;
            }
            _ => {}
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
            .env("DYLD_LIBRARY_PATH", "./lib")
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
}

fn main() -> Result<()> {
    // Parse command line arguments
    let args: Vec<String> = std::env::args().collect();
    
    // Create app
    let mut app = App::new()?;
    
    // Load PDF if provided
    if args.len() > 1 {
        app.load_pdf(&args[1])?;
    } else {
        // Load a test PDF or show help
        println!("Usage: chonker8 [pdf_file]");
        println!("Starting in demo mode...");
        
        // Create demo content for page 1
        let mut demo_content = vec![vec![' '; 80]; 24];
        let lines = vec![
            "╔══════════════════════════════════════╗",
            "║  Chonker8.1 Hot-Reload TUI Demo     ║",
            "╠══════════════════════════════════════╣",
            "║                                      ║",
            "║  Edit ui.toml to see live changes!  ║",
            "║                                      ║",
            "║  Hotkeys:                            ║",
            "║    q - quit                          ║",
            "║    Tab - next page                   ║",
            "║    n - next page                     ║",
            "║    p - previous page                 ║",
            "║    m - toggle mode                   ║",
            "║    w - toggle wrap                   ║",
            "║    r - reload config                 ║",
            "║                                      ║",
            "║  Try changing:                       ║",
            "║    - mode = \"full\"                   ║",
            "║    - highlight = \"green\"             ║",
            "║    - border = \"sharp\"                ║",
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