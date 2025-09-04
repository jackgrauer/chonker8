// Chonker8 - Simplified library with file browser and text editor

// Core modules
pub mod core {
    pub mod config;
    pub mod hot_reload;
}


// Alto structure editor (uses native Alto XML hierarchy)
pub mod alto_structure_editor;

// GROBID-style heuristics and CRF integration
pub mod grobid_heuristics;
pub mod crf_integration;

// Metal Document Classifier with ONNX
pub mod metal_document_classifier;

// Display modules
pub mod display;