// Screen mode management for Chonker8

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScreenMode {
    Editor,      // PDF view + Text editor  
    Debug,       // Full screen debug output
}

impl ScreenMode {
    pub fn next(self) -> Self {
        match self {
            ScreenMode::Editor => ScreenMode::Debug,
            ScreenMode::Debug => ScreenMode::Editor,
        }
    }
    
    pub fn prev(self) -> Self {
        match self {
            ScreenMode::Editor => ScreenMode::Debug,
            ScreenMode::Debug => ScreenMode::Editor,
        }
    }
}