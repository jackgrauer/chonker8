// OCR Engine module for TrOCR
use anyhow::Result;
use image::{DynamicImage, GenericImageView};
use ort::{session::Session, value::Value, inputs};
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OCRResult {
    pub text: String,
    pub confidence: f32,
    pub tokens: Vec<i64>,
}

pub struct OCREngine {
    encoder: Option<Session>,
    decoder: Option<Session>,
    initialized: bool,
}

impl OCREngine {
    pub fn new() -> Result<Self> {
        Ok(Self {
            encoder: None,
            decoder: None,
            initialized: false,
        })
    }
    
    pub async fn initialize(&mut self) -> Result<()> {
        if self.initialized {
            return Ok(());
        }
        
        let _ = ort::init();
        
        // Load encoder
        if std::path::Path::new("models/trocr_encoder.onnx").exists() {
            self.encoder = Some(
                Session::builder()?
                    .with_optimization_level(ort::session::builder::GraphOptimizationLevel::Level3)?
                    .with_intra_threads(4)?
                    .commit_from_file("models/trocr_encoder.onnx")?
            );
        }
        
        // Load decoder
        if std::path::Path::new("models/trocr.onnx").exists() {
            self.decoder = Some(
                Session::builder()?
                    .with_optimization_level(ort::session::builder::GraphOptimizationLevel::Level3)?
                    .with_intra_threads(4)?
                    .commit_from_file("models/trocr.onnx")?
            );
        }
        
        self.initialized = true;
        Ok(())
    }
    
    pub async fn extract_text(&mut self, image: &DynamicImage) -> Result<OCRResult> {
        if !self.initialized {
            self.initialize().await?;
        }
        
        let encoder = self.encoder.as_mut()
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
        let outputs = encoder.run(inputs![input])?;
        
        // For now, return placeholder
        // TODO: Implement decoder loop
        Ok(OCRResult {
            text: "Text extracted by TrOCR".to_string(),
            confidence: 0.95,
            tokens: vec![],
        })
    }
    
    pub fn is_ready(&self) -> bool {
        self.initialized && self.encoder.is_some()
    }
    
    pub fn has_encoder(&self) -> bool {
        self.encoder.is_some()
    }
    
    pub fn has_decoder(&self) -> bool {
        self.decoder.is_some()
    }
}