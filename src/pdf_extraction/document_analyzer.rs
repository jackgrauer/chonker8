use anyhow::Result;
use pdfium_render::prelude::*;
use std::path::Path;
use std::time::{Duration, Instant};
use super::pdfium_singleton::with_pdfium;

/// Page content fingerprint for routing decisions
#[derive(Debug, Clone)]
pub struct PageFingerprint {
    pub text_coverage: f32,      // 0.0-1.0 ratio of text area to page area
    pub image_coverage: f32,     // 0.0-1.0 ratio of image area to page area  
    pub char_count: usize,
    pub has_tables: bool,
    pub text_quality: f32,       // 0.0-1.0 quality score
    pub extraction_time_ms: u64, // Time taken for analysis
}

impl PageFingerprint {
    pub fn new() -> Self {
        Self {
            text_coverage: 0.0,
            image_coverage: 0.0,
            char_count: 0,
            has_tables: false,
            text_quality: 0.0,
            extraction_time_ms: 0,
        }
    }
}

/// Document analyzer for fingerprinting PDF pages
pub struct DocumentAnalyzer {}

impl DocumentAnalyzer {
    pub fn new() -> Result<Self> {
        Ok(Self {})
    }
    
    /// Analyze a single page and generate fingerprint
    pub fn analyze_page(&self, pdf_path: &Path, page_index: usize) -> Result<PageFingerprint> {
        let start = Instant::now();
        let mut fingerprint = PageFingerprint::new();
        
        // Open PDF document
        with_pdfium(|pdfium| {
            let document = pdfium.load_pdf_from_file(pdf_path, None)?;
        let pages = document.pages();
        let page = pages.get(page_index as u16)?;
        
        // Get page dimensions
        let page_width = page.width().value;
        let page_height = page.height().value;
        let page_area = page_width * page_height;
        
        // Quick text extraction
        let text_result = page.text()?.all();
        
        {
            let text = text_result;
            fingerprint.char_count = text.chars().count();
            
            // Calculate text coverage (simplified - assumes avg char size)
            let avg_char_area = 10.0; // Rough estimate in points²
            let text_area = fingerprint.char_count as f32 * avg_char_area;
            fingerprint.text_coverage = (text_area / page_area).min(1.0);
            
            // Assess text quality
            fingerprint.text_quality = calculate_text_quality(&text);
            
            // Check for table indicators
            fingerprint.has_tables = detect_tables(&text);
        }
        
        // Analyze images
        let objects = page.objects();
        let mut image_area = 0.0;
        
        for object in objects.iter() {
            if matches!(object.object_type(), PdfPageObjectType::Image) {
                if let Ok(bounds) = object.bounds() {
                    let width = bounds.right() - bounds.left();
                    let height = bounds.top() - bounds.bottom();
                    image_area += width.value * height.value;
                }
            }
        }
        
        fingerprint.image_coverage = (image_area / page_area).min(1.0);
            fingerprint.extraction_time_ms = start.elapsed().as_millis() as u64;
            
            Ok(fingerprint)
        })
    }
    
    /// Analyze entire document
    pub fn analyze_document(&self, pdf_path: &Path) -> Result<Vec<PageFingerprint>> {
        with_pdfium(|pdfium| {
            let document = pdfium.load_pdf_from_file(pdf_path, None)?;
        let page_count = document.pages().len();
        
            let mut fingerprints = Vec::new();
            for i in 0..page_count {
                fingerprints.push(self.analyze_page(pdf_path, i as usize)?);
            }
            
            Ok(fingerprints)
        })
    }
}

/// Calculate text quality score
fn calculate_text_quality(text: &str) -> f32 {
    if text.is_empty() {
        return 0.0;
    }
    
    let mut score = 0.0;
    let mut checks = 0.0;
    
    // Check 1: Has sentences (periods followed by spaces)
    if text.contains(". ") {
        score += 1.0;
    }
    checks += 1.0;
    
    // Check 2: Not mostly gibberish (has vowels)
    let vowel_ratio = text.chars()
        .filter(|c| "aeiouAEIOU".contains(*c))
        .count() as f32 / text.len() as f32;
    if vowel_ratio > 0.2 && vowel_ratio < 0.5 {
        score += 1.0;
    }
    checks += 1.0;
    
    // Check 3: Has dictionary words (simplified check)
    let words: Vec<&str> = text.split_whitespace().collect();
    let valid_words = words.iter()
        .filter(|w| w.len() > 2 && w.chars().all(|c| c.is_alphabetic()))
        .count();
    if valid_words > words.len() / 3 {
        score += 1.0;
    }
    checks += 1.0;
    
    // Check 4: Reasonable character distribution
    let alpha_count = text.chars().filter(|c| c.is_alphabetic()).count();
    let digit_count = text.chars().filter(|c| c.is_numeric()).count();
    let special_count = text.chars().filter(|c| !c.is_alphanumeric() && !c.is_whitespace()).count();
    
    let total = alpha_count + digit_count + special_count;
    if total > 0 {
        let alpha_ratio = alpha_count as f32 / total as f32;
        if alpha_ratio > 0.5 && alpha_ratio < 0.95 {
            score += 1.0;
        }
        checks += 1.0;
    }
    
    score / checks
}

/// Detect table structures in text
fn detect_tables(text: &str) -> bool {
    // Simple heuristic: look for patterns indicating tables
    let lines: Vec<&str> = text.lines().collect();
    
    // Check for column alignment patterns
    let has_pipes = text.contains('|');
    let has_tabs = text.contains('\t');
    
    // Check for repeated patterns across lines
    let mut has_alignment = false;
    if lines.len() > 2 {
        for window in lines.windows(3) {
            let spaces_0: Vec<usize> = window[0].match_indices("  ").map(|(i, _)| i).collect();
            let spaces_1: Vec<usize> = window[1].match_indices("  ").map(|(i, _)| i).collect();
            let spaces_2: Vec<usize> = window[2].match_indices("  ").map(|(i, _)| i).collect();
            
            if spaces_0.len() > 2 && spaces_0 == spaces_1 && spaces_1 == spaces_2 {
                has_alignment = true;
                break;
            }
        }
    }
    
    has_pipes || has_tabs || has_alignment
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_text_quality() {
        assert!(calculate_text_quality("This is a normal sentence. It has proper structure.") > 0.7);
        assert!(calculate_text_quality("xvqpz kljfd qwerty") < 0.3);
        assert!(calculate_text_quality("") == 0.0);
    }
    
    #[test]
    fn test_table_detection() {
        assert!(detect_tables("Name | Age | City"));
        assert!(detect_tables("John\t25\tNew York"));
        assert!(detect_tables("Col1    Col2    Col3\nVal1    Val2    Val3\nVal4    Val5    Val6"));
        assert!(!detect_tables("This is normal text without tables."));
    }
}