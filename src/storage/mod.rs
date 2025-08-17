// CHONKER8 STORAGE LAYER
// Week 1: Lance backend with feature flagging and parallel legacy support

#[cfg(feature = "lance-storage")]
pub mod lance_backend;

#[cfg(feature = "lance-storage")]
pub use lance_backend::ChonkerLanceBackend;

// Legacy fallback when Lance is disabled
pub mod legacy_storage;
pub use legacy_storage::LegacyStorage;

use crate::types::{GRID_WIDTH, GRID_HEIGHT};
use anyhow::Result;
use std::path::Path;

/// Storage abstraction for grid data
/// Allows switching between Vec<Vec<char>> and Lance backends
#[derive(Debug)]
pub enum StorageBackend {
    #[cfg(feature = "lance-storage")]
    Lance(ChonkerLanceBackend),
    Legacy(LegacyStorage),
}

impl StorageBackend {
    /// Create new storage backend based on features and preferences
    pub fn new(pdf_path: &Path, prefer_lance: bool) -> Result<Self> {
        #[cfg(feature = "lance-storage")]
        {
            if prefer_lance {
                match ChonkerLanceBackend::new(pdf_path) {
                    Ok(lance) => {
                        crate::debug_log("🚀 Lance storage initialized");
                        return Ok(StorageBackend::Lance(lance));
                    }
                    Err(e) => {
                        crate::debug_log(format!("Lance init failed, falling back to legacy: {}", e));
                    }
                }
            }
        }
        
        // Always have legacy fallback
        let legacy = LegacyStorage::new();
        crate::debug_log("📁 Legacy storage initialized");
        Ok(StorageBackend::Legacy(legacy))
    }
    
    /// Save page grid to storage
    pub fn save_page(&mut self, page_num: usize, grid: Vec<Vec<char>>) -> Result<u64> {
        match self {
            #[cfg(feature = "lance-storage")]
            StorageBackend::Lance(backend) => backend.save_page(page_num, grid),
            StorageBackend::Legacy(backend) => backend.save_page(page_num, grid),
        }
    }
    
    /// Load page grid from storage
    pub fn load_page(&self, page_num: usize) -> Result<Vec<Vec<char>>> {
        match self {
            #[cfg(feature = "lance-storage")]
            StorageBackend::Lance(backend) => backend.load_page(page_num),
            StorageBackend::Legacy(backend) => backend.load_page(page_num),
        }
    }
    
    /// Check if page exists in storage
    pub fn has_page(&self, page_num: usize) -> bool {
        match self {
            #[cfg(feature = "lance-storage")]
            StorageBackend::Lance(backend) => backend.has_page(page_num),
            StorageBackend::Legacy(backend) => backend.has_page(page_num),
        }
    }
    
    /// Get current version/revision
    pub fn current_version(&self) -> u64 {
        match self {
            #[cfg(feature = "lance-storage")]
            StorageBackend::Lance(backend) => backend.current_version(),
            StorageBackend::Legacy(_) => 1, // Legacy has no versioning
        }
    }
    
    /// Undo last change (Lance only)
    pub fn undo(&mut self) -> Result<bool> {
        match self {
            #[cfg(feature = "lance-storage")]
            StorageBackend::Lance(backend) => backend.undo(),
            StorageBackend::Legacy(_) => Ok(false), // Legacy has no undo
        }
    }
    
    /// Redo last undo (Lance only)
    pub fn redo(&mut self) -> Result<bool> {
        match self {
            #[cfg(feature = "lance-storage")]
            StorageBackend::Lance(backend) => backend.redo(),
            StorageBackend::Legacy(_) => Ok(false), // Legacy has no redo
        }
    }
    
    /// Get storage type for UI display
    pub fn storage_type(&self) -> &'static str {
        match self {
            #[cfg(feature = "lance-storage")]
            StorageBackend::Lance(_) => "Lance",
            StorageBackend::Legacy(_) => "Memory",
        }
    }
}