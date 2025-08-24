// Pathfinder-compatible renderer using tiny-skia backend for ARM compatibility
use anyhow::{Result, Context};
use image::{DynamicImage, RgbaImage};
use lopdf::{Document, Object};
use tiny_skia::{Pixmap, Paint, Color, PathBuilder, Transform, Stroke, FillRule, Path as SkPath};
use std::path::Path;

pub struct PathfinderPdfRenderer {
    document: Document,
}

impl PathfinderPdfRenderer {
    pub fn new(pdf_path: &Path) -> Result<Self> {
        let document = Document::load(pdf_path)
            .context("Failed to load PDF with lopdf")?;
        
        Ok(Self { document })
    }
    
    pub fn page_count(&self) -> usize {
        self.document.get_pages().len()
    }
    
    pub fn render_page(&mut self, page_num: usize, width: u32, height: u32) -> Result<DynamicImage> {
        // Get page object
        let page_id = *self.document.get_pages()
            .get(&((page_num + 1) as u32))
            .context("Page not found")?;
        
        let page = self.document.get_object(page_id)
            .context("Failed to get page object")?;
        
        // Extract page dimensions
        let (page_width, page_height) = self.extract_page_dimensions(page)?;
        
        // Calculate scale
        let scale_x = width as f32 / page_width;
        let scale_y = height as f32 / page_height;
        let scale = scale_x.min(scale_y);
        
        let canvas_width = (page_width * scale) as u32;
        let canvas_height = (page_height * scale) as u32;
        
        // Create pixmap (tiny-skia's canvas)
        let mut pixmap = Pixmap::new(canvas_width, canvas_height)
            .context("Failed to create pixmap")?;
        
        // Fill with white background
        pixmap.fill(Color::from_rgba8(255, 255, 255, 255));
        
        // Set up transform for scaling
        let transform = Transform::from_scale(scale, scale);
        
        // Parse and render page content
        self.render_page_content(&mut pixmap, page, transform)?;
        
        // Convert to image
        let image = self.pixmap_to_image(pixmap)?;
        
        Ok(DynamicImage::ImageRgba8(image))
    }
    
    fn extract_page_dimensions(&self, page: &Object) -> Result<(f32, f32)> {
        if let Object::Dictionary(dict) = page {
            // Try MediaBox first
            if let Ok(Object::Array(media_box)) = dict.get(b"MediaBox") {
                if media_box.len() >= 4 {
                    let x1 = self.object_to_float(&media_box[0])?;
                    let y1 = self.object_to_float(&media_box[1])?;
                    let x2 = self.object_to_float(&media_box[2])?;
                    let y2 = self.object_to_float(&media_box[3])?;
                    return Ok(((x2 - x1).abs(), (y2 - y1).abs()));
                }
            }
            
            // Try CropBox as fallback
            if let Ok(Object::Array(crop_box)) = dict.get(b"CropBox") {
                if crop_box.len() >= 4 {
                    let x1 = self.object_to_float(&crop_box[0])?;
                    let y1 = self.object_to_float(&crop_box[1])?;
                    let x2 = self.object_to_float(&crop_box[2])?;
                    let y2 = self.object_to_float(&crop_box[3])?;
                    return Ok(((x2 - x1).abs(), (y2 - y1).abs()));
                }
            }
        }
        
        // Default to US Letter size in points
        Ok((612.0, 792.0))
    }
    
    fn object_to_float(&self, obj: &Object) -> Result<f32> {
        match obj {
            Object::Integer(i) => Ok(*i as f32),
            Object::Real(f) => Ok(*f as f32),
            _ => Ok(0.0),
        }
    }
    
    fn render_page_content(&self, pixmap: &mut Pixmap, page: &Object, transform: Transform) -> Result<()> {
        // Extract content stream
        if let Object::Dictionary(dict) = page {
            if let Ok(contents) = dict.get(b"Contents") {
                let content_data = self.extract_content_data(contents)?;
                self.parse_and_render_content(pixmap, &content_data, transform)?;
            }
        }
        
        Ok(())
    }
    
    fn extract_content_data(&self, contents: &Object) -> Result<Vec<u8>> {
        match contents {
            Object::Stream(stream) => {
                // Decode stream if needed
                stream.decompressed_content()
                    .or_else(|_| Ok(stream.content.clone()))
            }
            Object::Reference(reference) => {
                if let Ok(obj) = self.document.get_object(*reference) {
                    self.extract_content_data(obj)
                } else {
                    Ok(Vec::new())
                }
            }
            Object::Array(arr) => {
                let mut combined = Vec::new();
                for item in arr {
                    combined.extend(self.extract_content_data(item)?);
                    combined.push(b' '); // Add space between streams
                }
                Ok(combined)
            }
            _ => Ok(Vec::new()),
        }
    }
    
    fn parse_and_render_content(&self, pixmap: &mut Pixmap, content: &[u8], transform: Transform) -> Result<()> {
        // Parse PDF content stream
        let content_str = String::from_utf8_lossy(content);
        
        // Create paint objects for stroke and fill
        let mut stroke_paint = Paint::default();
        stroke_paint.set_color(Color::from_rgba8(0, 0, 0, 255)); // Black
        stroke_paint.anti_alias = true;
        
        let mut fill_paint = Paint::default();
        fill_paint.set_color(Color::from_rgba8(0, 0, 0, 255)); // Black
        fill_paint.anti_alias = true;
        
        let mut stroke = Stroke::default();
        stroke.width = 1.0;
        
        // Current path being built
        let mut path_builder = PathBuilder::new();
        let mut current_x = 0.0f32;
        let mut current_y = 0.0f32;
        
        // Parse each line of the content stream
        for line in content_str.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.is_empty() {
                continue;
            }
            
            // Handle basic PDF operators
            match parts.last() {
                Some(&"m") if parts.len() >= 3 => {
                    // Move to
                    if let (Ok(x), Ok(y)) = (parts[0].parse::<f32>(), parts[1].parse::<f32>()) {
                        current_x = x;
                        current_y = y;
                        path_builder.move_to(x, y);
                    }
                }
                Some(&"l") if parts.len() >= 3 => {
                    // Line to
                    if let (Ok(x), Ok(y)) = (parts[0].parse::<f32>(), parts[1].parse::<f32>()) {
                        current_x = x;
                        current_y = y;
                        path_builder.line_to(x, y);
                    }
                }
                Some(&"c") if parts.len() >= 7 => {
                    // Cubic Bezier curve
                    if let (Ok(x1), Ok(y1), Ok(x2), Ok(y2), Ok(x3), Ok(y3)) = (
                        parts[0].parse::<f32>(),
                        parts[1].parse::<f32>(),
                        parts[2].parse::<f32>(),
                        parts[3].parse::<f32>(),
                        parts[4].parse::<f32>(),
                        parts[5].parse::<f32>(),
                    ) {
                        path_builder.cubic_to(x1, y1, x2, y2, x3, y3);
                        current_x = x3;
                        current_y = y3;
                    }
                }
                Some(&"v") if parts.len() >= 5 => {
                    // Cubic Bezier curve with first control point = current point
                    if let (Ok(x2), Ok(y2), Ok(x3), Ok(y3)) = (
                        parts[0].parse::<f32>(),
                        parts[1].parse::<f32>(),
                        parts[2].parse::<f32>(),
                        parts[3].parse::<f32>(),
                    ) {
                        path_builder.cubic_to(current_x, current_y, x2, y2, x3, y3);
                        current_x = x3;
                        current_y = y3;
                    }
                }
                Some(&"y") if parts.len() >= 5 => {
                    // Cubic Bezier curve with second control point = end point
                    if let (Ok(x1), Ok(y1), Ok(x3), Ok(y3)) = (
                        parts[0].parse::<f32>(),
                        parts[1].parse::<f32>(),
                        parts[2].parse::<f32>(),
                        parts[3].parse::<f32>(),
                    ) {
                        path_builder.cubic_to(x1, y1, x3, y3, x3, y3);
                        current_x = x3;
                        current_y = y3;
                    }
                }
                Some(&"re") if parts.len() >= 5 => {
                    // Rectangle
                    if let (Ok(x), Ok(y), Ok(w), Ok(h)) = (
                        parts[0].parse::<f32>(),
                        parts[1].parse::<f32>(),
                        parts[2].parse::<f32>(),
                        parts[3].parse::<f32>(),
                    ) {
                        path_builder.move_to(x, y);
                        path_builder.line_to(x + w, y);
                        path_builder.line_to(x + w, y + h);
                        path_builder.line_to(x, y + h);
                        path_builder.close();
                    }
                }
                Some(&"h") => {
                    // Close path
                    path_builder.close();
                }
                Some(&"S") => {
                    // Stroke path
                    if let Some(path) = path_builder.finish() {
                        pixmap.stroke_path(&path, &stroke_paint, &stroke, transform, None);
                    }
                    path_builder = PathBuilder::new();
                }
                Some(&"s") => {
                    // Close and stroke path
                    path_builder.close();
                    if let Some(path) = path_builder.finish() {
                        pixmap.stroke_path(&path, &stroke_paint, &stroke, transform, None);
                    }
                    path_builder = PathBuilder::new();
                }
                Some(&"f") | Some(&"F") | Some(&"f*") => {
                    // Fill path
                    if let Some(path) = path_builder.finish() {
                        let fill_rule = if parts.last() == Some(&"f*") {
                            FillRule::EvenOdd
                        } else {
                            FillRule::Winding
                        };
                        pixmap.fill_path(&path, &fill_paint, fill_rule, transform, None);
                    }
                    path_builder = PathBuilder::new();
                }
                Some(&"B") | Some(&"B*") => {
                    // Fill and stroke
                    if let Some(path) = path_builder.finish() {
                        let fill_rule = if parts.last() == Some(&"B*") {
                            FillRule::EvenOdd
                        } else {
                            FillRule::Winding
                        };
                        pixmap.fill_path(&path, &fill_paint, fill_rule, transform, None);
                        pixmap.stroke_path(&path, &stroke_paint, &stroke, transform, None);
                    }
                    path_builder = PathBuilder::new();
                }
                Some(&"n") => {
                    // End path without filling or stroking
                    path_builder = PathBuilder::new();
                }
                Some(&"w") if parts.len() >= 2 => {
                    // Set line width
                    if let Ok(width) = parts[0].parse::<f32>() {
                        stroke.width = width;
                    }
                }
                Some(&"RG") if parts.len() >= 4 => {
                    // Set RGB stroke color
                    if let (Ok(r), Ok(g), Ok(b)) = (
                        parts[0].parse::<f32>(),
                        parts[1].parse::<f32>(),
                        parts[2].parse::<f32>(),
                    ) {
                        stroke_paint.set_color(Color::from_rgba(r, g, b, 1.0).unwrap());
                    }
                }
                Some(&"rg") if parts.len() >= 4 => {
                    // Set RGB fill color
                    if let (Ok(r), Ok(g), Ok(b)) = (
                        parts[0].parse::<f32>(),
                        parts[1].parse::<f32>(),
                        parts[2].parse::<f32>(),
                    ) {
                        fill_paint.set_color(Color::from_rgba(r, g, b, 1.0).unwrap());
                    }
                }
                Some(&"G") if parts.len() >= 2 => {
                    // Set gray stroke color
                    if let Ok(gray) = parts[0].parse::<f32>() {
                        stroke_paint.set_color(Color::from_rgba(gray, gray, gray, 1.0).unwrap());
                    }
                }
                Some(&"g") if parts.len() >= 2 => {
                    // Set gray fill color
                    if let Ok(gray) = parts[0].parse::<f32>() {
                        fill_paint.set_color(Color::from_rgba(gray, gray, gray, 1.0).unwrap());
                    }
                }
                _ => {
                    // Ignore other operators for now
                }
            }
        }
        
        Ok(())
    }
    
    fn pixmap_to_image(&self, pixmap: Pixmap) -> Result<RgbaImage> {
        let width = pixmap.width();
        let height = pixmap.height();
        let data = pixmap.data();
        
        // Convert from premultiplied RGBA to straight RGBA
        let mut image = RgbaImage::new(width, height);
        for (i, pixel) in image.pixels_mut().enumerate() {
            let idx = i * 4;
            // tiny-skia uses premultiplied alpha in BGRA format
            let a = data[idx + 3];
            if a > 0 {
                let inv_alpha = 255.0 / a as f32;
                pixel[0] = (data[idx + 2] as f32 * inv_alpha).min(255.0) as u8; // R (BGRA -> RGBA)
                pixel[1] = (data[idx + 1] as f32 * inv_alpha).min(255.0) as u8; // G
                pixel[2] = (data[idx] as f32 * inv_alpha).min(255.0) as u8;     // B
                pixel[3] = a; // A
            } else {
                pixel[0] = 0;
                pixel[1] = 0;
                pixel[2] = 0;
                pixel[3] = 0;
            }
        }
        
        Ok(image)
    }
}

// Compatibility function to match existing API
pub fn render_pdf_page(pdf_path: &Path, page_num: usize, width: u32, height: u32) -> Result<DynamicImage> {
    let mut renderer = PathfinderPdfRenderer::new(pdf_path)?;
    renderer.render_page(page_num, width, height)
}

pub fn get_page_count(pdf_path: &Path) -> Result<usize> {
    let renderer = PathfinderPdfRenderer::new(pdf_path)?;
    Ok(renderer.page_count())
}