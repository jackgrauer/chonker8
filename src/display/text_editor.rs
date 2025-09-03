use anyhow::Result;
use ropey::Rope;
use std::{path::PathBuf, process::Command};
use super::html_renderer::HtmlRenderer;

pub struct TextEditor {
    rope: Rope,
    xml_rope: Rope,  // Separate rope for XML content
    pdf_path: PathBuf,
    text_cache: String,
    xml_cache: String,
    html_renderer: HtmlRenderer,
    modified: bool,
    block_mode: bool,
    render_html: bool,
    pan_offset: egui::Vec2,  // Manual pan offset for horizontal navigation
    current_page: usize,     // Current page being viewed (0-indexed)
    clipboard: Option<arboard::Clipboard>,
}

impl TextEditor {
    pub fn new(pdf_path: PathBuf) -> Result<Self> {
        let mut editor = Self {
            rope: Rope::new(),
            xml_rope: Rope::new(),
            pdf_path,
            text_cache: String::new(),
            xml_cache: String::new(),
            html_renderer: HtmlRenderer::new(),
            modified: false,
            block_mode: false,
            render_html: true,
            pan_offset: egui::Vec2::ZERO,
            current_page: 0,  // Start with page 1 (0-indexed)
            clipboard: arboard::Clipboard::new().ok(),
        };
        editor.extract_both_formats()?;
        Ok(editor)
    }
    
    fn extract_both_formats(&mut self) -> Result<()> {
        // Extract clean text using pdftotext
        let text = Command::new("pdftotext")
            .args(["-layout", "-nopgbrk", self.pdf_path.to_str().unwrap(), "-"])
            .output()
            .ok()
            .filter(|o| o.status.success())
            .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
            .unwrap_or_else(|| format!(
                "Failed to extract text from: {}\nInstall pdftotext: brew install poppler",
                self.pdf_path.display()
            ));
        
        // Try basic XML extraction like pdftotext (minimal flags)
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
                "Failed to extract HTML from: {}\nInstall pdftohtml: brew install poppler",
                self.pdf_path.display()
            ));
        
        // Store both formats
        self.text_cache = text;
        self.xml_cache = html.clone();
        
        // Parse HTML for rendering
        let _ = self.html_renderer.parse_html(&html);
        
        // Create ropes for both formats
        self.rope = Rope::from_str(&self.text_cache);
        self.xml_rope = Rope::from_str(&self.xml_cache);
        self.modified = false;
        Ok(())
    }
    
    pub fn get_text(&self) -> String { 
        // Always return the actual text content for editing, not XML
        self.text_cache.clone()
    }
    
    pub fn get_xml_content(&self) -> String {
        self.xml_cache.clone()
    }
    
    pub fn set_text(&mut self, text: String) {
        // Always edit the actual text content (not XML)
        if text != self.text_cache {
            self.rope = Rope::from_str(&text);
            self.text_cache = text;
            self.modified = true;
        }
    }
    
    pub fn is_block_mode(&self) -> bool { self.block_mode }
    pub fn toggle_block_selection(&mut self) { self.block_mode = !self.block_mode; }
    pub fn reload_pdf_content(&mut self) -> Result<()> { self.extract_both_formats() }
    
    pub fn toggle_html_rendering(&mut self) { self.render_html = !self.render_html; }
    pub fn is_html_rendering(&self) -> bool { self.render_html }
    
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
    
    pub fn copy_selection(&mut self) {
        if let Some(clipboard) = &mut self.clipboard {
            let _ = clipboard.set_text(self.text_cache.clone());
        }
    }
    
    pub fn cut_selection(&mut self) { self.copy_selection(); }
    pub fn paste(&mut self) { /* egui handles this */ }
    pub fn select_all(&mut self) { /* egui handles this */ }
}