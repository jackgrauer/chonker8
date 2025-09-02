use anyhow::Result;
use std::{fs, path::PathBuf};

pub struct FileBrowser {
    files: Vec<String>,
    current_dir: PathBuf,
    query: String,
    selected_index: usize,
}

impl FileBrowser {
    pub fn new() -> Result<Self> {
        let mut browser = Self {
            files: Vec::new(),
            current_dir: PathBuf::from("/Users/jack/Documents"),
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
        if self.current_dir.parent().is_some() {
            self.files.push("../".to_string());
        }
        
        if let Ok(entries) = fs::read_dir(&self.current_dir) {
            let (mut dirs, mut files): (Vec<_>, Vec<_>) = entries
                .flatten()
                .filter_map(|e| {
                    let name = e.file_name().to_string_lossy().to_string();
                    if name.starts_with('.') && name != ".." { return None; }
                    if e.path().is_dir() { Some((format!("{}/", name), true)) }
                    else if name.to_lowercase().ends_with(".pdf") { Some((name, false)) }
                    else { None }
                })
                .partition(|(_, is_dir)| *is_dir);
            
            dirs.sort();
            files.sort();
            self.files.extend(dirs.into_iter().map(|(name, _)| name));
            self.files.extend(files.into_iter().map(|(name, _)| name));
        }
        
        self.selected_index = 0;
        Ok(())
    }
    
    pub fn get_query_mut(&mut self) -> &mut String { &mut self.query }
    
    pub fn get_visible_files(&self) -> Vec<String> {
        if self.query.is_empty() { return self.files.clone(); }
        let query_lower = self.query.to_lowercase();
        self.files.iter()
            .filter(|f| f.to_lowercase().contains(&query_lower))
            .cloned().collect()
    }
    
    pub fn get_selected_index(&self) -> usize { self.selected_index }
    pub fn get_current_dir(&self) -> &PathBuf { &self.current_dir }
    pub fn get_selected_file(&self) -> Option<String> {
        self.get_visible_files().get(self.selected_index).cloned()
    }
    
    pub fn set_selected_index(&mut self, index: usize) {
        self.selected_index = index.min(self.get_visible_files().len().saturating_sub(1));
    }
    
    pub fn move_selection_up(&mut self) {
        self.selected_index = self.selected_index.saturating_sub(1);
    }
    
    pub fn move_selection_down(&mut self) {
        let max = self.get_visible_files().len().saturating_sub(1);
        self.selected_index = (self.selected_index + 1).min(max);
    }
    
    pub fn navigate_to(&mut self, file: &str) -> Result<()> {
        match file {
            "../" => if let Some(parent) = self.current_dir.parent() {
                self.current_dir = parent.to_path_buf();
            },
            dir if dir.ends_with('/') => {
                self.current_dir = self.current_dir.join(&dir[..dir.len()-1]);
            },
            _ => return Ok(()),
        }
        self.scan_directory()
    }
}