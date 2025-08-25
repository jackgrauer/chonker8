use anyhow::Result;
use serde::{Serialize, Deserialize};
use image::DynamicImage;
use std::sync::{Arc, Mutex};

use super::document_processor::{DocumentProcessor, ProcessedDocument};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UIRequest {
    pub action: String,
    pub page_number: Option<usize>,
    pub options: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UIResponse {
    pub success: bool,
    pub data: Option<serde_json::Value>,
    pub error: Option<String>,
    pub processing_time_ms: u64,
}

pub struct DocumentAIService {
    processor: Arc<Mutex<DocumentProcessor>>,
}

impl DocumentAIService {
    pub fn new() -> Result<Self> {
        Ok(Self {
            processor: Arc::new(Mutex::new(DocumentProcessor::new()?)),
        })
    }
    
    pub async fn process_request(&self, request: UIRequest, image: Option<DynamicImage>) -> UIResponse {
        let start = std::time::Instant::now();
        
        match request.action.as_str() {
            "process_page" => {
                if let Some(img) = image {
                    match self.process_page(img).await {
                        Ok(result) => UIResponse {
                            success: true,
                            data: Some(serde_json::to_value(result).unwrap()),
                            error: None,
                            processing_time_ms: start.elapsed().as_millis() as u64,
                        },
                        Err(e) => UIResponse {
                            success: false,
                            data: None,
                            error: Some(e.to_string()),
                            processing_time_ms: start.elapsed().as_millis() as u64,
                        }
                    }
                } else {
                    UIResponse {
                        success: false,
                        data: None,
                        error: Some("No image provided".to_string()),
                        processing_time_ms: start.elapsed().as_millis() as u64,
                    }
                }
            },
            "get_status" => {
                let processor = self.processor.lock().unwrap();
                let status = processor.get_status();
                UIResponse {
                    success: true,
                    data: Some(serde_json::to_value(status).unwrap()),
                    error: None,
                    processing_time_ms: start.elapsed().as_millis() as u64,
                }
            },
            _ => UIResponse {
                success: false,
                data: None,
                error: Some(format!("Unknown action: {}", request.action)),
                processing_time_ms: start.elapsed().as_millis() as u64,
            }
        }
    }
    
    async fn process_page(&self, image: DynamicImage) -> Result<ProcessedDocument> {
        let mut processor = self.processor.lock().unwrap();
        processor.process_image(&image).await
    }
}

pub fn create_service() -> Result<DocumentAIService> {
    DocumentAIService::new()
}
