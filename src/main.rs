use anyhow::Result;
use eframe::egui;
use std::path::PathBuf;
use chonker8::display::{file_browser::FileBrowser, text_editor::TextEditor};

#[derive(Debug, Clone, PartialEq)]
enum Mode { Browser, Editor }

struct App {
    mode: Mode,
    browser: FileBrowser,
    editor: Option<TextEditor>,
    file: Option<PathBuf>,
}

impl Default for App {
    fn default() -> Self {
        let mut app = Self {
            mode: Mode::Editor,  // Start in XML mode
            browser: FileBrowser::new().unwrap_or_else(|_| FileBrowser::new_empty()),
            editor: None,
            file: None,
        };
        
        // Auto-load the test PDF
        let test_pdf = std::path::PathBuf::from("/Users/jack/Documents/chonker_test.pdf");
        if test_pdf.exists() {
            if let Ok(editor) = TextEditor::new(test_pdf.clone()) {
                app.editor = Some(editor);
                app.file = Some(test_pdf);
            }
        }
        
        app
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.input(|i| {
            if i.key_pressed(egui::Key::Tab) && self.editor.is_some() {
                self.mode = match self.mode { Mode::Browser => Mode::Editor, Mode::Editor => Mode::Browser };
            }
            if i.key_pressed(egui::Key::U) && i.modifiers.ctrl {
                let _ = chonker8::core::hot_reload::HotReloadManager::trigger_reload();
            }
        });
        
        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(egui::Color32::from_rgb(12, 12, 12)))
            .show(ctx, |ui| {
                ui.spacing_mut().item_spacing = egui::Vec2::new(0.0, 2.0);
                ui.spacing_mut().button_padding = egui::Vec2::new(8.0, 4.0);
                match self.mode {
                    Mode::Browser => self.show_browser(ui),
                    Mode::Editor => self.show_editor(ui),
                }
            });
    }
}

impl App {
    fn show_browser(&mut self, ui: &mut egui::Ui) {
        ui.vertical(|ui| {
            // Simple search box - always focused
            let search_response = ui.add(egui::TextEdit::singleline(self.browser.get_query_mut())
                .font(egui::TextStyle::Monospace)
                .hint_text("Search...")
                .desired_width(f32::INFINITY));
            
            // Auto-focus the search box
            if !search_response.has_focus() && self.browser.get_query_mut().is_empty() {
                search_response.request_focus();
            }
            
            // File list
            let files = self.browser.get_visible_files();
            let mut selected_file = None;
            
            for (i, file) in files.iter().enumerate() {
                let is_selected = i == self.browser.get_selected_index();
                let file_color = match file {
                    f if f == "../" => egui::Color32::from_rgb(120, 120, 120),
                    f if f.ends_with('/') => egui::Color32::from_rgb(100, 200, 255),
                    f if f.to_lowercase().ends_with(".pdf") => egui::Color32::from_rgb(100, 255, 120),
                    _ => egui::Color32::from_rgb(220, 220, 225),
                };
                
                let (bg, fg) = if is_selected {
                    (egui::Color32::from_rgb(30, 90, 200), egui::Color32::WHITE)
                } else {
                    (egui::Color32::TRANSPARENT, file_color)
                };
                
                if ui.add(egui::Button::new(egui::RichText::new(file).color(fg).monospace().size(13.0))
                    .fill(bg).stroke(egui::Stroke::NONE)
                    .min_size(egui::Vec2::new(ui.available_width(), 20.0))).clicked() {
                    selected_file = Some((i, file.clone()));
                }
            }
            
            // Handle file selection
            if let Some((index, file)) = selected_file {
                self.browser.set_selected_index(index);
                if file.ends_with('/') {
                    let _ = self.browser.navigate_to(&file);
                } else if file.to_lowercase().ends_with(".pdf") {
                    self.open_pdf(&file);
                }
            }
        });
        
        // Keyboard navigation
        ui.input(|i| {
            if i.key_pressed(egui::Key::ArrowUp) { self.browser.move_selection_up(); }
            if i.key_pressed(egui::Key::ArrowDown) { self.browser.move_selection_down(); }
            if i.key_pressed(egui::Key::Enter) {
                if let Some(file) = self.browser.get_selected_file() {
                    if file.to_lowercase().ends_with(".pdf") {
                        self.open_pdf(&file);
                    }
                }
            }
        });
    }
    
    fn open_pdf(&mut self, file: &str) {
        let path = self.browser.get_current_dir().join(file);
        if let Ok(editor) = TextEditor::new(path.clone()) {
            self.editor = Some(editor);
            self.file = Some(path);
            self.mode = Mode::Editor;
        }
    }
    
    fn show_editor(&mut self, ui: &mut egui::Ui) {
        if let Some(editor) = &mut self.editor {
            // Mode toggle and controls
            ui.horizontal(|ui| {
                let (mode_text, tooltip) = if editor.is_html_rendering() { 
                    ("XML Layout", "Rich XML with exact positioning (pdftohtml -xml)")
                } else { 
                    ("Clean Text", "Plain text extraction (pdftotext -layout)")
                };
                
                if ui.button(format!("📄 {}", mode_text))
                    .on_hover_text(tooltip)
                    .clicked() {
                    editor.toggle_html_rendering();
                }
                
                if ui.button("⟳ Reload")
                    .on_hover_text("Re-extract from PDF")
                    .clicked() {
                    let _ = editor.reload_pdf_content();
                }
                
                ui.separator();
                
                // Page navigation
                if ui.button("◀ Prev").clicked() {
                    editor.prev_page();
                }
                ui.label(format!("Page {}/{}", editor.get_current_page() + 1, editor.get_total_pages()));
                if ui.button("Next ▶").clicked() {
                    editor.next_page();
                }
                
                ui.separator();
                ui.label(format!("Source: {}", 
                    self.file.as_ref().unwrap().file_name().unwrap().to_string_lossy()));
            });
            
            ui.separator();
            
            // Render content based on mode
            if editor.is_html_rendering() {
                // XML mode: Simple vertical scrolling
                egui::ScrollArea::vertical()
                    .show(ui, |ui| {
                        editor.render_html_content(ui);
                    });
            } else {
                // Text mode: Editable text with ropey
                let mut text = editor.get_text();
                let resp = ui.add_sized(ui.available_size(),
                    egui::TextEdit::multiline(&mut text)
                        .font(egui::TextStyle::Monospace)
                        .text_color(egui::Color32::from_rgb(100, 200, 255))
                        .code_editor());
                
                if resp.changed() { 
                    editor.set_text(text); 
                }
            }
            
            ui.input(|i| {
                // Controls for XML mode
                if editor.is_html_rendering() {
                    // Arrow key panning
                    if i.key_pressed(egui::Key::ArrowLeft) { editor.pan_left(); }
                    if i.key_pressed(egui::Key::ArrowRight) { editor.pan_right(); }
                    if i.key_pressed(egui::Key::ArrowUp) { editor.pan_up(); }
                    if i.key_pressed(egui::Key::ArrowDown) { editor.pan_down(); }
                    
                    // Page navigation
                    if i.key_pressed(egui::Key::PageDown) || i.key_pressed(egui::Key::N) {
                        editor.next_page();
                    }
                    if i.key_pressed(egui::Key::PageUp) || i.key_pressed(egui::Key::P) {
                        editor.prev_page();
                    }
                }
                
                if i.modifiers.ctrl {
                    match () {
                        _ if i.key_pressed(egui::Key::C) => editor.copy_selection(),
                        _ if i.key_pressed(egui::Key::X) => editor.cut_selection(),
                        _ if i.key_pressed(egui::Key::V) => editor.paste(),
                        _ if i.key_pressed(egui::Key::A) => editor.select_all(),
                        _ if i.key_pressed(egui::Key::B) => editor.toggle_block_selection(),
                        _ if i.key_pressed(egui::Key::R) => { let _ = editor.reload_pdf_content(); },
                        _ if i.key_pressed(egui::Key::H) => editor.toggle_html_rendering(),
                        _ => {}
                    }
                }
            });
        }
    }
}

fn main() -> Result<()> {
    // Suppress macOS task policy warnings
    #[cfg(target_os = "macos")]
    {
        std::env::set_var("OBJC_DISABLE_INITIALIZE_FORK_SAFETY", "YES");
    }
    
    // Get screen dimensions for positioning
    let screen_width = 1920.0; // Adjust to your actual screen width
    let screen_height = 1080.0; // Adjust to your actual screen height
    
    // Right half: exact center to right edge, full height
    let window_width = screen_width / 2.0;
    let window_height = screen_height; // Full screen height
    let x_pos = 960.0; // Explicit center position for 1920px screen
    let y_pos = 0.0; // Top of screen
    
    eframe::run_native(
        "Chonker8",
        eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_inner_size([window_width, window_height])
                .with_position([x_pos, y_pos])
                .with_resizable(true)
                .with_decorations(true)
                .with_title("Chonker8 - XML PDF Viewer"),
            ..Default::default()
        },
        Box::new(|cc| {
            setup_theme(cc);
            Ok(Box::new(App::default()))
        }),
    ).map_err(|e| anyhow::anyhow!("eframe error: {}", e))
}

fn setup_theme(cc: &eframe::CreationContext<'_>) {
    let mut visuals = egui::Visuals::dark();
    visuals.override_text_color = Some(egui::Color32::from_rgb(220, 220, 220));
    visuals.panel_fill = egui::Color32::from_rgb(12, 12, 12);
    visuals.window_fill = egui::Color32::from_rgb(12, 12, 12);
    visuals.window_shadow = egui::Shadow::NONE;
    visuals.popup_shadow = egui::Shadow::NONE;
    visuals.selection.bg_fill = egui::Color32::from_rgb(30, 90, 200);
    cc.egui_ctx.set_visuals(visuals);
    
    let mut fonts = egui::FontDefinitions::default();
    fonts.families.insert(egui::FontFamily::Proportional, 
        fonts.families[&egui::FontFamily::Monospace].clone());
    cc.egui_ctx.set_fonts(fonts);
}