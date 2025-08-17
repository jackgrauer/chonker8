// Theme module - color scheme and styling
use crossterm::style::Color;

pub struct ChonkerTheme;

impl ChonkerTheme {
    pub fn bg_status_dark() -> Color {
        Color::Rgb { r: 40, g: 40, b: 46 }
    }
    
    pub fn text_status_dark() -> Color {
        Color::Rgb { r: 200, g: 200, b: 200 }
    }
    
    pub fn text_primary() -> Color {
        Color::Rgb { r: 248, g: 248, b: 242 }
    }
    
    pub fn text_secondary() -> Color {
        Color::Rgb { r: 180, g: 180, b: 180 }
    }
    
    pub fn text_dim() -> Color {
        Color::Rgb { r: 120, g: 120, b: 120 }
    }
    
    pub fn text_header() -> Color {
        Color::Black
    }
    
    pub fn accent_load_file() -> Color {
        Color::Rgb { r: 219, g: 112, b: 147 }  // Soft pink
    }
    
    pub fn accent_text() -> Color {
        Color::Rgb { r: 176, g: 196, b: 222 }  // Light steel blue
    }
    
    pub fn success() -> Color {
        Color::Rgb { r: 152, g: 195, b: 121 }  // Soft green
    }
}