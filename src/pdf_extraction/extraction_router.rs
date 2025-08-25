// Simplified PDF text extraction using pdftotext
//
// This module has been simplified to use only the pdftotext utility for all PDF text extraction.
// Previous implementations using OCR (TrOCR) and LayoutLM have been removed.
// 
// The pdftotext utility is called with the -layout flag to preserve formatting:
// pdftotext -f [page] -l [page] -layout [pdf_path] -
//
// This provides reliable text extraction that works well for most PDFs.

use anyhow::Result;
use std::path::Path;
use super::document_analyzer::PageFingerprint;

/// Extraction method enum - now only contains PdfToText
#[derive(Debug, Clone, PartialEq)]
pub enum ExtractionMethod {
    PdfToText,  // Only pdftotext for all extraction
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

/// Router for determining extraction strategy - simplified
pub struct ExtractionRouter;

impl ExtractionRouter {
    /// Determine extraction strategy - always returns PdfToText
    pub fn determine_strategy(_fingerprint: &PageFingerprint) -> ExtractionMethod {
        ExtractionMethod::PdfToText
    }
    
    /// Get fallback chain - not needed anymore since we only have one method
    pub fn get_fallback_chain(_primary: &ExtractionMethod) -> Vec<ExtractionMethod> {
        vec![]
    }
    
    /// Execute extraction with pdftotext (synchronous version for UI)
    pub fn extract_with_fallback_sync(
        pdf_path: &Path,
        page_index: usize,
        _fingerprint: &PageFingerprint,
    ) -> Result<ExtractionResult> {
        Self::execute_extraction_sync(pdf_path, page_index, &ExtractionMethod::PdfToText)
    }
    
    /// Execute extraction with pdftotext (async version)
    pub async fn extract_with_fallback(
        pdf_path: &Path,
        page_index: usize,
        _fingerprint: &PageFingerprint,
    ) -> Result<ExtractionResult> {
        Self::execute_extraction(pdf_path, page_index, &ExtractionMethod::PdfToText).await
    }
    
    /// Execute extraction with pdftotext (synchronous)
    fn execute_extraction_sync(
        pdf_path: &Path,
        page_index: usize,
        _method: &ExtractionMethod,
    ) -> Result<ExtractionResult> {
        use std::time::Instant;
        use std::process::Command;
        let start = Instant::now();
        
        // Always use pdftotext command
        let output = Command::new("pdftotext")
            .args(&[
                "-f", &(page_index + 1).to_string(),
                "-l", &(page_index + 1).to_string(),
                "-layout",
                pdf_path.to_str().unwrap(),
                "-"
            ])
            .output()?;
            
        let text = if output.status.success() {
            String::from_utf8_lossy(&output.stdout).to_string()
        } else {
            anyhow::bail!("pdftotext failed");
        };
        
        let mut result = ExtractionResult::new(text, ExtractionMethod::PdfToText);
        result.extraction_time_ms = start.elapsed().as_millis() as u64;
        
        Ok(result)
    }
    
    /// Execute extraction with pdftotext (async)
    async fn execute_extraction(
        pdf_path: &Path,
        page_index: usize,
        _method: &ExtractionMethod,
    ) -> Result<ExtractionResult> {
        use std::time::Instant;
        use std::process::Command;
        let start = Instant::now();
        
        // Always use pdftotext command
        let output = Command::new("pdftotext")
            .args(&[
                "-f", &(page_index + 1).to_string(),
                "-l", &(page_index + 1).to_string(),
                "-layout",
                pdf_path.to_str().unwrap(),
                "-"
            ])
            .output()?;
            
        let text = if output.status.success() {
            String::from_utf8_lossy(&output.stdout).to_string()
        } else {
            anyhow::bail!("pdftotext failed");
        };
        
        let mut result = ExtractionResult::new(text, ExtractionMethod::PdfToText);
        result.extraction_time_ms = start.elapsed().as_millis() as u64;
        
        Ok(result)
    }
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
        let fingerprint = PageFingerprint::new();
        
        // Always returns PdfToText now
        assert_eq!(ExtractionRouter::determine_strategy(&fingerprint), ExtractionMethod::PdfToText);
    }
}