// PDF extraction module - Simplified to use only pdftotext
//
// This module provides PDF text extraction using the pdftotext utility.
// All extraction methods have been unified to use pdftotext with layout preservation.
//
// Main components:
// - extraction_router: Handles PDF text extraction using pdftotext
// - document_analyzer: Analyzes PDF pages (still available for metrics)

// Legacy modules (kept for compatibility but not actively used)
pub mod basic;
pub mod improved;
pub mod true_visual;
pub mod pdftotext_extraction;
pub mod braille;
pub mod pdfium_singleton;
pub mod document_ai;
pub mod layoutlm_extraction;
pub mod tokenizer;
pub mod trocr_extraction;
pub mod ocr_engine;
pub mod document_processor;
pub mod ui_api;

// Active extraction system - uses pdftotext exclusively
pub mod document_analyzer;
pub mod extraction_router;

// Main exports for PDF extraction
pub use document_analyzer::{DocumentAnalyzer, PageFingerprint};
pub use extraction_router::{ExtractionRouter, ExtractionMethod, ExtractionResult};

// Note: The following exports are kept for compatibility but are not used:
// - All ML-based extraction methods (OCR, LayoutLM, TrOCR)
// - PDFium-based extraction
// - Document AI processing
// The system now exclusively uses pdftotext for all text extraction
