use anyhow::Result;
use ropey::Rope;
use std::{path::PathBuf, process::Command};
use super::html_renderer::HtmlRenderer;

pub struct TextEditor {
    xml_rope: Rope,
    pdf_path: PathBuf,
    xml_cache: String,
    alto_cache: String,      // pdfalto output cache
    html_renderer: HtmlRenderer,
    alto_renderer: HtmlRenderer, // Separate renderer for Alto XML
    pan_offset: egui::Vec2,  // Manual pan offset for horizontal navigation
    current_page: usize,     // Current page being viewed (0-indexed)
    use_alto: bool,          // Toggle between pdftohtml and pdfalto
}

impl TextEditor {
    pub fn new(pdf_path: PathBuf) -> Result<Self> {
        let mut editor = Self {
            xml_rope: Rope::new(),
            pdf_path,
            xml_cache: String::new(),
            alto_cache: String::new(),
            html_renderer: HtmlRenderer::new(),
            alto_renderer: HtmlRenderer::new(),
            pan_offset: egui::Vec2::ZERO,
            current_page: 0,  // Start with page 1 (0-indexed)
            use_alto: false,
        };
        editor.extract_both_formats()?;
        Ok(editor)
    }
    
    fn extract_both_formats(&mut self) -> Result<()> {
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
        
        // Store pdftohtml XML
        self.xml_cache = html.clone();
        
        // Extract Alto XML using pdfalto (if available)
        let alto = if std::path::Path::new("/Users/jack/.local/bin/pdfalto").exists() {
            // pdfalto writes to a file, so we need to specify output location
            let temp_dir = std::env::temp_dir();
            let output_path = temp_dir.join("chonker8_alto");
            
            let result = Command::new("/Users/jack/.local/bin/pdfalto")
                .args([
                    "-noImage",         // Skip images for faster processing
                    "-readingOrder",    // Reorder blocks by reading sequence
                    self.pdf_path.to_str().unwrap(),
                    output_path.to_str().unwrap()
                ])
                .output();
                
            if result.is_ok() && result.unwrap().status.success() {
                // pdfalto creates a file without extension that contains the Alto XML
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
        
        // Parse both formats for rendering
        let _ = self.html_renderer.parse_html(&html);
        let _ = self.alto_renderer.parse_html(&alto);
        
        // Create rope for current format
        let current_xml = if self.use_alto { &self.alto_cache } else { &self.xml_cache };
        self.xml_rope = Rope::from_str(current_xml);
        
        Ok(())
    }
    
    pub fn get_xml_content(&self) -> String {
        if self.use_alto {
            self.alto_cache.clone()
        } else {
            self.xml_cache.clone()
        }
    }
    
    pub fn toggle_format(&mut self) {
        self.use_alto = !self.use_alto;
        // Update rope with current format
        let current_xml = if self.use_alto { &self.alto_cache } else { &self.xml_cache };
        self.xml_rope = Rope::from_str(current_xml);
    }
    
    pub fn is_using_alto(&self) -> bool {
        self.use_alto
    }
    
    pub fn get_current_format_name(&self) -> &str {
        if self.use_alto { "pdfalto-alto" } else { "pdftohtml-xml" }
    }
    
    
    pub fn render_html_content(&self, ui: &mut eframe::egui::Ui) {
        // Show current format XML for debugging  
        ui.collapsing(&format!("🔽 Raw {} XML", self.get_current_format_name()), |ui| {
            ui.visuals_mut().extreme_bg_color = egui::Color32::from_rgb(20, 20, 40); // Blue tint
            egui::ScrollArea::vertical()
                .max_height(150.0)
                .show(ui, |ui| {
                    let current_xml = self.get_xml_content();
                    ui.add(egui::TextEdit::multiline(&mut current_xml.as_str())
                        .font(egui::TextStyle::Monospace)
                        .desired_rows(10)
                        .desired_width(f32::INFINITY));
                });
        });
        
        // Show spatial layout using appropriate renderer
        if self.use_alto {
            self.render_alto_direct(ui);
        } else {
            self.html_renderer.render_page_with_offset(ui, self.current_page, self.pan_offset);
        }
    }
    
    fn render_alto_direct(&self, ui: &mut eframe::egui::Ui) {
        ui.label("🟢 Alto Mode Active - Parsing Alto XML...");
        println!("DEBUG: render_alto_direct() called");
        println!("DEBUG: Alto cache size: {} bytes", self.alto_cache.len());
        
        // Quick and dirty Alto XML renderer
        use quick_xml::Reader;
        use quick_xml::events::Event;
        
        let mut reader = Reader::from_str(&self.alto_cache);
        reader.config_mut().trim_text(true);
        let mut buf = Vec::new();
        
        let scale = 1.2;
        let available_size = ui.available_size();
        let (response, painter) = ui.allocate_painter(available_size, egui::Sense::click());
        let canvas_rect = response.rect;
        
        painter.rect_filled(canvas_rect, 0.0, egui::Color32::from_rgb(12, 12, 12));
        let origin = canvas_rect.min + self.pan_offset;
        
        // Parse and render String elements for current page only
        let mut string_count = 0;
        let mut current_page_num = 0;
        let mut in_target_page = false;
        
        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(e)) | Ok(Event::Empty(e)) => {
                    let tag_bytes = e.name().as_ref().to_vec();
                    let local_name = tag_bytes.split(|&b| b == b':').last().unwrap_or(&tag_bytes);
                    
                    // Track page boundaries
                    if local_name == b"Page" {
                        // Check PHYSICAL_IMG_NR to see if this is our target page
                        for attr in e.attributes() {
                            if let Ok(attr) = attr {
                                let key = std::str::from_utf8(attr.key.as_ref()).unwrap_or("");
                                if key == "PHYSICAL_IMG_NR" {
                                    let page_num: usize = String::from_utf8_lossy(&attr.value).parse().unwrap_or(1);
                                    in_target_page = page_num == (self.current_page + 1); // Convert 0-based to 1-based
                                    break;
                                }
                            }
                        }
                    }
                    
                    if local_name == b"String" && in_target_page {
                        string_count += 1;
                    let mut content = String::new();
                    let mut hpos = 0.0;
                    let mut vpos = 0.0;
                    let mut width = 0.0;
                    let mut height = 10.0;
                    
                    for attr in e.attributes() {
                        if let Ok(attr) = attr {
                            let key = std::str::from_utf8(attr.key.as_ref()).unwrap_or("");
                            let value = String::from_utf8_lossy(&attr.value);
                            
                            match key {
                                "CONTENT" => content = value.to_string(),
                                "HPOS" => hpos = value.parse().unwrap_or(0.0),
                                "VPOS" => vpos = value.parse().unwrap_or(0.0),
                                "WIDTH" => width = value.parse().unwrap_or(0.0),
                                "HEIGHT" => height = value.parse().unwrap_or(10.0),
                                _ => {}
                            }
                        }
                    }
                    
                    if !content.trim().is_empty() {
                        let pos = egui::pos2(
                            origin.x + hpos * scale,
                            origin.y + vpos * scale,
                        );
                        
                        painter.text(
                            pos,
                            egui::Align2::LEFT_TOP,
                            &content,
                            egui::FontId::monospace(height * scale),
                            egui::Color32::from_rgb(100, 255, 100), // Green for Alto
                        );
                    }
                    }
                }
                Ok(Event::End(e)) => {
                    let tag_bytes = e.name().as_ref().to_vec();
                    let local_name = tag_bytes.split(|&b| b == b':').last().unwrap_or(&tag_bytes);
                    if local_name == b"Page" {
                        in_target_page = false;
                    }
                }
                Ok(Event::Eof) => break,
                _ => {}
            }
            buf.clear();
        }
        
        println!("Alto renderer: Found {} String elements", string_count);
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