// Terminal UI module

pub mod grid;
pub mod renderer;

// Re-export main types
pub use grid::{Grid, SelectionMode};
pub use renderer::{UIRenderer, Screen};