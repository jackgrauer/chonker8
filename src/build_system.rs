// Background build system for hot-reload
use anyhow::Result;
use std::{
    process::{Command, Stdio},
    sync::mpsc::{Receiver, Sender},
    thread,
    time::{Duration, Instant},
    path::Path,
};

use crate::hot_reload_manager::{BuildRequest, BuildResult};

pub struct BuildSystem {
    request_rx: Receiver<BuildRequest>,
    result_tx: Sender<BuildResult>,
}

impl BuildSystem {
    pub fn new(request_rx: Receiver<BuildRequest>, result_tx: Sender<BuildResult>) -> Self {
        Self {
            request_rx,
            result_tx,
        }
    }
    
    pub fn start_worker(self) {
        thread::spawn(move || {
            self.run_build_loop();
        });
    }
    
    fn run_build_loop(self) {
        while let Ok(request) = self.request_rx.recv() {
            let start_time = Instant::now();
            let result = self.execute_build(&request);
            let build_time = start_time.elapsed();
            
            let build_result = BuildResult {
                success: result.is_ok(),
                output: result.unwrap_or_else(|e| format!("Build failed: {}", e)),
                binary_path: self.get_binary_path(&request.target),
                build_time,
            };
            
            if self.result_tx.send(build_result).is_err() {
                break; // Main thread disconnected
            }
        }
    }
    
    fn execute_build(&self, request: &BuildRequest) -> Result<String> {
        println!("🔨 Building target: {}", request.target);
        
        // Set library path for macOS
        let mut cmd = Command::new("cargo");
        cmd.env("DYLD_LIBRARY_PATH", "./lib");
        
        cmd.args(&["build", "--release"]);
        
        // Add target-specific flags
        match request.target.as_str() {
            "pdf-processor" => {
                cmd.args(&["--bin", "pdf-processor"]);
            },
            "chonker8-hot" => {
                cmd.args(&["--bin", "chonker8-hot"]);
            },
            _ => {
                cmd.arg("--all");
            }
        }
        
        // Add features
        if !request.features.is_empty() && !request.features.contains(&"default".to_string()) {
            cmd.arg("--features");
            cmd.arg(request.features.join(","));
        }
        
        // Capture output
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());
        
        let output = cmd.output()?;
        
        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            Ok(format!("✅ Build successful!\n{}\n{}", stdout, stderr))
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(anyhow::anyhow!("❌ Build failed:\n{}", stderr))
        }
    }
    
    fn get_binary_path(&self, target: &str) -> Option<String> {
        let binary_name = match target {
            "pdf-processor" => "pdf-processor",
            "chonker8-hot" => "chonker8-hot",
            _ => return None,
        };
        
        let path = format!("target/release/{}", binary_name);
        if Path::new(&path).exists() {
            Some(path)
        } else {
            None
        }
    }
}

// Fast incremental compilation helper
pub struct IncrementalBuilder {
    last_build_time: Option<Instant>,
    cached_dependencies: Vec<String>,
}

impl IncrementalBuilder {
    pub fn new() -> Self {
        Self {
            last_build_time: None,
            cached_dependencies: Vec::new(),
        }
    }
    
    pub fn should_rebuild(&mut self, changed_files: &[String]) -> bool {
        // Smart rebuild detection
        // - Always rebuild if no previous build
        // - Rebuild if core files changed
        // - Skip rebuild for minor changes (comments, whitespace)
        
        if self.last_build_time.is_none() {
            return true;
        }
        
        // Check if any changed files are in critical paths
        let critical_patterns = [
            "src/pdf_extraction/",
            "src/main_hotreload.rs",
            "src/ui_renderer.rs",
            "Cargo.toml",
        ];
        
        for file in changed_files {
            for pattern in &critical_patterns {
                if file.contains(pattern) {
                    return true;
                }
            }
        }
        
        false
    }
    
    pub fn mark_build_complete(&mut self) {
        self.last_build_time = Some(Instant::now());
    }
}