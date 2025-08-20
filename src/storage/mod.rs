// Storage layer module
pub mod sqlite_compat;
// Keep DuckDB ready for when build issues are resolved
// pub mod duckdb_storage;

// Export as DuckDBStorage to maintain API compatibility
pub use sqlite_compat::{DuckDBStorage, SearchResult, StorageStats, DocumentInfo};