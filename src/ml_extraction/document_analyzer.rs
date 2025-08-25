use anyhow::Result;
use lopdf::{Document, Object, Dictionary};
use std::path::Path;
use std::time::Instant;

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
        
        // Load PDF with lopdf
        let document = Document::load(pdf_path)?;
        
        // Get the page
        let pages = document.get_pages();
        let page_id = pages
            .get(&((page_index + 1) as u32))
            .ok_or_else(|| anyhow::anyhow!("Page {} not found", page_index + 1))?;
        
        let page_dict = document.get_object(*page_id)?
            .as_dict()?;
        
        // Get page dimensions
        let (page_width, page_height) = get_page_dimensions(&document, page_dict)?;
        let page_area = page_width * page_height;
        
        // Extract and analyze text
        let text = extract_page_text(&document, page_dict)?;
        fingerprint.char_count = text.chars().count();
        
        // Calculate text coverage (simplified - assumes avg char size)
        let avg_char_area = 10.0; // Rough estimate in points²
        let text_area = fingerprint.char_count as f32 * avg_char_area;
        fingerprint.text_coverage = (text_area / page_area).min(1.0);
        
        // Assess text quality
        fingerprint.text_quality = calculate_text_quality(&text);
        
        // Check for table indicators
        fingerprint.has_tables = detect_tables(&text);
        
        // Analyze images in content stream
        fingerprint.image_coverage = analyze_images(&document, page_dict, page_area)?;
        
        fingerprint.extraction_time_ms = start.elapsed().as_millis() as u64;
        
        Ok(fingerprint)
    }
    
    /// Analyze entire document
    pub fn analyze_document(&self, pdf_path: &Path) -> Result<Vec<PageFingerprint>> {
        let document = Document::load(pdf_path)?;
        let page_count = document.get_pages().len();
        
        let mut fingerprints = Vec::new();
        for i in 0..page_count {
            fingerprints.push(self.analyze_page(pdf_path, i)?);
        }
        
        Ok(fingerprints)
    }
}

// Get page dimensions from MediaBox
fn get_page_dimensions(document: &Document, page: &Dictionary) -> Result<(f32, f32)> {
    if let Ok(media_box) = page.get(b"MediaBox") {
        match media_box {
            Object::Reference(id) => {
                if let Ok(Object::Array(arr)) = document.get_object(*id) {
                    let mut bounds = Vec::new();
                    for obj in arr {
                        match obj {
                            Object::Integer(i) => bounds.push(*i as f32),
                            Object::Real(f) => bounds.push(*f),
                            _ => {}
                        }
                    }
                    if bounds.len() == 4 {
                        let width = bounds[2] - bounds[0];
                        let height = bounds[3] - bounds[1];
                        return Ok((width, height));
                    }
                }
            }
            Object::Array(ref arr) => {
                let mut bounds = Vec::new();
                for obj in arr {
                    match obj {
                        Object::Integer(i) => bounds.push(*i as f32),
                        Object::Real(f) => bounds.push(*f),
                        _ => {}
                    }
                }
                if bounds.len() == 4 {
                    let width = bounds[2] - bounds[0];
                    let height = bounds[3] - bounds[1];
                    return Ok((width, height));
                }
            }
            _ => {}
        }
    }
    
    // Default to US Letter
    Ok((612.0, 792.0))
}

// Extract text from page content stream
fn extract_page_text(document: &Document, page: &Dictionary) -> Result<String> {
    let mut text = String::new();
    
    if let Ok(contents) = page.get(b"Contents") {
        let content_data = get_content_data(document, contents)?;
        let content_str = String::from_utf8_lossy(&content_data);
        
        // Simple text extraction - look for Tj and TJ operators
        for line in content_str.lines() {
            if line.contains("Tj") {
                // Extract text from Tj operator
                if let Some(start) = line.find('(') {
                    if let Some(end) = line.rfind(')') {
                        if end > start {
                            text.push_str(&line[start + 1..end]);
                            text.push(' ');
                        }
                    }
                }
            } else if line.contains("TJ") {
                // Extract text from TJ array
                if let Some(start) = line.find('[') {
                    if let Some(end) = line.rfind(']') {
                        if end > start {
                            let array_content = &line[start + 1..end];
                            // Extract strings from array
                            let mut in_string = false;
                            let mut current_string = String::new();
                            
                            for ch in array_content.chars() {
                                if ch == '(' {
                                    in_string = true;
                                    current_string.clear();
                                } else if ch == ')' && in_string {
                                    in_string = false;
                                    text.push_str(&current_string);
                                } else if in_string {
                                    current_string.push(ch);
                                }
                            }
                            text.push(' ');
                        }
                    }
                }
            }
        }
    }
    
    Ok(text)
}

// Get content data from content object
fn get_content_data(document: &Document, contents: &Object) -> Result<Vec<u8>> {
    match contents {
        Object::Reference(r) => {
            let obj = document.get_object(*r)?;
            get_content_data(document, obj)
        }
        Object::Stream(stream) => {
            Ok(stream.decompressed_content()?)
        }
        Object::Array(arr) => {
            let mut data = Vec::new();
            for item in arr {
                let item_data = get_content_data(document, item)?;
                data.extend_from_slice(&item_data);
            }
            Ok(data)
        }
        _ => Ok(Vec::new())
    }
}

// Analyze images in page content
fn analyze_images(document: &Document, page: &Dictionary, page_area: f32) -> Result<f32> {
    let mut image_area = 0.0;
    
    // Check for XObject resources (images)
    if let Ok(resources) = page.get(b"Resources") {
        let res_dict = match resources {
            Object::Reference(id) => {
                if let Ok(Object::Dictionary(dict)) = document.get_object(*id) {
                    dict
                } else {
                    return Ok(0.0);
                }
            }
            Object::Dictionary(dict) => dict,
            _ => return Ok(0.0),
        };
        if let Ok(xobject) = res_dict.get(b"XObject") {
            let xobj_dict = match xobject {
                Object::Reference(id) => {
                    if let Ok(Object::Dictionary(dict)) = document.get_object(*id) {
                        dict
                    } else {
                        return Ok(0.0);
                    }
                }
                Object::Dictionary(dict) => dict,
                _ => return Ok(0.0),
            };
            for (_name, obj_ref) in xobj_dict.iter() {
                let stream = match obj_ref {
                    Object::Reference(id) => {
                        if let Ok(Object::Stream(s)) = document.get_object(*id) {
                            s
                        } else {
                            continue;
                        }
                    }
                    Object::Stream(s) => s,
                    _ => continue,
                };
                {
                    let stream_dict = &stream.dict;
                    // Check if it's an image
                    if let Ok(subtype) = stream_dict.get(b"Subtype") {
                        let is_image = match subtype {
                            Object::Reference(id) => {
                                if let Ok(Object::Name(name)) = document.get_object(*id) {
                                    name == b"Image"
                                } else {
                                    false
                                }
                            }
                            Object::Name(name) => name == b"Image",
                            _ => false,
                        };
                        if is_image {
                            // Get image dimensions
                            let width = get_number(&document, &stream_dict, b"Width").unwrap_or(100.0);
                            let height = get_number(&document, &stream_dict, b"Height").unwrap_or(100.0);
                            image_area += width * height;
                        }
                    }
                }
            }
        }
    }
    
    Ok((image_area / page_area).min(1.0))
}

// Helper to get numeric value from dictionary
fn get_number(document: &Document, dict: &Dictionary, key: &[u8]) -> Option<f32> {
    if let Ok(obj) = dict.get(key) {
        match obj {
            Object::Integer(i) => return Some(*i as f32),
            Object::Real(f) => return Some(*f),
            Object::Reference(id) => {
                if let Ok(val) = document.get_object(*id) {
                    match val {
                        Object::Integer(i) => return Some(*i as f32),
                        Object::Real(f) => return Some(*f),
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }
    None
}

/// Calculate text quality score (0.0-1.0)
fn calculate_text_quality(text: &str) -> f32 {
    if text.is_empty() {
        return 0.0;
    }
    
    let total_chars = text.len() as f32;
    let alphabetic = text.chars().filter(|c| c.is_alphabetic()).count() as f32;
    let spaces = text.chars().filter(|c| c.is_whitespace()).count() as f32;
    
    // Quality based on ratio of meaningful characters
    let meaningful_ratio = (alphabetic + spaces) / total_chars;
    
    // Penalize text with too many special characters
    let special_chars = text.chars()
        .filter(|c| !c.is_alphanumeric() && !c.is_whitespace())
        .count() as f32;
    let special_ratio = special_chars / total_chars;
    
    // Calculate quality score
    let quality = meaningful_ratio * (1.0 - special_ratio.min(0.5));
    
    quality.min(1.0).max(0.0)
}

/// Detect if text likely contains tables
fn detect_tables(text: &str) -> bool {
    // Simple heuristics for table detection
    let lines: Vec<&str> = text.lines().collect();
    
    // Check for multiple lines with consistent delimiter patterns
    let mut delimiter_counts = vec![0; lines.len()];
    
    for (i, line) in lines.iter().enumerate() {
        // Count potential column delimiters
        let pipes = line.matches('|').count();
        let tabs = line.matches('\t').count();
        let multiple_spaces = line.contains("  ");
        
        delimiter_counts[i] = pipes + tabs + if multiple_spaces { 1 } else { 0 };
    }
    
    // If multiple consecutive lines have similar delimiter counts, likely a table
    let mut consecutive_similar = 0;
    for window in delimiter_counts.windows(2) {
        if window[0] > 0 && window[0] == window[1] {
            consecutive_similar += 1;
        }
    }
    
    consecutive_similar >= 2
}