// PDF extraction module - Simplified to use only pdftotext
//
// This module provides PDF text extraction using the pdftotext utility.
// All extraction methods have been unified to use pdftotext with layout preservation.
//
// Main components:
// - extraction_router: Handles PDF text extraction using pdftotext
// - document_analyzer: Analyzes PDF pages (still available for metrics)

// Active modules - Pure Rust implementation
pub mod pdftotext_extraction;  // Text extraction using pdftotext
pub mod lopdf_helper;         // Pure Rust PDF parsing
pub mod document_processor;   // Document processing
pub mod ui_api;               // UI API integration

// Active extraction system - uses pdftotext exclusively
pub mod document_analyzer;
pub mod extraction_router;

// Main exports for PDF extraction
pub use document_analyzer::{DocumentAnalyzer, PageFingerprint};
pub use extraction_router::{ExtractionRouter, ExtractionMethod, ExtractionResult};

// Note: The following exports are kept for compatibility but are not used:
// - All ML-based extraction methods (OCR, LayoutLM, TrOCR)
// - Document AI processing
// The system now exclusively uses pdftotext for all text extraction
