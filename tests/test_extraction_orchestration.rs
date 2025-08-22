use anyhow::Result;
use rexpect::spawn;
use std::fs;
use std::path::Path;
use std::time::Duration;

/// Test orchestrator for PDF extraction pipeline
pub struct ExtractionOrchestrator {
    binary_path: String,
    test_pdfs: Vec<String>,
}

impl ExtractionOrchestrator {
    pub fn new() -> Self {
        Self {
            binary_path: "./target/release/chonker8-hot".to_string(),
            test_pdfs: vec![],
        }
    }
    
    /// Add test PDF to the orchestration
    pub fn add_test_pdf(&mut self, path: &str) -> &mut Self {
        self.test_pdfs.push(path.to_string());
        self
    }
    
    /// Run extraction test with rexpect
    pub fn test_extraction(&self, pdf_path: &str) -> Result<ExtractionTestResult> {
        let mut cmd = format!("DYLD_LIBRARY_PATH=./lib {} ", self.binary_path);
        cmd.push_str(pdf_path);
        
        let mut session = spawn(&cmd, Some(30000))?;
        let mut result = ExtractionTestResult::new(pdf_path);
        
        // Wait for initial menu
        session.exp_string("Choose extraction method:")?;
        
        // Test document analyzer first
        session.send_line("a")?; // Analyze option
        
        // Capture analysis output
        if let Ok(analysis) = session.exp_regex(r"Page Fingerprint: (.+)") {
            result.fingerprint_output = Some(analysis.1);
        }
        
        // Go back to menu
        session.send_control('c')?;
        session.exp_string("Choose extraction method:")?;
        
        // Test auto-routing
        session.send_line("r")?; // Router option
        
        // Capture routing decision
        if let Ok(routing) = session.exp_regex(r"Selected method: (\w+)") {
            result.selected_method = Some(routing.1);
        }
        
        // Capture extraction result
        if let Ok(text) = session.exp_regex(r"Extracted text: (.{0,100})") {
            result.extracted_text = Some(text.1);
        }
        
        // Capture quality score
        if let Ok(quality) = session.exp_regex(r"Quality score: ([\d.]+)") {
            result.quality_score = quality.1.parse().ok();
        }
        
        // Test extraction
        session.send("q")?; // Quit
        session.exp_eof()?;
        
        Ok(result)
    }
    
    /// Run comprehensive pipeline test
    pub fn test_full_pipeline(&self) -> Result<Vec<ExtractionTestResult>> {
        let mut results = Vec::new();
        
        for pdf_path in &self.test_pdfs {
            println!("Testing PDF: {}", pdf_path);
            if let Ok(result) = self.test_extraction(pdf_path) {
                results.push(result);
            }
        }
        
        Ok(results)
    }
    
    /// Test fallback chain
    pub fn test_fallback_chain(&self, pdf_path: &str) -> Result<FallbackTestResult> {
        let mut cmd = format!("DYLD_LIBRARY_PATH=./lib {} --test-fallback ", self.binary_path);
        cmd.push_str(pdf_path);
        
        let mut session = spawn(&cmd, Some(30000))?;
        let mut result = FallbackTestResult::new(pdf_path);
        
        // Capture each fallback attempt
        while let Ok(method) = session.exp_regex(r"Trying method: (\w+)") {
            result.methods_tried.push(method.1);
            
            if let Ok(score) = session.exp_regex(r"Quality: ([\d.]+)") {
                result.quality_scores.push(score.1.parse().unwrap_or(0.0));
            }
        }
        
        // Get final result
        if let Ok(final_method) = session.exp_regex(r"Final method: (\w+)") {
            result.final_method = Some(final_method.1);
        }
        
        session.exp_eof()?;
        Ok(result)
    }
}

#[derive(Debug)]
pub struct ExtractionTestResult {
    pub pdf_path: String,
    pub fingerprint_output: Option<String>,
    pub selected_method: Option<String>,
    pub extracted_text: Option<String>,
    pub quality_score: Option<f32>,
}

impl ExtractionTestResult {
    fn new(pdf_path: &str) -> Self {
        Self {
            pdf_path: pdf_path.to_string(),
            fingerprint_output: None,
            selected_method: None,
            extracted_text: None,
            quality_score: None,
        }
    }
}

#[derive(Debug)]
pub struct FallbackTestResult {
    pub pdf_path: String,
    pub methods_tried: Vec<String>,
    pub quality_scores: Vec<f32>,
    pub final_method: Option<String>,
}

impl FallbackTestResult {
    fn new(pdf_path: &str) -> Self {
        Self {
            pdf_path: pdf_path.to_string(),
            methods_tried: Vec::new(),
            quality_scores: Vec::new(),
            final_method: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_orchestrator_creation() {
        let mut orchestrator = ExtractionOrchestrator::new();
        orchestrator
            .add_test_pdf("/tmp/test1.pdf")
            .add_test_pdf("/tmp/test2.pdf");
        
        assert_eq!(orchestrator.test_pdfs.len(), 2);
    }
    
    #[test]
    #[ignore] // Run with: cargo test test_live_extraction -- --ignored
    fn test_live_extraction() {
        let orchestrator = ExtractionOrchestrator::new();
        
        // This would test with a real PDF
        if Path::new("/tmp/sample.pdf").exists() {
            let result = orchestrator.test_extraction("/tmp/sample.pdf").unwrap();
            println!("Extraction result: {:?}", result);
            assert!(result.quality_score.unwrap_or(0.0) > 0.0);
        }
    }
    
    #[test]
    #[ignore] // Run with: cargo test test_fallback_behavior -- --ignored
    fn test_fallback_behavior() {
        let orchestrator = ExtractionOrchestrator::new();
        
        if Path::new("/tmp/scanned.pdf").exists() {
            let result = orchestrator.test_fallback_chain("/tmp/scanned.pdf").unwrap();
            println!("Fallback chain: {:?}", result);
            assert!(!result.methods_tried.is_empty());
        }
    }
}