use anyhow::Result;
use std::{path::PathBuf, process::Command};
use crate::alto_structure_editor::AltoStructureEditor;


pub struct TextEditor {
    pdf_path: PathBuf,
    alto_cache: String,      // pdfalto output cache
    pan_offset: egui::Vec2,  // Manual pan offset for horizontal navigation
    current_page: usize,     // Current page being viewed (0-indexed)
    alto_editor: Option<AltoStructureEditor>, // Native Alto XML structure
}

impl TextEditor {
    pub fn new(pdf_path: PathBuf) -> Result<Self> {
        let mut editor = Self {
            pdf_path,
            alto_cache: String::new(),
            pan_offset: egui::Vec2::ZERO,
            current_page: 0,  // Start with page 1 (0-indexed)
            alto_editor: None,
        };
        editor.extract_alto_xml()?;
        Ok(editor)
    }
    
    fn extract_alto_xml(&mut self) -> Result<()> {
        // Extract Alto XML using pdfalto
        let alto = if std::path::Path::new("/Users/jack/.local/bin/pdfalto").exists() {
            let temp_dir = std::env::temp_dir();
            let output_path = temp_dir.join("chonker8_alto");
            
            let result = Command::new("/Users/jack/.local/bin/pdfalto")
                .args([
                    "-noImage",         // Skip images for faster processing
                    "-readingOrder",    // Reorder blocks by reading sequence (key flag!)
                    "-outline",         // Include document outline/structure
                    "-noLineNumbers",   // Remove line numbering artifacts
                    "-fullFontName",    // Better font classification
                    self.pdf_path.to_str().unwrap(),
                    output_path.to_str().unwrap()
                ])
                .output();
                
            if result.is_ok() && result.unwrap().status.success() {
                std::fs::read_to_string(&output_path)
                    .unwrap_or_else(|_| "Failed to read generated Alto XML file".to_string())
            } else {
                "pdfalto execution failed".to_string()
            }
        } else {
            "pdfalto not found in PATH".to_string()
        };
        
        // Store Alto XML
        self.alto_cache = alto.clone();
        
        // Create Alto structure editor for all pages
        if !self.alto_cache.is_empty() {
            self.alto_editor = Some(AltoStructureEditor::from_alto_xml_all_pages(&self.alto_cache));
        }
        
        Ok(())
    }
    
    pub fn get_xml_content(&self) -> String {
        self.alto_cache.clone()
    }
    
    
    pub fn render_html_content(&mut self, ui: &mut eframe::egui::Ui) {
        // Show Alto XML for debugging  
        ui.collapsing("🔽 Raw Alto XML", |ui| {
            ui.visuals_mut().extreme_bg_color = egui::Color32::from_rgb(40, 20, 40); // Purple tint
            egui::ScrollArea::vertical()
                .max_height(150.0)
                .show(ui, |ui| {
                    let alto_xml = self.get_xml_content();
                    ui.add(egui::TextEdit::multiline(&mut alto_xml.as_str())
                        .font(egui::TextStyle::Monospace)
                        .desired_rows(10)
                        .desired_width(f32::INFINITY));
                });
        });
        
        // Show spatial grid editor
        self.render_alto_direct(ui);
    }
    
    fn render_alto_direct(&mut self, ui: &mut eframe::egui::Ui) {
        if let Some(alto_editor) = &mut self.alto_editor {
            // Render using native Alto XML structure
            let _changed = alto_editor.render(ui);
        } else {
            ui.label("Alto structure editor not initialized");
        }
    }
    
    
    
    
    pub fn pan_left(&mut self) {
        self.pan_offset.x += 50.0;
    }
    
    pub fn pan_right(&mut self) {
        self.pan_offset.x -= 50.0;
    }
    
    pub fn pan_up(&mut self) {
        self.pan_offset.y += 50.0;
    }
    
    pub fn pan_down(&mut self) {
        self.pan_offset.y -= 50.0;
    }
    
    pub fn get_current_page(&self) -> usize { self.current_page }
    pub fn get_total_pages(&self) -> usize { 
        // Extract page count from Alto XML
        2 // Simplified for now
    }
    
    pub fn next_page(&mut self) {
        let total = self.get_total_pages();
        if total > 0 && self.current_page < total - 1 {
            self.current_page += 1;
        }
    }
    
    pub fn prev_page(&mut self) {
        if self.current_page > 0 {
            self.current_page -= 1;
        }
    }
    
}