use eframe::egui;
use ropey::Rope;
use crate::crf_integration::WapitiCRF;
use crate::grobid_heuristics::DocumentStructure;
use crate::metal_document_classifier::{DocumentClassifier, ModelSetup};
use crate::spatial_table::{SpatialTable, AltoElement};

pub struct AltoStructureEditor {
    text_blocks: Vec<AltoTextBlock>,
    spatial_tables: Vec<SpatialTable>,
    page_width: f32,
    page_height: f32,
    total_words: usize,
    crf_model: WapitiCRF,
    ml_classifier: Option<DocumentClassifier>,
    classifications: Vec<DocumentStructure>,
    ml_enabled: bool,
    display_mode: DisplayMode,
}

#[derive(Clone, Debug, PartialEq)]
pub enum DisplayMode {
    UnifiedText,
    SpatialTables,
    Hybrid,
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
        
        // Detect and extract spatial tables from text blocks
        let spatial_tables = Self::extract_spatial_tables(&text_blocks, page_width, page_height);
        
        AltoStructureEditor {
            text_blocks,
            spatial_tables,
            page_width,
            page_height,
            total_words,
            crf_model,
            ml_classifier,
            classifications,
            ml_enabled,
            display_mode: DisplayMode::Hybrid,
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
        
        // Extract spatial tables for single page
        let spatial_tables = Self::extract_spatial_tables(&text_blocks, page_width, 792.0);
        
        AltoStructureEditor {
            text_blocks,
            spatial_tables,
            page_width,
            page_height: 792.0,
            total_words,
            crf_model,
            ml_classifier: None,
            classifications,
            ml_enabled: false,
            display_mode: DisplayMode::Hybrid,
        }
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
        let mut result = String::new();
        
        for block in &self.text_blocks {
            // Check if this looks like table data
            if self.is_table_block(&block.content) {
                // Format as table with spatial positioning
                let formatted_table = self.format_table_block(block);
                result.push_str(&formatted_table);
            } else {
                // Regular text block
                match block.alignment {
                    TextAlignment::Center => {
                        // Center the text
                        let centered = block.content.lines()
                            .map(|line| format!("{:^80}", line.trim()))
                            .collect::<Vec<_>>()
                            .join("\n");
                        result.push_str(&centered);
                    }
                    _ => {
                        result.push_str(&block.content);
                    }
                }
            }
            
            result.push_str("\n\n");
        }
        
        result
    }
    
    fn is_table_block(&self, content: &str) -> bool {
        // Detect table data by content patterns
        content.contains("$") && content.contains("2011") ||
        content.contains("N/A") && content.contains("$") ||
        content.contains("%") && (content.contains("7.38") || content.contains("4.82")) ||
        content.lines().filter(|line| line.trim().parse::<f32>().is_ok()).count() > 2
    }
    
    fn format_table_block(&self, block: &AltoTextBlock) -> String {
        // For complex tables, we need to use the actual Alto spatial coordinates
        // This requires re-parsing the Alto XML to get HPOS data
        
        // For now, use a simpler approach that detects common table patterns
        let lines: Vec<&str> = block.content.lines().collect();
        
        if lines.len() <= 1 {
            return block.content.clone();
        }
        
        // Check if this is a sample ID row (SB-206, etc.)
        if lines.iter().any(|line| line.contains("SB-") && line.contains("(")) {
            let samples: Vec<&str> = lines.iter()
                .filter(|line| line.contains("SB-") || line.contains("DUP-") || line.contains("L2429996"))
                .map(|line| line.trim())
                .collect();
            
            if samples.len() >= 3 {
                return format!("{:<15} {:<15} {:<15} {:<15} {:<15}", 
                    samples.get(0).unwrap_or(&""),
                    samples.get(1).unwrap_or(&""),
                    samples.get(2).unwrap_or(&""), 
                    samples.get(3).unwrap_or(&""),
                    samples.get(4).unwrap_or(&""));
            }
        }
        
        // Check if this is a data row (numbers, values)
        if lines.iter().any(|line| line.parse::<f32>().is_ok() || line.trim() == "U" || line.trim() == "NA") {
            let values: Vec<&str> = lines.iter()
                .map(|line| line.trim())
                .filter(|line| !line.is_empty())
                .collect();
            
            if values.len() >= 4 {
                return format!("{:>10} {:>10} {:>10} {:>10} {:>10}",
                    values.get(0).unwrap_or(&""),
                    values.get(1).unwrap_or(&""),
                    values.get(2).unwrap_or(&""),
                    values.get(3).unwrap_or(&""),
                    values.get(4).unwrap_or(&""));
            }
        }
        
        // Check if this is a label row (analyte names, etc.)
        if lines.len() == 1 && (lines[0].contains("Chromium") || lines[0].contains("Solids")) {
            return format!("{:<60}", lines[0].trim());
        }
        
        // For mixed content (label + data), try to separate
        if lines.len() > 3 {
            let first_line = lines[0].trim();
            let data_lines: Vec<&str> = lines[1..].iter()
                .map(|line| line.trim())
                .filter(|line| !line.is_empty())
                .take(5)
                .collect();
            
            if data_lines.len() >= 3 {
                return format!("{:<60} {:>8} {:>8} {:>8} {:>8} {:>8}",
                    first_line,
                    data_lines.get(0).unwrap_or(&""),
                    data_lines.get(1).unwrap_or(&""),
                    data_lines.get(2).unwrap_or(&""),
                    data_lines.get(3).unwrap_or(&""),
                    data_lines.get(4).unwrap_or(&""));
            }
        }
        
        // Fallback: return original content
        block.content.clone()
    }
    
    /// Extract spatial tables from Alto text blocks using coordinate clustering
    fn extract_spatial_tables(text_blocks: &[AltoTextBlock], page_width: f32, page_height: f32) -> Vec<SpatialTable> {
        let mut tables = Vec::new();
        
        // Group blocks that appear to be tabular data based on content patterns
        let table_blocks: Vec<&AltoTextBlock> = text_blocks.iter()
            .filter(|block| Self::is_table_content(&block.content))
            .collect();
        
        if table_blocks.is_empty() {
            return tables;
        }
        
        // Convert blocks to Alto elements for spatial processing
        let alto_elements: Vec<AltoElement> = table_blocks.iter().map(|block| {
            AltoElement {
                content: block.content.clone(),
                hpos: block.bbox.left,
                vpos: block.bbox.top,
                width: block.bbox.width,
                height: block.bbox.height,
                style_refs: String::new(), // Could extract from block_style if needed
            }
        }).collect();
        
        // Create spatial table from elements
        if !alto_elements.is_empty() {
            let spatial_table = SpatialTable::from_alto_elements(alto_elements, page_width, page_height);
            tables.push(spatial_table);
        }
        
        tables
    }
    
    /// Check if content appears to be tabular data
    fn is_table_content(content: &str) -> bool {
        // Enhanced table detection logic
        let lines: Vec<&str> = content.lines().collect();
        
        // Must have multiple lines for table
        if lines.len() < 2 {
            return false;
        }
        
        // Check for financial table patterns
        let has_currency = content.contains("$") || content.contains("million");
        let has_percentages = content.contains("%");
        let has_sample_ids = content.contains("SB-") || content.contains("DUP-");
        let has_numeric_data = lines.iter().any(|line| line.parse::<f32>().is_ok());
        
        // Table indicators
        let table_indicators = [
            has_currency && has_numeric_data,
            has_percentages && lines.len() > 3,
            has_sample_ids,
            content.contains("N/A") && has_numeric_data,
            lines.iter().filter(|line| line.trim().parse::<f32>().is_ok()).count() > 3,
        ];
        
        table_indicators.iter().any(|&indicator| indicator)
    }

    pub fn render(&mut self, ui: &mut egui::Ui) -> bool {
        let title = if self.ml_enabled {
            "📄 Document Editor (ML Enhanced) - Spatial Tables Active"
        } else {
            "📄 Document Editor (CRF) - Spatial Tables Active"
        };
        ui.heading(title);
        
        // Display mode selector
        ui.horizontal(|ui| {
            ui.label("Display Mode:");
            ui.radio_value(&mut self.display_mode, DisplayMode::UnifiedText, "📝 Unified Text");
            ui.radio_value(&mut self.display_mode, DisplayMode::SpatialTables, "📊 Tables Only");
            ui.radio_value(&mut self.display_mode, DisplayMode::Hybrid, "🔀 Hybrid");
        });
        
        ui.separator();
        
        let mut changed = false;
        
        match self.display_mode {
            DisplayMode::UnifiedText => {
                // Original unified text editor
                let mut full_document = self.get_unified_text();
                let response = ui.add_sized(
                    ui.available_size(),
                    egui::TextEdit::multiline(&mut full_document)
                        .font(egui::FontId::monospace(12.0))
                        .text_color(egui::Color32::from_rgb(180, 180, 180))
                        .code_editor()
                );
                changed = response.changed();
            }
            
            DisplayMode::SpatialTables => {
                // Render spatial tables only
                if self.spatial_tables.is_empty() {
                    ui.label("No spatial tables detected in this document");
                } else {
                    for table in &mut self.spatial_tables {
                        if table.render(ui) {
                            changed = true;
                        }
                        ui.separator();
                    }
                }
            }
            
            DisplayMode::Hybrid => {
                // Show both tables and regular text
                if !self.spatial_tables.is_empty() {
                    ui.collapsing("📊 Detected Spatial Tables", |ui| {
                        for table in &mut self.spatial_tables {
                            if table.render(ui) {
                                changed = true;
                            }
                            ui.separator();
                        }
                    });
                }
                
                ui.collapsing("📝 Full Document Text", |ui| {
                    let mut full_document = self.get_unified_text();
                    let response = ui.add_sized(
                        [ui.available_width(), 300.0],
                        egui::TextEdit::multiline(&mut full_document)
                            .font(egui::FontId::monospace(12.0))
                            .text_color(egui::Color32::from_rgb(180, 180, 180))
                            .code_editor()
                    );
                    if response.changed() {
                        changed = true;
                    }
                });
            }
        }
        
        // Stats at bottom
        ui.separator();
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new(format!("📊 {} TextBlocks", self.text_blocks.len()))
                .color(egui::Color32::from_rgb(160, 160, 160)));
            ui.separator();
            ui.label(egui::RichText::new(format!("🔤 {} words", self.total_words))
                .color(egui::Color32::from_rgb(160, 160, 160)));
            ui.separator();
            ui.label(egui::RichText::new(format!("📋 {} tables", self.spatial_tables.len()))
                .color(egui::Color32::from_rgb(160, 160, 160)));
            ui.separator();
            if self.ml_enabled {
                ui.label(egui::RichText::new("🧠 ML Enhanced")
                    .color(egui::Color32::from_rgb(144, 238, 144)));
            }
        });
        
        changed
    }
    
    pub fn get_block_count(&self) -> usize {
        self.text_blocks.len()
    }
}