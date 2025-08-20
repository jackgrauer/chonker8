// SQLite storage backend with DuckDB-compatible API
use anyhow::{Result, Context};
use rusqlite::{Connection, params};
use std::path::{Path, PathBuf};
use std::time::Instant;

pub struct DuckDBStorage {
    memory_conn: Connection,
    disk_path: Option<PathBuf>,
    last_sync: Instant,
    dirty: bool,
    memory_threshold_mb: usize,
}

impl DuckDBStorage {
    pub fn new(path: Option<&Path>) -> Result<Self> {
        // SQLite in-memory with optional disk backup
        let memory_conn = if let Some(p) = path {
            Connection::open(p)?
        } else {
            Connection::open_in_memory()?
        };
        
        let disk_path = path.map(|p| p.to_path_buf());
        
        Self::create_schema(&memory_conn)?;
        
        Ok(Self {
            memory_conn,
            disk_path,
            last_sync: Instant::now(),
            dirty: false,
            memory_threshold_mb: 500,
        })
    }
    
    fn create_schema(conn: &Connection) -> Result<()> {
        conn.execute_batch(r#"
            -- PDF documents table
            CREATE TABLE IF NOT EXISTS documents (
                id INTEGER PRIMARY KEY,
                path TEXT NOT NULL UNIQUE,
                filename TEXT NOT NULL,
                total_pages INTEGER NOT NULL,
                file_size INTEGER,
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                last_accessed TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            );
            
            -- Extracted pages table (stores the character grid)
            CREATE TABLE IF NOT EXISTS pages (
                id INTEGER PRIMARY KEY,
                document_id INTEGER NOT NULL,
                page_num INTEGER NOT NULL,
                width INTEGER NOT NULL,
                height INTEGER NOT NULL,
                grid_data TEXT NOT NULL,  -- JSON array of arrays
                char_count INTEGER NOT NULL,
                extracted_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                extraction_method TEXT DEFAULT 'basic',
                FOREIGN KEY (document_id) REFERENCES documents(id),
                UNIQUE(document_id, page_num)
            );
            
            -- Text content table (for full-text search)
            CREATE TABLE IF NOT EXISTS text_content (
                id INTEGER PRIMARY KEY,
                document_id INTEGER NOT NULL,
                page_num INTEGER NOT NULL,
                text_content TEXT NOT NULL,
                line_num INTEGER,
                x_pos INTEGER,
                y_pos INTEGER,
                FOREIGN KEY (document_id) REFERENCES documents(id)
            );
            
            -- Create indexes for performance
            CREATE INDEX IF NOT EXISTS idx_pages_doc_page 
                ON pages(document_id, page_num);
            CREATE INDEX IF NOT EXISTS idx_text_doc_page 
                ON text_content(document_id, page_num);
            CREATE INDEX IF NOT EXISTS idx_text_content 
                ON text_content(text_content);
        "#)?;
        
        Ok(Self { conn })
    }
    
    /// Register a PDF document
    pub fn register_document(&self, path: &Path, total_pages: usize) -> Result<i64> {
        let path_str = path.to_string_lossy();
        let filename = path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown.pdf");
        
        // Get file size
        let file_size = std::fs::metadata(path)
            .map(|m| m.len() as i64)
            .unwrap_or(0);
        
        // Insert or update document
        self.conn.execute(
            r#"INSERT INTO documents (path, filename, total_pages, file_size) 
               VALUES (?1, ?2, ?3, ?4)
               ON CONFLICT(path) DO UPDATE SET 
                   last_accessed = CURRENT_TIMESTAMP,
                   total_pages = excluded.total_pages"#,
            params![path_str, filename, total_pages as i64, file_size],
        )?;
        
        // Get document ID
        let doc_id: i64 = self.conn.query_row(
            "SELECT id FROM documents WHERE path = ?1",
            params![path_str],
            |row| row.get(0),
        )?;
        
        Ok(doc_id)
    }
    
    /// Save extracted page grid
    pub fn save_page(
        &self, 
        doc_id: i64, 
        page_num: usize, 
        grid: &[Vec<char>],
        width: usize,
        height: usize,
    ) -> Result<()> {
        // Convert grid to JSON
        let grid_json = serde_json::to_string(&grid)?;
        
        // Count non-space characters
        let char_count = grid.iter()
            .flat_map(|row| row.iter())
            .filter(|&&c| c != ' ')
            .count() as i64;
        
        // Insert page data
        self.conn.execute(
            r#"INSERT INTO pages (document_id, page_num, width, height, grid_data, char_count)
               VALUES (?1, ?2, ?3, ?4, ?5, ?6)
               ON CONFLICT(document_id, page_num) DO UPDATE SET
                   grid_data = excluded.grid_data,
                   char_count = excluded.char_count,
                   extracted_at = CURRENT_TIMESTAMP"#,
            params![doc_id, page_num as i64, width as i64, height as i64, grid_json, char_count],
        )?;
        
        // Also save text lines for searching
        self.save_text_content(doc_id, page_num, grid)?;
        
        Ok(())
    }
    
    /// Save text content for full-text search
    fn save_text_content(&self, doc_id: i64, page_num: usize, grid: &[Vec<char>]) -> Result<()> {
        // Clear existing text for this page
        self.conn.execute(
            "DELETE FROM text_content WHERE document_id = ?1 AND page_num = ?2",
            params![doc_id, page_num as i64],
        )?;
        
        // Extract text lines
        for (y, row) in grid.iter().enumerate() {
            let line: String = row.iter().collect();
            let trimmed = line.trim();
            
            if !trimmed.is_empty() {
                self.conn.execute(
                    r#"INSERT INTO text_content (document_id, page_num, text_content, line_num, y_pos)
                       VALUES (?1, ?2, ?3, ?4, ?5)"#,
                    params![doc_id, page_num as i64, trimmed, y as i64, y as i64],
                )?;
            }
        }
        
        Ok(())
    }
    
    /// Load page grid
    pub fn load_page(&self, doc_id: i64, page_num: usize) -> Result<Vec<Vec<char>>> {
        let grid_json: String = self.conn.query_row(
            "SELECT grid_data FROM pages WHERE document_id = ?1 AND page_num = ?2",
            params![doc_id, page_num as i64],
            |row| row.get(0),
        ).context("Page not found in database")?;
        
        let grid: Vec<Vec<char>> = serde_json::from_str(&grid_json)?;
        Ok(grid)
    }
    
    /// Search for text across all documents
    pub fn search_text(&self, query: &str) -> Result<Vec<SearchResult>> {
        let pattern = format!("%{}%", query);
        let mut stmt = self.conn.prepare(
            r#"SELECT d.filename, d.path, t.page_num, t.text_content, t.line_num
               FROM text_content t
               JOIN documents d ON t.document_id = d.id
               WHERE t.text_content LIKE ?1
               ORDER BY d.filename, t.page_num, t.line_num
               LIMIT 100"#,
        )?;
        
        let results = stmt.query_map(params![pattern], |row| {
            Ok(SearchResult {
                filename: row.get(0)?,
                path: row.get(1)?,
                page_num: row.get(2)?,
                text: row.get(3)?,
                line_num: row.get(4)?,
            })
        })?;
        
        Ok(results.collect::<std::result::Result<Vec<_>, _>>()?)
    }
    
    /// Get statistics
    pub fn get_stats(&self) -> Result<StorageStats> {
        let doc_count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM documents",
            [],
            |row| row.get(0),
        )?;
        
        let page_count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM pages",
            [],
            |row| row.get(0),
        )?;
        
        let total_chars: i64 = self.conn.query_row(
            "SELECT COALESCE(SUM(char_count), 0) FROM pages",
            [],
            |row| row.get(0),
        )?;
        
        let total_size: i64 = self.conn.query_row(
            "SELECT COALESCE(SUM(file_size), 0) FROM documents",
            [],
            |row| row.get(0),
        )?;
        
        Ok(StorageStats {
            document_count: doc_count as usize,
            page_count: page_count as usize,
            total_characters: total_chars as usize,
            total_file_size: total_size as usize,
        })
    }
    
    /// List all documents
    pub fn list_documents(&self) -> Result<Vec<DocumentInfo>> {
        let mut stmt = self.conn.prepare(
            r#"SELECT id, filename, path, total_pages, file_size, created_at
               FROM documents
               ORDER BY last_accessed DESC"#,
        )?;
        
        let docs = stmt.query_map([], |row| {
            Ok(DocumentInfo {
                id: row.get(0)?,
                filename: row.get(1)?,
                path: row.get(2)?,
                total_pages: row.get(3)?,
                file_size: row.get(4)?,
                created_at: row.get(5)?,
            })
        })?;
        
        Ok(docs.collect::<std::result::Result<Vec<_>, _>>()?)
    }
    
    /// Execute custom SQL query
    pub fn query(&self, sql: &str) -> Result<Vec<Vec<String>>> {
        let mut stmt = self.conn.prepare(sql)?;
        let column_count = stmt.column_count();
        
        let rows = stmt.query_map([], |row| {
            let mut values = Vec::new();
            for i in 0..column_count {
                let value: rusqlite::Result<String> = row.get(i);
                values.push(value.unwrap_or_else(|_| "NULL".to_string()));
            }
            Ok(values)
        })?;
        
        Ok(rows.collect::<std::result::Result<Vec<_>, _>>()?)
    }
}

#[derive(Debug)]
pub struct SearchResult {
    pub filename: String,
    pub path: String,
    pub page_num: i64,
    pub text: String,
    pub line_num: i64,
}

#[derive(Debug)]
pub struct StorageStats {
    pub document_count: usize,
    pub page_count: usize,
    pub total_characters: usize,
    pub total_file_size: usize,
}

#[derive(Debug)]
pub struct DocumentInfo {
    pub id: i64,
    pub filename: String,
    pub path: String,
    pub total_pages: i64,
    pub file_size: i64,
    pub created_at: String,
}