// TrOCR Tokenizer Module
use anyhow::Result;
use tokenizers::tokenizer::Tokenizer;
use std::path::Path;
use std::collections::HashMap;
use serde_json;

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
        let _ = writeln!(file, "[{}] [TOKENIZER] {}", 
            Local::now().format("%H:%M:%S%.3f"), 
            message);
    }
}

pub struct TrOCRTokenizer {
    tokenizer: Option<Tokenizer>,
    vocab: HashMap<String, u32>,
    id_to_token: HashMap<u32, String>,
    bos_token_id: u32,
    eos_token_id: u32,
    pad_token_id: u32,
}

impl TrOCRTokenizer {
    pub fn new() -> Result<Self> {
        log_debug("  📚 Loading TrOCR tokenizer...");
        
        // Load vocabulary
        let vocab_path = Path::new("models/vocab.json");
        let mut vocab: HashMap<String, u32> = HashMap::new();
        let mut id_to_token: HashMap<u32, String> = HashMap::new();
        
        if vocab_path.exists() {
            let vocab_str = std::fs::read_to_string(vocab_path)?;
            vocab = serde_json::from_str(&vocab_str)?;
            
            // Create reverse mapping
            for (token, id) in &vocab {
                id_to_token.insert(*id, token.clone());
            }
            
            log_debug(&format!("  ✅ Loaded vocabulary with {} tokens", vocab.len()));
        } else {
            log_debug("  ⚠️ Vocabulary file not found, using minimal vocab");
            // Minimal vocabulary for testing
            vocab.insert("<s>".to_string(), 0);
            vocab.insert("</s>".to_string(), 2);
            vocab.insert("<pad>".to_string(), 1);
            id_to_token.insert(0, "<s>".to_string());
            id_to_token.insert(2, "</s>".to_string());
            id_to_token.insert(1, "<pad>".to_string());
        }
        
        // Try to load full tokenizer if available
        let tokenizer = if Path::new("models/tokenizer.json").exists() {
            match Tokenizer::from_file("models/tokenizer.json") {
                Ok(t) => Some(t),
                Err(e) => {
                    log_debug(&format!("  ⚠️ Failed to load tokenizer file: {:?}", e));
                    None
                }
            }
        } else {
            None
        };
        
        Ok(Self {
            tokenizer,
            vocab,
            id_to_token,
            bos_token_id: 0,
            eos_token_id: 2,
            pad_token_id: 1,
        })
    }
    
    pub fn decode_ids(&self, token_ids: &[u32]) -> String {
        // If tokenizer library is available, use it directly
        if let Some(ref tokenizer) = self.tokenizer {
            return tokenizer.decode(&token_ids, true).unwrap_or_default();
        }
        
        // Otherwise, manual decode with proper byte-level BPE handling
        let mut decoded_tokens = Vec::new();
        
        for &id in token_ids {
            if id == self.eos_token_id {
                break;
            }
            
            if let Some(token) = self.id_to_token.get(&id) {
                // Skip special tokens
                if token.starts_with('<') && token.ends_with('>') {
                    continue;
                }
                
                // Handle Ġ prefix (represents space in GPT-2 BPE)
                let decoded = if token.starts_with("Ġ") {
                    format!(" {}", &token[3..]) // Skip the 3-byte "Ġ" prefix
                } else {
                    token.clone()
                };
                
                decoded_tokens.push(decoded);
            }
        }
        
        decoded_tokens.join("").trim().to_string()
    }
    
    pub fn get_decoder_start_ids(&self) -> Vec<i64> {
        vec![self.bos_token_id as i64]
    }
    
    pub fn get_eos_token_id(&self) -> u32 {
        self.eos_token_id
    }
}

// LayoutLM Tokenizer support
pub struct LayoutLMTokenizer {
    tokenizer: Option<Tokenizer>,
    vocab: HashMap<String, u32>,
}

impl LayoutLMTokenizer {
    pub fn new() -> Result<Self> {
        log_debug("  📚 Loading LayoutLM tokenizer...");
        
        // Try to load LayoutLM vocabulary
        let vocab_path = Path::new("models/layoutlm_vocab.json");
        let mut vocab = HashMap::new();
        
        if vocab_path.exists() {
            let vocab_str = std::fs::read_to_string(vocab_path)?;
            vocab = serde_json::from_str(&vocab_str)?;
            log_debug(&format!("  ✅ Loaded LayoutLM vocabulary with {} tokens", vocab.len()));
        }
        
        // Load tokenizer if available
        let tokenizer = if Path::new("models/layoutlm_tokenizer.json").exists() {
            match Tokenizer::from_file("models/layoutlm_tokenizer.json") {
                Ok(t) => Some(t),
                Err(e) => {
                    log_debug(&format!("  ⚠️ Failed to load tokenizer: {:?}", e));
                    None
                }
            }
        } else {
            None
        };
        
        Ok(Self { tokenizer, vocab })
    }
    
    pub fn tokenize(&self, text: &str) -> Vec<u32> {
        // Simple tokenization for testing
        if let Some(ref tokenizer) = self.tokenizer {
            if let Ok(encoding) = tokenizer.encode(text, false) {
                return encoding.get_ids().to_vec();
            }
        }
        // Fallback: return dummy tokens
        vec![101, 102] // [CLS], [SEP]
    }
}
