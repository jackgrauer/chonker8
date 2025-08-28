// Grid editor for PDF text extraction
// Now uses RopeGrid (ropey) for all operations for better performance

use crossterm::event::KeyCode;
use super::rope_grid::{RopeGrid, DirtyRegion};

// Grid is now a thin wrapper around RopeGrid for backward compatibility
pub struct Grid {
    rope_grid: RopeGrid,
}

impl Clone for Grid {
    fn clone(&self) -> Self {
        // RopeGrid doesn't implement Clone, so recreate from text
        let text = self.rope_grid.to_string();
        let width = self.rope_grid.width();
        Self {
            rope_grid: RopeGrid::from_pdftext(&text, width),
        }
    }
}

impl std::fmt::Debug for Grid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Grid")
            .field("cursor", &self.rope_grid.cursor)
            .field("selecting", &self.rope_grid.selecting)
            .field("anchor", &self.rope_grid.anchor)
            .field("width", &self.rope_grid.width())
            .field("height", &self.rope_grid.height())
            .finish()
    }
}

impl Default for Grid {
    fn default() -> Self {
        Self::new(80, 24)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SelectionMode {
    Character,
    Word,
    Line,
    Block,
    All,
}

impl Grid {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            rope_grid: RopeGrid::new(width, height),
        }
    }
    
    pub fn from_pdftext(text: &str, min_width: usize) -> Self {
        Self {
            rope_grid: RopeGrid::from_pdftext(text, min_width),
        }
    }
    
    // Get a single character at position without generating entire grid
    pub fn get_char(&self, x: usize, y: usize) -> char {
        self.rope_grid.get_char(x, y)
    }
    
    pub fn cursor(&self) -> (usize, usize) { self.rope_grid.cursor }
    pub fn selecting(&self) -> bool { self.rope_grid.selecting }
    pub fn anchor(&self) -> (usize, usize) { self.rope_grid.anchor }
    pub fn width(&self) -> usize { self.rope_grid.width() }
    pub fn height(&self) -> usize { self.rope_grid.height() }
    pub fn search_query(&self) -> &str { &self.rope_grid.search_query }
    pub fn searching(&self) -> bool { self.rope_grid.searching }
    pub fn scroll_x(&self) -> usize { self.rope_grid.scroll_x }
    pub fn scroll_y(&self) -> usize { self.rope_grid.scroll_y }
    
    // Mutable accessors for renderer
    pub fn scroll_x_mut(&mut self) -> &mut usize { &mut self.rope_grid.scroll_x }
    pub fn scroll_y_mut(&mut self) -> &mut usize { &mut self.rope_grid.scroll_y }
    
    // Delegation methods
    pub fn set_viewport_size(&mut self, viewport_width: usize, viewport_height: usize) {
        self.rope_grid.set_viewport_size(viewport_width, viewport_height);
    }
    
    pub fn ensure_cursor_visible(&mut self, viewport_width: usize, viewport_height: usize) {
        self.rope_grid.ensure_cursor_visible(viewport_width, viewport_height);
    }
    
    pub fn handle_key_with_modifiers_with_viewport(&mut self, key: KeyCode, shift: bool, ctrl: bool, alt: bool, viewport_width: usize, viewport_height: usize) {
        self.rope_grid.handle_key_with_modifiers_with_viewport(key, shift, ctrl, alt, viewport_width, viewport_height);
    }
    
    pub fn handle_key_with_modifiers(&mut self, key: KeyCode, shift: bool, ctrl: bool, alt: bool) {
        self.rope_grid.handle_key_with_modifiers(key, shift, ctrl, alt);
    }
    
    pub fn handle_key(&mut self, key: KeyCode, shift_held: bool) {
        self.rope_grid.handle_key(key, shift_held);
    }
    
    pub fn clear_selection(&mut self) {
        self.rope_grid.clear_selection();
    }
    
    pub fn get_selection_bounds(&self) -> (usize, usize, usize, usize) {
        self.rope_grid.get_selection_bounds()
    }
    
    pub fn is_selected(&self, x: usize, y: usize) -> bool {
        self.rope_grid.is_selected(x, y)
    }
    
    pub fn to_string(&self) -> String {
        self.rope_grid.to_string()
    }
    
    pub fn copy_to_clipboard(&mut self) {
        self.rope_grid.copy_to_clipboard();
    }
    
    pub fn cut_to_clipboard(&mut self) {
        self.rope_grid.cut_to_clipboard();
    }
    
    pub fn paste_from_clipboard(&mut self) {
        self.rope_grid.paste_from_clipboard();
    }
    
    pub fn get_status_message(&self) -> Option<&str> {
        self.rope_grid.get_status_message()
    }
    
    pub fn clear_status_message(&mut self) {
        self.rope_grid.clear_status_message();
    }
    
    pub fn handle_mouse_down(&mut self, col: usize, row: usize) {
        self.rope_grid.handle_mouse_down(col, row);
    }
    
    pub fn handle_mouse_down_with_viewport(&mut self, col: usize, row: usize, viewport_width: usize, viewport_height: usize) {
        self.rope_grid.handle_mouse_down_with_viewport(col, row, viewport_width, viewport_height);
    }
    
    pub fn handle_mouse_drag(&mut self, col: usize, row: usize) {
        self.rope_grid.handle_mouse_drag(col, row);
    }
    
    pub fn handle_mouse_up(&mut self, col: usize, row: usize) {
        self.rope_grid.handle_mouse_up(col, row);
    }
    
    pub fn handle_mouse_up_with_viewport(&mut self, col: usize, row: usize, viewport_width: usize, viewport_height: usize) {
        self.rope_grid.handle_mouse_up_with_viewport(col, row, viewport_width, viewport_height);
    }
    
    pub fn start_search(&mut self) {
        self.rope_grid.start_search();
    }
    
    pub fn exit_search(&mut self) {
        self.rope_grid.exit_search();
    }
    
    pub fn perform_search(&mut self) {
        self.rope_grid.perform_search();
    }
    
    pub fn find_next(&mut self) {
        self.rope_grid.find_next();
    }
    
    pub fn find_previous(&mut self) {
        self.rope_grid.find_previous();
    }
    
    pub fn is_search_match(&self, col: usize, row: usize) -> bool {
        self.rope_grid.is_search_match(col, row)
    }
    
    pub fn insert_text(&mut self, text: &str) {
        self.rope_grid.insert_text(text);
    }
    
    pub fn copy_selection(&self) -> String {
        self.rope_grid.copy_selection()
    }
    
    pub fn paste_text(&mut self, text: &str) {
        self.rope_grid.paste_text(text);
    }
    pub fn get_dirty_regions(&self) -> &[DirtyRegion] {
        self.rope_grid.get_dirty_regions()
    }
    
    pub fn clear_dirty_regions(&mut self) {
        self.rope_grid.clear_dirty_regions();
    }
    
    pub fn mark_dirty(&mut self, x: usize, y: usize, width: usize, height: usize) {
        self.rope_grid.mark_dirty(x, y, width, height);
    }
    
    pub fn mark_all_dirty(&mut self) {
        self.rope_grid.mark_all_dirty();
    }
    
    pub fn revision(&self) -> u64 {
        self.rope_grid.revision()
    }
}