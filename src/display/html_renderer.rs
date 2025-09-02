use anyhow::Result;
use eframe::egui;
use quick_xml::events::Event;
use quick_xml::reader::Reader;
use regex::Regex;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct HtmlElement {
    pub tag: String,
    pub text: String,
    pub style: HtmlStyle,
    pub children: Vec<HtmlElement>,
}

#[derive(Debug, Clone, Default)]
pub struct FontSpec {
    pub id: String,
    pub size: f32,
    pub family: String,
    pub color: egui::Color32,
}

#[derive(Debug, Clone, Default)]
pub struct PageInfo {
    pub number: u32,
    pub width: f32,
    pub height: f32,
}

#[derive(Debug, Clone)]
pub struct TextBlock {
    pub text: String,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub font_size: f32,
    pub color: egui::Color32,
    pub font_family: String,
}

pub struct PageLayout {
    pub page_info: PageInfo,
    pub text_blocks: Vec<TextBlock>,
    pub occupied_areas: Vec<egui::Rect>,
}

#[derive(Debug, Clone, Default)]
pub struct HtmlStyle {
    pub font_size: f32,
    pub color: egui::Color32,
    pub bold: bool,
    pub italic: bool,
    pub left: f32,
    pub top: f32,
    pub width: f32,
    pub height: f32,
    pub font_id: Option<String>,
}

pub struct HtmlRenderer {
    elements: Vec<HtmlElement>,
    fontspecs: HashMap<String, FontSpec>,
    pages: Vec<PageInfo>,
    page_layout: Option<PageLayout>,
    style_regex: Regex,
}

impl HtmlRenderer {
    pub fn new() -> Self {
        Self {
            elements: Vec::new(),
            fontspecs: HashMap::new(),
            pages: Vec::new(),
            page_layout: None,
            style_regex: Regex::new(r"(\w+):\s*([^;]+)").unwrap(),
        }
    }
    
    pub fn parse_html(&mut self, html: &str) -> Result<()> {
        self.elements.clear();
        let mut reader = Reader::from_str(html);
        reader.config_mut().trim_text(true);
        
        let mut current_element = HtmlElement {
            tag: "root".to_string(),
            text: String::new(),
            style: HtmlStyle::default(),
            children: Vec::new(),
        };
        
        let mut buf = Vec::new();
        let mut element_stack = Vec::new();
        
        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(ref e)) => {
                    let tag_name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                    
                    // Parse metadata but preserve hierarchy
                    match tag_name.as_str() {
                        "fontspec" => {
                            let fontspec = self.parse_fontspec(e.attributes())?;
                            self.fontspecs.insert(fontspec.id.clone(), fontspec);
                        }
                        "page" => {
                            let page = self.parse_page_info(e.attributes())?;
                            self.pages.push(page);
                        }
                        "b" | "i" | "em" | "strong" => {
                            // For inline formatting tags, don't create separate elements
                            // Just continue with current element
                            continue;
                        }
                        _ => {}
                    }
                    
                    // Create element for everything except inline formatting
                    let element = HtmlElement {
                        tag: tag_name.clone(),
                        text: String::new(),
                        style: self.parse_xml_attributes(e.attributes())?,
                        children: Vec::new(),
                    };
                    
                    element_stack.push(current_element);
                    current_element = element;
                }
                Ok(Event::End(ref e)) => {
                    let tag_name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                    
                    // Skip end tags for inline formatting
                    if matches!(tag_name.as_str(), "b" | "i" | "em" | "strong") {
                        continue;
                    }
                    
                    if let Some(mut parent) = element_stack.pop() {
                        parent.children.push(current_element);
                        current_element = parent;
                    }
                }
                Ok(Event::Text(e)) => {
                    let text = String::from_utf8_lossy(&e);
                    current_element.text.push_str(&text);
                }
                Ok(Event::CData(e)) => {
                    let text = String::from_utf8_lossy(&e);
                    current_element.text.push_str(&text);
                }
                Ok(Event::Eof) => break,
                _ => {}
            }
            buf.clear();
        }
        
        self.elements = current_element.children;
        
        // Debug: count what we actually parsed
        let mut text_count = 0;
        let mut page_count = 0;
        self.count_elements(&self.elements, &mut text_count, &mut page_count);
        println!("XML parsing complete: {} text elements, {} page elements", text_count, page_count);
        
        Ok(())
    }
    
    
    fn build_page_layout(&mut self) {
        if let Some(page_info) = self.pages.first().cloned() {
            let mut text_blocks = Vec::new();
            let mut occupied_areas = Vec::new();
            
            // Convert all text elements to unified text blocks
            self.collect_text_blocks(&self.elements, &mut text_blocks, &mut occupied_areas);
            
            self.page_layout = Some(PageLayout {
                page_info,
                text_blocks,
                occupied_areas,
            });
        }
    }
    
    fn collect_text_blocks(&self, elements: &[HtmlElement], blocks: &mut Vec<TextBlock>, areas: &mut Vec<egui::Rect>) {
        for element in elements {
            if element.tag == "text" && !element.text.trim().is_empty() {
                // Get font properties
                let (font_size, color, font_family) = if let Some(font_id) = &element.style.font_id {
                    if let Some(fontspec) = self.fontspecs.get(font_id) {
                        (fontspec.size, fontspec.color, fontspec.family.clone())
                    } else {
                        (12.0, egui::Color32::from_rgb(220, 220, 220), "Arial".to_string())
                    }
                } else {
                    (12.0, egui::Color32::from_rgb(220, 220, 220), "Arial".to_string())
                };
                
                let text_block = TextBlock {
                    text: element.text.clone(),
                    x: element.style.left,
                    y: element.style.top,
                    width: element.style.width,
                    height: element.style.height,
                    font_size,
                    color,
                    font_family,
                };
                
                // Track occupied area for overlap detection
                let area = egui::Rect::from_min_size(
                    egui::pos2(element.style.left, element.style.top),
                    egui::vec2(element.style.width, element.style.height)
                );
                
                blocks.push(text_block);
                areas.push(area);
            }
            
            // Recurse through children
            self.collect_text_blocks(&element.children, blocks, areas);
        }
    }
    
    fn parse_xml_attributes(&self, attributes: quick_xml::events::attributes::Attributes) -> Result<HtmlStyle> {
        let mut style = HtmlStyle::default();
        style.color = egui::Color32::from_rgb(220, 220, 220); // Default text color
        
        for attr in attributes {
            let attr = attr?;
            let key = String::from_utf8_lossy(attr.key.as_ref());
            let value = String::from_utf8_lossy(&attr.value);
            
            match key.as_ref() {
                "top" => {
                    if let Ok(top) = value.parse::<f32>() {
                        style.top = top;
                    }
                }
                "left" => {
                    if let Ok(left) = value.parse::<f32>() {
                        style.left = left;
                    }
                }
                "width" => {
                    if let Ok(width) = value.parse::<f32>() {
                        style.width = width;
                    }
                }
                "height" => {
                    if let Ok(height) = value.parse::<f32>() {
                        style.height = height;
                        // Don't use height as font size - get it from fontspec instead
                    }
                }
                "font" => {
                    style.font_id = Some(value.to_string());
                }
                "style" => {
                    style = self.parse_css_style(&value, style);
                }
                _ => {}
            }
        }
        
        Ok(style)
    }
    
    fn parse_fontspec(&self, attributes: quick_xml::events::attributes::Attributes) -> Result<FontSpec> {
        let mut fontspec = FontSpec::default();
        
        for attr in attributes {
            let attr = attr?;
            let key = String::from_utf8_lossy(attr.key.as_ref());
            let value = String::from_utf8_lossy(&attr.value);
            
            match key.as_ref() {
                "id" => fontspec.id = value.to_string(),
                "size" => {
                    if let Ok(size) = value.parse::<f32>() {
                        fontspec.size = size;
                    }
                }
                "family" => fontspec.family = value.to_string(),
                "color" => {
                    if let Some(color) = self.parse_color(&value) {
                        fontspec.color = color;
                    } else {
                        fontspec.color = egui::Color32::from_rgb(220, 220, 220);
                    }
                }
                _ => {}
            }
        }
        
        Ok(fontspec)
    }
    
    fn parse_page_info(&self, attributes: quick_xml::events::attributes::Attributes) -> Result<PageInfo> {
        let mut page = PageInfo::default();
        
        for attr in attributes {
            let attr = attr?;
            let key = String::from_utf8_lossy(attr.key.as_ref());
            let value = String::from_utf8_lossy(&attr.value);
            
            match key.as_ref() {
                "number" => {
                    if let Ok(num) = value.parse::<u32>() {
                        page.number = num;
                    }
                }
                "width" => {
                    if let Ok(width) = value.parse::<f32>() {
                        page.width = width;
                    }
                }
                "height" => {
                    if let Ok(height) = value.parse::<f32>() {
                        page.height = height;
                    }
                }
                _ => {}
            }
        }
        
        Ok(page)
    }
    
    pub fn get_page_count(&self) -> usize {
        self.pages.len()
    }
    
    fn find_page_elements<'a>(&self, elements: &'a [HtmlElement], page_elements: &mut Vec<&'a HtmlElement>) {
        for element in elements {
            if element.tag == "page" {
                page_elements.push(element);
            }
            // Recurse through children
            self.find_page_elements(&element.children, page_elements);
        }
    }
    
    fn count_elements(&self, elements: &[HtmlElement], text_count: &mut i32, page_count: &mut i32) {
        for element in elements {
            match element.tag.as_str() {
                "text" => *text_count += 1,
                "page" => *page_count += 1,
                _ => {}
            }
            self.count_elements(&element.children, text_count, page_count);
        }
    }
    
    fn get_page_number(&self, element: &HtmlElement) -> usize {
        // For now, just use the page index from the pages vector
        // since we're storing pages in order
        if element.tag == "page" {
            // Find which page this is by comparing with stored pages
            for (index, _) in self.pages.iter().enumerate() {
                // This is a simplified approach - match by index
                return index + 1;
            }
        }
        0
    }
    
    fn parse_css_style(&self, css: &str, mut style: HtmlStyle) -> HtmlStyle {
        for cap in self.style_regex.captures_iter(css) {
            let property = cap.get(1).map(|m| m.as_str()).unwrap_or("");
            let value = cap.get(2).map(|m| m.as_str()).unwrap_or("");
            
            match property {
                "font-size" => {
                    if let Ok(size) = value.trim_end_matches("px").parse::<f32>() {
                        style.font_size = size;
                    }
                }
                "color" => {
                    if let Some(color) = self.parse_color(value) {
                        style.color = color;
                    }
                }
                "left" => {
                    if let Ok(left) = value.trim_end_matches("px").parse::<f32>() {
                        style.left = left;
                    }
                }
                "top" => {
                    if let Ok(top) = value.trim_end_matches("px").parse::<f32>() {
                        style.top = top;
                    }
                }
                "width" => {
                    if let Ok(width) = value.trim_end_matches("px").parse::<f32>() {
                        style.width = width;
                    }
                }
                "height" => {
                    if let Ok(height) = value.trim_end_matches("px").parse::<f32>() {
                        style.height = height;
                    }
                }
                "font-weight" => {
                    style.bold = value.contains("bold");
                }
                "font-style" => {
                    style.italic = value.contains("italic");
                }
                _ => {}
            }
        }
        
        style
    }
    
    fn parse_color(&self, color_str: &str) -> Option<egui::Color32> {
        if color_str.starts_with('#') && color_str.len() == 7 {
            if let Ok(hex) = u32::from_str_radix(&color_str[1..], 16) {
                let r = ((hex >> 16) & 0xFF) as u8;
                let g = ((hex >> 8) & 0xFF) as u8;
                let b = (hex & 0xFF) as u8;
                return Some(egui::Color32::from_rgb(r, g, b));
            }
        } else if color_str.starts_with("rgb(") {
            // Basic RGB parsing - could be expanded
            let rgb_str = color_str.trim_start_matches("rgb(").trim_end_matches(')');
            let parts: Vec<&str> = rgb_str.split(',').collect();
            if parts.len() == 3 {
                if let (Ok(r), Ok(g), Ok(b)) = (
                    parts[0].trim().parse::<u8>(),
                    parts[1].trim().parse::<u8>(),
                    parts[2].trim().parse::<u8>(),
                ) {
                    return Some(egui::Color32::from_rgb(r, g, b));
                }
            }
        }
        None
    }
    
    pub fn render(&self, ui: &mut egui::Ui) {
        self.render_with_offset(ui, egui::Vec2::ZERO);
    }
    
    pub fn render_with_offset(&self, ui: &mut egui::Ui, pan_offset: egui::Vec2) {
        self.render_page_with_offset(ui, 0, pan_offset);
    }
    
    pub fn render_page_with_offset(&self, ui: &mut egui::Ui, page_index: usize, pan_offset: egui::Vec2) {
        if self.elements.is_empty() {
            ui.label("No content");
            return;
        }
        
        // Debug: parsing summary panel
        ui.collapsing("⚙️ Parsing Summary", |ui| {
            ui.visuals_mut().extreme_bg_color = egui::Color32::from_rgb(20, 40, 20); // Green tint
            egui::ScrollArea::vertical()
                .max_height(100.0)
                .show(ui, |ui| {
                    // Get page info first
                    let mut page_elements = Vec::new();
                    self.find_page_elements(&self.elements, &mut page_elements);
                    
                    let mut summary_text = format!(
                        "Found {} pages in XML\n\
                        Total root elements: {}\n\
                        Requested page: {}\n\
                        Page elements found: {}\n",
                        self.pages.len(), self.elements.len(), page_index + 1, page_elements.len()
                    );
                    
                    if let Some(page_elem) = page_elements.get(page_index) {
                        summary_text.push_str(&format!("Rendering page {} with {} children\n", 
                            page_index + 1, page_elem.children.len()));
                    }
                    
                    summary_text.push_str("Root element tags: ");
                    for element in &self.elements {
                        summary_text.push_str(&format!("[{}] ", element.tag));
                    }
                    
                    ui.add(egui::TextEdit::multiline(&mut summary_text)
                        .font(egui::TextStyle::Monospace)
                        .desired_rows(5)
                        .desired_width(f32::INFINITY));
                });
        });
        
        // Get the page elements  
        let mut page_elements = Vec::new();
        self.find_page_elements(&self.elements, &mut page_elements);
        
        if let Some(page_elem) = page_elements.get(page_index) {
            
            // Debug: scrollable text content preview
            ui.collapsing("🔍 Parsed Elements", |ui| {
                ui.visuals_mut().extreme_bg_color = egui::Color32::from_rgb(40, 20, 20); // Red tint
                egui::ScrollArea::vertical()
                    .max_height(150.0)
                    .show(ui, |ui| {
                        let mut debug_text = String::new();
                        for (i, child) in page_elem.children.iter().enumerate() {
                            // Show ALL children, even empty ones, to see what we're missing
                            let text_preview = if child.text.trim().is_empty() {
                                "[EMPTY]".to_string()
                            } else {
                                child.text.chars().take(80).collect::<String>()
                            };
                            debug_text.push_str(&format!("{}. [{}] at ({:.0},{:.0}): \"{}\"\n", 
                                i+1, child.tag, child.style.left, child.style.top, text_preview));
                        }
                        ui.add(egui::TextEdit::multiline(&mut debug_text)
                            .font(egui::TextStyle::Monospace)
                            .desired_rows(10)
                            .desired_width(f32::INFINITY));
                    });
            });
            ui.separator();
            
            let (page_width, page_height) = if let Some(page) = self.pages.get(page_index) {
                (page.width, page.height)
            } else {
                (612.0, 792.0)
            };
            
            let scale = 1.2;
            let canvas_size = egui::vec2(page_width * scale * 2.0, page_height * scale);
            
            let (response, painter) = ui.allocate_painter(canvas_size, egui::Sense::click());
            let canvas_rect = response.rect;
            
            painter.rect_filled(canvas_rect, 0.0, egui::Color32::from_rgb(12, 12, 12));
            
            let origin = canvas_rect.min + pan_offset;
            
            // Render only this page's content
            self.render_pdf_page(ui, &page_elem.children, scale, origin, &painter);
        } else {
            ui.label(format!("Page {} not available", page_index + 1));
        }
    }
    
    fn render_pdf_page(&self, ui: &mut egui::Ui, elements: &[HtmlElement], scale: f32, origin: egui::Pos2, painter: &egui::Painter) {
        // Get page dimensions for coordinate transformation
        let page_height = if let Some(page) = self.pages.first() {
            page.height
        } else {
            792.0 // Default page height
        };
        
        for element in elements {
            // Render ANY element with content (not just "text" tags) 
            if !element.text.trim().is_empty() {
                let pos = egui::pos2(
                    origin.x + element.style.left * scale,
                    origin.y + element.style.top * scale,
                );
                
                // Debug: show element type and coordinates
                painter.text(
                    egui::pos2(pos.x - 60.0, pos.y),
                    egui::Align2::LEFT_TOP,
                    &format!("[{}]({:.0},{:.0})", element.tag, element.style.left, element.style.top),
                    egui::FontId::monospace(7.0),
                    egui::Color32::YELLOW,
                );
                
                // Get proper font size from fontspec (not element height)
                let (font_size, color) = if let Some(font_id) = &element.style.font_id {
                    if let Some(fontspec) = self.fontspecs.get(font_id) {
                        (fontspec.size * scale, egui::Color32::WHITE)
                    } else {
                        // Fallback: use element height as rough font size approximation, but scale it properly
                        (element.style.height * scale * 0.8, egui::Color32::WHITE)
                    }
                } else {
                    (element.style.height * scale * 0.8, egui::Color32::WHITE)
                };
                
                // Draw small position marker for debugging
                painter.circle_filled(pos, 2.0, egui::Color32::RED);
                
                // Draw text at transformed position
                painter.text(
                    pos,
                    egui::Align2::LEFT_TOP,
                    &element.text,
                    egui::FontId::proportional(font_size),
                    color,
                );
            }
            
            // Recurse through children
            self.render_pdf_page(ui, &element.children, scale, origin, painter);
        }
    }
    
    fn render_spatial_layout(&self, ui: &mut egui::Ui) {
        if let Some(page_layout) = &self.page_layout {
            let page_info = &page_layout.page_info;
            
            // Use large fixed scale for readability
            let scale = 2.5; // Even bigger for better spacing
            
            // Create large canvas based on actual page dimensions
            let canvas_size = egui::vec2(page_info.width * scale, page_info.height * scale);
            
            // Allocate painter for the entire page
            let (response, painter) = ui.allocate_painter(canvas_size, egui::Sense::click());
            let canvas_rect = response.rect;
            
            // Draw page background
            painter.rect_filled(canvas_rect, 0.0, egui::Color32::from_rgb(12, 12, 12));
            
            // Render all text blocks as one cohesive page
            for text_block in &page_layout.text_blocks {
                self.render_text_block(ui, text_block, scale, canvas_rect.min, &painter);
            }
        } else {
            ui.label("No page layout available");
        }
    }
    
    fn render_text_block(&self, ui: &mut egui::Ui, block: &TextBlock, scale: f32, origin: egui::Pos2, painter: &egui::Painter) {
        // Get page height for coordinate transformation
        let page_height = if let Some(page) = self.pages.first() {
            page.height
        } else {
            792.0
        };
        
        // Calculate scaled position with PDF-to-screen coordinate transformation
        let pos = egui::pos2(
            origin.x + block.x * scale,
            origin.y + (page_height - block.y - block.height) * scale,
        );
        
        // Calculate scaled size
        let size = egui::vec2(
            block.width * scale,
            block.height * scale,
        );
        
        let text_rect = egui::Rect::from_min_size(pos, size);
        
        // Render text at exact position with proper font
        painter.text(
            pos,
            egui::Align2::LEFT_TOP,
            &block.text,
            egui::FontId::proportional(block.font_size * scale),
            block.color,
        );
        
        // Make it clickable for editing
        let response = ui.allocate_rect(text_rect, egui::Sense::click());
        
        if response.hovered() {
            painter.rect_stroke(text_rect, 0.0, egui::Stroke::new(1.0, egui::Color32::YELLOW));
        }
        
        if response.clicked() {
            // Handle editing (could add edit state here)
        }
    }
    
    fn render_positioned_text(&self, ui: &mut egui::Ui, element: &HtmlElement, scale: f32, origin: egui::Pos2, painter: &egui::Painter) {
        if element.tag == "text" && !element.text.trim().is_empty() {
            // Get page height for coordinate transformation
            let page_height = if let Some(page) = self.pages.first() {
                page.height
            } else {
                792.0
            };
            
            // Calculate exact position with PDF-to-screen coordinate transformation
            let pos = egui::pos2(
                origin.x + element.style.left * scale,
                origin.y + (page_height - element.style.top - element.style.height) * scale,
            );
            
            // Get font properties from fontspec
            let (font_size, color) = if let Some(font_id) = &element.style.font_id {
                if let Some(fontspec) = self.fontspecs.get(font_id) {
                    (fontspec.size * scale, fontspec.color)
                } else {
                    (12.0 * scale, egui::Color32::from_rgb(220, 220, 220))
                }
            } else {
                (12.0 * scale, egui::Color32::from_rgb(220, 220, 220))
            };
            
            // Create invisible button for click detection
            let text_size = egui::vec2(element.style.width * scale, element.style.height * scale);
            let text_rect = egui::Rect::from_min_size(pos, text_size);
            
            let text_response = ui.allocate_rect(text_rect, egui::Sense::click());
            
            // Paint the text at exact coordinates
            painter.text(
                pos,
                egui::Align2::LEFT_TOP,
                &element.text,
                egui::FontId::proportional(font_size),
                color,
            );
            
            // Show edit indicator if hovered
            if text_response.hovered() {
                painter.rect_stroke(text_rect, 0.0, egui::Stroke::new(1.0, egui::Color32::YELLOW));
            }
            
            // Handle click for editing (would need to implement editing state)
            if text_response.clicked() {
                // Could switch to edit mode for this text element
            }
        }
        
        // Render children
        for child in &element.children {
            self.render_positioned_text(ui, child, scale, origin, painter);
        }
    }
    
    fn collect_text_elements(&self, elements: &[HtmlElement], collector: &mut Vec<HtmlElement>) {
        for element in elements {
            if element.tag == "text" {
                collector.push(element.clone());
            }
            self.collect_text_elements(&element.children, collector);
        }
    }
    
    fn render_positioned_element(&self, ui: &mut egui::Ui, element: &HtmlElement, scale: f32, canvas_origin: egui::Pos2) -> Option<String> {
        let mut edited_text = None;
        
        if element.tag == "text" && !element.text.trim().is_empty() {
            // Get page height for coordinate transformation
            let page_height = if let Some(page) = self.pages.first() {
                page.height
            } else {
                792.0
            };
            
            // Calculate scaled position and size with PDF-to-screen coordinate transformation
            let pos = egui::pos2(
                canvas_origin.x + element.style.left * scale,
                canvas_origin.y + (page_height - element.style.top - element.style.height) * scale,
            );
            
            let size = egui::vec2(
                element.style.width * scale,
                element.style.height * scale,
            );
            
            // Get font info
            let (font_size, font_color) = if let Some(font_id) = &element.style.font_id {
                if let Some(fontspec) = self.fontspecs.get(font_id) {
                    (fontspec.size * scale, fontspec.color)
                } else {
                    (element.style.font_size * scale, element.style.color)
                }
            } else {
                (element.style.font_size * scale, element.style.color)
            };
            
            // Create an editable text field at the exact position
            let text_rect = egui::Rect::from_min_size(pos, size);
            let mut text_content = element.text.clone();
            
            ui.allocate_new_ui(egui::UiBuilder::new().max_rect(text_rect), |ui| {
                let response = ui.add(
                    egui::TextEdit::singleline(&mut text_content)
                        .font(egui::FontId::proportional(font_size))
                        .text_color(font_color)
                        .frame(false) // No border for seamless look
                        .margin(egui::Vec2::ZERO)
                );
                
                if response.changed() {
                    edited_text = Some(text_content);
                }
            });
        }
        
        // Render children and collect their edits
        for child in &element.children {
            if let Some(_) = self.render_positioned_element(ui, child, scale, canvas_origin) {
                // Handle child edits if needed
            }
        }
        
        edited_text
    }
    
    pub fn get_plain_text(&self) -> String {
        let mut text = String::new();
        for element in &self.elements {
            self.extract_text_from_element(element, &mut text);
        }
        text
    }
    
    fn extract_text_from_element(&self, element: &HtmlElement, text: &mut String) {
        if !element.text.trim().is_empty() {
            text.push_str(&element.text);
            text.push(' ');
        }
        
        for child in &element.children {
            self.extract_text_from_element(child, text);
        }
        
        // Add newlines for block elements
        match element.tag.as_str() {
            "p" | "div" | "br" | "h1" | "h2" | "h3" => text.push('\n'),
            _ => {}
        }
    }
}