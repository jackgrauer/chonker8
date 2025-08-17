// Theme constants for Chonker8
use crossterm::style::Color;

// Dark theme colors
pub const BG_DARK: Color = Color::Rgb { r: 30, g: 30, b: 30 };
pub const FG_DARK: Color = Color::Rgb { r: 200, g: 200, b: 200 };
pub const BG_STATUS: Color = Color::Rgb { r: 50, g: 50, b: 50 };
pub const FG_STATUS: Color = Color::Rgb { r: 180, g: 180, b: 180 };

// Panel header colors - subtle palette matching file picker
pub const HEADER_PDF: Color = Color::Rgb { r: 219, g: 112, b: 147 };   // Soft pink (palevioletred)
pub const HEADER_TEXT: Color = Color::Rgb { r: 176, g: 196, b: 222 };  // Light steel blue  
pub const HEADER_DEBUG: Color = Color::Rgb { r: 152, g: 195, b: 121 }; // Soft green

// Selection and cursor colors
pub const SELECTION_BG: Color = Color::DarkBlue;
pub const SELECTION_FG: Color = Color::White;
pub const CURSOR_BG: Color = Color::Rgb { r: 100, g: 149, b: 237 };  // Cornflower blue
pub const CURSOR_FG: Color = Color::Black;