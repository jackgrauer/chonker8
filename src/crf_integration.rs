use crate::grobid_heuristics::{DocumentStructure, DocumentContext};
use crate::alto_structure_editor::AltoTextBlock;
use serde::{Deserialize, Serialize};
use regex::Regex;
use std::sync::LazyLock;

// Pre-compiled regex for better performance
static DATE_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\d{4}").expect("Failed to compile date regex")
});

pub struct WapitiCRF {
    model_path: String,
    feature_template: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CRFFeatures {
    // Layout features (GROBID-style)
    pub indentation_norm: f32,      // Normalized indentation (0-1)
    pub relative_y: f32,            // Y position on page (0-1)
    pub font_size_norm: f32,        // Normalized font size
    pub line_spacing_norm: f32,     // Normalized spacing above
    pub width_ratio: f32,           // Width relative to page
    
    // Typography features
    pub is_bold: bool,
    pub is_italic: bool,
    pub is_uppercase: bool,
    pub caps_ratio: f32,            // Ratio of uppercase characters
    
    // Content features  
    pub length: usize,              // Character count
    pub word_count: usize,          // Word count
    pub has_numbers: bool,
    pub has_punctuation: bool,
    pub starts_with_number: bool,
    pub ends_with_period: bool,
    
    // Context features
    pub position_in_document: f32,  // Document position (0-1)
    pub previous_label: String,     // Previous block classification
    pub next_preview: String,       // Next block preview (first few words)
    
    // Specialized features
    pub contains_currency: bool,    // Financial document specific
    pub contains_percentage: bool,
    pub contains_date: bool,
    pub table_likelihood: f32,      // Probability of being table content
}

impl WapitiCRF {
    pub fn new() -> Self {
        Self {
            model_path: "/tmp/grobid_models/".to_string(),
            feature_template: Self::create_feature_template(),
        }
    }
    
    /// Create Wapiti feature template (GROBID-style)
    fn create_feature_template() -> String {
        // This mirrors GROBID's feature template format
        String::from(r#"
# Layout features
u00:%x[0,0]
u01:%x[0,1] 
u02:%x[0,2]
u03:%x[0,3]
u04:%x[0,4]

# Typography features  
u10:%x[0,5]
u11:%x[0,6]
u12:%x[0,7]
u13:%x[0,8]

# Content features
u20:%x[0,9]
u21:%x[0,10]
u22:%x[0,11] 
u23:%x[0,12]
u24:%x[0,13]
u25:%x[0,14]

# Context features
u30:%x[0,15]
u31:%x[0,16]
u32:%x[0,17]

# Bigram features
b00:%x[-1,0]/%x[0,0]
b01:%x[0,0]/%x[1,0]

# Trigram features  
t00:%x[-1,0]/%x[0,0]/%x[1,0]
        "#)
    }
    
    /// Extract CRF features from Alto text block (GROBID methodology)
    pub fn extract_features(
        block: &AltoTextBlock,
        page_width: f32,
        page_height: f32,
        context: &DocumentContext
    ) -> CRFFeatures {
        
        // Layout features
        let indentation_norm = block.bbox.left / page_width;
        let relative_y = block.bbox.top / page_height;
        let font_size_norm = (block.block_style.font_size - 8.0) / 12.0; // Normalize to 8-20pt range
        let width_ratio = block.bbox.width / page_width;
        
        // Typography analysis
        let text = &block.content;
        let is_uppercase = text.chars().filter(|c| c.is_uppercase()).count() as f32 / text.len() as f32 > 0.5;
        let caps_ratio = text.chars().filter(|c| c.is_uppercase()).count() as f32 / text.len() as f32;
        
        // Content analysis
        let word_count = text.split_whitespace().count();
        let has_numbers = text.chars().any(|c| c.is_numeric());
        let has_punctuation = text.chars().any(|c| c.is_ascii_punctuation());
        let starts_with_number = text.trim_start().chars().next().map(|c| c.is_numeric()).unwrap_or(false);
        let ends_with_period = text.trim_end().ends_with('.');
        
        // Financial document features
        let contains_currency = text.contains('$') || text.contains("million") || text.contains("dollars");
        let contains_percentage = text.contains('%');
        let contains_date = DATE_REGEX.is_match(text) ||
                           text.contains("June") || text.contains("November");
        
        // Table likelihood (spatial analysis)
        let table_likelihood = if contains_currency || contains_percentage {
            0.8
        } else if has_numbers && word_count < 10 {
            0.6  
        } else {
            0.1
        };
        
        // Context features
        let position_in_document = context.previous_structures.len() as f32 / 20.0; // Rough estimate
        let previous_label = context.previous_structures.last()
            .map(|s| format!("{:?}", s))
            .unwrap_or_else(|| "START".to_string());
        
        CRFFeatures {
            indentation_norm,
            relative_y,
            font_size_norm,
            line_spacing_norm: 0.0, // TODO: Calculate from previous block
            width_ratio,
            is_bold: block.block_style.is_bold,
            is_italic: false, // TODO: Extract from style
            is_uppercase,
            caps_ratio,
            length: text.len(),
            word_count,
            has_numbers,
            has_punctuation,
            starts_with_number,
            ends_with_period,
            position_in_document,
            previous_label,
            next_preview: "".to_string(), // TODO: Get from next block
            contains_currency,
            contains_percentage,
            contains_date,
            table_likelihood,
        }
    }
    
    /// Convert features to Wapiti input format
    fn features_to_wapiti_format(features: &CRFFeatures) -> String {
        format!(
            "{:.3} {:.3} {:.3} {:.3} {:.3} {} {} {} {:.3} {} {} {} {} {} {} {:.3} {} {} {} {} {} {:.3}",
            features.indentation_norm,
            features.relative_y,
            features.font_size_norm,
            features.line_spacing_norm,
            features.width_ratio,
            if features.is_bold { 1 } else { 0 },
            if features.is_italic { 1 } else { 0 },
            if features.is_uppercase { 1 } else { 0 },
            features.caps_ratio,
            features.length,
            features.word_count,
            if features.has_numbers { 1 } else { 0 },
            if features.has_punctuation { 1 } else { 0 },
            if features.starts_with_number { 1 } else { 0 },
            if features.ends_with_period { 1 } else { 0 },
            features.position_in_document,
            features.previous_label,
            features.next_preview,
            if features.contains_currency { 1 } else { 0 },
            if features.contains_percentage { 1 } else { 0 },
            if features.contains_date { 1 } else { 0 },
            features.table_likelihood
        )
    }
    
    /// Run CRF classification using Wapiti (if available)
    pub fn classify_blocks(&self, blocks: &[AltoTextBlock], page_width: f32, page_height: f32) -> Vec<DocumentStructure> {
        // For now, use rule-based fallback
        // TODO: Implement actual Wapiti CRF integration
        
        let mut context = DocumentContext::new();
        let mut results = Vec::new();
        
        for (i, block) in blocks.iter().enumerate() {
            // Extract CRF features
            let features = Self::extract_features(block, page_width, page_height, &context);
            
            // Use rule-based classification (fallback until CRF is integrated)
            let structure = self.classify_with_rules(&block.content, &features);
            
            context.update_context(structure.clone());
            results.push(structure);
            
            // Debug: Print features for first few blocks
            if i < 3 {
                println!("Block {}: {:?}", i, features);
                println!("Wapiti format: {}", Self::features_to_wapiti_format(&features));
            }
        }
        
        results
    }
    
    fn classify_with_rules(&self, content: &str, features: &CRFFeatures) -> DocumentStructure {
        // GROBID-style multi-criteria classification
        
        // Strong title indicators
        if features.relative_y < 0.2 && features.is_bold && features.caps_ratio > 0.7 {
            return DocumentStructure::Title;
        }
        
        // Section headers
        if features.is_bold && features.word_count < 8 && features.indentation_norm < 0.2 {
            return DocumentStructure::SectionHeader;
        }
        
        // Table classification
        if features.table_likelihood > 0.7 {
            if content.starts_with("Table") {
                return DocumentStructure::TableTitle;
            } else if features.contains_currency || features.contains_percentage {
                return DocumentStructure::TableRow;
            } else if features.word_count < 10 && features.has_numbers {
                return DocumentStructure::TableHeader;
            }
        }
        
        // Footnotes
        if features.relative_y > 0.8 || (features.font_size_norm < 0.0 && content.len() < 200) {
            return DocumentStructure::Footnote;
        }
        
        // List items
        if content.starts_with("(i)") || content.starts_with("(ii)") || features.indentation_norm > 0.1 {
            return DocumentStructure::ListItem;
        }
        
        // Default paragraph
        DocumentStructure::Paragraph
    }
    
    /// Initialize CRF models (download if needed)
    pub fn initialize_models() -> Result<(), Box<dyn std::error::Error>> {
        // TODO: Download GROBID CRF models
        // For now, create placeholder
        std::fs::create_dir_all("/tmp/grobid_models")?;
        
        println!("CRF models initialized (placeholder)");
        println!("To use real GROBID models:");
        println!("1. Download GROBID release");  
        println!("2. Extract models from grobid-home/models/");
        println!("3. Install Wapiti CRF library");
        
        Ok(())
    }
}

impl Default for WapitiCRF {
    fn default() -> Self {
        Self::new()
    }
}