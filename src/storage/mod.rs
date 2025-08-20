// Storage layer module
pub mod sqlite_compat;

// Export as DuckDBStorage to maintain API compatibility
pub use sqlite_compat::{DuckDBStorage, SearchResult, StorageStats, DocumentInfo};