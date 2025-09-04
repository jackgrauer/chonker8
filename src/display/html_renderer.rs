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
    style_regex: Regex,
}

impl HtmlRenderer {
    pub fn new() -> Self {
        Self {
            elements: Vec::new(),
            fontspecs: HashMap::new(),
            pages: Vec::new(),
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
                        // Alto XML elements
                        "Page" => {
                            let page = self.parse_alto_page(e.attributes())?;
                            self.pages.push(page);
                        }
                        "String" => {
                            // Alto String elements are self-closing with CONTENT attribute
                            let text_elem = self.parse_alto_string(e.attributes())?;
                            println!("Found Alto String: '{}'", text_elem.text);
                            current_element.children.push(text_elem);
                            continue; // Don't push to stack, it's self-closing
                        }
                        "b" | "i" | "em" | "strong" | "SP" | "TextBlock" | "TextLine" | "Layout" | "PrintSpace" => {
                            continue;
                        }
                        _ => {}
                    }
                    
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
    
    fn parse_xml_attributes(&self, attributes: quick_xml::events::attributes::Attributes) -> Result<HtmlStyle> {
        let mut style = HtmlStyle::default();
        style.color = egui::Color32::from_rgb(220, 220, 220);
        
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
    
    fn parse_alto_page(&self, attributes: quick_xml::events::attributes::Attributes) -> Result<PageInfo> {
        let mut page = PageInfo::default();
        
        for attr in attributes {
            let attr = attr?;
            let key = String::from_utf8_lossy(attr.key.as_ref());
            let value = String::from_utf8_lossy(&attr.value);
            
            match key.as_ref() {
                "PHYSICAL_IMG_NR" => {
                    if let Ok(num) = value.parse::<u32>() {
                        page.number = num;
                    }
                }
                "WIDTH" => {
                    if let Ok(width) = value.parse::<f32>() {
                        page.width = width;
                    }
                }
                "HEIGHT" => {
                    if let Ok(height) = value.parse::<f32>() {
                        page.height = height;
                    }
                }
                _ => {}
            }
        }
        
        Ok(page)
    }
    
    fn parse_alto_string(&self, attributes: quick_xml::events::attributes::Attributes) -> Result<HtmlElement> {
        let mut element = HtmlElement {
            tag: "text".to_string(), // Convert to our format
            text: String::new(),
            style: HtmlStyle::default(),
            children: Vec::new(),
        };
        
        for attr in attributes {
            let attr = attr?;
            let key = String::from_utf8_lossy(attr.key.as_ref());
            let value = String::from_utf8_lossy(&attr.value);
            
            match key.as_ref() {
                "CONTENT" => element.text = value.to_string(),
                "HPOS" => {
                    if let Ok(left) = value.parse::<f32>() {
                        element.style.left = left;
                    }
                }
                "VPOS" => {
                    if let Ok(top) = value.parse::<f32>() {
                        element.style.top = top;
                    }
                }
                "WIDTH" => {
                    if let Ok(width) = value.parse::<f32>() {
                        element.style.width = width;
                    }
                }
                "HEIGHT" => {
                    if let Ok(height) = value.parse::<f32>() {
                        element.style.height = height;
                    }
                }
                "STYLEREFS" => {
                    element.style.font_id = Some(value.to_string());
                }
                _ => {}
            }
        }
        
        Ok(element)
    }
    
    pub fn get_page_count(&self) -> usize {
        self.pages.len()
    }
    
    pub fn get_fontspec(&self, font_id: &str) -> Option<&FontSpec> {
        self.fontspecs.get(font_id)
    }
    
    pub fn get_text_elements(&self, page_index: usize) -> Vec<&HtmlElement> {
        let mut page_elements = Vec::new();
        self.find_page_elements(&self.elements, &mut page_elements);
        
        if let Some(page_elem) = page_elements.get(page_index) {
            // For Alto XML, collect String elements recursively
            let mut text_elements = Vec::new();
            self.collect_text_elements_recursive(&page_elem.children, &mut text_elements);
            text_elements
        } else {
            Vec::new()
        }
    }
    
    fn collect_text_elements_recursive<'a>(&self, elements: &'a [HtmlElement], collector: &mut Vec<&'a HtmlElement>) {
        for element in elements {
            if element.tag == "text" && !element.text.trim().is_empty() {
                collector.push(element);
            } else if element.tag == "String" {
                // Alto String elements always have content (in CONTENT attribute)
                collector.push(element);
            }
            self.collect_text_elements_recursive(&element.children, collector);
        }
    }
    
    fn find_page_elements<'a>(&self, elements: &'a [HtmlElement], page_elements: &mut Vec<&'a HtmlElement>) {
        for element in elements {
            if element.tag == "page" || element.tag == "Page" {
                page_elements.push(element);
            }
            self.find_page_elements(&element.children, page_elements);
        }
    }
    
    fn count_elements(&self, elements: &[HtmlElement], text_count: &mut i32, page_count: &mut i32) {
        for element in elements {
            match element.tag.as_str() {
                "text" | "String" => *text_count += 1,
                "page" | "Page" => *page_count += 1,
                _ => {}
            }
            self.count_elements(&element.children, text_count, page_count);
        }
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
            ui.visuals_mut().extreme_bg_color = egui::Color32::from_rgb(20, 40, 20);
            egui::ScrollArea::vertical()
                .max_height(100.0)
                .show(ui, |ui| {
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
            ui.collapsing("🔍 Parsed Elements", |ui| {
                ui.visuals_mut().extreme_bg_color = egui::Color32::from_rgb(40, 20, 20);
                egui::ScrollArea::vertical()
                    .max_height(150.0)
                    .show(ui, |ui| {
                        let mut debug_text = String::new();
                        for (i, child) in page_elem.children.iter().enumerate() {
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
        for element in elements {
            if !element.text.trim().is_empty() {
                let pos = egui::pos2(
                    origin.x + element.style.left * scale,
                    origin.y + element.style.top * scale,
                );
                
                
                // Get font properties
                let (font_size, color) = if let Some(font_id) = &element.style.font_id {
                    if let Some(fontspec) = self.fontspecs.get(font_id) {
                        (fontspec.size * scale, egui::Color32::WHITE)
                    } else {
                        (element.style.height * scale * 0.8, egui::Color32::WHITE)
                    }
                } else {
                    (12.0 * scale, egui::Color32::WHITE)
                };
                
                // Render text
                painter.text(
                    pos,
                    egui::Align2::LEFT_TOP,
                    &element.text,
                    egui::FontId::proportional(font_size),
                    color,
                );
            }
            
            self.render_pdf_page(ui, &element.children, scale, origin, painter);
        }
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
        
        match element.tag.as_str() {
            "p" | "div" | "br" | "h1" | "h2" | "h3" => text.push('\n'),
            _ => {}
        }
    }
}