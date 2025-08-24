// Vello-based PDF renderer - GPU-accelerated rendering that works on ARM/Metal
use anyhow::{Result, Context};
use image::{DynamicImage, RgbaImage};
use lopdf::{Document, Object};
use vello::Scene;
use vello::kurbo::{Affine, BezPath, Point, Rect, Shape};
use vello::peniko::{Brush, Color, Fill};
use kurbo::Stroke;
use std::path::Path;
use std::sync::Arc;
use std::collections::HashMap;
use vello::wgpu::{Device, Queue};

// Font rendering
use ttf_parser::{Face, GlyphId};
use rusttype::{Font, Scale, point, PositionedGlyph};

pub struct VelloPdfRenderer {
    document: Document,
    device: Option<Arc<Device>>,
    queue: Option<Arc<Queue>>,
    fonts: HashMap<String, Font<'static>>,
    default_font: Option<Font<'static>>,
}

impl VelloPdfRenderer {
    pub fn new(pdf_path: &Path) -> Result<Self> {
        let document = Document::load(pdf_path)
            .context("Failed to load PDF with lopdf")?;
        
        // Try to initialize GPU device for rendering
        let (device, queue) = Self::init_gpu_device();
        
        // Load a default font (we'll embed a basic one for now)
        let default_font = Self::load_default_font().ok();
        
        Ok(Self { 
            document,
            device,
            queue,
            fonts: HashMap::new(),
            default_font,
        })
    }
    
    fn init_gpu_device() -> (Option<Arc<Device>>, Option<Arc<Queue>>) {
        // Try to create a wgpu device for GPU rendering
        // This will work on Metal (macOS), Vulkan (Linux), or DX12 (Windows)
        pollster::block_on(async {
            let instance = vello::wgpu::Instance::new(vello::wgpu::InstanceDescriptor {
                backends: vello::wgpu::Backends::all(),
                ..Default::default()
            });
            
            let adapter = instance
                .request_adapter(&vello::wgpu::RequestAdapterOptions {
                    power_preference: vello::wgpu::PowerPreference::HighPerformance,
                    compatible_surface: None,
                    force_fallback_adapter: false,
                })
                .await?;
            
            let (device, queue) = adapter
                .request_device(
                    &vello::wgpu::DeviceDescriptor {
                        label: Some("Vello PDF Renderer"),
                        required_features: vello::wgpu::Features::empty(),
                        required_limits: vello::wgpu::Limits::default(),
                        memory_hints: Default::default(),
                    },
                    None,
                )
                .await
                .ok()?;
            
            Some((Arc::new(device), Arc::new(queue)))
        }).map(|(device, queue)| (Some(device), Some(queue)))
          .unwrap_or((None, None))
    }
    
    fn load_default_font() -> Result<Font<'static>> {
        // Use a simple built-in font - we can load system fonts later
        // For now, let's try to load from common system locations
        let font_paths = [
            "/System/Library/Fonts/Arial.ttf",
            "/System/Library/Fonts/Helvetica.ttc",
            "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
            "/usr/share/fonts/TTF/arial.ttf",
        ];
        
        for path in &font_paths {
            if let Ok(font_data) = std::fs::read(path) {
                if let Some(font) = Font::try_from_vec(font_data) {
                    eprintln!("[FONT] Loaded system font: {}", path);
                    return Ok(font);
                }
            }
        }
        
        eprintln!("[FONT] No system fonts found, using placeholder rectangles");
        Err(anyhow::anyhow!("No system fonts available"))
    }
    
    pub fn page_count(&self) -> usize {
        self.document.get_pages().len()
    }
    
    pub fn render_page(&mut self, page_num: usize, width: u32, height: u32) -> Result<DynamicImage> {
        // Debug: Render page called
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
        
        // Create a new scene
        let mut scene = Scene::new();
        
        // Set up transform for scaling
        let transform = Affine::scale(scale as f64);
        
        // White background
        scene.fill(
            Fill::NonZero,
            transform,
            &Brush::Solid(Color::rgb8(255, 255, 255)),
            None,
            &Rect::new(0.0, 0.0, page_width as f64, page_height as f64),
        );
        
        // Parse and render page content
        self.render_page_content(&mut scene, page, transform)?;
        
        // Render to pixels
        let image = self.render_scene_to_image(scene, canvas_width, canvas_height)?;
        
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
    
    fn render_page_content(&self, scene: &mut Scene, page: &Object, transform: Affine) -> Result<()> {
        // Extract content stream
        if let Object::Dictionary(dict) = page {
            if let Ok(contents) = dict.get(b"Contents") {
                let content_data = self.extract_content_data(contents)?;
                // Pass the page object so we can access Resources
                self.parse_and_render_content(scene, &content_data, page, transform)?;
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
    
    fn parse_and_render_content(&self, scene: &mut Scene, content: &[u8], page: &Object, transform: Affine) -> Result<()> {
        // For now, render a placeholder that shows we have PDF content
        // TODO: Implement full PDF content parsing for text, images, and complex graphics
        
        let content_str = String::from_utf8_lossy(content);
        
        // Always render placeholders to show PDF structure regardless of content parsing
        eprintln!("[VELLO] Content stream length: {} bytes", content.len());
        eprintln!("[VELLO] Content preview: {}", 
            String::from_utf8_lossy(&content[..content.len().min(200)]).replace('\n', "\\n"));
        
        // Draw a grid pattern to show PDF structure
        self.render_content_placeholder(scene, transform)?;
        
        // Add some sample text areas to simulate content
        self.render_text_placeholders(scene, transform)?;
        
        // If we have actual content, try to parse it too
        if !content_str.trim().is_empty() {
            eprintln!("[VELLO] Processing {} bytes of PDF content stream", content.len());
        }
        
        // Current graphics state for any actual path content we can render
        let mut stroke_color = Color::rgb8(0, 0, 0); // Black
        let mut fill_color = Color::rgb8(0, 0, 0); // Black
        let mut line_width = 1.0;
        let mut text_size = 12.0;
        let mut text_position = Point::new(0.0, 0.0);
        let mut in_text_block = false;
        
        // Current path being built
        let mut current_path = BezPath::new();
        let mut current_point = Point::new(0.0, 0.0);
        
        // Parse each line of the content stream
        for line in content_str.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.is_empty() {
                continue;
            }
            
            // First, scan for XObject Do commands anywhere in the line
            for i in 0..parts.len() {
                if parts[i] == "Do" && i > 0 {
                    let xobject_name = parts[i - 1];
                    eprintln!("[VELLO] Found XObject reference: {}", xobject_name);
                    
                    // Try to extract the actual XObject image
                    if let Ok(xobject_data) = self.extract_xobject_image(page, xobject_name) {
                        eprintln!("[VELLO] Extracted XObject image data: {} bytes", xobject_data.len());
                        
                        // Try to decode the image data
                        match image::load_from_memory(&xobject_data) {
                            Ok(img) => {
                                eprintln!("[VELLO] Successfully decoded image: {}x{}", img.width(), img.height());
                                
                                // Convert to RGBA if needed
                                let rgba_img = img.to_rgba8();
                                let img_width = rgba_img.width();
                                let img_height = rgba_img.height();
                                
                                // Create Vello image from the decoded data
                                let vello_image = peniko::Image::new(
                                    rgba_img.into_raw().into(),
                                    peniko::Format::Rgba8,
                                    img_width,
                                    img_height,
                                );
                                
                                // Calculate appropriate position and size (scale to fit in viewport)
                                let scale = (500.0 / img_width as f64).min(600.0 / img_height as f64);
                                let scaled_width = img_width as f64 * scale;
                                let scaled_height = img_height as f64 * scale;
                                
                                // Center the image
                                let x = (600.0 - scaled_width) / 2.0;
                                let y = (800.0 - scaled_height) / 2.0;
                                
                                // Draw the actual image
                                scene.draw_image(
                                    &vello_image,
                                    Affine::translate((x, y)) * Affine::scale_non_uniform(scale, scale),
                                );
                                
                                eprintln!("[VELLO] Rendered image at ({}, {}) with scale {}", x, y, scale);
                            }
                            Err(e) => {
                                eprintln!("[VELLO] Failed to decode image: {}", e);
                                eprintln!("[VELLO] First 20 bytes: {:?}", &xobject_data[..20.min(xobject_data.len())]);
                                
                                // Fall back to placeholder
                                let image_placeholder = Rect::new(50.0, 50.0, 500.0, 400.0);
                                scene.fill(
                                    Fill::NonZero,
                                    transform,
                                    &Brush::Solid(Color::rgb8(200, 150, 150)), // Red tint for decode failure
                                    None,
                                    &image_placeholder,
                                );
                            }
                        }
                        
                        eprintln!("[VELLO] Found image data for XObject: {}", xobject_name);
                    } else {
                        // Fallback to blue placeholder if we can't extract the image
                        let image_placeholder = Rect::new(50.0, 50.0, 500.0, 400.0);
                        scene.fill(
                            Fill::NonZero,
                            transform,
                            &Brush::Solid(Color::rgb8(100, 150, 200)), // Light blue
                            None,
                            &image_placeholder,
                        );
                    }
                    
                    // Add a border
                    let stroke = Stroke::new(2.0);
                    let image_rect = Rect::new(50.0, 50.0, 500.0, 400.0);
                    scene.stroke(
                        &stroke,
                        transform,
                        &Brush::Solid(Color::rgb8(50, 100, 150)),
                        None,
                        &image_rect,
                    );
                    
                    eprintln!("[VELLO] Rendered XObject: {}", xobject_name);
                }
            }
            
            // Handle basic PDF operators
            match parts.last() {
                Some(&"m") if parts.len() >= 3 => {
                    // Move to
                    if let (Ok(x), Ok(y)) = (parts[0].parse::<f64>(), parts[1].parse::<f64>()) {
                        current_point = Point::new(x, y);
                        current_path.move_to(current_point);
                    }
                }
                Some(&"l") if parts.len() >= 3 => {
                    // Line to
                    if let (Ok(x), Ok(y)) = (parts[0].parse::<f64>(), parts[1].parse::<f64>()) {
                        current_point = Point::new(x, y);
                        current_path.line_to(current_point);
                    }
                }
                Some(&"c") if parts.len() >= 7 => {
                    // Cubic Bezier curve
                    if let (Ok(x1), Ok(y1), Ok(x2), Ok(y2), Ok(x3), Ok(y3)) = (
                        parts[0].parse::<f64>(),
                        parts[1].parse::<f64>(),
                        parts[2].parse::<f64>(),
                        parts[3].parse::<f64>(),
                        parts[4].parse::<f64>(),
                        parts[5].parse::<f64>(),
                    ) {
                        current_path.curve_to(
                            Point::new(x1, y1),
                            Point::new(x2, y2),
                            Point::new(x3, y3),
                        );
                        current_point = Point::new(x3, y3);
                    }
                }
                Some(&"v") if parts.len() >= 5 => {
                    // Cubic Bezier curve with first control point = current point
                    if let (Ok(x2), Ok(y2), Ok(x3), Ok(y3)) = (
                        parts[0].parse::<f64>(),
                        parts[1].parse::<f64>(),
                        parts[2].parse::<f64>(),
                        parts[3].parse::<f64>(),
                    ) {
                        current_path.curve_to(
                            current_point,
                            Point::new(x2, y2),
                            Point::new(x3, y3),
                        );
                        current_point = Point::new(x3, y3);
                    }
                }
                Some(&"y") if parts.len() >= 5 => {
                    // Cubic Bezier curve with second control point = end point
                    if let (Ok(x1), Ok(y1), Ok(x3), Ok(y3)) = (
                        parts[0].parse::<f64>(),
                        parts[1].parse::<f64>(),
                        parts[2].parse::<f64>(),
                        parts[3].parse::<f64>(),
                    ) {
                        let end_point = Point::new(x3, y3);
                        current_path.curve_to(
                            Point::new(x1, y1),
                            end_point,
                            end_point,
                        );
                        current_point = end_point;
                    }
                }
                Some(&"re") if parts.len() >= 5 => {
                    // Rectangle
                    if let (Ok(x), Ok(y), Ok(w), Ok(h)) = (
                        parts[0].parse::<f64>(),
                        parts[1].parse::<f64>(),
                        parts[2].parse::<f64>(),
                        parts[3].parse::<f64>(),
                    ) {
                        let rect = Rect::new(x, y, x + w, y + h);
                        current_path = rect.to_path(0.01);
                    }
                }
                Some(&"h") => {
                    // Close path
                    current_path.close_path();
                }
                Some(&"S") => {
                    // Stroke path
                    let stroke = Stroke::new(line_width);
                    scene.stroke(
                        &stroke,
                        transform,
                        &Brush::Solid(stroke_color),
                        None,
                        &current_path,
                    );
                    current_path = BezPath::new();
                }
                Some(&"s") => {
                    // Close and stroke path
                    current_path.close_path();
                    let stroke = Stroke::new(line_width);
                    scene.stroke(
                        &stroke,
                        transform,
                        &Brush::Solid(stroke_color),
                        None,
                        &current_path,
                    );
                    current_path = BezPath::new();
                }
                Some(&"f") | Some(&"F") => {
                    // Fill path (non-zero winding)
                    scene.fill(
                        Fill::NonZero,
                        transform,
                        &Brush::Solid(fill_color),
                        None,
                        &current_path,
                    );
                    current_path = BezPath::new();
                }
                Some(&"f*") => {
                    // Fill path (even-odd)
                    scene.fill(
                        Fill::EvenOdd,
                        transform,
                        &Brush::Solid(fill_color),
                        None,
                        &current_path,
                    );
                    current_path = BezPath::new();
                }
                Some(&"B") => {
                    // Fill and stroke (non-zero)
                    scene.fill(
                        Fill::NonZero,
                        transform,
                        &Brush::Solid(fill_color),
                        None,
                        &current_path,
                    );
                    let stroke = Stroke::new(line_width);
                    scene.stroke(
                        &stroke,
                        transform,
                        &Brush::Solid(stroke_color),
                        None,
                        &current_path,
                    );
                    current_path = BezPath::new();
                }
                Some(&"B*") => {
                    // Fill and stroke (even-odd)
                    scene.fill(
                        Fill::EvenOdd,
                        transform,
                        &Brush::Solid(fill_color),
                        None,
                        &current_path,
                    );
                    let stroke = Stroke::new(line_width);
                    scene.stroke(
                        &stroke,
                        transform,
                        &Brush::Solid(stroke_color),
                        None,
                        &current_path,
                    );
                    current_path = BezPath::new();
                }
                Some(&"n") => {
                    // End path without filling or stroking
                    current_path = BezPath::new();
                }
                Some(&"w") if parts.len() >= 2 => {
                    // Set line width
                    if let Ok(width) = parts[0].parse::<f64>() {
                        line_width = width;
                    }
                }
                Some(&"RG") if parts.len() >= 4 => {
                    // Set RGB stroke color
                    if let (Ok(r), Ok(g), Ok(b)) = (
                        parts[0].parse::<f32>(),
                        parts[1].parse::<f32>(),
                        parts[2].parse::<f32>(),
                    ) {
                        stroke_color = Color::rgb(r as f64, g as f64, b as f64);
                    }
                }
                Some(&"rg") if parts.len() >= 4 => {
                    // Set RGB fill color
                    if let (Ok(r), Ok(g), Ok(b)) = (
                        parts[0].parse::<f32>(),
                        parts[1].parse::<f32>(),
                        parts[2].parse::<f32>(),
                    ) {
                        fill_color = Color::rgb(r as f64, g as f64, b as f64);
                    }
                }
                Some(&"G") if parts.len() >= 2 => {
                    // Set gray stroke color
                    if let Ok(gray) = parts[0].parse::<f32>() {
                        stroke_color = Color::rgb(gray as f64, gray as f64, gray as f64);
                    }
                }
                Some(&"g") if parts.len() >= 2 => {
                    // Set gray fill color
                    if let Ok(gray) = parts[0].parse::<f32>() {
                        fill_color = Color::rgb(gray as f64, gray as f64, gray as f64);
                    }
                }
                Some(&"BT") => {
                    // Begin text block
                    in_text_block = true;
                    text_position = Point::new(0.0, 0.0);
                }
                Some(&"ET") => {
                    // End text block
                    in_text_block = false;
                }
                Some(&"Td") if parts.len() >= 3 && in_text_block => {
                    // Text position
                    if let (Ok(x), Ok(y)) = (parts[0].parse::<f64>(), parts[1].parse::<f64>()) {
                        text_position = Point::new(text_position.x + x, text_position.y + y);
                    }
                }
                Some(&"Tm") if parts.len() >= 7 && in_text_block => {
                    // Text matrix
                    if let Ok(e) = parts[4].parse::<f64>() {
                        if let Ok(f) = parts[5].parse::<f64>() {
                            text_position = Point::new(e, f);
                        }
                    }
                }
                Some(&"Tf") if parts.len() >= 3 && in_text_block => {
                    // Set font and size
                    if let Ok(size) = parts[1].parse::<f64>() {
                        text_size = size;
                    }
                }
                Some(&"Tj") if parts.len() >= 2 && in_text_block => {
                    // Show text string
                    let text_content = if parts[0].starts_with('(') && parts[0].ends_with(')') {
                        // Extract text from parentheses
                        parts[0].trim_start_matches('(').trim_end_matches(')')
                    } else {
                        parts[0]
                    };
                    
                    eprintln!("[VELLO] Rendering text: '{}'", text_content);
                    
                    // Calculate text dimensions based on content
                    let text_width = text_size * 0.6 * text_content.len() as f64;
                    let text_height = text_size;
                    
                    // Draw a background rectangle for the text
                    let text_rect = Rect::new(
                        text_position.x,
                        text_position.y - text_height as f64 * 0.8,
                        text_position.x + text_width as f64,
                        text_position.y + text_height as f64 * 0.2,
                    );
                    
                    // Use a light background to show text area
                    scene.fill(
                        Fill::NonZero,
                        transform,
                        &Brush::Solid(Color::rgb8(250, 250, 200)), // Light yellow background
                        None,
                        &text_rect,
                    );
                    
                    // Draw text outline (since we don't have font rendering yet)
                    let stroke = Stroke::new(0.5);
                    scene.stroke(
                        &stroke,
                        transform,
                        &Brush::Solid(fill_color),
                        None,
                        &text_rect,
                    );
                    
                    // Move position forward
                    text_position.x += text_width as f64;
                }
                Some(&"TJ") if parts.len() >= 2 && in_text_block => {
                    // Show text array - simplified handling
                    let text_width = text_size * 5.0; // Approximate
                    text_position.x += text_width;
                }
                Some(&"q") => {
                    // Save graphics state - would need a stack in full implementation
                }
                Some(&"Q") => {
                    // Restore graphics state
                }
                Some(&"cm") if parts.len() >= 7 => {
                    // Modify transformation matrix - simplified for now
                    eprintln!("[VELLO] Found transformation matrix: {:?}", parts);
                }
                // Old XObject handling removed - now handled earlier in the loop
                _ => {
                    // Ignore other operators for now
                }
            }
        }
        
        Ok(())
    }
    
    fn render_content_placeholder(&self, scene: &mut Scene, transform: Affine) -> Result<()> {
        // Draw a subtle grid pattern to show PDF structure
        let grid_color = Color::rgb8(200, 200, 200);
        let stroke = kurbo::Stroke::new(0.5);
        
        // Vertical lines
        for x in (0..600).step_by(100) {
            let line = BezPath::from_iter([
                kurbo::PathEl::MoveTo(Point::new(x as f64, 0.0)),
                kurbo::PathEl::LineTo(Point::new(x as f64, 800.0)),
            ]);
            scene.stroke(&stroke, transform, &Brush::Solid(grid_color), None, &line);
        }
        
        // Horizontal lines  
        for y in (0..800).step_by(100) {
            let line = BezPath::from_iter([
                kurbo::PathEl::MoveTo(Point::new(0.0, y as f64)),
                kurbo::PathEl::LineTo(Point::new(600.0, y as f64)),
            ]);
            scene.stroke(&stroke, transform, &Brush::Solid(grid_color), None, &line);
        }
        
        Ok(())
    }
    
    fn render_text_placeholders(&self, scene: &mut Scene, transform: Affine) -> Result<()> {
        // Simulate text blocks with light backgrounds and labels
        
        // Title area with light background
        let title_rect = Rect::new(50.0, 50.0, 550.0, 100.0);
        scene.fill(
            Fill::NonZero,
            transform,
            &Brush::Solid(Color::rgb8(250, 250, 200)), // Light yellow for text areas
            None,
            &title_rect,
        );
        
        // Add border to show it's a text area
        let stroke = Stroke::new(1.0);
        scene.stroke(
            &stroke,
            transform,
            &Brush::Solid(Color::rgb8(100, 100, 100)),
            None,
            &title_rect,
        );
        
        // Paragraph blocks
        let paragraphs = [
            Rect::new(50.0, 150.0, 550.0, 170.0),
            Rect::new(50.0, 180.0, 400.0, 200.0),
            Rect::new(50.0, 210.0, 480.0, 230.0),
            Rect::new(50.0, 250.0, 520.0, 270.0),
            Rect::new(50.0, 280.0, 350.0, 300.0),
            Rect::new(50.0, 330.0, 500.0, 350.0),
            Rect::new(50.0, 360.0, 450.0, 380.0),
            Rect::new(50.0, 390.0, 480.0, 410.0),
        ];
        
        for rect in paragraphs.iter() {
            scene.fill(
                Fill::NonZero,
                transform,
                &Brush::Solid(Color::rgb8(240, 240, 220)), // Light yellow-gray for text blocks
                None,
                rect,
            );
        }
        
        // Add a "PDF CONTENT" watermark
        let watermark_rect = Rect::new(200.0, 600.0, 400.0, 650.0);
        scene.fill(
            Fill::NonZero,
            transform,
            &Brush::Solid(Color::rgb8(100, 100, 100)),
            None,
            &watermark_rect,
        );
        
        Ok(())
    }
    
    fn extract_xobject_image(&self, page: &Object, xobject_name: &str) -> Result<Vec<u8>> {
        // Extract XObject image data from the page's Resources dictionary
        if let Object::Dictionary(page_dict) = page {
            // Get Resources dictionary
            if let Ok(Object::Dictionary(resources)) = page_dict.get(b"Resources") {
                // Get XObject dictionary from Resources
                if let Ok(Object::Dictionary(xobjects)) = resources.get(b"XObject") {
                    // Remove leading slash from XObject name if present
                    let name = xobject_name.trim_start_matches('/');
                    
                    // Look up the specific XObject
                    if let Ok(xobj_ref) = xobjects.get(name.as_bytes()) {
                        // Resolve reference if needed
                        let xobj = match xobj_ref {
                            Object::Reference(ref_id) => {
                                self.document.get_object(*ref_id).unwrap_or(xobj_ref)
                            }
                            _ => xobj_ref
                        };
                        
                        // Extract image data from XObject stream
                        if let Object::Stream(stream) = xobj {
                            eprintln!("[VELLO] Found XObject stream, extracting image data...");
                            
                            // Get the stream dictionary to check subtype
                            if let Ok(Object::Name(subtype)) = stream.dict.get(b"Subtype") {
                                eprintln!("[VELLO] XObject subtype: {:?}", String::from_utf8_lossy(subtype));
                                
                                if subtype == b"Image" {
                                    // This is an image XObject
                                    if let Ok(width) = stream.dict.get(b"Width") {
                                        if let Ok(height) = stream.dict.get(b"Height") {
                                            eprintln!("[VELLO] Image dimensions: {:?} x {:?}", width, height);
                                        }
                                    }
                                    
                                    // Return the raw image data
                                    return stream.decompressed_content()
                                        .or_else(|_| Ok(stream.content.clone()));
                                }
                            }
                        }
                    }
                }
            }
        }
        
        Err(anyhow::anyhow!("Could not extract XObject image"))
    }
    
    fn render_scene_to_image(&self, scene: Scene, width: u32, height: u32) -> Result<RgbaImage> {
        // If we have GPU device, use it for rendering
        if let (Some(device), Some(queue)) = (&self.device, &self.queue) {
            self.render_with_gpu(scene, width, height, device, queue)
        } else {
            // Fallback to CPU rendering
            self.render_with_cpu(scene, width, height)
        }
    }
    
    fn render_with_gpu(
        &self, 
        scene: Scene, 
        width: u32, 
        height: u32,
        device: &Device,
        queue: &Queue,
    ) -> Result<RgbaImage> {
        // Create texture for rendering
        let texture_desc = vello::wgpu::TextureDescriptor {
            label: Some("PDF render target"),
            size: vello::wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: vello::wgpu::TextureDimension::D2,
            format: vello::wgpu::TextureFormat::Rgba8Unorm,
            usage: vello::wgpu::TextureUsages::RENDER_ATTACHMENT | vello::wgpu::TextureUsages::COPY_SRC | vello::wgpu::TextureUsages::STORAGE_BINDING,
            view_formats: &[],
        };
        
        let texture = device.create_texture(&texture_desc);
        let view = texture.create_view(&Default::default());
        
        // Create Vello renderer
        let mut renderer = vello::Renderer::new(
            device,
            vello::RendererOptions {
                surface_format: Some(vello::wgpu::TextureFormat::Rgba8Unorm),
                use_cpu: false,
                antialiasing_support: vello::AaSupport::all(),
                num_init_threads: std::num::NonZeroUsize::new(1),
            },
        ).context("Failed to create Vello renderer")?;
        
        // Render the scene
        renderer
            .render_to_texture(
                device,
                queue,
                &scene,
                &view,
                &vello::RenderParams {
                    base_color: vello::peniko::Color::WHITE,
                    width,
                    height,
                    antialiasing_method: vello::AaConfig::Msaa16,
                },
            )
            .context("Failed to render scene")?;
        
        // Read back the pixels
        // Align bytes per row to 256 bytes (COPY_BYTES_PER_ROW_ALIGNMENT)
        let unpadded_bytes_per_row = width * 4;
        let align = 256;
        let padded_bytes_per_row = ((unpadded_bytes_per_row + align - 1) / align) * align;
        let buffer_size = (padded_bytes_per_row * height) as u64;
        
        let buffer_desc = vello::wgpu::BufferDescriptor {
            label: Some("PDF pixel buffer"),
            size: buffer_size,
            usage: vello::wgpu::BufferUsages::COPY_DST | vello::wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        };
        
        let buffer = device.create_buffer(&buffer_desc);
        
        // Copy texture to buffer
        let mut encoder = device.create_command_encoder(&vello::wgpu::CommandEncoderDescriptor {
            label: Some("PDF render encoder"),
        });
        
        encoder.copy_texture_to_buffer(
            vello::wgpu::ImageCopyTexture {
                texture: &texture,
                mip_level: 0,
                origin: vello::wgpu::Origin3d::ZERO,
                aspect: vello::wgpu::TextureAspect::All,
            },
            vello::wgpu::ImageCopyBuffer {
                buffer: &buffer,
                layout: vello::wgpu::ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some(padded_bytes_per_row),
                    rows_per_image: Some(height),
                },
            },
            vello::wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );
        
        queue.submit(std::iter::once(encoder.finish()));
        
        // Wait for GPU to finish and read pixels
        let buffer_slice = buffer.slice(..);
        let (tx, rx) = std::sync::mpsc::channel();
        buffer_slice.map_async(vello::wgpu::MapMode::Read, move |result| {
            tx.send(result).unwrap();
        });
        
        device.poll(vello::wgpu::Maintain::Wait);
        rx.recv().unwrap().context("Failed to map buffer")?;
        
        let data = buffer_slice.get_mapped_range();
        
        // Remove padding from each row when creating the image
        let unpadded_bytes_per_row = width * 4;
        let align = 256u32;
        let padded_bytes_per_row = ((unpadded_bytes_per_row + align - 1) / align) * align;
        
        let mut pixels = Vec::with_capacity((width * height * 4) as usize);
        for row in 0..height {
            let row_start = (row * padded_bytes_per_row) as usize;
            let row_end = row_start + (width * 4) as usize;
            pixels.extend_from_slice(&data[row_start..row_end]);
        }
        
        let image = RgbaImage::from_raw(width, height, pixels)
            .context("Failed to create image from pixels")?;
        
        Ok(image)
    }
    
    fn render_with_cpu(&self, _scene: Scene, width: u32, height: u32) -> Result<RgbaImage> {
        // CPU rendering fallback using Vello's CPU renderer
        // For now, create a white image as placeholder
        // The actual CPU renderer would be similar to the GPU version
        // but using Vello's CPU backend
        
        let mut image = RgbaImage::new(width, height);
        
        // Fill with white background
        for pixel in image.pixels_mut() {
            *pixel = image::Rgba([255, 255, 255, 255]);
        }
        
        // TODO: Implement actual CPU rendering when Vello's CPU backend is available
        
        Ok(image)
    }
}

// Compatibility function to match existing API
pub fn render_pdf_page(pdf_path: &Path, page_num: usize, width: u32, height: u32) -> Result<DynamicImage> {
    let mut renderer = VelloPdfRenderer::new(pdf_path)?;
    renderer.render_page(page_num, width, height)
}

pub fn get_page_count(pdf_path: &Path) -> Result<usize> {
    let renderer = VelloPdfRenderer::new(pdf_path)?;
    Ok(renderer.page_count())
}