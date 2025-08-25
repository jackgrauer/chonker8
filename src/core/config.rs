// Hot-reloadable UI configuration structures
use serde::{Deserialize, Serialize};
use std::fs;
use anyhow::Result;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct UIConfig {
    pub mode: String,
    pub layout: LayoutConfig,
    pub theme: ThemeConfig,
    pub panels: PanelsConfig,
    #[serde(default)]
    pub hotkeys: HotkeyConfig,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LayoutConfig {
    pub left_panel: String,
    pub right_panel: String,
    pub status_bar: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ThemeConfig {
    pub border: String,
    pub highlight: String,
    pub background: String,
    pub text_color: String,
    #[serde(default = "default_true")]
    pub clear_on_resize: bool,
}

fn default_true() -> bool { true }

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PanelsConfig {
    pub pdf: PdfPanelConfig,
    pub text: TextPanelConfig,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PdfPanelConfig {
    pub width_percent: f32,
    pub show_page_num: bool,
    pub show_scroll_bar: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TextPanelConfig {
    pub width_percent: f32,
    pub show_cursor: bool,
    pub wrap_text: bool,
    pub line_numbers: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct HotkeyConfig {
    #[serde(default = "default_quit")]
    pub quit: String,
    #[serde(default = "default_next_page")]
    pub next_page: String,
    #[serde(default = "default_prev_page")]
    pub prev_page: String,
    #[serde(default = "default_toggle_wrap")]
    pub toggle_wrap: String,
    #[serde(default = "default_toggle_mode")]
    pub toggle_mode: String,
    #[serde(default = "default_reload_config")]
    pub reload_config: String,
}

impl Default for HotkeyConfig {
    fn default() -> Self {
        Self {
            quit: default_quit(),
            next_page: default_next_page(),
            prev_page: default_prev_page(),
            toggle_wrap: default_toggle_wrap(),
            toggle_mode: default_toggle_mode(),
            reload_config: default_reload_config(),
        }
    }
}

fn default_quit() -> String { "q".to_string() }
fn default_next_page() -> String { "n".to_string() }
fn default_prev_page() -> String { "p".to_string() }
fn default_toggle_wrap() -> String { "w".to_string() }
fn default_toggle_mode() -> String { "m".to_string() }
fn default_reload_config() -> String { "r".to_string() }

impl UIConfig {
    pub fn load() -> Result<Self> {
        let content = fs::read_to_string("ui.toml")?;
        Ok(toml::from_str(&content)?)
    }
    
    pub fn save(&self) -> Result<()> {
        let content = toml::to_string_pretty(self)?;
        fs::write("ui.toml", content)?;
        Ok(())
    }
    
    pub fn get_border_chars(&self) -> (char, char, char, char, char, char, char, char) {
        match self.theme.border.as_str() {
            "rounded" => ('╭', '╮', '╰', '╯', '─', '│', '├', '┤'),
            "sharp" => ('┌', '┐', '└', '┘', '─', '│', '├', '┤'),
            _ => (' ', ' ', ' ', ' ', ' ', ' ', ' ', ' '),
        }
    }
    
    pub fn get_highlight_color(&self) -> crossterm::style::Color {
        use crossterm::style::Color;
        match self.theme.highlight.as_str() {
            "yellow" => Color::Yellow,
            "green" => Color::Green,
            "blue" => Color::Blue,
            "red" => Color::Red,
            "magenta" => Color::Magenta,
            "cyan" => Color::Cyan,
            _ => Color::White,
        }
    }
    
    pub fn get_text_color(&self) -> crossterm::style::Color {
        use crossterm::style::Color;
        match self.theme.text_color.as_str() {
            "white" => Color::White,
            "grey" | "gray" => Color::Grey,
            "black" => Color::Black,
            _ => Color::White,
        }
    }
}