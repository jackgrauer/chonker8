// TrOCR (Transformer-based OCR) implementation for chonker8
use anyhow::{Result, Context};
use std::path::Path;
use image::imageops::FilterType;
use ort::{
    init,
    session::Session,
    session::builder::GraphOptimizationLevel,
    value::Value,
    inputs
};
use crate::pdf_extraction::tokenizer::TrOCRTokenizer;

// Helper function to log to debug file
fn log_debug(message: &str) {
    // Print to stderr so it shows in terminal
    eprintln!("{}", message);
    
    // Also write to debug log file
    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open("/tmp/chonker8_debug.log")
    {
        use std::io::Write;
        use chrono::Local;
        let _ = writeln!(file, "[{}] [TROCR] {}", 
            Local::now().format("%H:%M:%S%.3f"), 
            message);
    }
}

pub struct SimpleTrOCR {
    encoder: Option<Session>,
    decoder: Option<Session>,
    tokenizer: Option<TrOCRTokenizer>,
}

impl SimpleTrOCR {
    pub fn new() -> Result<Self> {
        log_debug("🚀 Initializing SimpleTrOCR...");
        
        // Initialize ONNX Runtime
        let _ = init();
        
        // Load encoder
        let encoder = if Path::new("models/trocr_encoder.onnx").exists() {
            log_debug("  📦 Loading TrOCR encoder...");
            let session = Session::builder()?
                .with_optimization_level(GraphOptimizationLevel::Level3)?
                .with_intra_threads(4)?
                .commit_from_file("models/trocr_encoder.onnx")?;
            log_debug("  ✅ Encoder loaded");
            Some(session)
        } else {
            None
        };
        
        // Load decoder
        let decoder = if Path::new("models/trocr.onnx").exists() {
            log_debug("  📦 Loading TrOCR decoder...");
            let session = Session::builder()?
                .with_optimization_level(GraphOptimizationLevel::Level3)?
                .with_intra_threads(4)?
                .commit_from_file("models/trocr.onnx")?;
            log_debug("  ✅ Decoder loaded");
            Some(session)
        } else {
            None
        };
        
        // Load tokenizer
        let tokenizer = match TrOCRTokenizer::new() {
            Ok(t) => Some(t),
            Err(e) => {
                log_debug(&format!("  ⚠️ Failed to load tokenizer: {:?}", e));
                None
            }
        };
        
        Ok(Self { encoder, decoder, tokenizer })
    }
    
    pub async fn extract_text(&mut self, image_data: &[u8]) -> Result<String> {
        if self.encoder.is_none() || self.decoder.is_none() {
            return Err(anyhow::anyhow!("TrOCR models not loaded"));
        }
        
        if self.tokenizer.is_none() {
            return Err(anyhow::anyhow!("TrOCR tokenizer not loaded"));
        }
        
        // Load and preprocess image
        let image = image::load_from_memory(image_data)
            .context("Failed to load image")?;
        
        // Convert to grayscale first for better OCR
        let gray = image.to_luma8();
        
        // Convert back to RGB (TrOCR expects RGB input)
        let mut rgb_image = image::RgbImage::new(gray.width(), gray.height());
        for (x, y, pixel) in gray.enumerate_pixels() {
            let gray_val = pixel[0];
            rgb_image.put_pixel(x, y, image::Rgb([gray_val, gray_val, gray_val]));
        }
        
        // Resize to 384x384 as expected by TrOCR (using high-quality Lanczos3 filter)
        let processed = image::DynamicImage::ImageRgb8(rgb_image)
            .resize_exact(384, 384, FilterType::Lanczos3)
            .to_rgb8();
        
        // Convert to CHW format and normalize
        let mut pixels = Vec::with_capacity(3 * 384 * 384);
        for channel in 0..3 {
            for y in 0..384 {
                for x in 0..384 {
                    let pixel = processed.get_pixel(x, y);
                    pixels.push(pixel[channel] as f32 / 255.0);
                }
            }
        }
        
        // Run encoder
        log_debug("  🔬 Running TrOCR encoder...");
        let encoder_input = Value::from_array(([1_usize, 3, 384, 384], pixels.into_boxed_slice()))?;
        let encoder = self.encoder.as_mut().unwrap();
        let encoder_outputs = encoder.run(inputs![encoder_input])?;
        
        // Extract encoder hidden states
        let encoder_last_hidden_state = encoder_outputs[0].try_extract_tensor::<f32>()?;
        let (enc_shape, enc_data) = encoder_last_hidden_state;
        log_debug(&format!("  ✅ Encoder produced hidden states with shape: {:?}", enc_shape));
        
        // Get tokenizer
        let tokenizer = self.tokenizer.as_ref().unwrap();
        
        // Initialize decoder with BOS token
        let mut decoder_input_ids = tokenizer.get_decoder_start_ids();
        let mut generated_tokens: Vec<u32> = Vec::new();
        let max_length = 256;  // Reasonable max for a single page (avg page ~150-200 tokens)
        
        log_debug("  🔄 Running autoregressive decoding...");
        
        // Convert encoder data to Vec for reuse
        let enc_data_vec: Vec<f32> = enc_data.to_vec();
        
        // Autoregressive generation loop
        for step in 0..max_length {
            // Prepare decoder inputs
            let decoder_input_ids_array = Value::from_array((
                [1_usize, decoder_input_ids.len()],
                decoder_input_ids.clone().into_boxed_slice()
            ))?;
            
            // Prepare encoder hidden states for cross-attention
            // Shape should be [batch_size, seq_len, hidden_dim]
            let encoder_hidden_states = Value::from_array((enc_shape.clone(), enc_data_vec.clone().into_boxed_slice()))?;
            
            // Run decoder
            // TrOCR decoder needs use_cache_branch for conditional execution (as boolean)
            let use_cache = Value::from_array(([1_usize], vec![false].into_boxed_slice()))?;
            
            // Run with all required inputs for TrOCR decoder
            let decoder = self.decoder.as_mut().unwrap();
            let decoder_outputs = decoder.run(inputs![
                "input_ids" => decoder_input_ids_array,
                "encoder_hidden_states" => encoder_hidden_states,
                "use_cache_branch" => use_cache
            ])?;
            
            // Extract logits from decoder output
            let logits = decoder_outputs[0].try_extract_tensor::<f32>()?;
            let (logits_shape, logits_data) = logits;
            
            // Get the last token's logits
            let vocab_size = logits_shape[2] as usize;
            let last_token_start = ((logits_shape[1] - 1) * logits_shape[2]) as usize;
            let last_token_end = last_token_start + vocab_size;
            let last_token_logits: &[f32] = &logits_data[last_token_start..last_token_end];
            
            // Apply temperature to reduce repetition (temperature = 0.8)
            let temperature = 0.8_f32;
            let scaled_logits: Vec<f32> = last_token_logits
                .iter()
                .map(|&logit| logit / temperature)
                .collect();
            
            // Find the token with highest probability (greedy decoding with temperature)
            let next_token_id = scaled_logits
                .iter()
                .enumerate()
                .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
                .map(|(idx, _)| idx as u32)
                .unwrap();
            
            // Check for EOS token
            if next_token_id == tokenizer.get_eos_token_id() {
                log_debug(&format!("  🛑 EOS token detected at step {}", step));
                break;
            }
            
            // Add to generated tokens
            generated_tokens.push(next_token_id);
            decoder_input_ids.push(next_token_id as i64);
            
            // Print progress and check for issues
            if step % 10 == 0 && step > 0 {
                log_debug(&format!("  📝 Generated {} tokens...", generated_tokens.len()));
                
                // Warning if we're generating a lot without finding EOS
                if step > 100 && generated_tokens.len() > 100 {
                    log_debug("  ⚠️ Generated >100 tokens without EOS - may be stuck in repetition");
                }
            }
            
            // Early exit if stuck in repetition (same token or pattern repeatedly)
            if generated_tokens.len() >= 10 {
                // Check for single token repetition
                let last_5 = &generated_tokens[generated_tokens.len()-5..];
                if last_5.iter().all(|&t| t == last_5[0]) {
                    log_debug("  ⚠️ Detected repetition loop (same token 5x), stopping");
                    break;
                }
                
                // Check for 2-token pattern repetition (like "the az")
                if generated_tokens.len() >= 8 {
                    let last_8 = &generated_tokens[generated_tokens.len()-8..];
                    // Check if it's a repeating 2-token pattern
                    if last_8[0] == last_8[2] && last_8[0] == last_8[4] && last_8[0] == last_8[6] &&
                       last_8[1] == last_8[3] && last_8[1] == last_8[5] && last_8[1] == last_8[7] {
                        log_debug("  ⚠️ Detected 2-token repetition pattern, stopping");
                        break;
                    }
                }
                
                // Check for 3-token pattern repetition
                if generated_tokens.len() >= 9 {
                    let last_9 = &generated_tokens[generated_tokens.len()-9..];
                    if last_9[0] == last_9[3] && last_9[0] == last_9[6] &&
                       last_9[1] == last_9[4] && last_9[1] == last_9[7] &&
                       last_9[2] == last_9[5] && last_9[2] == last_9[8] {
                        log_debug("  ⚠️ Detected 3-token repetition pattern, stopping");
                        break;
                    }
                }
            }
        }
        
        // Decode token IDs to text
        let extracted_text = tokenizer.decode_ids(&generated_tokens);
        
        log_debug(&format!("  ✅ Decoding complete: {} tokens -> {} chars", 
                 generated_tokens.len(), extracted_text.len()));
        
        // Show extraction quality indicators
        if generated_tokens.is_empty() {
            log_debug("  ❌ No tokens generated - image may be blank or unreadable");
        } else if generated_tokens.len() < 5 {
            log_debug(&format!("  ⚠️ Very few tokens ({}) - partial extraction or small text region", generated_tokens.len()));
        } else if extracted_text.len() > 50 {
            // Show preview of longer text
            log_debug(&format!("  📄 Extracted text preview: '{}'...", &extracted_text[..50.min(extracted_text.len())]));
        } else {
            log_debug(&format!("  📄 Extracted text: '{}'", extracted_text));
        }
        
        // Quality assessment
        let tokens_per_char = if extracted_text.is_empty() { 
            0.0 
        } else { 
            generated_tokens.len() as f32 / extracted_text.len() as f32 
        };
        
        if tokens_per_char > 2.0 {
            log_debug(&format!("  ⚠️ High token/char ratio ({:.1}) - possible encoding issues", tokens_per_char));
        }
        
        Ok(extracted_text)
    }
}

// Public function for integration (async)
pub async fn extract_with_simple_trocr(image_data: &[u8]) -> Result<String> {
    let mut trocr = SimpleTrOCR::new()?;
    trocr.extract_text(image_data).await
}

// Synchronous version for UI integration
pub fn extract_with_simple_trocr_sync(image_data: &[u8]) -> Result<String> {
    // For now, check if models exist
    if !Path::new("models/trocr_encoder.onnx").exists() || !Path::new("models/trocr.onnx").exists() {
        return Err(anyhow::anyhow!("TrOCR models not found in models/ directory"));
    }
    
    // Create runtime for sync execution
    let runtime = tokio::runtime::Runtime::new()?;
    runtime.block_on(extract_with_simple_trocr(image_data))
}