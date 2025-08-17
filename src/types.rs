// Core types and constants for Chonker8

// Grid dimensions
pub const GRID_WIDTH: usize = 200;
pub const GRID_HEIGHT: usize = 100;

// Position type for cursor and selections
#[derive(Debug, Copy, Clone, Default, PartialEq, Eq)]
pub struct Pos {
    pub x: usize,
    pub y: usize,
}

impl Pos {
    pub const fn new(x: usize, y: usize) -> Self {
        Self { x, y }
    }
}

impl From<(usize, usize)> for Pos {
    fn from((x, y): (usize, usize)) -> Self {
        Self { x, y }
    }
}

impl From<Pos> for (usize, usize) {
    fn from(pos: Pos) -> (usize, usize) {
        (pos.x, pos.y)
    }
}

// Selection state
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum Selection {
    #[default]
    None,
    Active { start: Pos, end: Pos },
}

impl Selection {
    pub fn is_active(&self) -> bool {
        matches!(self, Selection::Active { .. })
    }
    
    pub fn clear(&mut self) {
        *self = Selection::None;
    }
    
    pub fn start(&mut self, pos: Pos) {
        *self = Selection::Active { start: pos, end: pos };
    }
    
    pub fn update_end(&mut self, pos: Pos) {
        if let Selection::Active { start, .. } = self {
            *self = Selection::Active { start: *start, end: pos };
        }
    }
    
    pub fn get_bounds(&self) -> Option<(Pos, Pos)> {
        match self {
            Selection::Active { start, end } => {
                // Normalize so start is always before end
                if start.y < end.y || (start.y == end.y && start.x < end.x) {
                    Some((*start, *end))
                } else {
                    Some((*end, *start))
                }
            }
            Selection::None => None,
        }
    }
}

// App state flags using bitflags
bitflags::bitflags! {
    #[derive(Debug)]
    pub struct AppFlags: u8 {
        const DARK_MODE = 0b0001;
        const EXIT      = 0b0010;
        const REDRAW    = 0b0100;
        const SELECTING = 0b1000;
    }
}

// Error types
#[derive(Debug, thiserror::Error)]
pub enum ChonkerError {
    #[error("PDF error: {0}")]
    Pdf(String),
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Terminal error: {0}")]
    Terminal(String),
    
    #[error("Clipboard error: {0}")]
    Clipboard(String),
    
    #[error("File picker cancelled")]
    FilePickerCancelled,
}

pub type Result<T> = std::result::Result<T, ChonkerError>;