// Hot-reload manager for Rust code changes
use anyhow::Result;
use notify::{RecommendedWatcher, RecursiveMode, Watcher, Event, EventKind};
use chrono;
use std::{
    path::Path,
    process::{Command, Stdio},
    sync::mpsc::{channel, Receiver, Sender},
    thread,
    time::{Duration, Instant},
    env,
    os::unix::process::CommandExt,
};

/// Strip ANSI escape codes from string to prevent ribbon output
fn strip_ansi_codes(s: &str) -> String {
    let mut result = String::new();
    let mut chars = s.chars().peekable();
    
    while let Some(c) = chars.next() {
        if c == '\x1b' {
            // Skip the escape sequence
            if chars.peek() == Some(&'[') {
                chars.next(); // consume '['
                // Skip until we find a letter (end of sequence)
                while let Some(ch) = chars.next() {
                    if ch.is_alphabetic() {
                        break;
                    }
                }
            }
        } else {
            result.push(c);
        }
    }
    result
}

pub struct HotReloadManager {
    watcher: RecommendedWatcher,
    file_rx: Receiver<notify::Result<Event>>,
    build_req_tx: Sender<BuildRequest>,
    build_result_rx: Receiver<BuildResult>,
    last_build: Instant,
    build_debounce: Duration,
}

#[derive(Debug)]
pub struct BuildRequest {
    pub target: String,
    pub features: Vec<String>,
}

#[derive(Debug)]
pub struct BuildResult {
    pub success: bool,
    pub output: String,
    pub binary_path: Option<String>,
    pub build_time: Duration,
    pub should_restart: bool,
}

impl HotReloadManager {
    pub fn new() -> Result<Self> {
        let (file_tx, file_rx) = channel();
        let (build_req_tx, build_req_rx) = channel();
        let (build_result_tx, build_result_rx) = channel();
        
        // Setup file watcher for Rust source files
        let mut watcher = notify::recommended_watcher(file_tx)?;
        watcher.watch(Path::new("src/"), RecursiveMode::Recursive)?;
        watcher.watch(Path::new("Cargo.toml"), RecursiveMode::NonRecursive)?;
        
        // Start background build thread
        thread::spawn(move || {
            Self::build_worker(build_req_rx, build_result_tx);
        });
        
        Ok(Self {
            watcher,
            file_rx,
            build_req_tx,
            build_result_rx,
            last_build: Instant::now() - Duration::from_secs(60), // Allow immediate first build
            build_debounce: Duration::from_millis(500), // Wait 500ms after last file change
        })
    }
    
    pub fn check_for_changes(&mut self) -> Result<Option<BuildResult>> {
        // Check for file changes - more aggressive detection
        let mut needs_rebuild = false;
        let mut changed_files = Vec::new();
        
        // Process all pending file events
        while let Ok(Ok(event)) = self.file_rx.try_recv() {
            // Accept any kind of file event (not just Modify/Create)
            if matches!(event.kind, 
                EventKind::Modify(_) | 
                EventKind::Create(_) | 
                EventKind::Access(_) |
                EventKind::Other
            ) {
                for path in &event.paths {
                    // Check if it's a Rust file we care about
                    if let Some(ext) = path.extension() {
                        if ext == "rs" || path.file_name() == Some(std::ffi::OsStr::new("Cargo.toml")) {
                            // Special focus on pdf_processor.rs
                            if path.to_string_lossy().contains("pdf_processor.rs") {
                                changed_files.push(path.clone());
                                needs_rebuild = true;
                                println!("🔥 Detected change in pdf_processor.rs - triggering immediate rebuild");
                            } else if ext == "rs" {
                                changed_files.push(path.clone());
                                needs_rebuild = true;
                            }
                        }
                    }
                }
            }
        }
        
        // Immediate rebuild on any change - zero debouncing
        if needs_rebuild {
            println!("🔄 Files changed: {:?}", changed_files);
            
            // Check if main app files changed (requires full restart)
            let main_app_changed = changed_files.iter().any(|path| {
                let path_str = path.to_string_lossy();
                path_str.contains("main_hotreload.rs") ||
                path_str.contains("ui_renderer.rs") ||
                path_str.contains("integrated_file_picker.rs") ||
                path_str.contains("file_picker.rs") ||
                path_str.contains("theme.rs") ||
                path_str.contains("Cargo.toml")
            });
            
            let build_request = if main_app_changed {
                println!("🔥 Main app files changed - will restart after rebuild");
                BuildRequest {
                    target: "chonker8-hot".to_string(),
                    features: vec!["default".to_string()],
                }
            } else {
                // Other files - just rebuild pdf-processor
                BuildRequest {
                    target: "pdf-processor".to_string(),
                    features: vec!["default".to_string()],
                }
            };
            
            self.build_req_tx.send(build_request)?;
            self.last_build = Instant::now();
        }
        
        // Check for build results
        if let Ok(result) = self.build_result_rx.try_recv() {
            return Ok(Some(result));
        }
        
        Ok(None)
    }
    
    fn build_worker(build_req_rx: Receiver<BuildRequest>, build_result_tx: Sender<BuildResult>) {
        while let Ok(request) = build_req_rx.recv() {
            let start_time = Instant::now();
            
            println!("🔨 Building {}...", request.target);
            
            // Log build start to debug file
            if let Ok(mut file) = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open("/tmp/chonker8_debug.log")
            {
                use std::io::Write;
                let _ = writeln!(file, "[{}] [BUILD] Starting build for {}...", 
                    chrono::Local::now().format("%H:%M:%S%.3f"), 
                    request.target);
            }
            
            // Build with clean output - no ANSI codes or ribboning
            let build_result = Command::new("cargo")
                .env("CARGO_TERM_COLOR", "never")  // Disable color output
                .env("RUSTFLAGS", "--error-format=short")  // Simple error format
                .args(&["build", "--release", "--bin", &request.target])
                .output();
            
            let (success, stderr_output, stdout_output) = match build_result {
                Ok(output) => {
                    let success = output.status.success();
                    // Strip ANSI codes to prevent ribbon output
                    let stderr = strip_ansi_codes(&String::from_utf8_lossy(&output.stderr));
                    let stdout = strip_ansi_codes(&String::from_utf8_lossy(&output.stdout));
                    (success, stderr, stdout)
                }
                Err(e) => {
                    eprintln!("[BUILD] Failed to execute cargo: {}", e);
                    (false, format!("Failed to execute cargo: {}", e), String::new())
                }
            };
            
            // Log build output to debug instead of letting it corrupt the screen
            // Log both stderr (errors) and stdout (warnings/info) 
            if !stderr_output.is_empty() || !stdout_output.is_empty() {
                // Write to a debug file that UIRenderer can read
                if let Ok(mut file) = std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open("/tmp/chonker8_debug.log")
                {
                    use std::io::Write;
                    // Log stdout first (usually contains compilation progress and warnings)
                    for line in stdout_output.lines() {
                        let _ = writeln!(file, "[{}] [BUILD] {}", 
                            chrono::Local::now().format("%H:%M:%S%.3f"), 
                            line);
                        eprintln!("[BUILD] {}", line);
                    }
                    // Then log stderr (usually contains errors)
                    for line in stderr_output.lines() {
                        let _ = writeln!(file, "[{}] [BUILD] {}", 
                            chrono::Local::now().format("%H:%M:%S%.3f"), 
                            line);
                        eprintln!("[BUILD] {}", line);
                    }
                }
            }
            
            let should_restart = request.target == "chonker8-hot";
            
            let result = BuildResult {
                success,
                output: if success { 
                    if should_restart {
                        "Build successful - restarting app".to_string()
                    } else {
                        "Build successful".to_string()
                    }
                } else { 
                    "Build failed".to_string() 
                },
                binary_path: if success { Some(format!("target/release/{}", request.target)) } else { None },
                build_time: start_time.elapsed(),
                should_restart,
            };
            
            if success {
                println!("✅ Build completed in {:?}", result.build_time);
            } else {
                println!("❌ Build failed for {}", request.target);
            }
            
            if build_result_tx.send(result).is_err() {
                break; // Main thread disconnected
            }
        }
    }
    
    pub fn trigger_manual_build(&self, target: &str) -> Result<()> {
        let request = BuildRequest {
            target: target.to_string(),
            features: vec!["default".to_string()],
        };
        self.build_req_tx.send(request)?;
        Ok(())
    }
    
    pub fn restart_app() -> ! {
        println!("🔄 Hot-reloading app...");
        
        // Get current args
        let args: Vec<String> = env::args().collect();
        let binary_path = &args[0];
        
        // Re-exec with same args AND preserve Kitty environment
        let mut cmd = Command::new("./target/release/chonker8-hot");
        
        // Preserve Kitty environment for perfect hot-reload
        if let Ok(kitty_id) = env::var("KITTY_WINDOW_ID") {
            cmd.env("KITTY_WINDOW_ID", kitty_id);
            eprintln!("✅ Preserving Kitty graphics context");
        }
        if let Ok(term) = env::var("TERM") {
            cmd.env("TERM", term);
        }
        
        // Add original args (skip first arg which is the binary name)
        if args.len() > 1 {
            cmd.args(&args[1..]);
        }
        
        // Replace current process
        let err = cmd.exec();
        
        // If exec fails, exit with error
        eprintln!("Failed to restart app: {}", err);
        std::process::exit(1);
    }
}

// Plugin interface for hot-reloadable PDF processing
pub trait PDFProcessor: Send + Sync {
    fn process_page(&self, pdf_path: &Path, page: usize) -> Result<Vec<Vec<char>>>;
    fn get_page_count(&self, pdf_path: &Path) -> Result<usize>;
    fn get_version(&self) -> String;
}

pub struct HotReloadablePDFProcessor {
    current_processor: Option<Box<dyn PDFProcessor>>,
    plugin_path: String,
}

impl HotReloadablePDFProcessor {
    pub fn new(plugin_path: String) -> Self {
        Self {
            current_processor: None,
            plugin_path,
        }
    }
    
    pub fn reload_plugin(&mut self) -> Result<()> {
        // In a real implementation, this would:
        // 1. Unload current plugin
        // 2. Load new plugin from plugin_path
        // 3. Update current_processor
        
        println!("🔄 Reloading PDF processor plugin...");
        // TODO: Implement dynamic library loading
        Ok(())
    }
    
    pub fn process_page_safe(&self, pdf_path: &Path, page: usize) -> Result<Vec<Vec<char>>> {
        if let Some(processor) = &self.current_processor {
            processor.process_page(pdf_path, page)
        } else {
            // Fallback to basic processing
            Ok(vec![vec!['E', 'r', 'r', 'o', 'r']; 10])
        }
    }
}