use anyhow::Result;
use std::path::Path;
use super::document_analyzer::PageFingerprint;
use image::DynamicImage;
use chrono;

/// Extraction methods available
#[derive(Debug, Clone, PartialEq)]
pub enum ExtractionMethod {
    NativeText,      // PDFium native text extraction
    FastText,        // pdftotext for layout preservation
    OCR,            // TrOCR for scanned documents
    LayoutAnalysis, // LayoutLM for complex layouts
}

/// Extraction result with quality metrics
#[derive(Debug, Clone)]
pub struct ExtractionResult {
    pub text: String,
    pub method: ExtractionMethod,
    pub quality_score: f32,
    pub extraction_time_ms: u64,
}

impl ExtractionResult {
    pub fn new(text: String, method: ExtractionMethod) -> Self {
        let quality_score = calculate_quality_score(&text);
        Self {
            text,
            method,
            quality_score,
            extraction_time_ms: 0,
        }
    }
}

/// Router for determining extraction strategy
pub struct ExtractionRouter;

impl ExtractionRouter {
    /// Helper to log to debug file
    fn log_to_debug_file(message: &str) {
        if let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open("/tmp/chonker8_debug.log")
        {
            use std::io::Write;
            use chrono::Local;
            let _ = writeln!(file, "[{}] [EXTRACTION] {}", 
                Local::now().format("%H:%M:%S%.3f"), 
                message);
        }
    }
    /// Determine extraction strategy based on page fingerprint
    pub fn determine_strategy(fingerprint: &PageFingerprint) -> ExtractionMethod {
        // Decision tree based on content analysis
        match (fingerprint.text_coverage, fingerprint.image_coverage) {
            // High text coverage, low image coverage -> Native text extraction
            (t, i) if t > 0.8 && i < 0.2 => ExtractionMethod::NativeText,
            
            // Low text coverage, high image coverage -> OCR
            (t, i) if t < 0.1 && i > 0.8 => ExtractionMethod::OCR,
            
            // Has tables -> Layout analysis
            _ if fingerprint.has_tables => ExtractionMethod::LayoutAnalysis,
            
            // Moderate text coverage -> Fast text extraction
            (t, _) if t > 0.5 => ExtractionMethod::FastText,
            
            // Poor text quality -> Re-OCR
            _ if fingerprint.text_quality < 0.5 => ExtractionMethod::OCR,
            
            // Default fallback
            _ => ExtractionMethod::NativeText,
        }
    }
    
    /// Get fallback chain for extraction methods
    pub fn get_fallback_chain(primary: &ExtractionMethod) -> Vec<ExtractionMethod> {
        match primary {
            ExtractionMethod::NativeText => vec![
                ExtractionMethod::FastText,
                ExtractionMethod::OCR,
            ],
            ExtractionMethod::FastText => vec![
                ExtractionMethod::NativeText,
                ExtractionMethod::OCR,
            ],
            ExtractionMethod::OCR => vec![
                ExtractionMethod::LayoutAnalysis,
                ExtractionMethod::NativeText,
            ],
            ExtractionMethod::LayoutAnalysis => vec![
                ExtractionMethod::OCR,
                ExtractionMethod::FastText,
                ExtractionMethod::NativeText,
            ],
        }
    }
    
    /// Execute extraction with fallback chain (synchronous version for UI)
    pub fn extract_with_fallback_sync(
        pdf_path: &Path,
        page_index: usize,
        fingerprint: &PageFingerprint,
    ) -> Result<ExtractionResult> {
        let primary_method = Self::determine_strategy(fingerprint);
        eprintln!("[DEBUG] Primary extraction method selected: {:?}", primary_method);
        Self::log_to_debug_file(&format!("Primary extraction method selected: {:?}", primary_method));
        
        // Try primary method
        if let Ok(result) = Self::execute_extraction_sync(pdf_path, page_index, &primary_method) {
            eprintln!("[DEBUG] Quality score: {:.2}", result.quality_score);
            Self::log_to_debug_file(&format!("Quality score: {:.2}", result.quality_score));
            if result.quality_score >= 0.7 {
                eprintln!("[DEBUG] Using primary method: {:?}", primary_method);
                Self::log_to_debug_file(&format!("Using primary method: {:?}", primary_method));
                return Ok(result);
            }
        }
        
        // Try fallback chain
        let fallbacks = Self::get_fallback_chain(&primary_method);
        for fallback_method in fallbacks {
            eprintln!("[DEBUG] Trying fallback method: {:?}", fallback_method);
            Self::log_to_debug_file(&format!("Trying fallback method: {:?}", fallback_method));
            if let Ok(result) = Self::execute_extraction_sync(pdf_path, page_index, &fallback_method) {
                if result.quality_score >= 0.5 {
                    eprintln!("[DEBUG] Using fallback method: {:?}", fallback_method);
                    Self::log_to_debug_file(&format!("Using fallback method: {:?}", fallback_method));
                    return Ok(result);
                }
            }
        }
        
        // Last resort - just use pdftotext
        eprintln!("[DEBUG] Using last resort: pdftotext");
        Self::log_to_debug_file("Using last resort: pdftotext");
        Self::execute_extraction_sync(pdf_path, page_index, &ExtractionMethod::FastText)
    }
    
    /// Execute extraction with fallback chain (async version)
    pub async fn extract_with_fallback(
        pdf_path: &Path,
        page_index: usize,
        fingerprint: &PageFingerprint,
    ) -> Result<ExtractionResult> {
        let primary_method = Self::determine_strategy(fingerprint);
        
        // Try primary method
        if let Ok(result) = Self::execute_extraction(pdf_path, page_index, &primary_method).await {
            if result.quality_score >= 0.7 {
                return Ok(result);
            }
        }
        
        // Try fallback chain
        let fallback_chain = Self::get_fallback_chain(&primary_method);
        for method in fallback_chain {
            if let Ok(result) = Self::execute_extraction(pdf_path, page_index, &method).await {
                if result.quality_score >= 0.5 {
                    return Ok(result);
                }
            }
        }
        
        // Last resort: return best quality result
        Self::execute_extraction(pdf_path, page_index, &ExtractionMethod::NativeText).await
    }
    
    /// Execute extraction with specific method (synchronous)
    fn execute_extraction_sync(
        pdf_path: &Path,
        page_index: usize,
        method: &ExtractionMethod,
    ) -> Result<ExtractionResult> {
        use std::time::Instant;
        use std::process::Command;
        let start = Instant::now();
        
        let text = match method {
            ExtractionMethod::NativeText => {
                // Use Pdfium with fresh instance (chonker7 style)
                crate::pdf_extraction::basic::extract_with_pdfium_sync(pdf_path, page_index)?
            }
            ExtractionMethod::FastText => {
                // Use pdftotext command directly
                let output = Command::new("pdftotext")
                    .args(&[
                        "-f", &(page_index + 1).to_string(),
                        "-l", &(page_index + 1).to_string(),
                        "-layout",
                        pdf_path.to_str().unwrap(),
                        "-"
                    ])
                    .output()?;
                    
                if output.status.success() {
                    String::from_utf8_lossy(&output.stdout).to_string()
                } else {
                    anyhow::bail!("pdftotext failed");
                }
            }
            ExtractionMethod::OCR => {
                // Try to use TrOCR model for OCR
                eprintln!("[DEBUG] Attempting TrOCR extraction...");
                Self::log_to_debug_file("Attempting TrOCR extraction...");
                
                // First render the page to an image
                // We need to use pdfium directly here since we're in the pdf_extraction module
                match render_page_to_image(pdf_path, page_index) {
                    Ok(image) => {
                        // Convert image to bytes for TrOCR
                        let mut buffer = Vec::new();
                        image.write_to(&mut std::io::Cursor::new(&mut buffer), image::ImageFormat::Png)
                            .map_err(|e| anyhow::anyhow!("Failed to encode image: {}", e))?;
                        
                        // Try to use TrOCR
                        match crate::pdf_extraction::trocr_extraction::extract_with_simple_trocr_sync(&buffer) {
                            Ok(text) => {
                                eprintln!("[DEBUG] TrOCR extraction successful");
                                Self::log_to_debug_file("TrOCR extraction successful");
                                text
                            }
                            Err(e) => {
                                eprintln!("[DEBUG] TrOCR failed: {}, falling back to pdftotext", e);
                                Self::log_to_debug_file(&format!("TrOCR failed: {}, falling back to pdftotext", e));
                                let output = Command::new("pdftotext")
                                    .args(&[
                                        "-f", &(page_index + 1).to_string(),
                                        "-l", &(page_index + 1).to_string(),
                                        pdf_path.to_str().unwrap(),
                                        "-"
                                    ])
                                    .output()?;
                                String::from_utf8_lossy(&output.stdout).to_string()
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("[DEBUG] Failed to render PDF page: {}, using pdftotext", e);
                        Self::log_to_debug_file(&format!("Failed to render PDF page: {}, using pdftotext", e));
                        let output = Command::new("pdftotext")
                            .args(&[
                                "-f", &(page_index + 1).to_string(),
                                "-l", &(page_index + 1).to_string(),
                                pdf_path.to_str().unwrap(),
                                "-"
                            ])
                            .output()?;
                        String::from_utf8_lossy(&output.stdout).to_string()
                    }
                }
            }
            ExtractionMethod::LayoutAnalysis => {
                // Try to use LayoutLMv3 for structured extraction
                eprintln!("[DEBUG] Attempting LayoutLM extraction...");
                Self::log_to_debug_file("Attempting LayoutLM extraction...");
                
                match crate::pdf_extraction::layoutlm_extraction::extract_with_layoutlm_sync(pdf_path, page_index) {
                    Ok(text) => {
                        eprintln!("[DEBUG] LayoutLM extraction successful");
                        Self::log_to_debug_file("LayoutLM extraction successful");
                        text
                    }
                    Err(e) => {
                        eprintln!("[DEBUG] LayoutLM failed: {}, falling back to pdftotext with layout", e);
                        Self::log_to_debug_file(&format!("LayoutLM failed: {}, falling back to pdftotext with layout", e));
                        let output = Command::new("pdftotext")
                            .args(&[
                                "-f", &(page_index + 1).to_string(),
                                "-l", &(page_index + 1).to_string(),
                                "-layout",
                                "-table",
                                pdf_path.to_str().unwrap(),
                                "-"
                            ])
                            .output()?;
                        String::from_utf8_lossy(&output.stdout).to_string()
                    }
                }
            }
        };
        
        let mut result = ExtractionResult::new(text, method.clone());
        result.extraction_time_ms = start.elapsed().as_millis() as u64;
        eprintln!("[DEBUG] Extraction took {}ms", result.extraction_time_ms);
        Self::log_to_debug_file(&format!("Extraction took {}ms", result.extraction_time_ms));
        
        Ok(result)
    }
    
    /// Execute extraction with specific method (async)
    async fn execute_extraction(
        pdf_path: &Path,
        page_index: usize,
        method: &ExtractionMethod,
    ) -> Result<ExtractionResult> {
        use std::time::Instant;
        let start = Instant::now();
        
        let text = match method {
            ExtractionMethod::NativeText => {
                crate::pdf_extraction::basic::extract_with_pdfium(pdf_path, page_index).await?
            }
            ExtractionMethod::FastText => {
                let matrix = crate::pdf_extraction::pdftotext_extraction::extract_with_pdftotext(
                    pdf_path,
                    page_index,
                    80,
                    40,
                ).await?;
                // Convert matrix to string
                matrix.iter()
                    .map(|row| row.iter().collect::<String>())
                    .collect::<Vec<_>>()
                    .join("\n")
            }
            ExtractionMethod::OCR => {
                // For now, fallback to native - OCR integration would go here
                crate::pdf_extraction::basic::extract_with_pdfium(pdf_path, page_index).await?
            }
            ExtractionMethod::LayoutAnalysis => {
                // For now, fallback to native - LayoutLM integration would go here
                crate::pdf_extraction::basic::extract_with_pdfium(pdf_path, page_index).await?
            }
        };
        
        let mut result = ExtractionResult::new(text, method.clone());
        result.extraction_time_ms = start.elapsed().as_millis() as u64;
        
        Ok(result)
    }
}

// Helper function to render a PDF page to an image
fn render_page_to_image(pdf_path: &Path, page_index: usize) -> Result<DynamicImage> {
    use super::pdfium_singleton::with_pdfium;
    use pdfium_render::prelude::*;
    
    with_pdfium(|pdfium| {
        let document = pdfium.load_pdf_from_file(pdf_path, None)?;
        let page = document.pages().get(page_index as u16)?;
        
        // Render at a reasonable resolution
        let bitmap = page.render_with_config(
            &PdfRenderConfig::new()
                .set_target_size(1200, 1600)
                .rotate_if_landscape(PdfPageRenderRotation::None, false)
        )?;
        
        Ok(bitmap.as_image())
    })
}

/// Calculate quality score for extracted text
pub fn calculate_quality_score(text: &str) -> f32 {
    if text.is_empty() {
        return 0.0;
    }
    
    let checks = [
        text.len() > 10,                          // Has content
        text.contains(". "),                      // Has sentences
        !is_mostly_gibberish(text),              // Not gibberish
        has_dictionary_words(text),              // Has real words
        has_reasonable_whitespace(text),         // Proper formatting
    ];
    
    let passed = checks.iter().filter(|&&x| x).count() as f32;
    passed / checks.len() as f32
}

/// Check if text is mostly gibberish
fn is_mostly_gibberish(text: &str) -> bool {
    if text.is_empty() {
        return true;
    }
    
    // Check vowel ratio
    let vowel_count = text.chars()
        .filter(|c| "aeiouAEIOU".contains(*c))
        .count();
    let vowel_ratio = vowel_count as f32 / text.len() as f32;
    
    vowel_ratio < 0.1 || vowel_ratio > 0.6
}

/// Check if text has dictionary words
fn has_dictionary_words(text: &str) -> bool {
    let words: Vec<&str> = text.split_whitespace().collect();
    if words.is_empty() {
        return false;
    }
    
    // Simple check: words should be mostly alphabetic and reasonable length
    let valid_words = words.iter()
        .filter(|w| w.len() >= 2 && w.len() <= 20)
        .filter(|w| {
            let alpha_ratio = w.chars().filter(|c| c.is_alphabetic()).count() as f32 / w.len() as f32;
            alpha_ratio > 0.7
        })
        .count();
    
    valid_words as f32 / words.len() as f32 > 0.5
}

/// Check if text has reasonable whitespace
fn has_reasonable_whitespace(text: &str) -> bool {
    if text.is_empty() {
        return false;
    }
    
    let whitespace_count = text.chars().filter(|c| c.is_whitespace()).count();
    let whitespace_ratio = whitespace_count as f32 / text.len() as f32;
    
    whitespace_ratio > 0.05 && whitespace_ratio < 0.5
}

// Add helper for basic PDFium extraction
mod basic {
    use anyhow::Result;
    use pdfium_render::prelude::*;
    use std::path::Path;
    
    pub async fn extract_with_pdfium(pdf_path: &Path, page_index: usize) -> Result<String> {
        let lib_path = if cfg!(target_os = "macos") {
            "./lib/libpdfium.dylib"
        } else {
            "./lib/libpdfium.so"
        };
        
        let pdfium = Pdfium::new(
            Pdfium::bind_to_library(lib_path)
                .or_else(|_| Pdfium::bind_to_system_library())?
        );
        
        let document = pdfium.load_pdf_from_file(pdf_path, None)?;
        let page = document.pages().get(page_index as u16)?;
        let text = page.text()?.all();
        
        Ok(text)
    }
}

// Re-export for use in main
pub use basic::extract_with_pdfium;

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_quality_score() {
        assert!(calculate_quality_score("This is a normal sentence. It has good structure.") > 0.7);
        assert!(calculate_quality_score("xvqpz kljfd qwerty") < 0.3);
        assert!(calculate_quality_score("") == 0.0);
    }
    
    #[test]
    fn test_strategy_selection() {
        let mut fingerprint = PageFingerprint::new();
        
        // High text coverage
        fingerprint.text_coverage = 0.9;
        fingerprint.image_coverage = 0.1;
        assert_eq!(ExtractionRouter::determine_strategy(&fingerprint), ExtractionMethod::NativeText);
        
        // High image coverage
        fingerprint.text_coverage = 0.05;
        fingerprint.image_coverage = 0.9;
        assert_eq!(ExtractionRouter::determine_strategy(&fingerprint), ExtractionMethod::OCR);
        
        // Has tables
        fingerprint.has_tables = true;
        assert_eq!(ExtractionRouter::determine_strategy(&fingerprint), ExtractionMethod::LayoutAnalysis);
    }
}