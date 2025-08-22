// Pdfium helper - chonker7 style (no singleton, create fresh instances)
use anyhow::Result;
use pdfium_render::prelude::*;

/// Create a new Pdfium instance for each operation (chonker7 style)
/// This avoids all borrowing/threading issues by not sharing instances
pub fn with_pdfium<F, R>(f: F) -> Result<R>
where
    F: FnOnce(&Pdfium) -> Result<R>,
{
    // Create a fresh instance for this operation
    let pdfium = Pdfium::new(
        Pdfium::bind_to_library(Pdfium::pdfium_platform_library_name_at_path("./lib"))
            .or_else(|_| Pdfium::bind_to_system_library())?
    );
    
    f(&pdfium)
}