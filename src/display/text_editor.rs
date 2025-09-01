// Clean egui-based text editor with rope backend
use anyhow::Result;
use ropey::Rope;
use std::{
    path::PathBuf,
    process::Command,
};

#[derive(Debug, Clone, PartialEq)]
pub enum SelectionMode {
    None,
    Normal,
    Block,
}

pub struct TextEditor {
    rope: Rope,
    pdf_path: PathBuf,
    text_cache: String,
    modified: bool,
    block_selection_mode: bool,
    clipboard: Option<arboard::Clipboard>,
}

impl TextEditor {
    pub fn new(pdf_path: PathBuf) -> Result<Self> {
        let mut editor = Self {
            rope: Rope::new(),
            pdf_path,
            text_cache: String::new(),
            modified: false,
            block_selection_mode: false,
            clipboard: arboard::Clipboard::new().ok(),
        };
        
        editor.extract_pdf_text()?;
        Ok(editor)
    }
    
    fn extract_pdf_text(&mut self) -> Result<()> {
        let output = Command::new("pdftotext")
            .args(&[
                "-layout",
                "-nopgbrk",
                self.pdf_path.to_str().unwrap(),
                "-"
            ])
            .output();
        
        let text = match output {
            Ok(output) if output.status.success() => {
                String::from_utf8_lossy(&output.stdout).to_string()
            }
            _ => {
                format!(
                    "Failed to extract text from: {}\n\
                    Make sure 'pdftotext' is installed: brew install poppler",
                    self.pdf_path.display()
                )
            }
        };
        
        self.rope = Rope::from_str(&text);
        self.text_cache = text;
        self.modified = false;
        
        Ok(())
    }
    
    pub fn get_text(&self) -> String {
        self.text_cache.clone()
    }
    
    pub fn set_text(&mut self, text: String) {
        if text != self.text_cache {
            self.rope = Rope::from_str(&text);
            self.text_cache = text;
            self.modified = true;
        }
    }
    
    pub fn is_block_mode(&self) -> bool {
        self.block_selection_mode
    }
    
    pub fn toggle_block_selection(&mut self) {
        self.block_selection_mode = !self.block_selection_mode;
    }
    
    pub fn copy_selection(&mut self) {
        if let Some(ref mut clipboard) = self.clipboard {
            let _ = clipboard.set_text(self.text_cache.clone());
        }
    }
    
    pub fn cut_selection(&mut self) {
        self.copy_selection();
    }
    
    pub fn paste(&mut self) {
        // egui handles this
    }
    
    pub fn select_all(&mut self) {
        // egui handles this
    }
    
    pub fn reload_pdf_content(&mut self) -> Result<()> {
        self.extract_pdf_text()
    }
}