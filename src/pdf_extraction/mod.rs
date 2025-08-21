// PDF extraction module
pub mod basic;
pub mod improved;
pub mod true_visual;
pub mod extractous_extraction;
pub mod braille;

pub use basic::get_page_count;
pub use true_visual::render_true_visual;
pub use extractous_extraction::extract_with_extractous_advanced;
pub mod document_ai;
pub use document_ai::extract_with_document_ai;
pub mod document_understanding;
// pub use document_understanding::{DocumentAnalyzer, DocumentStructure, DocumentType, analyze_pdf_structure}; // Currently unused

pub mod tokenizer;
pub mod document_ai_simple;
// pub use document_ai_simple::{SimpleTrOCR, extract_with_simple_trocr}; // Available but currently unused

// Add OCR engine module
pub mod ocr_engine;

// UI modules (created by deployment script)
pub mod document_processor;
pub mod ui_api;

// Export main components
pub use ocr_engine::{OCREngine, OCRResult};
pub use document_processor::{DocumentProcessor, ProcessedDocument, ExtractedText, DocumentSection};
pub use ui_api::{DocumentAIService, UIRequest, UIResponse, create_service};
