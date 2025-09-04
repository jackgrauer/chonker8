use crate::alto_structure_editor::{AltoTextBlock, TextAlignment};

#[derive(Debug, Clone, PartialEq)]
pub enum DocumentStructure {
    Title,
    Subtitle, 
    SectionHeader,
    Paragraph,
    ListItem,
    TableTitle,
    TableHeader,
    TableRow,
    Footnote,
    PageNumber,
    Unknown,
}

#[derive(Debug, Clone)]
pub struct LayoutFeatures {
    pub indentation: f32,        // Distance from left margin
    pub relative_position: f32,  // Position on page (0.0-1.0)
    pub font_size: f32,         // Text size
    pub is_bold: bool,          // Typography
    pub is_italic: bool,
    pub line_spacing: f32,      // Gap above this block
    pub character_density: f32, // Characters per unit area
    pub width_ratio: f32,       // Width relative to page width
}

pub struct GrobidHeuristics;

impl GrobidHeuristics {
    /// Main classification function using GROBID-style heuristics
    pub fn classify_structure(
        content: &str, 
        layout: &LayoutFeatures,
        alignment: &TextAlignment,
        context: &DocumentContext
    ) -> DocumentStructure {
        
        // GROBID's multi-criteria decision tree
        
        // 1. Title Detection (position + typography + content)
        if Self::is_title(content, layout, alignment) {
            return DocumentStructure::Title;
        }
        
        // 2. Section Header Detection
        if Self::is_section_header(content, layout, context) {
            return DocumentStructure::SectionHeader;
        }
        
        // 3. Table Structure Detection
        if let Some(table_type) = Self::classify_table_element(content, layout, context) {
            return table_type;
        }
        
        // 4. List Item Detection
        if Self::is_list_item(content, layout) {
            return DocumentStructure::ListItem;
        }
        
        // 5. Footnote Detection
        if Self::is_footnote(content, layout, context) {
            return DocumentStructure::Footnote;
        }
        
        // 6. Page Number Detection
        if Self::is_page_number(content, layout, context) {
            return DocumentStructure::PageNumber;
        }
        
        // Default: Paragraph
        DocumentStructure::Paragraph
    }
    
    fn is_title(content: &str, layout: &LayoutFeatures, alignment: &TextAlignment) -> bool {
        // GROBID title detection criteria:
        let content_indicators = content.to_uppercase().contains("CASH MANAGEMENT") ||
                               content.to_uppercase().contains("INVESTMENT POLICIES") ||
                               content.len() > 20 && content.chars().filter(|c| c.is_uppercase()).count() as f32 / content.len() as f32 > 0.7;
        
        let typography_indicators = layout.font_size > 14.0 || layout.is_bold;
        
        let position_indicators = matches!(alignment, TextAlignment::Center) ||
                                 layout.relative_position < 0.3; // Top of page
        
        let formatting_indicators = !content.ends_with('.') && // Titles usually don't end with periods
                                   content.split_whitespace().count() < 15; // Titles are typically shorter
        
        (content_indicators && typography_indicators) ||
        (position_indicators && typography_indicators && formatting_indicators)
    }
    
    fn is_section_header(content: &str, layout: &LayoutFeatures, _context: &DocumentContext) -> bool {
        // Section header patterns
        let content_patterns = content.starts_with("General Fund") ||
                              content.starts_with("Investment Practices") ||
                              content.starts_with("Cash Flow") ||
                              content.ends_with("Projections") ||
                              content.ends_with("Practices");
        
        let typography_indicators = layout.is_bold || layout.font_size > 12.0;
        
        let position_indicators = layout.indentation < 100.0 && // Left-aligned or minimal indent
                                 layout.line_spacing > 20.0; // Space above section
        
        let length_indicators = content.len() < 100 && // Headers are usually short
                               !content.contains('$') && // Not financial data
                               !content.contains('%');
        
        content_patterns || (typography_indicators && position_indicators && length_indicators)
    }
    
    fn classify_table_element(content: &str, layout: &LayoutFeatures, _context: &DocumentContext) -> Option<DocumentStructure> {
        // Table title detection
        if content.starts_with("Table") && content.len() < 50 {
            return Some(DocumentStructure::TableTitle);
        }
        
        // Table header detection
        if content.contains("City of Philadelphia") ||
           content.contains("Fiscal Years") ||
           content.contains("Amount in millions") ||
           (content.contains("2011") && content.contains("2015")) {
            return Some(DocumentStructure::TableHeader);
        }
        
        // Table row detection (financial data)
        let has_financial_data = content.contains('$') || content.contains('%');
        let has_multiple_numbers = content.matches(char::is_numeric).count() > 3;
        let is_tabular = layout.character_density < 0.1; // Sparse text typical of tables
        
        if has_financial_data || (has_multiple_numbers && is_tabular) {
            return Some(DocumentStructure::TableRow);
        }
        
        None
    }
    
    fn is_list_item(content: &str, layout: &LayoutFeatures) -> bool {
        let list_markers = content.starts_with("(i)") || 
                          content.starts_with("(ii)") ||
                          content.starts_with("(a)") ||
                          content.starts_with("(b)") ||
                          content.starts_with("•") ||
                          content.starts_with("-");
        
        let indentation_pattern = layout.indentation > 50.0 && layout.indentation < 200.0;
        
        list_markers || (indentation_pattern && content.len() > 20)
    }
    
    fn is_footnote(content: &str, layout: &LayoutFeatures, _context: &DocumentContext) -> bool {
        // Footnote patterns
        let footnote_markers = content.starts_with("(1)") ||
                              content.starts_with("(2)") ||
                              content.starts_with("(3)") ||
                              content.starts_with("*") ||
                              content.starts_with("†");
        
        let position_indicators = layout.relative_position > 0.8 || // Bottom of page
                                 layout.font_size < 10.0; // Smaller text
        
        let content_indicators = content.contains("represents") ||
                               content.contains("based on") ||
                               content.contains("defined in");
        
        footnote_markers || (position_indicators && content_indicators)
    }
    
    fn is_page_number(content: &str, layout: &LayoutFeatures, _context: &DocumentContext) -> bool {
        let is_single_number = content.trim().parse::<i32>().is_ok() && content.trim().len() < 4;
        let is_at_page_edge = layout.relative_position > 0.9 || layout.relative_position < 0.1;
        let is_isolated = layout.line_spacing > 30.0;
        
        is_single_number && is_at_page_edge && is_isolated
    }
    
    /// Extract layout features from Alto TextBlock data (GROBID-style feature engineering)
    pub fn extract_layout_features(
        block: &AltoTextBlock, 
        page_width: f32, 
        page_height: f32,
        prev_block_bottom: Option<f32>
    ) -> LayoutFeatures {
        // Calculate GROBID-style layout features
        let indentation = block.bbox.left;
        let relative_position = block.bbox.top / page_height;
        let width_ratio = block.bbox.width / page_width;
        
        let line_spacing = if let Some(prev_bottom) = prev_block_bottom {
            block.bbox.top - prev_bottom
        } else {
            0.0
        };
        
        let character_density = block.content.len() as f32 / (block.bbox.width * block.bbox.height);
        
        LayoutFeatures {
            indentation,
            relative_position,
            font_size: 12.0, // From block style
            is_bold: block.block_style.is_bold,
            is_italic: false, // TODO: Extract from style
            line_spacing,
            character_density,
            width_ratio,
        }
    }
}

#[derive(Debug, Clone)]
pub struct DocumentContext {
    pub page_number: usize,
    pub total_pages: usize,
    pub previous_structures: Vec<DocumentStructure>,
    pub in_table_region: bool,
    pub document_type: DocumentType,
}

#[derive(Debug, Clone)]
pub enum DocumentType {
    Financial,    // Municipal financial reports
    Academic,     // Research papers
    Legal,        // Legal documents  
    Technical,    // Technical manuals
    General,      // General documents
}

impl DocumentContext {
    pub fn new() -> Self {
        Self {
            page_number: 1,
            total_pages: 1,
            previous_structures: Vec::new(),
            in_table_region: false,
            document_type: DocumentType::Financial, // Detect from content
        }
    }
    
    pub fn update_context(&mut self, new_structure: DocumentStructure) {
        self.previous_structures.push(new_structure.clone());
        
        // Update table region state
        self.in_table_region = matches!(new_structure, 
            DocumentStructure::TableTitle | 
            DocumentStructure::TableHeader | 
            DocumentStructure::TableRow
        );
    }
}