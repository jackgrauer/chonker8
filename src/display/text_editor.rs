use anyhow::Result;
use ropey::Rope;
use std::{path::PathBuf, process::Command};
use super::html_renderer::HtmlRenderer;

pub struct TextEditor {
    xml_rope: Rope,
    pdf_path: PathBuf,
    xml_cache: String,
    html_renderer: HtmlRenderer,
    pan_offset: egui::Vec2,  // Manual pan offset for horizontal navigation
    current_page: usize,     // Current page being viewed (0-indexed)
}

impl TextEditor {
    pub fn new(pdf_path: PathBuf) -> Result<Self> {
        let mut editor = Self {
            xml_rope: Rope::new(),
            pdf_path,
            xml_cache: String::new(),
            html_renderer: HtmlRenderer::new(),
            pan_offset: egui::Vec2::ZERO,
            current_page: 0,  // Start with page 1 (0-indexed)
        };
        editor.extract_xml()?;
        Ok(editor)
    }
    
    fn extract_xml(&mut self) -> Result<()> {
        // Extract XML using pdftohtml
        let html = Command::new("pdftohtml")
            .args([
                "-xml",             // XML output format with coordinates
                "-stdout",          // Output to stdout
                self.pdf_path.to_str().unwrap()
            ])
            .output()
            .ok()
            .filter(|o| o.status.success())
            .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
            .unwrap_or_else(|| format!(
                "Failed to extract XML from: {}\nInstall pdftohtml: brew install poppler",
                self.pdf_path.display()
            ));
        
        // Store XML
        self.xml_cache = html.clone();
        
        // Parse HTML for rendering
        let _ = self.html_renderer.parse_html(&html);
        
        // Create rope for XML
        self.xml_rope = Rope::from_str(&self.xml_cache);
        Ok(())
    }
    
    pub fn get_xml_content(&self) -> String {
        self.xml_cache.clone()
    }
    
    
    pub fn render_html_content(&self, ui: &mut eframe::egui::Ui) {
        // Show raw XML for debugging  
        ui.collapsing("📄 Raw XML", |ui| {
            ui.visuals_mut().extreme_bg_color = egui::Color32::from_rgb(20, 20, 40); // Blue tint
            egui::ScrollArea::vertical()
                .max_height(150.0)
                .show(ui, |ui| {
                    ui.add(egui::TextEdit::multiline(&mut self.xml_cache.clone())
                        .font(egui::TextStyle::Monospace)
                        .desired_rows(10)
                        .desired_width(f32::INFINITY));
                });
        });
        
        // Show spatial layout for current page only
        self.html_renderer.render_page_with_offset(ui, self.current_page, self.pan_offset);
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
    pub fn get_total_pages(&self) -> usize { self.html_renderer.get_page_count() }
    
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