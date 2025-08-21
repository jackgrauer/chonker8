// Storage layer - SQLite implementation
use anyhow::Result;
use rusqlite::{params, Connection};
use std::path::Path;

#[derive(Debug)]
pub struct DuckDBStorage {
    conn: Connection,
}

#[derive(Debug)]
pub struct SearchResult {
    pub score: f64,
    pub content: String,
    pub path: String,
}

impl DuckDBStorage {
    pub fn new(path: Option<&Path>) -> Result<Self> {
        let conn = match path {
            Some(p) => Connection::open(p)?,
            None => Connection::open_in_memory()?,
        };
        
        // Create tables
        conn.execute(
            "CREATE TABLE IF NOT EXISTS documents (
                id INTEGER PRIMARY KEY,
                path TEXT UNIQUE NOT NULL,
                content TEXT NOT NULL,
                metadata TEXT,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )",
            [],
        )?;
        
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_documents_path ON documents(path)",
            [],
        )?;
        
        Ok(DuckDBStorage { conn })
    }
    
    pub fn store_document(&mut self, path: &str, content: &str, metadata: Option<&str>) -> Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO documents (path, content, metadata) VALUES (?1, ?2, ?3)",
            params![path, content, metadata],
        )?;
        Ok(())
    }
    
    pub fn search(&self, query: &str, limit: Option<usize>) -> Result<Vec<SearchResult>> {
        let limit = limit.unwrap_or(10);
        
        // Simple LIKE search for now
        let mut stmt = self.conn.prepare(
            "SELECT path, content, 
             LENGTH(content) - LENGTH(REPLACE(LOWER(content), LOWER(?1), '')) AS score
             FROM documents 
             WHERE content LIKE '%' || ?1 || '%'
             ORDER BY score DESC
             LIMIT ?2"
        )?;
        
        let results = stmt.query_map(params![query, limit], |row| {
            Ok(SearchResult {
                path: row.get(0)?,
                content: row.get(1)?,
                score: row.get::<_, i64>(2)? as f64,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
        
        Ok(results)
    }
    
    pub fn get_stats(&self) -> Result<String> {
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM documents",
            [],
            |row| row.get(0),
        )?;
        
        let total_size: Option<i64> = self.conn.query_row(
            "SELECT SUM(LENGTH(content)) FROM documents",
            [],
            |row| row.get(0),
        ).unwrap_or(None);
        
        Ok(format!(
            "Documents: {}\nTotal size: {} bytes",
            count,
            total_size.unwrap_or(0)
        ))
    }
}