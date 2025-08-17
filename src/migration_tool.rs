// CHONKER8 MIGRATION TOOL
// One-time migrator: Vec<Vec<char>> → Lance dataset
// Usage: Run once to migrate existing data to Lance format

use crate::storage::LegacyStorage;
#[cfg(feature = "lance-storage")]
use crate::storage::ChonkerLanceBackend;
use anyhow::{Result, anyhow};
use std::path::Path;
use std::collections::HashMap;

/// Migration tool for converting legacy data to Lance format
pub struct MigrationTool {
    pdf_path: std::path::PathBuf,
}

impl MigrationTool {
    pub fn new(pdf_path: &Path) -> Self {
        Self {
            pdf_path: pdf_path.to_path_buf(),
        }
    }
    
    /// Check if migration is needed
    pub fn migration_needed(&self) -> Result<bool> {
        let chonker_dir = std::env::current_dir()?.join("chonker_data");
        if !chonker_dir.exists() {
            return Ok(false); // No data to migrate
        }
        
        let pdf_name = self.pdf_path.file_stem()
            .ok_or_else(|| anyhow!("Invalid PDF path"))?
            .to_string_lossy();
        let lance_path = chonker_dir.join(format!("{}.lance", pdf_name));
        
        // Check if Lance dataset already exists
        Ok(!lance_path.exists())
    }
    
    /// Perform migration from legacy HashMap to Lance dataset
    pub fn migrate(&self, legacy_data: &HashMap<usize, Vec<Vec<char>>>) -> Result<()> {
        if legacy_data.is_empty() {
            crate::debug_log("Migration: No legacy data to migrate");
            return Ok(());
        }
        
        crate::debug_log(format!("Migration: Starting migration of {} pages", legacy_data.len()));
        
        // Create new Lance backend
        #[cfg(feature = "lance-storage")]
        {
            let mut lance_backend = ChonkerLanceBackend::new(&self.pdf_path)?;
            
            // Sort pages by number for consistent migration
            let mut sorted_pages: Vec<_> = legacy_data.iter().collect();
            sorted_pages.sort_by_key(|(page_num, _)| *page_num);
            
            // Migrate each page
            for (page_num, grid) in sorted_pages {
                crate::debug_log(format!("Migration: Converting page {}", page_num));
                lance_backend.save_page(*page_num, grid.clone())?;
            }
            
            // Force flush all writes with migration tag
            lance_backend.save_tagged(Some("MIGRATED_FROM_LEGACY".to_string()))?;
            
            crate::debug_log(format!("Migration: Successfully migrated {} pages to Lance", legacy_data.len()));
            Ok(())
        }
        
        #[cfg(not(feature = "lance-storage"))]
        {
            Err(anyhow!("Lance storage feature not enabled - cannot migrate"))
        }
    }
    
    /// Migrate from legacy storage instance
    pub fn migrate_from_legacy(&self, legacy: &LegacyStorage) -> Result<()> {
        // Extract data from legacy storage
        let legacy_data = self.extract_legacy_data(legacy)?;
        self.migrate(&legacy_data)
    }
    
    /// Extract data from legacy storage (this is a bit hacky since the HashMap is private)
    fn extract_legacy_data(&self, legacy: &LegacyStorage) -> Result<HashMap<usize, Vec<Vec<char>>>> {
        let mut data = HashMap::new();
        
        // Since we can't directly access the private HashMap, we'll try loading pages
        // until we find all of them. This is inefficient but works for migration.
        for page_num in 0..1000 { // Reasonable upper bound
            if legacy.has_page(page_num) {
                let grid = legacy.load_page(page_num)?;
                data.insert(page_num, grid);
            }
        }
        
        Ok(data)
    }
    
    /// Helper function to migrate a standalone HashMap
    pub fn migrate_standalone_data(
        pdf_path: &Path, 
        data: HashMap<usize, Vec<Vec<char>>>
    ) -> Result<()> {
        let migrator = MigrationTool::new(pdf_path);
        migrator.migrate(&data)
    }
    
    /// Verify migration integrity (compare legacy vs Lance data)
    pub fn verify_migration(&self, legacy: &LegacyStorage) -> Result<bool> {
        #[cfg(feature = "lance-storage")]
        {
            let lance_backend = ChonkerLanceBackend::new(&self.pdf_path)?;
            
            // Check each page in legacy storage
            for page_num in 0..1000 {
                if legacy.has_page(page_num) {
                    let legacy_grid = legacy.load_page(page_num)?;
                    let lance_grid = lance_backend.load_page(page_num)?;
                    
                    // Compare grids
                    if legacy_grid != lance_grid {
                        crate::debug_log(format!("Migration verification failed for page {}", page_num));
                        return Ok(false);
                    }
                }
            }
            
            crate::debug_log("Migration verification passed - all pages match");
            Ok(true)
        }
        
        #[cfg(not(feature = "lance-storage"))]
        {
            Err(anyhow!("Lance storage feature not enabled - cannot verify"))
        }
    }
    
    /// Get migration statistics
    pub fn migration_stats(&self, legacy: &LegacyStorage) -> Result<MigrationStats> {
        let page_count = legacy.page_count();
        
        // Estimate data size
        let mut total_chars = 0;
        let mut non_space_chars = 0;
        
        for page_num in 0..1000 {
            if legacy.has_page(page_num) {
                let grid = legacy.load_page(page_num)?;
                for row in &grid {
                    for &ch in row {
                        total_chars += 1;
                        if ch != ' ' {
                            non_space_chars += 1;
                        }
                    }
                }
            }
        }
        
        let density = if total_chars > 0 {
            non_space_chars as f64 / total_chars as f64
        } else {
            0.0
        };
        
        Ok(MigrationStats {
            page_count,
            total_chars,
            non_space_chars,
            density,
            estimated_compression_ratio: if density < 0.2 { 0.1 } else { 0.8 },
        })
    }
}

/// Migration statistics for user display
#[derive(Debug)]
pub struct MigrationStats {
    pub page_count: usize,
    pub total_chars: usize,
    pub non_space_chars: usize,
    pub density: f64,
    pub estimated_compression_ratio: f64,
}

impl std::fmt::Display for MigrationStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, 
            "Migration Stats:\n\
             - Pages: {}\n\
             - Total chars: {}\n\
             - Non-space chars: {}\n\
             - Density: {:.1}%\n\
             - Est. compression: {:.1}%",
            self.page_count,
            self.total_chars,
            self.non_space_chars,
            self.density * 100.0,
            self.estimated_compression_ratio * 100.0
        )
    }
}

/// Auto-migration helper for seamless transitions
pub fn migrate_if_needed(pdf_path: &Path, legacy: &LegacyStorage) -> Result<bool> {
    let migrator = MigrationTool::new(pdf_path);
    
    if migrator.migration_needed()? && legacy.page_count() > 0 {
        crate::debug_log("Auto-migration: Detected legacy data, starting migration...");
        
        // Show stats before migration
        let stats = migrator.migration_stats(legacy)?;
        crate::debug_log(format!("Migration stats: {}", stats));
        
        // Perform migration
        migrator.migrate_from_legacy(legacy)?;
        
        // Verify migration
        if migrator.verify_migration(legacy)? {
            crate::debug_log("Auto-migration: Completed successfully");
            Ok(true)
        } else {
            crate::debug_log("Auto-migration: Verification failed");
            Ok(false)
        }
    } else {
        Ok(false) // No migration needed
    }
}