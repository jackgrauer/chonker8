use anyhow::Result;
use image::{DynamicImage, GenericImageView};
use ort::{session::Session, value::Value, inputs};
use serde::{Serialize, Deserialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedText {
    pub text: String,
    pub confidence: f32,
    pub bbox: Option<[f32; 4]>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentSection {
    pub section_type: String,
    pub content: Vec<ExtractedText>,
    pub confidence: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessedDocument {
    pub extracted_text: Vec<ExtractedText>,
    pub sections: Vec<DocumentSection>,
    pub metadata: HashMap<String, String>,
    pub processing_time_ms: u64,
}

pub struct DocumentProcessor {
    trocr_encoder: Option<Session>,
    trocr_decoder: Option<Session>,
    layoutlm: Option<Session>,
    initialized: bool,
}

impl DocumentProcessor {
    pub fn new() -> Result<Self> {
        let _ = ort::init();
        
        let mut processor = Self {
            trocr_encoder: None,
            trocr_decoder: None,
            layoutlm: None,
            initialized: false,
        };
        
        processor.initialize()?;
        Ok(processor)
    }
    
    pub fn initialize(&mut self) -> Result<()> {
        if self.initialized {
            return Ok(());
        }
        
        // Load TrOCR models
        if std::path::Path::new("models/trocr_encoder.onnx").exists() {
            self.trocr_encoder = Some(
                Session::builder()?
                    .with_optimization_level(ort::session::builder::GraphOptimizationLevel::Level3)?
                    .with_intra_threads(4)?
                    .commit_from_file("models/trocr_encoder.onnx")?
            );
            println!("✅ TrOCR Encoder loaded");
        }
        
        if std::path::Path::new("models/trocr.onnx").exists() {
            self.trocr_decoder = Some(
                Session::builder()?
                    .with_optimization_level(ort::session::builder::GraphOptimizationLevel::Level3)?
                    .with_intra_threads(4)?
                    .commit_from_file("models/trocr.onnx")?
            );
            println!("✅ TrOCR Decoder loaded");
        }
        
        // Load LayoutLM
        if std::path::Path::new("models/layoutlm.onnx").exists() {
            self.layoutlm = Some(
                Session::builder()?
                    .with_optimization_level(ort::session::builder::GraphOptimizationLevel::Level3)?
                    .with_intra_threads(4)?
                    .commit_from_file("models/layoutlm.onnx")?
            );
            println!("✅ LayoutLMv3 loaded");
        }
        
        self.initialized = true;
        Ok(())
    }
    
    pub async fn process_image(&mut self, image: &DynamicImage) -> Result<ProcessedDocument> {
        let start = std::time::Instant::now();
        
        // Extract text with TrOCR
        let extracted_text = if self.trocr_encoder.is_some() {
            self.extract_text_trocr(image).await?
        } else {
            vec![]
        };
        
        // Analyze structure with LayoutLM
        let sections = if self.layoutlm.is_some() {
            self.analyze_structure_layoutlm(image, &extracted_text).await?
        } else {
            vec![]
        };
        
        // Create metadata
        let mut metadata = HashMap::new();
        metadata.insert("width".to_string(), image.width().to_string());
        metadata.insert("height".to_string(), image.height().to_string());
        metadata.insert("has_trocr".to_string(), self.trocr_encoder.is_some().to_string());
        metadata.insert("has_layoutlm".to_string(), self.layoutlm.is_some().to_string());
        
        Ok(ProcessedDocument {
            extracted_text,
            sections,
            metadata,
            processing_time_ms: start.elapsed().as_millis() as u64,
        })
    }
    
    async fn extract_text_trocr(&mut self, image: &DynamicImage) -> Result<Vec<ExtractedText>> {
        let encoder = self.trocr_encoder.as_mut()
            .ok_or_else(|| anyhow::anyhow!("TrOCR encoder not loaded"))?;
        
        // Resize to 384x384
        let img = image.resize_exact(384, 384, image::imageops::FilterType::Lanczos3);
        
        // Convert to CHW format
        let mut pixels = Vec::with_capacity(3 * 384 * 384);
        for c in 0..3 {
            for y in 0..384 {
                for x in 0..384 {
                    let pixel = img.get_pixel(x, y);
                    let value = pixel[c] as f32 / 255.0;
                    pixels.push(value);
                }
            }
        }
        
        // Run encoder
        let input = Value::from_array(([1_usize, 3, 384, 384], pixels.into_boxed_slice()))?;
        let encoder_outputs = encoder.run(inputs![input])?;
        
        // TODO: Run decoder for actual text generation
        // For now, return placeholder
        Ok(vec![
            ExtractedText {
                text: "Document text extracted by TrOCR".to_string(),
                confidence: 0.95,
                bbox: Some([0.1, 0.1, 0.9, 0.2]),
            }
        ])
    }
    
    async fn analyze_structure_layoutlm(
        &mut self, 
        image: &DynamicImage,
        text: &[ExtractedText]
    ) -> Result<Vec<DocumentSection>> {
        let layoutlm = self.layoutlm.as_mut()
            .ok_or_else(|| anyhow::anyhow!("LayoutLM not loaded"))?;
        
        // Resize to 224x224
        let img = image.resize_exact(224, 224, image::imageops::FilterType::Lanczos3);
        
        // Convert to CHW format with normalization
        let mut pixels = Vec::with_capacity(3 * 224 * 224);
        for c in 0..3 {
            for y in 0..224 {
                for x in 0..224 {
                    let pixel = img.get_pixel(x, y);
                    let value = (pixel[c] as f32 / 255.0 - 0.5) / 0.5;
                    pixels.push(value);
                }
            }
        }
        
        // Prepare inputs
        let seq_len = 512;
        let input_ids = Value::from_array(([1_usize, seq_len], vec![101i64; seq_len].into_boxed_slice()))?;
        let bbox = Value::from_array(([1_usize, seq_len, 4], vec![0i64; seq_len * 4].into_boxed_slice()))?;
        let attention_mask = Value::from_array(([1_usize, seq_len], vec![1i64; seq_len].into_boxed_slice()))?;
        let pixel_values = Value::from_array(([1_usize, 3, 224, 224], pixels.into_boxed_slice()))?;
        
        // Run LayoutLM
        let _outputs = layoutlm.run(inputs![input_ids, bbox, attention_mask, pixel_values])?;
        
        // TODO: Analyze hidden states for structure
        // For now, return placeholder
        Ok(vec![
            DocumentSection {
                section_type: "header".to_string(),
                content: text.to_vec(),
                confidence: 0.92,
            }
        ])
    }
    
    pub fn get_status(&self) -> HashMap<String, bool> {
        let mut status = HashMap::new();
        status.insert("initialized".to_string(), self.initialized);
        status.insert("trocr_encoder".to_string(), self.trocr_encoder.is_some());
        status.insert("trocr_decoder".to_string(), self.trocr_decoder.is_some());
        status.insert("layoutlm".to_string(), self.layoutlm.is_some());
        status
    }
}
