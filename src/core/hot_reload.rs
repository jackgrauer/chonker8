// Hot-reload functionality for Ctrl+U rebuild and rerun
use anyhow::Result;
use std::process::Command;

pub struct HotReloadManager {
    enabled: bool,
}

impl HotReloadManager {
    pub fn new(_path: &str) -> Result<Self> {
        Ok(Self {
            enabled: true,
        })
    }
    
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }
    
    /// Trigger hot reload - rebuild and restart the application
    pub fn trigger_reload() -> Result<()> {
        println!("🔄 Hot reload triggered - rebuilding...");
        
        // Step 1: Rebuild the application
        let build_output = Command::new("cargo")
            .args(&["build", "--release", "--quiet"])
            .output()?;
        
        if !build_output.status.success() {
            let stderr = String::from_utf8_lossy(&build_output.stderr);
            eprintln!("❌ Build failed:");
            eprintln!("{}", stderr);
            std::thread::sleep(std::time::Duration::from_secs(2));
            return Err(anyhow::anyhow!("Build failed"));
        }
        
        println!("✅ Build successful - restarting...");
        std::thread::sleep(std::time::Duration::from_millis(500));
        
        // Step 2: Restart the application
        let exe_path = "./target/release/chonker8";
        
        // Use exec to replace current process (Unix-like systems)
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            let err = Command::new(exe_path)
                .exec();
            // If we get here, exec failed
            return Err(anyhow::anyhow!("Failed to restart: {}", err));
        }
        
        // Fallback for non-Unix systems
        #[cfg(not(unix))]
        {
            Command::new(exe_path)
                .spawn()?;
            std::process::exit(0);
        }
    }
}