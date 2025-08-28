// Terminal UI module

pub mod grid;
pub mod renderer;
pub mod rope;
pub mod rope_grid;

// Re-export main types
pub use grid::{Grid, SelectionMode};
pub use renderer::{UIRenderer, Screen};