// Theme constants - softer color palette
use crossterm::style::Color;

// Headers - soft colors, not electric
pub const HEADER_PDF: Color = Color::Rgb { r: 219, g: 112, b: 147 };    // Soft pink
pub const HEADER_TEXT: Color = Color::Rgb { r: 176, g: 196, b: 222 };   // Light steel blue  
pub const HEADER_DEBUG: Color = Color::Rgb { r: 152, g: 195, b: 121 };  // Soft green

// Text colors
pub const TEXT_PRIMARY: Color = Color::Rgb { r: 248, g: 248, b: 242 };
pub const TEXT_SECONDARY: Color = Color::Rgb { r: 180, g: 180, b: 180 };
pub const TEXT_DIM: Color = Color::Rgb { r: 120, g: 120, b: 120 };

// UI elements
pub const BORDER: Color = Color::Rgb { r: 100, g: 100, b: 100 };
pub const SELECTION_BG: Color = Color::Rgb { r: 68, g: 71, b: 90 };
pub const SELECTION_FG: Color = Color::Rgb { r: 248, g: 248, b: 242 };
pub const CURSOR_BG: Color = Color::Rgb { r: 248, g: 248, b: 242 };
pub const CURSOR_FG: Color = Color::Black;

// Status colors
pub const ERROR: Color = Color::Rgb { r: 255, g: 85, b: 85 };
pub const WARNING: Color = Color::Rgb { r: 255, g: 184, b: 108 };
pub const INFO: Color = Color::Rgb { r: 139, g: 233, b: 253 };
pub const SUCCESS: Color = Color::Rgb { r: 152, g: 195, b: 121 };

// Accent colors
pub const ACCENT_TEXT: Color = Color::Rgb { r: 176, g: 196, b: 222 };