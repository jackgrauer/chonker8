// Application state management for switching between browser and editor
use anyhow::Result;
use std::path::PathBuf;

use crate::display::file_browser::{FileBrowser, FileBrowserResult};
use crate::display::text_editor::TextEditor;

#[derive(Debug, Clone, PartialEq)]
pub enum AppMode {
    FileBrowser,
    TextEditor,
}

pub enum StateTransition {
    Continue,
    Exit,
    SwitchTo(AppMode),
    OpenFile(PathBuf),
    HotReload,
}

pub struct AppState {
    pub current_mode: AppMode,
    pub file_browser: FileBrowser,
    pub text_editor: Option<TextEditor>,
    pub current_file: Option<PathBuf>,
}

impl AppState {
    pub fn new() -> Result<Self> {
        Ok(Self {
            current_mode: AppMode::FileBrowser,
            file_browser: FileBrowser::new()?,
            text_editor: None,
            current_file: None,
        })
    }
    
    pub fn run(&mut self) -> Result<()> {
        use crossterm::{
            execute,
            terminal::{self, EnterAlternateScreen, LeaveAlternateScreen},
            cursor::{Hide, Show},
        };
        use std::io::stdout;
        
        // Enter alternate screen mode once
        execute!(stdout(), EnterAlternateScreen, Hide)?;
        terminal::enable_raw_mode()?;
        
        let result = self.run_loop();
        
        // Cleanup - exit alternate screen mode
        terminal::disable_raw_mode()?;
        execute!(stdout(), Show, LeaveAlternateScreen)?;
        
        result
    }
    
    fn run_loop(&mut self) -> Result<()> {
        loop {
            match self.current_mode {
                AppMode::FileBrowser => {
                    match self.run_file_browser()? {
                        StateTransition::Continue => continue,
                        StateTransition::Exit => break,
                        StateTransition::SwitchTo(AppMode::TextEditor) => {
                            self.current_mode = AppMode::TextEditor;
                        }
                        StateTransition::OpenFile(path) => {
                            self.open_file(path)?;
                            self.current_mode = AppMode::TextEditor;
                        }
                        StateTransition::HotReload => {
                            return self.handle_hot_reload();
                        }
                        _ => {}
                    }
                }
                AppMode::TextEditor => {
                    if self.text_editor.is_some() {
                        let transition = {
                            let editor = self.text_editor.as_mut().unwrap();
                            editor.run_once()?
                        };
                        match transition {
                            StateTransition::Continue => continue,
                            StateTransition::Exit => break,
                            StateTransition::SwitchTo(AppMode::FileBrowser) => {
                                self.current_mode = AppMode::FileBrowser;
                            }
                            StateTransition::HotReload => {
                                return self.handle_hot_reload();
                            }
                            _ => {}
                        }
                    } else {
                        // No editor available, go back to browser
                        self.current_mode = AppMode::FileBrowser;
                    }
                }
            }
        }
        
        Ok(())
    }
    
    fn run_file_browser(&mut self) -> Result<StateTransition> {
        loop {
            match self.file_browser.run()? {
                FileBrowserResult::FileSelected(selected_path) => {
                    if !selected_path.as_os_str().is_empty() && selected_path.extension().and_then(|s| s.to_str()) == Some("pdf") {
                        return Ok(StateTransition::OpenFile(selected_path));
                    }
                    // Continue loop for directory navigation or empty selection
                }
                FileBrowserResult::SwitchToEditor => {
                    // Switch to editor if we have one, otherwise stay in browser
                    if self.text_editor.is_some() {
                        return Ok(StateTransition::SwitchTo(AppMode::TextEditor));
                    }
                    // Continue loop if no editor
                }
                FileBrowserResult::HotReload => {
                    return Ok(StateTransition::HotReload);
                }
                FileBrowserResult::Exit => return Ok(StateTransition::Exit),
            }
        }
    }
    
    fn run_text_editor(&mut self, editor: &mut TextEditor) -> Result<StateTransition> {
        editor.run_once()
    }
    
    fn open_file(&mut self, path: PathBuf) -> Result<()> {
        self.current_file = Some(path.clone());
        self.text_editor = Some(TextEditor::new(path)?);
        Ok(())
    }
    
    fn handle_hot_reload(&self) -> Result<()> {
        use crossterm::{execute, terminal, cursor::Show};
        use std::io::stdout;
        
        // First, properly exit terminal mode
        terminal::disable_raw_mode()?;
        execute!(stdout(), Show, terminal::LeaveAlternateScreen)?;
        
        // Then trigger the hot reload
        use crate::core::hot_reload::HotReloadManager;
        HotReloadManager::trigger_reload()
    }
}