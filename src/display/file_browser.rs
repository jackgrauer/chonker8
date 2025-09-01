// egui-based file browser for PDFs
use anyhow::Result;
use std::{
    fs,
    path::PathBuf,
};

pub struct FileBrowser {
    files: Vec<String>,
    current_dir: PathBuf,
    query: String,
    selected_index: usize,
}

impl FileBrowser {
    pub fn new() -> Result<Self> {
        let current_dir = PathBuf::from("/Users/jack/Documents");
        let mut browser = Self {
            files: Vec::new(),
            current_dir,
            query: String::new(),
            selected_index: 0,
        };
        
        browser.scan_directory()?;
        Ok(browser)
    }
    
    pub fn new_empty() -> Self {
        Self {
            files: vec!["Error loading files".to_string()],
            current_dir: PathBuf::from("/"),
            query: String::new(),
            selected_index: 0,
        }
    }
    
    fn scan_directory(&mut self) -> Result<()> {
        self.files.clear();
        
        // Add parent directory if not at root
        if self.current_dir.parent().is_some() {
            self.files.push("../".to_string());
        }
        
        // Read directory entries
        if let Ok(entries) = fs::read_dir(&self.current_dir) {
            let mut dirs = Vec::new();
            let mut files = Vec::new();
            
            for entry in entries.flatten() {
                let file_name = entry.file_name();
                let file_str = file_name.to_string_lossy().to_string();
                
                // Skip hidden files
                if file_str.starts_with('.') && file_str != ".." {
                    continue;
                }
                
                if entry.path().is_dir() {
                    dirs.push(format!("{}/", file_str));
                } else if file_str.to_lowercase().ends_with(".pdf") {
                    files.push(file_str);
                }
            }
            
            // Sort and combine
            dirs.sort();
            files.sort();
            self.files.extend(dirs);
            self.files.extend(files);
        }
        
        self.selected_index = 0;
        Ok(())
    }
    
    pub fn get_query_mut(&mut self) -> &mut String {
        &mut self.query
    }
    
    pub fn get_visible_files(&self) -> Vec<String> {
        if self.query.is_empty() {
            self.files.clone()
        } else {
            let query_lower = self.query.to_lowercase();
            self.files
                .iter()
                .filter(|f| f.to_lowercase().contains(&query_lower))
                .cloned()
                .collect()
        }
    }
    
    pub fn get_selected_index(&self) -> usize {
        self.selected_index
    }
    
    pub fn set_selected_index(&mut self, index: usize) {
        let visible_count = self.get_visible_files().len();
        self.selected_index = index.min(visible_count.saturating_sub(1));
    }
    
    pub fn move_selection_up(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
        }
    }
    
    pub fn move_selection_down(&mut self) {
        let visible_count = self.get_visible_files().len();
        if self.selected_index + 1 < visible_count {
            self.selected_index += 1;
        }
    }
    
    pub fn get_selected_file(&self) -> Option<String> {
        let visible = self.get_visible_files();
        visible.get(self.selected_index).cloned()
    }
    
    pub fn get_current_dir(&self) -> &PathBuf {
        &self.current_dir
    }
    
    pub fn navigate_to(&mut self, file: &str) -> Result<()> {
        if file == "../" {
            if let Some(parent) = self.current_dir.parent() {
                self.current_dir = parent.to_path_buf();
                self.scan_directory()?;
            }
        } else if file.ends_with('/') {
            let dir_name = &file[..file.len()-1];
            self.current_dir = self.current_dir.join(dir_name);
            self.scan_directory()?;
        }
        Ok(())
    }
}