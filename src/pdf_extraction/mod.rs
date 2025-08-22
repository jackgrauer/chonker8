// PDF extraction module
pub mod basic;
pub mod improved;
pub mod true_visual;
pub mod pdftotext_extraction;
pub mod braille;
pub mod pdfium_singleton;

// Document-agnostic extraction system
pub mod document_analyzer;
pub mod extraction_router;

pub use basic::get_page_count;
pub use true_visual::render_true_visual;
pub use pdftotext_extraction::extract_with_pdftotext_advanced;
pub mod document_ai;
pub use document_ai::extract_with_document_ai;
pub mod layoutlm_extraction;
// pub use layoutlm_extraction::{DocumentAnalyzer, DocumentStructure, DocumentType, analyze_pdf_structure}; // Currently unused

pub mod tokenizer;
pub mod trocr_extraction;
// pub use trocr_extraction::{SimpleTrOCR, extract_with_simple_trocr}; // Available but currently unused

// Add OCR engine module
pub mod ocr_engine;

// UI modules (created by deployment script)
pub mod document_processor;
pub mod ui_api;

// Export main components
pub use ocr_engine::{OCREngine, OCRResult};
pub use document_processor::{DocumentProcessor, ProcessedDocument, ExtractedText, DocumentSection};
pub use ui_api::{DocumentAIService, UIRequest, UIResponse, create_service};

// Export document-agnostic system
pub use document_analyzer::{DocumentAnalyzer, PageFingerprint};
pub use extraction_router::{ExtractionRouter, ExtractionMethod, ExtractionResult};
