// LEGACY STORAGE BACKEND
// Maintains existing Vec<Vec<char>> behavior for backwards compatibility

use crate::types::{GRID_WIDTH, GRID_HEIGHT};
use anyhow::Result;
use std::collections::HashMap;

/// Legacy in-memory storage using HashMap<page_num, Vec<Vec<char>>>
#[derive(Debug)]
pub struct LegacyStorage {
    pages: HashMap<usize, Vec<Vec<char>>>,
}

impl LegacyStorage {
    pub fn new() -> Self {
        Self {
            pages: HashMap::new(),
        }
    }
    
    /// Save page grid (just stores in HashMap)
    pub fn save_page(&mut self, page_num: usize, grid: Vec<Vec<char>>) -> Result<u64> {
        self.pages.insert(page_num, grid);
        crate::debug_log(format!("Legacy: Saved page {} to memory", page_num));
        Ok(1) // Always version 1 in legacy
    }
    
    /// Load page grid (returns empty grid if not found)
    pub fn load_page(&self, page_num: usize) -> Result<Vec<Vec<char>>> {
        if let Some(grid) = self.pages.get(&page_num) {
            crate::debug_log(format!("Legacy: Loaded page {} from memory", page_num));
            Ok(grid.clone())
        } else {
            crate::debug_log(format!("Legacy: Page {} not found, returning empty grid", page_num));
            Ok(vec![vec![' '; GRID_WIDTH]; GRID_HEIGHT])
        }
    }
    
    /// Check if page exists
    pub fn has_page(&self, page_num: usize) -> bool {
        self.pages.contains_key(&page_num)
    }
    
    /// Get total pages stored
    pub fn page_count(&self) -> usize {
        self.pages.len()
    }
    
    /// Clear all pages (for testing)
    pub fn clear(&mut self) {
        self.pages.clear();
        crate::debug_log("Legacy: Cleared all pages from memory");
    }
}

impl Default for LegacyStorage {
    fn default() -> Self {
        Self::new()
    }
}