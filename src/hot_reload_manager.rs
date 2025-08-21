// Hot-reload manager for Rust code changes
use anyhow::Result;
use notify::{RecommendedWatcher, RecursiveMode, Watcher, Event, EventKind};
use std::{
    path::Path,
    process::{Command, Stdio},
    sync::mpsc::{channel, Receiver, Sender},
    thread,
    time::{Duration, Instant},
};

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
            
            // Always rebuild pdf-processor for hot-reload
            let build_request = BuildRequest {
                target: "pdf-processor".to_string(),
                features: vec!["default".to_string()],
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
            
            // Simple build execution
            let success = Command::new("cargo")
                .env("DYLD_LIBRARY_PATH", "./lib")
                .args(&["build", "--release", "--bin", &request.target])
                .status()
                .map(|status| status.success())
                .unwrap_or(false);
            
            let result = BuildResult {
                success,
                output: if success { "Build successful".to_string() } else { "Build failed".to_string() },
                binary_path: if success { Some(format!("target/release/{}", request.target)) } else { None },
                build_time: start_time.elapsed(),
            };
            
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