// Chonker8 - egui-based PDF text editor
use anyhow::Result;
use eframe::egui;
use std::path::PathBuf;

use chonker8::display::file_browser::FileBrowser;
use chonker8::display::text_editor::TextEditor;

#[derive(Debug, Clone, PartialEq)]
enum AppMode {
    FileBrowser,
    TextEditor,
}

struct Chonker8App {
    mode: AppMode,
    file_browser: FileBrowser,
    text_editor: Option<TextEditor>,
    current_file: Option<PathBuf>,
}

impl Default for Chonker8App {
    fn default() -> Self {
        Self {
            mode: AppMode::FileBrowser,
            file_browser: FileBrowser::new().unwrap_or_else(|_| {
                // Fallback if file browser creation fails
                FileBrowser::new_empty()
            }),
            text_editor: None,
            current_file: None,
        }
    }
}

impl eframe::App for Chonker8App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Handle Tab key for mode switching
        if ctx.input(|i| i.key_pressed(egui::Key::Tab)) {
            match self.mode {
                AppMode::FileBrowser => {
                    if self.text_editor.is_some() {
                        self.mode = AppMode::TextEditor;
                    }
                }
                AppMode::TextEditor => {
                    self.mode = AppMode::FileBrowser;
                }
            }
        }
        
        // Handle Ctrl+U for hot reload
        if ctx.input(|i| i.key_pressed(egui::Key::U) && i.modifiers.ctrl) {
            self.trigger_hot_reload();
        }
        
        // Unified UI with consistent styling across both modes
        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(egui::Color32::from_rgb(12, 12, 12))) // Consistent background
            .show(ctx, |ui| {
                // Consistent spacing for both file browser and editor
                ui.spacing_mut().item_spacing = egui::Vec2::new(0.0, 2.0);
                ui.spacing_mut().button_padding = egui::Vec2::new(8.0, 4.0); // Comfortable padding
                ui.spacing_mut().menu_margin = egui::Margin::ZERO;
                ui.spacing_mut().indent = 0.0;
                
                
                match self.mode {
                    AppMode::FileBrowser => {
                        self.show_file_browser(ui);
                    }
                    AppMode::TextEditor => {
                        self.show_text_editor(ui);
                    }
                }
            });
    }
}

impl Chonker8App {
    fn show_file_browser(&mut self, ui: &mut egui::Ui) {
        // Beautiful terminal file browser
        ui.spacing_mut().item_spacing = egui::Vec2::new(0.0, 2.0); // Subtle spacing for readability
        
        ui.vertical(|ui| {
            // Enhanced search input with golden cursor
            ui.horizontal(|ui| {
                ui.add(egui::TextEdit::singleline(self.file_browser.get_query_mut())
                    .font(egui::TextStyle::Monospace)
                    .text_color(egui::Color32::from_rgb(240, 240, 245))
                    .desired_width(f32::INFINITY));
                ui.label(egui::RichText::new("▊").color(egui::Color32::from_rgb(255, 215, 0))); // Golden cursor
            });
            
            // File list - pure text like terminal
            let files = self.file_browser.get_visible_files();
            let mut selected_file = None;
            
            for (i, file) in files.iter().enumerate() {
                let is_selected = i == self.file_browser.get_selected_index();
                
                // Beautiful terminal color palette
                let color = if file.ends_with('/') {
                    if file == "../" {
                        egui::Color32::from_rgb(120, 120, 120) // Softer grey
                    } else {
                        egui::Color32::from_rgb(100, 200, 255) // Beautiful cyan
                    }
                } else if file.to_lowercase().ends_with(".pdf") {
                    egui::Color32::from_rgb(100, 255, 120) // Vibrant but pleasant green
                } else {
                    egui::Color32::from_rgb(220, 220, 225) // Warm white
                };
                
                // Sophisticated selection styling
                let (bg_color, text_color) = if is_selected {
                    (
                        egui::Color32::from_rgb(30, 90, 200), // Deep blue background
                        egui::Color32::from_rgb(255, 255, 255) // Pure white text
                    )
                } else {
                    (
                        egui::Color32::from_rgba_premultiplied(0, 0, 0, 0), // Transparent
                        color
                    )
                };
                
                let response = ui.add(
                    egui::Button::new(
                        egui::RichText::new(file)
                            .color(text_color)
                            .monospace()
                            .size(13.0) // Clean, readable size
                    )
                    .fill(bg_color)
                    .stroke(egui::Stroke::NONE)
                    .rounding(egui::Rounding::same(1.0)) // Minimal rounding
                    .min_size(egui::Vec2::new(ui.available_width(), 20.0))
                );
                
                if response.clicked() {
                    selected_file = Some((i, file.clone()));
                }
            }
            
            // Handle file selection
            if let Some((index, file)) = selected_file {
                self.file_browser.set_selected_index(index);
                
                if file.ends_with('/') {
                    let _ = self.file_browser.navigate_to(&file);
                } else if file.to_lowercase().ends_with(".pdf") {
                    let full_path = self.file_browser.get_current_dir().join(&file);
                    if let Ok(editor) = TextEditor::new(full_path.clone()) {
                        self.text_editor = Some(editor);
                        self.current_file = Some(full_path);
                        self.mode = AppMode::TextEditor;
                    }
                }
            }
        });
        
        // Handle keyboard navigation
        if ui.input(|i| i.key_pressed(egui::Key::ArrowUp)) {
            self.file_browser.move_selection_up();
        }
        if ui.input(|i| i.key_pressed(egui::Key::ArrowDown)) {
            self.file_browser.move_selection_down();
        }
        if ui.input(|i| i.key_pressed(egui::Key::Enter)) {
            let selected_file = self.file_browser.get_selected_file();
            if let Some(file) = selected_file {
                if file.to_lowercase().ends_with(".pdf") {
                    let full_path = self.file_browser.get_current_dir().join(&file);
                    if let Ok(editor) = TextEditor::new(full_path.clone()) {
                        self.text_editor = Some(editor);
                        self.current_file = Some(full_path);
                        self.mode = AppMode::TextEditor;
                    }
                }
            }
        }
    }
    
    fn show_text_editor(&mut self, ui: &mut egui::Ui) {
        if let Some(ref mut editor) = self.text_editor {
            // Coherent styling with file browser
            ui.spacing_mut().item_spacing = egui::Vec2::new(0.0, 2.0);
            
            
            // Text editor with beautiful cyan text matching file browser
            let mut text = editor.get_text();
            let response = ui.add_sized(
                ui.available_size(),
                egui::TextEdit::multiline(&mut text)
                    .font(egui::TextStyle::Monospace)
                    .text_color(egui::Color32::from_rgb(100, 200, 255)) // Beautiful cyan matching directories
                    .code_editor() // Enable code editor features for better text editing
            );
            
            // Update the rope if text changed
            if response.changed() {
                editor.set_text(text);
            }
            
            // Handle keyboard shortcuts
            if ui.input(|i| i.key_pressed(egui::Key::C) && i.modifiers.ctrl) {
                editor.copy_selection();
            }
            if ui.input(|i| i.key_pressed(egui::Key::X) && i.modifiers.ctrl) {
                editor.cut_selection();
            }
            if ui.input(|i| i.key_pressed(egui::Key::V) && i.modifiers.ctrl) {
                editor.paste();
            }
            if ui.input(|i| i.key_pressed(egui::Key::A) && i.modifiers.ctrl) {
                editor.select_all();
            }
            if ui.input(|i| i.key_pressed(egui::Key::B) && i.modifiers.ctrl) {
                editor.toggle_block_selection();
            }
            if ui.input(|i| i.key_pressed(egui::Key::R) && i.modifiers.ctrl) {
                let _ = editor.reload_pdf_content();
            }
        }
    }
    
    fn trigger_hot_reload(&self) {
        // Same hot reload logic as before
        use chonker8::core::hot_reload::HotReloadManager;
        let _ = HotReloadManager::trigger_reload();
    }
}

fn main() -> Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1200.0, 800.0])
            .with_title("Chonker8 - PDF Text Editor"),
        multisampling: 0,
        depth_buffer: 0,
        stencil_buffer: 0,
        ..Default::default()
    };
    
    let _ = eframe::run_native(
        "Chonker8",
        options,
        Box::new(|cc| {
            // Setup terminal-like theme and fonts
            setup_terminal_theme(cc);
            Ok(Box::new(Chonker8App::default()))
        }),
    );
    
    Ok(())
}

fn setup_terminal_theme(cc: &eframe::CreationContext<'_>) {
    // Beautiful dark terminal theme with subtle improvements
    let mut visuals = egui::Visuals::dark();
    
    // Enhanced terminal colors - softer than pure black/white
    visuals.override_text_color = Some(egui::Color32::from_rgb(220, 220, 220)); // Softer white
    visuals.panel_fill = egui::Color32::from_rgb(12, 12, 12); // Very dark grey instead of pure black
    visuals.window_fill = egui::Color32::from_rgb(12, 12, 12); // Subtle warmth
    visuals.extreme_bg_color = egui::Color32::from_rgb(8, 8, 8); // Deeper black for contrast
    
    // Remove borders and shadows for terminal look
    visuals.window_shadow = egui::Shadow::NONE;
    visuals.popup_shadow = egui::Shadow::NONE;
    visuals.window_stroke = egui::Stroke::NONE;
    
    // Beautiful selection colors - sophisticated terminal palette
    visuals.selection.bg_fill = egui::Color32::from_rgb(30, 90, 200); // Softer, deeper blue
    visuals.selection.stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(50, 120, 255)); // Subtle blue border
    
    // Apply theme
    cc.egui_ctx.set_visuals(visuals);
    
    // Use built-in monospace font for terminal look
    let mut fonts = egui::FontDefinitions::default();
    
    // Make everything use monospace for terminal feel
    fonts.families.insert(
        egui::FontFamily::Proportional,
        fonts.families[&egui::FontFamily::Monospace].clone(),
    );
    
    cc.egui_ctx.set_fonts(fonts);
}