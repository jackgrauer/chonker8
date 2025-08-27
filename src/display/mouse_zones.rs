// Mouse zone detection and handling for PDF viewer
use crossterm::event::{MouseEvent, MouseEventKind};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MouseZone {
    PdfPanel,
    TextEditor,
    Divider,
    StatusBar,
    None,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PanelFocus {
    Pdf,
    Text,
}

pub struct MouseHandler {
    // Zone tracking
    pub current_zone: MouseZone,
    pub active_panel: PanelFocus,
    
    // Scrolling state
    pub pdf_scroll_x: f32,
    pub pdf_scroll_y: f32,
    pub text_scroll_x: usize,
    pub text_scroll_y: usize,
    
    // Zoom state
    pub pdf_zoom: f32,
    pub text_zoom: f32,
    
    // Pinch zoom detection
    last_touch_distance: Option<f32>,
    pinch_initial_zoom: f32,
    
    // Smooth scrolling
    scroll_momentum_x: f32,
    scroll_momentum_y: f32,
    
    // Panel dimensions
    split_x: u16,
    term_width: u16,
    term_height: u16,
}

impl MouseHandler {
    pub fn new() -> Self {
        Self {
            current_zone: MouseZone::None,
            active_panel: PanelFocus::Text,
            pdf_scroll_x: 0.0,
            pdf_scroll_y: 0.0,
            text_scroll_x: 0,
            text_scroll_y: 0,
            pdf_zoom: 1.0,
            text_zoom: 1.0,
            last_touch_distance: None,
            pinch_initial_zoom: 1.0,
            scroll_momentum_x: 0.0,
            scroll_momentum_y: 0.0,
            split_x: 40,
            term_width: 80,
            term_height: 24,
        }
    }
    
    /// Update terminal dimensions
    pub fn update_dimensions(&mut self, width: u16, height: u16) {
        self.term_width = width;
        self.term_height = height;
        self.split_x = width / 2;
    }
    
    /// Detect which zone the mouse is in
    pub fn detect_zone(&mut self, x: u16, y: u16) -> MouseZone {
        // Status bar (bottom row)
        if y >= self.term_height - 1 {
            self.current_zone = MouseZone::StatusBar;
            return MouseZone::StatusBar;
        }
        
        // Divider (split column ± 1)
        if x >= self.split_x.saturating_sub(1) && x <= self.split_x + 1 {
            self.current_zone = MouseZone::Divider;
            return MouseZone::Divider;
        }
        
        // PDF panel (left side)
        if x < self.split_x {
            self.current_zone = MouseZone::PdfPanel;
            self.active_panel = PanelFocus::Pdf;
            return MouseZone::PdfPanel;
        }
        
        // Text editor (right side)
        if x > self.split_x {
            self.current_zone = MouseZone::TextEditor;
            self.active_panel = PanelFocus::Text;
            return MouseZone::TextEditor;
        }
        
        self.current_zone = MouseZone::None;
        MouseZone::None
    }
    
    /// Handle scroll events with smooth scrolling
    pub fn handle_scroll(&mut self, event: &MouseEvent, horizontal: bool) -> (bool, ScrollAction) {
        let zone = self.detect_zone(event.column, event.row);
        
        match event.kind {
            MouseEventKind::ScrollUp => {
                match zone {
                    MouseZone::PdfPanel => {
                        if horizontal {
                            self.pdf_scroll_x -= 5.0;
                            self.scroll_momentum_x = -2.0;
                        } else {
                            self.pdf_scroll_y -= 5.0;
                            self.scroll_momentum_y = -2.0;
                        }
                        (true, ScrollAction::PdfScroll(self.pdf_scroll_x, self.pdf_scroll_y))
                    }
                    MouseZone::TextEditor => {
                        if horizontal {
                            self.text_scroll_x = self.text_scroll_x.saturating_sub(5);
                        } else {
                            self.text_scroll_y = self.text_scroll_y.saturating_sub(3);
                        }
                        (true, ScrollAction::TextScroll(self.text_scroll_x, self.text_scroll_y))
                    }
                    _ => (false, ScrollAction::None),
                }
            }
            MouseEventKind::ScrollDown => {
                match zone {
                    MouseZone::PdfPanel => {
                        if horizontal {
                            self.pdf_scroll_x += 5.0;
                            self.scroll_momentum_x = 2.0;
                        } else {
                            self.pdf_scroll_y += 5.0;
                            self.scroll_momentum_y = 2.0;
                        }
                        (true, ScrollAction::PdfScroll(self.pdf_scroll_x, self.pdf_scroll_y))
                    }
                    MouseZone::TextEditor => {
                        if horizontal {
                            self.text_scroll_x += 5;
                        } else {
                            self.text_scroll_y += 3;
                        }
                        (true, ScrollAction::TextScroll(self.text_scroll_x, self.text_scroll_y))
                    }
                    _ => (false, ScrollAction::None),
                }
            }
            MouseEventKind::ScrollLeft => {
                // Horizontal scroll left
                self.handle_scroll(&MouseEvent {
                    kind: MouseEventKind::ScrollUp,
                    column: event.column,
                    row: event.row,
                    modifiers: event.modifiers,
                }, true)
            }
            MouseEventKind::ScrollRight => {
                // Horizontal scroll right
                self.handle_scroll(&MouseEvent {
                    kind: MouseEventKind::ScrollDown,
                    column: event.column,
                    row: event.row,
                    modifiers: event.modifiers,
                }, true)
            }
            _ => (false, ScrollAction::None),
        }
    }
    
    /// Handle pinch zoom gesture (simulated via Ctrl+Scroll)
    pub fn handle_zoom(&mut self, event: &MouseEvent, zoom_in: bool) -> (bool, ZoomAction) {
        let zone = self.detect_zone(event.column, event.row);
        let zoom_delta = if zoom_in { 0.1 } else { -0.1 };
        
        match zone {
            MouseZone::PdfPanel => {
                self.pdf_zoom = (self.pdf_zoom + zoom_delta).clamp(0.5, 5.0);
                (true, ZoomAction::PdfZoom(self.pdf_zoom))
            }
            MouseZone::TextEditor => {
                self.text_zoom = (self.text_zoom + zoom_delta).clamp(0.5, 3.0);
                (true, ZoomAction::TextZoom(self.text_zoom))
            }
            _ => (false, ZoomAction::None),
        }
    }
    
    /// Apply momentum for smooth scrolling
    pub fn apply_momentum(&mut self) -> Option<ScrollAction> {
        let mut needs_update = false;
        
        // Apply and decay momentum
        if self.scroll_momentum_x.abs() > 0.1 {
            self.pdf_scroll_x += self.scroll_momentum_x;
            self.scroll_momentum_x *= 0.9; // Decay factor
            needs_update = true;
        } else {
            self.scroll_momentum_x = 0.0;
        }
        
        if self.scroll_momentum_y.abs() > 0.1 {
            self.pdf_scroll_y += self.scroll_momentum_y;
            self.scroll_momentum_y *= 0.9;
            needs_update = true;
        } else {
            self.scroll_momentum_y = 0.0;
        }
        
        if needs_update && self.current_zone == MouseZone::PdfPanel {
            Some(ScrollAction::PdfScroll(self.pdf_scroll_x, self.pdf_scroll_y))
        } else {
            None
        }
    }
    
    /// Get visual indicator for active panel
    pub fn get_panel_highlight_color(&self, panel: PanelFocus) -> crossterm::style::Color {
        use crossterm::style::Color;
        
        if self.active_panel == panel {
            // Active panel - bright blue border
            Color::Rgb { r: 100, g: 200, b: 255 }
        } else {
            // Inactive panel - dim gray border
            Color::Rgb { r: 60, g: 60, b: 80 }
        }
    }
    
    /// Get cursor style for current zone
    pub fn get_cursor_style(&self) -> &'static str {
        match self.current_zone {
            MouseZone::PdfPanel => "🔍", // Magnifying glass for PDF
            MouseZone::TextEditor => "✏️", // Pencil for text editor
            MouseZone::Divider => "↔️", // Resize cursor
            MouseZone::StatusBar => "ℹ️", // Info cursor
            MouseZone::None => "➤", // Default cursor
        }
    }
}

#[derive(Debug, Clone)]
pub enum ScrollAction {
    None,
    PdfScroll(f32, f32),  // x, y offsets
    TextScroll(usize, usize),  // column, row offsets
}

#[derive(Debug, Clone)]
pub enum ZoomAction {
    None,
    PdfZoom(f32),  // zoom level
    TextZoom(f32),  // zoom level
}