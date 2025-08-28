pub mod render_with_pdftoppm;
pub mod page_renderer;
pub mod ocr;

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_ocr_needs_detection() {
        use ocr::needs_ocr;
        
        // Empty text should need OCR
        assert!(needs_ocr(""));
        assert!(needs_ocr("   "));
        
        // Very short text should need OCR
        assert!(needs_ocr("Hello"));
        
        // Sufficient text should not need OCR
        let long_text = "This is a much longer piece of text that contains many words and should not trigger OCR detection because it has sufficient content to be considered valid extracted text from a PDF document.";
        assert!(!needs_ocr(long_text));
    }

    #[test]
    fn test_page_number_validation() {
        use page_renderer::get_pdf_page_count;
        
        // This will fail for non-existent file, which is expected
        let result = get_pdf_page_count(Path::new("/nonexistent.pdf"));
        assert!(result.is_err());
    }
    
    #[test]
    fn test_pdftoppm_renderer_creation() {
        use render_with_pdftoppm::SystemPdfRenderer;
        
        // Should be able to create renderer
        let _renderer = SystemPdfRenderer::new();
        // Just verify we can create one without panic
    }
}