// CHONKER8 LANCE BACKEND - Week 1 Implementation
// Mock implementation with exact API structure for future Lance integration
// TODO: Replace with real LanceDB when Arrow dependency conflicts are resolved

use crate::types::{GRID_WIDTH, GRID_HEIGHT};
use anyhow::{Result, anyhow};
use std::path::{Path, PathBuf};
use std::collections::HashMap;
use std::fs;

// Mock Lance API that matches the intended interface
use serde::{Serialize, Deserialize};
use chrono::Utc;

const SCHEMA_VERSION: &str = "1.0.0";
const SPARSE_THRESHOLD: f64 = 0.2; // Use sparse when <20% density

/// Mock Lance dataset that will be replaced with real LanceDB
#[derive(Debug, Serialize, Deserialize)]
struct MockDataset {
    records: Vec<DataRecord>,
    current_version: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DataRecord {
    page_num: u32,
    timestamp: i64,
    grid_data: Vec<u8>,
    is_sparse: bool,
    density: f64,
    schema_version: String,
    metadata: Option<String>,
    tag: Option<String>,
    version: u64,
}

/// Lance-based storage with Vortex compression and versioning
#[derive(Debug)]
pub struct ChonkerLanceBackend {
    dataset: MockDataset,
    dataset_path: PathBuf,
    pdf_path: PathBuf,
    current_version: u64,
    page_cache: HashMap<usize, Vec<Vec<char>>>, // LRU cache for last 5 pages
    write_buffer: Vec<PendingWrite>, // Batch writes
    last_write: std::time::Instant,
}

#[derive(Debug, Clone)]
struct PendingWrite {
    page_num: usize,
    grid: Vec<Vec<char>>,
    timestamp: std::time::Instant,
}

impl ChonkerLanceBackend {
    /// Initialize Lance dataset for a PDF
    pub fn new(pdf_path: &Path) -> Result<Self> {
        // Create chonker_data directory
        let chonker_dir = std::env::current_dir()?.join("chonker_data");
        std::fs::create_dir_all(&chonker_dir)?;
        
        // Dataset path: chonker_data/{pdf_name}.lance/
        let pdf_name = pdf_path.file_stem()
            .ok_or_else(|| anyhow!("Invalid PDF path"))?
            .to_string_lossy();
        let dataset_path = chonker_dir.join(format!("{}.lance", pdf_name));
        
        let dataset = if dataset_path.exists() {
            // Load existing dataset
            crate::debug_log(format!("Opening existing Lance dataset: {:?}", dataset_path));
            let data = fs::read(&dataset_path)?;
            bincode::deserialize(&data)?
        } else {
            // Create new empty dataset
            crate::debug_log(format!("Creating new Lance dataset: {:?}", dataset_path));
            MockDataset {
                records: Vec::new(),
                current_version: 1,
            }
        };
        
        let current_version = dataset.current_version;
        crate::debug_log(format!("Lance backend initialized at version {}", current_version));
        
        Ok(Self {
            dataset,
            dataset_path,
            pdf_path: pdf_path.to_path_buf(),
            current_version,
            page_cache: HashMap::with_capacity(5), // LRU cache for 5 pages
            write_buffer: Vec::new(),
            last_write: std::time::Instant::now(),
        })
    }
    
    /// Save page with batching (100 edits OR 5 sec idle OR explicit save)
    pub fn save_page(&mut self, page_num: usize, grid: Vec<Vec<char>>) -> Result<u64> {
        // Add to write buffer
        self.write_buffer.push(PendingWrite {
            page_num,
            grid: grid.clone(),
            timestamp: std::time::Instant::now(),
        });
        
        // Cache the current page
        self.page_cache.insert(page_num, grid);
        
        // Check if we should flush
        let should_flush = self.write_buffer.len() >= 100 ||  // 100 edits
                          self.last_write.elapsed().as_secs() >= 5;  // 5 sec idle
        
        if should_flush {
            self.flush_writes(None)?;
        }
        
        Ok(self.current_version)
    }
    
    /// Explicit save with tag (Ctrl+S)
    pub fn save_tagged(&mut self, tag: Option<String>) -> Result<u64> {
        self.flush_writes(tag)
    }
    
    /// Flush pending writes to Lance
    fn flush_writes(&mut self, tag: Option<String>) -> Result<u64> {
        if self.write_buffer.is_empty() {
            return Ok(self.current_version);
        }
        
        crate::debug_log(format!("Flushing {} writes to Lance", self.write_buffer.len()));
        
        // Group writes by page (latest wins)
        let mut latest_writes: HashMap<usize, &PendingWrite> = HashMap::new();
        for write in &self.write_buffer {
            latest_writes.insert(write.page_num, write);
        }
        
        // Collect data for Table
        let mut page_nums = Vec::new();
        let mut timestamps = Vec::new();
        let mut grid_data = Vec::new();
        let mut is_sparse = Vec::new();
        let mut densities = Vec::new();
        let mut schema_versions = Vec::new();
        let mut metadata = Vec::new();
        let mut tags = Vec::new();
        
        for write in latest_writes.values() {
            let compressed = self.compress_grid(&write.grid)?;
            
            page_nums.push(write.page_num as u32);
            timestamps.push(Utc::now().timestamp_millis());
            grid_data.push(compressed.data);
            is_sparse.push(compressed.is_sparse);
            densities.push(compressed.density);
            schema_versions.push(SCHEMA_VERSION.to_string());
            metadata.push(Some("{}".to_string())); // Empty JSON for now
            tags.push(tag.clone());
        }
        
        // Create records and add to mock dataset
        for i in 0..page_nums.len() {
            let record = DataRecord {
                page_num: page_nums[i],
                timestamp: timestamps[i],
                grid_data: grid_data[i].clone(),
                is_sparse: is_sparse[i],
                density: densities[i],
                schema_version: schema_versions[i].clone(),
                metadata: metadata[i].clone(),
                tag: tags[i].clone(),
                version: self.current_version + 1,
            };
            self.dataset.records.push(record);
        }
        
        // Increment version and save to disk
        self.dataset.current_version += 1;
        self.current_version = self.dataset.current_version;
        
        // Persist to disk
        let data = bincode::serialize(&self.dataset)?;
        fs::write(&self.dataset_path, data)?;
        
        // Clear write buffer
        self.write_buffer.clear();
        self.last_write = std::time::Instant::now();
        
        crate::debug_log(format!("Lance write complete, new version: {}", self.current_version));
        Ok(self.current_version)
    }
    
    /// Load page from Lance (with caching)
    pub fn load_page(&self, page_num: usize) -> Result<Vec<Vec<char>>> {
        // Check cache first
        if let Some(grid) = self.page_cache.get(&page_num) {
            crate::debug_log(format!("Cache hit for page {}", page_num));
            return Ok(grid.clone());
        }
        
        // Check pending writes first (uncommitted changes)
        for write in self.write_buffer.iter().rev() {
            if write.page_num == page_num {
                crate::debug_log(format!("Found page {} in write buffer", page_num));
                return Ok(write.grid.clone());
            }
        }
        
        // Query Lance for latest version of page
        crate::debug_log(format!("Loading page {} from Lance dataset", page_num));
        
        // Query mock dataset for the latest version of this page
        let page_records: Vec<_> = self.dataset.records
            .iter()
            .filter(|r| r.page_num == page_num as u32)
            .collect();
            
        if page_records.is_empty() {
            // Page doesn't exist, return empty grid
            crate::debug_log(format!("Page {} not found in dataset, returning empty", page_num));
            return Ok(vec![vec![' '; GRID_WIDTH]; GRID_HEIGHT]);
        }
        
        // Get the most recent version
        let latest_record = page_records
            .iter()
            .max_by_key(|r| r.version)
            .unwrap();
        
        // Extract data
        let grid_data = &latest_record.grid_data;
        let is_sparse = latest_record.is_sparse;
        
        // Create compressed grid and decode
        let compressed = CompressedGrid {
            data: grid_data.clone(),
            is_sparse,
            density: 0.0, // We don't need density for decoding
        };
        
        let grid = self.decode_grid(&compressed)?;
        crate::debug_log(format!("Successfully loaded page {} from Lance", page_num));
        
        Ok(grid)
    }
    
    /// Check if page exists
    pub fn has_page(&self, page_num: usize) -> bool {
        // Check cache first
        if self.page_cache.contains_key(&page_num) {
            return true;
        }
        
        // Check pending writes
        for write in &self.write_buffer {
            if write.page_num == page_num {
                return true;
            }
        }
        
        // Query mock dataset
        self.dataset.records
            .iter()
            .any(|r| r.page_num == page_num as u32)
    }
    
    /// Get current version
    pub fn current_version(&self) -> u64 {
        self.current_version
    }
    
    /// Undo to previous version
    pub fn undo(&mut self) -> Result<bool> {
        // Flush any pending writes first
        if !self.write_buffer.is_empty() {
            self.flush_writes(None)?;
        }
        
        if self.current_version > 1 {
            let target_version = self.current_version - 1;
            crate::debug_log(format!("Attempting undo from version {} to {}", self.current_version, target_version));
            
            // Filter dataset to target version
            self.dataset.records.retain(|r| r.version <= target_version);
            self.dataset.current_version = target_version;
            self.current_version = target_version;
            
            // Clear cache since we've changed versions
            self.page_cache.clear();
            
            // Save updated dataset
            let data = bincode::serialize(&self.dataset)?;
            fs::write(&self.dataset_path, data)?;
            
            crate::debug_log(format!("Successfully undid to version {}", self.current_version));
            Ok(true)
        } else {
            crate::debug_log("Cannot undo: already at first version");
            Ok(false)
        }
    }
    
    /// Redo to next version
    pub fn redo(&mut self) -> Result<bool> {
        // Get the latest version by reloading from disk
        let latest_version = if self.dataset_path.exists() {
            let data = fs::read(&self.dataset_path)?;
            let full_dataset: MockDataset = bincode::deserialize(&data)?;
            full_dataset.current_version
        } else {
            self.current_version
        };
        
        if self.current_version < latest_version {
            let target_version = self.current_version + 1;
            crate::debug_log(format!("Attempting redo from version {} to {}", self.current_version, target_version));
            
            // Reload full dataset and set current version
            let data = fs::read(&self.dataset_path)?;
            let full_dataset: MockDataset = bincode::deserialize(&data)?;
            
            self.dataset = full_dataset;
            self.current_version = target_version;
            self.dataset.current_version = target_version;
            
            // Clear cache since we've changed versions
            self.page_cache.clear();
            
            crate::debug_log(format!("Successfully redid to version {}", self.current_version));
            Ok(true)
        } else {
            crate::debug_log("Cannot redo: already at latest version");
            Ok(false)
        }
    }
    
    /// Get available version range for UI
    pub fn version_info(&self) -> Result<(u64, u64)> {
        let latest_version = if self.dataset_path.exists() {
            let data = fs::read(&self.dataset_path)?;
            let full_dataset: MockDataset = bincode::deserialize(&data)?;
            full_dataset.current_version
        } else {
            self.current_version
        };
        Ok((1, latest_version)) // (min_version, max_version)
    }
    
    /// Jump to specific version
    pub fn checkout_version(&mut self, version: u64) -> Result<bool> {
        // Flush any pending writes first
        if !self.write_buffer.is_empty() {
            self.flush_writes(None)?;
        }
        
        if version == self.current_version {
            return Ok(true); // Already at target version
        }
        
        crate::debug_log(format!("Checking out version {}", version));
        
        // Load full dataset and filter to version
        let data = fs::read(&self.dataset_path)?;
        let mut full_dataset: MockDataset = bincode::deserialize(&data)?;
        
        // Filter records to target version
        full_dataset.records.retain(|r| r.version <= version);
        full_dataset.current_version = version;
        
        self.dataset = full_dataset;
        self.current_version = version;
        
        // Clear cache since we've changed versions
        self.page_cache.clear();
        
        crate::debug_log(format!("Successfully checked out version {}", version));
        Ok(true)
    }
    
    /// Compress grid using sparse encoding when beneficial
    fn compress_grid(&self, grid: &Vec<Vec<char>>) -> Result<CompressedGrid> {
        let total_cells = GRID_WIDTH * GRID_HEIGHT;
        let mut non_space_count = 0;
        
        // Count non-space characters
        for row in grid {
            for &ch in row {
                if ch != ' ' {
                    non_space_count += 1;
                }
            }
        }
        
        let density = non_space_count as f64 / total_cells as f64;
        
        if density < SPARSE_THRESHOLD {
            // Use sparse encoding
            let compressed = self.encode_sparse(grid)?;
            Ok(CompressedGrid {
                data: compressed,
                is_sparse: true,
                density,
            })
        } else {
            // Use dense encoding (simple serialization)
            let compressed = self.encode_dense(grid)?;
            Ok(CompressedGrid {
                data: compressed,
                is_sparse: false,
                density,
            })
        }
    }
    
    /// Encode grid as sparse (indices + values)
    fn encode_sparse(&self, grid: &Vec<Vec<char>>) -> Result<Vec<u8>> {
        let mut data = Vec::new();
        
        // Write sparse format: [count: u32][index: u32, value: u8]...
        let mut pairs = Vec::new();
        for (y, row) in grid.iter().enumerate() {
            for (x, &ch) in row.iter().enumerate() {
                if ch != ' ' {
                    let index = (y * GRID_WIDTH + x) as u32;
                    pairs.push((index, ch as u8));
                }
            }
        }
        
        // Serialize count + pairs
        data.extend_from_slice(&(pairs.len() as u32).to_le_bytes());
        for (index, value) in pairs {
            data.extend_from_slice(&index.to_le_bytes());
            data.push(value);
        }
        
        Ok(data)
    }
    
    /// Encode grid as dense (row-major order)
    fn encode_dense(&self, grid: &Vec<Vec<char>>) -> Result<Vec<u8>> {
        let mut data = Vec::new();
        for row in grid {
            for &ch in row {
                data.push(ch as u8);
            }
        }
        Ok(data)
    }
    
    /// Decode compressed grid data back to Vec<Vec<char>>
    pub fn decode_grid(&self, compressed: &CompressedGrid) -> Result<Vec<Vec<char>>> {
        if compressed.is_sparse {
            self.decode_sparse(&compressed.data)
        } else {
            self.decode_dense(&compressed.data)
        }
    }
    
    /// Decode sparse format: [count: u32][index: u32, value: u8]...
    fn decode_sparse(&self, data: &[u8]) -> Result<Vec<Vec<char>>> {
        if data.len() < 4 {
            return Err(anyhow!("Invalid sparse data: too short"));
        }
        
        let mut grid = vec![vec![' '; GRID_WIDTH]; GRID_HEIGHT];
        
        // Read count
        let count = u32::from_le_bytes([data[0], data[1], data[2], data[3]]) as usize;
        let mut offset = 4;
        
        // Read index-value pairs
        for _ in 0..count {
            if offset + 5 > data.len() {
                return Err(anyhow!("Invalid sparse data: truncated"));
            }
            
            let index = u32::from_le_bytes([
                data[offset], data[offset + 1], data[offset + 2], data[offset + 3]
            ]) as usize;
            let value = data[offset + 4] as char;
            offset += 5;
            
            // Convert linear index to y,x coordinates
            let y = index / GRID_WIDTH;
            let x = index % GRID_WIDTH;
            
            if y < GRID_HEIGHT && x < GRID_WIDTH {
                grid[y][x] = value;
            }
        }
        
        Ok(grid)
    }
    
    /// Decode dense format: row-major char bytes
    fn decode_dense(&self, data: &[u8]) -> Result<Vec<Vec<char>>> {
        if data.len() != GRID_WIDTH * GRID_HEIGHT {
            return Err(anyhow!("Invalid dense data: wrong size, expected {}, got {}", 
                GRID_WIDTH * GRID_HEIGHT, data.len()));
        }
        
        let mut grid = vec![vec![' '; GRID_WIDTH]; GRID_HEIGHT];
        let mut offset = 0;
        
        for y in 0..GRID_HEIGHT {
            for x in 0..GRID_WIDTH {
                grid[y][x] = data[offset] as char;
                offset += 1;
            }
        }
        
        Ok(grid)
    }
}

#[derive(Debug)]
struct CompressedGrid {
    data: Vec<u8>,
    is_sparse: bool,
    density: f64,
}