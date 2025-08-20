// OPTIMIZED STORAGE BACKEND with LRU Cache
// Only keeps configured number of pages in memory to prevent memory bloat

use crate::config::{GRID_WIDTH, GRID_HEIGHT, MAX_CACHED_PAGES};
use anyhow::Result;
use lru::LruCache;
use std::num::NonZeroUsize;

/// Optimized storage using LRU cache - only keeps 5 pages in memory
#[derive(Debug)]
pub struct LegacyStorage {
    pages: LruCache<usize, Vec<Vec<char>>>,
}

impl LegacyStorage {
    pub fn new() -> Self {
        let cache_size = NonZeroUsize::new(MAX_CACHED_PAGES).unwrap();
        Self {
            pages: LruCache::new(cache_size),
        }
    }
    
    /// Save page grid (stores in LRU cache, evicts oldest if at capacity)
    pub fn save_page(&mut self, page_num: usize, grid: Vec<Vec<char>>) -> Result<u64> {
        // LRU cache automatically evicts least recently used page if at capacity
        let evicted = self.pages.push(page_num, grid);
        
        if evicted.is_some() {
            // Evicted old page
        }
        Ok(1) // Always version 1
    }
    
    /// Load page grid (returns empty grid if not found)
    pub fn load_page(&mut self, page_num: usize) -> Result<Vec<Vec<char>>> {
        if let Some(grid) = self.pages.get(&page_num) {
            Ok(grid.clone())
        } else {
            Ok(vec![vec![' '; GRID_WIDTH]; GRID_HEIGHT])
        }
    }
    
    /// Check if page exists
    pub fn has_page(&self, page_num: usize) -> bool {
        self.pages.contains(&page_num)
    }
    
    /// Get total pages stored
    pub fn page_count(&self) -> usize {
        self.pages.len()
    }
    
    /// Clear all pages (for testing)
    pub fn clear(&mut self) {
        self.pages.clear();
    }
    
    
    /// Get current version (always 1 for legacy)
    pub fn current_version(&self) -> u64 {
        1
    }
    
    /// Undo last change (not supported)
    pub fn undo(&mut self) -> Result<bool> {
        Ok(false)
    }
    
    /// Redo last undo (not supported)
    pub fn redo(&mut self) -> Result<bool> {
        Ok(false)
    }
    
    /// Get storage type for UI display
    pub fn storage_type(&self) -> &'static str {
        "LRU Cache"
    }
}

impl Default for LegacyStorage {
    fn default() -> Self {
        Self::new()
    }
}