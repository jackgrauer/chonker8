use anyhow::Result;
use std::process::Command;

pub struct HotReloadManager;

impl HotReloadManager {
    pub fn trigger_reload() -> Result<()> {
        println!("🔄 Hot reload triggered - rebuilding...");
        
        let build_output = Command::new("cargo")
            .args(["build", "--release", "--quiet"])
            .output()?;
        
        if !build_output.status.success() {
            eprintln!("❌ Build failed:\n{}", String::from_utf8_lossy(&build_output.stderr));
            std::thread::sleep(std::time::Duration::from_secs(2));
            return Err(anyhow::anyhow!("Build failed"));
        }
        
        println!("✅ Build successful - restarting...");
        std::thread::sleep(std::time::Duration::from_millis(500));
        
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            let _ = Command::new("./target/release/chonker8").exec();
        }
        
        #[cfg(not(unix))]
        {
            Command::new("./target/release/chonker8").spawn()?;
            std::process::exit(0);
        }
        
        Ok(())
    }
}