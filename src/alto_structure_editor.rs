use eframe::egui;
use ropey::Rope;
use crate::crf_integration::WapitiCRF;
use crate::grobid_heuristics::DocumentStructure;
use crate::metal_document_classifier::{DocumentClassifier, ModelSetup};

pub struct AltoStructureEditor {
    text_blocks: Vec<AltoTextBlock>,
    page_width: f32,
    page_height: f32,
    total_words: usize,
    crf_model: WapitiCRF,
    ml_classifier: Option<DocumentClassifier>,
    classifications: Vec<DocumentStructure>,
    ml_enabled: bool,
}

#[derive(Clone, Debug)]
pub struct AltoTextBlock {
    pub content: String,
    pub rope: Rope,
    block_id: String,
    alignment: TextAlignment,
    pub block_style: BlockStyle,
    pub bbox: BoundingBox,
}

#[derive(Clone, Debug)]
pub enum TextAlignment {
    Left,
    Center,
    Right,
}

#[derive(Clone, Debug)]
pub struct BlockStyle {
    pub color: egui::Color32,
    pub font_size: f32,
    pub is_bold: bool,
}

#[derive(Clone, Debug)]
pub struct BoundingBox {
    pub left: f32,
    pub top: f32,
    pub width: f32,
    pub height: f32,
}

impl AltoStructureEditor {
    pub fn from_alto_xml_all_pages(xml: &str) -> Self {
        use quick_xml::{Reader, events::Event};
        
        let mut text_blocks = Vec::new();
        let mut page_width = 612.0;
        let mut page_height = 792.0;
        let mut total_words = 0;
        
        let mut reader = Reader::from_str(xml);
        reader.config_mut().trim_text(true);
        let mut buf = Vec::new();
        
        let mut current_page_num = 0;
        let mut current_block_content = String::new();
        let mut current_block_id = String::new();
        let mut current_block_hpos = 0.0;
        let mut current_block_vpos = 0.0;
        let mut current_block_width = 0.0;
        let mut current_block_height = 0.0;
        let mut in_textblock = false;
        let mut line_words: Vec<String> = Vec::new();
        let mut collected_styles: Vec<String> = Vec::new();
        
        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(e)) => {
                    let tag_name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                    
                    if tag_name.contains("Page") {
                        for attr in e.attributes() {
                            if let Ok(attr) = attr {
                                let key = std::str::from_utf8(attr.key.as_ref()).unwrap_or("");
                                let value = String::from_utf8_lossy(&attr.value);
                                
                                match key {
                                    "PHYSICAL_IMG_NR" => {
                                        current_page_num = value.parse().unwrap_or(1);
                                    }
                                    "WIDTH" => page_width = value.parse().unwrap_or(612.0),
                                    "HEIGHT" => page_height = value.parse().unwrap_or(792.0),
                                    _ => {}
                                }
                            }
                        }
                    } else if tag_name.contains("TextBlock") {
                        // Add page separator for pages after the first
                        if current_page_num > 1 && !text_blocks.is_empty() {
                            text_blocks.push(AltoTextBlock {
                                content: format!("\n\n--- PAGE {} ---\n", current_page_num),
                                rope: Rope::from_str(&format!("\n\n--- PAGE {} ---\n", current_page_num)),
                                block_id: format!("page_separator_{}", current_page_num),
                                alignment: TextAlignment::Center,
                                block_style: BlockStyle {
                                    color: egui::Color32::from_rgb(120, 120, 120),
                                    font_size: 14.0,
                                    is_bold: true,
                                },
                                bbox: BoundingBox {
                                    left: page_width / 2.0 - 100.0,
                                    top: 0.0,
                                    width: 200.0,
                                    height: 20.0,
                                },
                            });
                        }
                        
                        in_textblock = true;
                        current_block_content.clear();
                        collected_styles.clear();
                        
                        for attr in e.attributes() {
                            if let Ok(attr) = attr {
                                let key = std::str::from_utf8(attr.key.as_ref()).unwrap_or("");
                                let value = String::from_utf8_lossy(&attr.value);
                                
                                match key {
                                    "ID" => current_block_id = value.to_string(),
                                    "HPOS" => current_block_hpos = value.parse().unwrap_or(0.0),
                                    "VPOS" => current_block_vpos = value.parse().unwrap_or(0.0),
                                    "WIDTH" => current_block_width = value.parse().unwrap_or(0.0),
                                    "HEIGHT" => current_block_height = value.parse().unwrap_or(0.0),
                                    _ => {}
                                }
                            }
                        }
                    } else if tag_name.contains("TextLine") && in_textblock {
                        // Start collecting words for this line
                        line_words.clear();
                    }
                }
                Ok(Event::Empty(e)) => {
                    let tag_name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                    
                    if tag_name.contains("String") && in_textblock {
                        let mut content = String::new();
                        let mut style_refs = String::new();
                        
                        for attr in e.attributes() {
                            if let Ok(attr) = attr {
                                let key = std::str::from_utf8(attr.key.as_ref()).unwrap_or("");
                                let value = String::from_utf8_lossy(&attr.value);
                                
                                match key {
                                    "CONTENT" => content = value.to_string(),
                                    "STYLEREFS" => style_refs = value.to_string(),
                                    _ => {}
                                }
                            }
                        }
                        
                        // Collect style references for italic detection
                        if !style_refs.is_empty() {
                            collected_styles.push(style_refs.clone());
                        }
                        
                        if !content.trim().is_empty() {
                            // Conservative superscript detection - only obvious footnote markers
                            let is_superscript = style_refs.to_lowercase().contains("superscript") &&
                                               (content.starts_with("(") && content.ends_with(")") && content.len() <= 4) ||
                                               (content.len() <= 2 && content.chars().all(|c| c.is_numeric()));
                            
                            // Convert to Unicode superscript if possible
                            if is_superscript {
                                let superscript_text = Self::convert_to_superscript(&content);
                                
                                // Merge with previous word if it exists (no space)
                                if let Some(last_word) = line_words.last_mut() {
                                    last_word.push_str(&superscript_text);
                                    // Don't add to line_words - already merged
                                } else {
                                    line_words.push(superscript_text);
                                }
                            } else {
                                line_words.push(content);
                            };
                            total_words += 1;
                        }
                    }
                }
                Ok(Event::End(e)) => {
                    let tag_name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                    
                    if tag_name.contains("TextLine") && in_textblock {
                        // End of line - add words to block content
                        if !line_words.is_empty() {
                            if !current_block_content.is_empty() {
                                current_block_content.push('\n');
                            }
                            current_block_content.push_str(&line_words.join(" "));
                            line_words.clear();
                        }
                    } else if tag_name.contains("TextBlock") && in_textblock {
                        // End of TextBlock - create block
                        if !current_block_content.trim().is_empty() {
                            // Determine alignment from HPOS
                            let alignment = if current_block_hpos < 120.0 {
                                TextAlignment::Left
                            } else if current_block_hpos > page_width / 2.0 - 100.0 && 
                                     current_block_hpos < page_width / 2.0 + 100.0 {
                                TextAlignment::Center
                            } else {
                                TextAlignment::Left
                            };
                            
                            // Create accurate bounding box from Alto data
                            let bbox = BoundingBox {
                                left: current_block_hpos,
                                top: current_block_vpos,
                                width: current_block_width,
                                height: current_block_height,
                            };
                            
                            // Detect italic from collected style references
                            let _has_italic = collected_styles.iter().any(|style| 
                                style.to_lowercase().contains("italic") || 
                                style.to_lowercase().contains("oblique")
                            );
                            
                            // Style based on content
                            let style = if current_block_content.to_uppercase().contains("CITY CASH MANAGEMENT") ||
                                          current_block_content.to_uppercase().contains("INVESTMENT POLICIES") {
                                BlockStyle {
                                    color: egui::Color32::from_rgb(220, 220, 220),
                                    font_size: 16.0,
                                    is_bold: true,
                                }
                            } else if current_block_content.starts_with("General Fund") ||
                                     current_block_content.starts_with("Investment Practices") ||
                                     current_block_content.starts_with("Cash Flow Projections") {
                                BlockStyle {
                                    color: egui::Color32::from_rgb(200, 200, 200),
                                    font_size: 14.0,
                                    is_bold: true,
                                }
                            } else {
                                BlockStyle {
                                    color: egui::Color32::from_rgb(180, 180, 180),
                                    font_size: 12.0,
                                    is_bold: false,
                                }
                            };
                            
                            text_blocks.push(AltoTextBlock {
                                content: current_block_content.clone(),
                                rope: Rope::from_str(&current_block_content),
                                block_id: current_block_id.clone(),
                                alignment,
                                block_style: style,
                                bbox,
                            });
                        }
                        
                        in_textblock = false;
                    }
                }
                Ok(Event::Eof) => break,
                _ => {}
            }
            buf.clear();
        }
        
        // Sort blocks by reading order (page, then Y position, then X position)
        text_blocks.sort_by(|a, b| {
            // Extract page number from block ID
            let page_a = Self::extract_page_number(&a.block_id);
            let page_b = Self::extract_page_number(&b.block_id);
            
            page_a.cmp(&page_b)
                .then_with(|| a.bbox.top.partial_cmp(&b.bbox.top).unwrap())
                .then_with(|| a.bbox.left.partial_cmp(&b.bbox.left).unwrap())
        });
        
        // Initialize CRF model
        let crf_model = WapitiCRF::new();
        
        // Try to initialize ML classifier
        let (model_available, tokenizer_available) = ModelSetup::check_model_availability();
        let ml_classifier = if model_available && tokenizer_available {
            println!("🔧 Attempting to load Metal Document Classifier...");
            match DocumentClassifier::new("models/model.safetensors", "models/tokenizer.json") {
                Ok(classifier) => {
                    println!("✅ Metal Document Classifier loaded successfully!");
                    Some(classifier)
                }
                Err(e) => {
                    println!("❌ Metal Document Classifier failed to load: {}", e);
                    println!("   Falling back to CRF-only classification");
                    None
                }
            }
        } else {
            None
        };
        
        let ml_enabled = ml_classifier.is_some();
        
        // Run classification (hybrid ML + CRF if available, CRF-only otherwise)
        let classifications = if !text_blocks.is_empty() {
            let crf_predictions = crf_model.classify_blocks(&text_blocks, page_width, page_height);
            
            if let Some(ref classifier) = ml_classifier {
                println!("🔥 CALLING ML CLASSIFIER with {} blocks", text_blocks.len());
                
                // Extract CRF features for ML input
                let crf_features: Vec<_> = text_blocks.iter()
                    .map(|block| WapitiCRF::extract_features(block, page_width, page_height, &crate::grobid_heuristics::DocumentContext::new()))
                    .collect();
                
                println!("📊 CRF features extracted: {} feature vectors", crf_features.len());
                
                // Hybrid ML + CRF classification
                match classifier.hybrid_classify(&text_blocks, &crf_features, &crf_predictions, page_width, page_height, 0.6) {
                    Ok(ml_results) => {
                        println!("✅ ML classification succeeded: {} predictions", ml_results.len());
                        ml_results
                    }
                    Err(e) => {
                        println!("❌ ML classification failed: {}", e);
                        println!("📋 Falling back to CRF predictions");
                        crf_predictions
                    }
                }
            } else {
                // CRF-only classification
                crf_predictions
            }
        } else {
            Vec::new()
        };
        
        if ml_enabled {
            println!("🚀 Hybrid ML + CRF classification active");
        } else {
            println!("⚙️ CRF-only classification (ML models not found)");
        }
        
        AltoStructureEditor {
            text_blocks,
            page_width,
            page_height,
            total_words,
            crf_model,
            ml_classifier,
            classifications,
            ml_enabled,
        }
    }
    
    fn extract_page_number(block_id: &str) -> i32 {
        // Extract page number from block ID like "p1_b1", "p2_b3", etc.
        if let Some(p_pos) = block_id.find('p') {
            if let Some(underscore_pos) = block_id[p_pos + 1..].find('_') {
                let page_str = &block_id[p_pos + 1..p_pos + 1 + underscore_pos];
                return page_str.parse().unwrap_or(1);
            }
        }
        1 // Default to page 1
    }
    
    pub fn from_alto_xml(xml: &str, page_num: usize) -> Self {
        use quick_xml::{Reader, events::Event};
        
        let mut text_blocks = Vec::new();
        let mut page_width = 612.0;
        let mut total_words = 0;
        
        let mut reader = Reader::from_str(xml);
        reader.config_mut().trim_text(true);
        let mut buf = Vec::new();
        
        let mut in_target_page = false;
        let mut current_block_content = String::new();
        let mut current_block_id = String::new();
        let mut current_block_hpos = 0.0;
        let mut current_block_vpos = 0.0;
        let mut current_block_width = 0.0;
        let mut current_block_height = 0.0;
        let mut in_textblock = false;
        let mut line_words: Vec<String> = Vec::new();
        let mut collected_styles: Vec<String> = Vec::new();
        
        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(e)) => {
                    let tag_name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                    
                    if tag_name.contains("Page") {
                        for attr in e.attributes() {
                            if let Ok(attr) = attr {
                                let key = std::str::from_utf8(attr.key.as_ref()).unwrap_or("");
                                let value = String::from_utf8_lossy(&attr.value);
                                
                                match key {
                                    "PHYSICAL_IMG_NR" => {
                                        let this_page: usize = value.parse().unwrap_or(1);
                                        in_target_page = this_page == page_num;
                                    }
                                    "WIDTH" => page_width = value.parse().unwrap_or(612.0),
                                    _ => {}
                                }
                            }
                        }
                    } else if tag_name.contains("TextBlock") && in_target_page {
                        in_textblock = true;
                        current_block_content.clear();
                        
                        for attr in e.attributes() {
                            if let Ok(attr) = attr {
                                let key = std::str::from_utf8(attr.key.as_ref()).unwrap_or("");
                                let value = String::from_utf8_lossy(&attr.value);
                                
                                match key {
                                    "ID" => current_block_id = value.to_string(),
                                    "HPOS" => current_block_hpos = value.parse().unwrap_or(0.0),
                                    "VPOS" => current_block_vpos = value.parse().unwrap_or(0.0),
                                    "WIDTH" => current_block_width = value.parse().unwrap_or(0.0),
                                    "HEIGHT" => current_block_height = value.parse().unwrap_or(0.0),
                                    _ => {}
                                }
                            }
                        }
                    } else if tag_name.contains("TextLine") && in_textblock {
                        // Start collecting words for this line
                        line_words.clear();
                    }
                }
                Ok(Event::Empty(e)) => {
                    let tag_name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                    
                    if tag_name.contains("String") && in_textblock {
                        let mut content = String::new();
                        let mut style_refs = String::new();
                        let mut vpos = 0.0;
                        
                        for attr in e.attributes() {
                            if let Ok(attr) = attr {
                                let key = std::str::from_utf8(attr.key.as_ref()).unwrap_or("");
                                let value = String::from_utf8_lossy(&attr.value);
                                
                                match key {
                                    "CONTENT" => content = value.to_string(),
                                    "STYLEREFS" => style_refs = value.to_string(),
                                    "VPOS" => vpos = value.parse().unwrap_or(0.0),
                                    _ => {}
                                }
                            }
                        }
                        
                        // Collect style references for italic detection
                        if !style_refs.is_empty() {
                            collected_styles.push(style_refs.clone());
                        }
                        
                        if !content.trim().is_empty() {
                            // Conservative superscript detection - only obvious footnote markers
                            let is_superscript = style_refs.to_lowercase().contains("superscript") &&
                                               (content.starts_with("(") && content.ends_with(")") && content.len() <= 4) ||
                                               (content.len() <= 2 && content.chars().all(|c| c.is_numeric()));
                            
                            // Convert to Unicode superscript if possible
                            if is_superscript {
                                let superscript_text = Self::convert_to_superscript(&content);
                                
                                // Merge with previous word if it exists (no space)
                                if let Some(last_word) = line_words.last_mut() {
                                    last_word.push_str(&superscript_text);
                                    // Don't add to line_words - already merged
                                } else {
                                    line_words.push(superscript_text);
                                }
                            } else {
                                line_words.push(content);
                            };
                            total_words += 1;
                        }
                    }
                }
                Ok(Event::End(e)) => {
                    let tag_name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                    
                    if tag_name.contains("TextLine") && in_textblock {
                        // End of line - add words to block content
                        if !line_words.is_empty() {
                            if !current_block_content.is_empty() {
                                current_block_content.push('\n');
                            }
                            current_block_content.push_str(&line_words.join(" "));
                            line_words.clear();
                        }
                    } else if tag_name.contains("TextBlock") && in_textblock {
                        // End of TextBlock - create block
                        if !current_block_content.trim().is_empty() {
                            // Determine alignment from HPOS
                            let alignment = if current_block_hpos < 120.0 {
                                TextAlignment::Left
                            } else if current_block_hpos > page_width / 2.0 - 100.0 && 
                                     current_block_hpos < page_width / 2.0 + 100.0 {
                                TextAlignment::Center
                            } else {
                                TextAlignment::Left
                            };
                            
                            // Create accurate bounding box from Alto data
                            let bbox = BoundingBox {
                                left: current_block_hpos,
                                top: current_block_vpos,
                                width: current_block_width,
                                height: current_block_height,
                            };
                            
                            // Detect italic from collected style references
                            let has_italic = collected_styles.iter().any(|style| 
                                style.to_lowercase().contains("italic") || 
                                style.to_lowercase().contains("oblique")
                            );
                            
                            // Style based on content
                            let style = if current_block_content.to_uppercase().contains("CITY CASH MANAGEMENT") ||
                                          current_block_content.to_uppercase().contains("INVESTMENT POLICIES") {
                                BlockStyle {
                                    color: egui::Color32::from_rgb(220, 220, 220),
                                    font_size: 16.0,
                                    is_bold: true,
                                }
                            } else if current_block_content.starts_with("General Fund") ||
                                     current_block_content.starts_with("Investment Practices") ||
                                     current_block_content.starts_with("Cash Flow Projections") {
                                BlockStyle {
                                    color: egui::Color32::from_rgb(200, 200, 200),
                                    font_size: 14.0,
                                    is_bold: true,
                                }
                            } else {
                                BlockStyle {
                                    color: egui::Color32::from_rgb(180, 180, 180),
                                    font_size: 12.0,
                                    is_bold: false,
                                }
                            };
                            
                            text_blocks.push(AltoTextBlock {
                                content: current_block_content.clone(),
                                rope: Rope::from_str(&current_block_content),
                                block_id: current_block_id.clone(),
                                alignment,
                                block_style: style,
                                bbox,
                            });
                        }
                        
                        in_textblock = false;
                    } else if tag_name.contains("Page") {
                        in_target_page = false;
                    }
                }
                Ok(Event::Eof) => break,
                _ => {}
            }
            buf.clear();
        }
        
        // Initialize CRF model
        let crf_model = WapitiCRF::new();
        
        // Run CRF classification on all blocks
        let classifications = if !text_blocks.is_empty() {
            crf_model.classify_blocks(&text_blocks, page_width, 792.0)
        } else {
            Vec::new()
        };
        
        AltoStructureEditor {
            text_blocks,
            page_width,
            page_height: 792.0,
            total_words,
            crf_model,
            ml_classifier: None,
            classifications,
            ml_enabled: false,
        }
    }
    
    pub fn render(&mut self, ui: &mut egui::Ui) -> bool {
        let title = if self.ml_enabled {
            "🚀 Hybrid ML + CRF Classification"
        } else {
            "⚙️ CRF-Only Classification"
        };
        ui.heading(title);
        
        let mut changed = false;
        
        egui::ScrollArea::vertical()
            .auto_shrink([false, false]) 
            .show(ui, |ui| {
                for (i, block) in self.text_blocks.iter_mut().enumerate() {
                    // Block type indicator with CRF classification
                    ui.horizontal(|ui| {
                        let alignment_emoji = match block.alignment {
                            TextAlignment::Left => "👈",
                            TextAlignment::Center => "🎯", 
                            TextAlignment::Right => "👉",
                        };
                        
                        let classification = self.classifications.get(i)
                            .map(|c| format!("{:?}", c))
                            .unwrap_or_else(|| "Unknown".to_string());
                        
                        let structure_emoji = match self.classifications.get(i) {
                            Some(DocumentStructure::Title) => "👑",
                            Some(DocumentStructure::SectionHeader) => "📝",
                            Some(DocumentStructure::Paragraph) => "📄",
                            Some(DocumentStructure::TableTitle) => "📊",
                            Some(DocumentStructure::TableRow) => "🔢",
                            Some(DocumentStructure::Footnote) => "📌",
                            Some(DocumentStructure::ListItem) => "📋",
                            _ => "❓",
                        };
                        
                        ui.label(egui::RichText::new(format!("{} {} {}", structure_emoji, alignment_emoji, classification))
                            .color(egui::Color32::from_rgb(120, 120, 120))
                            .size(10.0));
                    });
                    
                    // Editable text block with proper alignment
                    let mut block_text = block.content.clone();
                    
                    ui.group(|ui| {
                        // Black background
                        ui.visuals_mut().extreme_bg_color = egui::Color32::BLACK;
                        ui.visuals_mut().panel_fill = egui::Color32::BLACK;
                        ui.visuals_mut().window_fill = egui::Color32::BLACK;
                        
                        // Apply alignment
                        match block.alignment {
                            TextAlignment::Center => {
                                ui.vertical_centered(|ui| {
                                    let response = ui.add(
                                        egui::TextEdit::multiline(&mut block_text)
                                            .font(if block.block_style.is_bold {
                                                egui::FontId::monospace(block.block_style.font_size)
                                            } else {
                                                egui::FontId::proportional(block.block_style.font_size) 
                                            })
                                            .text_color(block.block_style.color)
                                            .desired_width(ui.available_width() * 0.8) // Centered width
                                    );
                                    
                                    if response.changed() {
                                        block.content = block_text.clone();
                                        block.rope = Rope::from_str(&block_text);
                                        changed = true;
                                    }
                                });
                            }
                            _ => {
                                let response = ui.add_sized(
                                    egui::Vec2::new(ui.available_width(), 100.0),
                                    egui::TextEdit::multiline(&mut block_text)
                                        .font(if block.block_style.is_bold {
                                            egui::FontId::monospace(block.block_style.font_size)
                                        } else {
                                            egui::FontId::proportional(block.block_style.font_size)
                                        })
                                        .text_color(block.block_style.color)
                                        .desired_width(f32::INFINITY)
                                );
                                
                                if response.changed() {
                                    block.content = block_text.clone();
                                    block.rope = Rope::from_str(&block_text);
                                    changed = true;
                                }
                            }
                        }
                    });
                    
                    ui.add_space(15.0); // Paragraph spacing
                }
            });
        
        // Stats
        ui.separator();
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new(format!("📊 {} TextBlocks", self.text_blocks.len()))
                .color(egui::Color32::from_rgb(160, 160, 160)));
            ui.separator();
            ui.label(egui::RichText::new(format!("🔤 {} words", self.total_words))
                .color(egui::Color32::from_rgb(160, 160, 160)));
        });
        
        changed
    }
    
    fn convert_to_superscript(text: &str) -> String {
        let mut result = String::new();
        
        for ch in text.chars() {
            let superscript_char = match ch {
                '0' => '⁰',
                '1' => '¹',
                '2' => '²', 
                '3' => '³',
                '4' => '⁴',
                '5' => '⁵',
                '6' => '⁶',
                '7' => '⁷',
                '8' => '⁸',
                '9' => '⁹',
                '+' => '⁺',
                '-' => '⁻',
                '=' => '⁼',
                '(' => '⁽',
                ')' => '⁾',
                'a' | 'A' => 'ᵃ',
                'b' | 'B' => 'ᵇ',
                'c' | 'C' => 'ᶜ',
                'd' | 'D' => 'ᵈ',
                'e' | 'E' => 'ᵉ',
                'f' | 'F' => 'ᶠ',
                'g' | 'G' => 'ᵍ',
                'h' | 'H' => 'ʰ',
                'i' | 'I' => 'ⁱ',
                'j' | 'J' => 'ʲ',
                'k' | 'K' => 'ᵏ',
                'l' | 'L' => 'ˡ',
                'm' | 'M' => 'ᵐ',
                'n' | 'N' => 'ⁿ',
                'o' | 'O' => 'ᵒ',
                'p' | 'P' => 'ᵖ',
                'r' | 'R' => 'ʳ',
                's' | 'S' => 'ˢ',
                't' | 'T' => 'ᵗ',
                'u' | 'U' => 'ᵘ',
                'v' | 'V' => 'ᵛ',
                'w' | 'W' => 'ʷ',
                'x' | 'X' => 'ˣ',
                'y' | 'Y' => 'ʸ',
                'z' | 'Z' => 'ᶻ',
                // Fallback for unsupported characters
                _ => {
                    result.push('^');
                    ch
                }
            };
            
            result.push(superscript_char);
        }
        
        result
    }
    
    pub fn get_unified_text(&self) -> String {
        self.text_blocks.iter()
            .map(|block| match block.alignment {
                TextAlignment::Center => {
                    // Center the text
                    block.content.lines()
                        .map(|line| format!("{:^80}", line.trim()))
                        .collect::<Vec<_>>()
                        .join("\n")
                }
                _ => block.content.clone()
            })
            .collect::<Vec<_>>()
            .join("\n\n")
    }
    
    pub fn get_block_count(&self) -> usize {
        self.text_blocks.len()
    }
}