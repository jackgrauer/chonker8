// Kitty Graphics Protocol implementation for terminal image display
use anyhow::{Result, bail};
use image::{DynamicImage, ImageFormat};
use std::io::Cursor;
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};

/// Kitty graphics protocol handler
pub struct KittyProtocol {
    supported: bool,
    next_image_id: u32,
    active_images: Vec<u32>,
    chunk_size: usize,
}

impl KittyProtocol {
    /// Create a new Kitty protocol handler
    pub fn new() -> Self {
        Self {
            supported: Self::detect_support(),
            next_image_id: 1,
            active_images: Vec::new(),
            chunk_size: 4096, // 4KB chunks for safe transmission
        }
    }
    
    /// Detect if the terminal supports Kitty graphics protocol
    fn detect_support() -> bool {
        // Check TERM environment variable
        if let Ok(term) = std::env::var("TERM") {
            if term.contains("kitty") {
                return true;
            }
        }
        
        // Check KITTY_WINDOW_ID (more reliable)
        if std::env::var("KITTY_WINDOW_ID").is_ok() {
            return true;
        }
        
        false
    }
    
    /// Check if Kitty protocol is supported
    pub fn is_supported(&self) -> bool {
        self.supported
    }
    
    /// Force enable Kitty support (for testing)
    pub fn force_enable(&mut self) {
        self.supported = true;
    }
    
    /// Display an image at the specified position
    pub fn display_image(
        &mut self,
        image: &DynamicImage,
        x: u32,
        y: u32,
        width: Option<u32>,
        height: Option<u32>,
    ) -> Result<u32> {
        if !self.supported {
            bail!("Kitty graphics protocol not supported");
        }
        
        // Convert image to PNG format
        let mut png_data = Vec::new();
        let mut cursor = Cursor::new(&mut png_data);
        image.write_to(&mut cursor, ImageFormat::Png)?;
        
        // Get dimensions
        let (img_width, img_height) = (image.width(), image.height());
        let display_width = width.unwrap_or(img_width);
        let display_height = height.unwrap_or(img_height);
        
        // Transmit image
        let image_id = self.transmit_image_data(&png_data, display_width, display_height)?;
        
        // Place image at position
        self.place_image(image_id, x, y)?;
        
        self.active_images.push(image_id);
        Ok(image_id)
    }
    
    /// Transmit image data to terminal
    fn transmit_image_data(
        &mut self,
        data: &[u8],
        width: u32,
        height: u32,
    ) -> Result<u32> {
        let image_id = self.next_image_id;
        self.next_image_id += 1;
        
        // Split data into chunks
        let chunks: Vec<&[u8]> = data.chunks(self.chunk_size).collect();
        let total_chunks = chunks.len();
        
        for (i, chunk) in chunks.iter().enumerate() {
            let is_first = i == 0;
            let is_last = i == total_chunks - 1;
            
            // Encode chunk to base64
            let encoded = BASE64.encode(chunk);
            
            // Build control data
            let mut control = String::new();
            
            if is_first {
                // First chunk: specify format and image ID
                control.push_str(&format!(
                    "a=T,f=100,t=d,i={},s={},v={},m={}",
                    image_id,
                    width,
                    height,
                    if is_last { 0 } else { 1 }
                ));
            } else {
                // Continuation chunks
                control.push_str(&format!(
                    "a=T,i={},m={}",
                    image_id,
                    if is_last { 0 } else { 1 }
                ));
            }
            
            // Send to terminal using Kitty protocol format
            print!("\x1b_G{};{}\x1b\\", control, encoded);
        }
        
        Ok(image_id)
    }
    
    /// Place an already transmitted image at a position
    fn place_image(&self, image_id: u32, x: u32, y: u32) -> Result<()> {
        // Place the image at the specified position
        // Using C=1 for relative cursor positioning
        print!("\x1b_Ga=p,i={},x={},y={},C=1\x1b\\", image_id, x, y);
        Ok(())
    }
    
    /// Clear a specific image
    pub fn clear_image(&mut self, image_id: u32) -> Result<()> {
        if !self.supported {
            return Ok(());
        }
        
        // Delete specific image
        print!("\x1b_Ga=d,d=i,i={}\x1b\\", image_id);
        
        self.active_images.retain(|&id| id != image_id);
        Ok(())
    }
    
    /// Clear all images
    pub fn clear_all_images(&mut self) -> Result<()> {
        if !self.supported {
            return Ok(());
        }
        
        // Delete all images
        print!("\x1b_Ga=d,d=a\x1b\\");
        
        self.active_images.clear();
        Ok(())
    }
    
    /// Update an existing image (replace)
    pub fn update_image(
        &mut self,
        image_id: u32,
        image: &DynamicImage,
        width: Option<u32>,
        height: Option<u32>,
    ) -> Result<()> {
        if !self.supported {
            bail!("Kitty graphics protocol not supported");
        }
        
        // Clear the old image
        self.clear_image(image_id)?;
        
        // Display new image with same ID
        let mut png_data = Vec::new();
        let mut cursor = Cursor::new(&mut png_data);
        image.write_to(&mut cursor, ImageFormat::Png)?;
        
        let (img_width, img_height) = (image.width(), image.height());
        let display_width = width.unwrap_or(img_width);
        let display_height = height.unwrap_or(img_height);
        
        // Reuse the same image ID for update
        self.next_image_id = image_id;
        self.transmit_image_data(&png_data, display_width, display_height)?;
        self.next_image_id = image_id + 1;
        
        Ok(())
    }
}

impl Drop for KittyProtocol {
    fn drop(&mut self) {
        // Clean up images when dropping
        let _ = self.clear_all_images();
    }
}