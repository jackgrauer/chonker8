// SQLite storage backend with DuckDB-compatible API
use anyhow::{Result, Context};
use rusqlite::{Connection, params};
use std::path::{Path, PathBuf};
use std::time::Instant;

pub struct DuckDBStorage {
    conn: Connection,
    disk_path: Option<PathBuf>,
    last_sync: Instant,
    dirty: bool,
    memory_threshold_mb: usize,
}

impl DuckDBStorage {
    pub fn new(path: Option<&Path>) -> Result<Self> {
        let conn = if let Some(p) = path {
            Connection::open(p)?
        } else {
            Connection::open_in_memory()?
        };
        
        let disk_path = path.map(|p| p.to_path_buf());
        
        Self::create_schema(&conn)?;
        
        Ok(Self {
            conn,
            disk_path,
            last_sync: Instant::now(),
            dirty: false,
            memory_threshold_mb: 500,
        })
    }
    
    fn create_schema(conn: &Connection) -> Result<()> {
        conn.execute_batch(r#"
            CREATE TABLE IF NOT EXISTS documents (
                id INTEGER PRIMARY KEY,
                path TEXT NOT NULL UNIQUE,
                filename TEXT NOT NULL,
                total_pages INTEGER NOT NULL,
                file_size INTEGER,
                created_at TEXT DEFAULT CURRENT_TIMESTAMP,
                last_accessed TEXT DEFAULT CURRENT_TIMESTAMP
            );
            
            CREATE TABLE IF NOT EXISTS pages (
                id INTEGER PRIMARY KEY,
                document_id INTEGER NOT NULL,
                page_num INTEGER NOT NULL,
                width INTEGER NOT NULL,
                height INTEGER NOT NULL,
                grid_data TEXT NOT NULL,
                char_count INTEGER NOT NULL,
                extracted_at TEXT DEFAULT CURRENT_TIMESTAMP,
                extraction_method TEXT DEFAULT 'basic',
                FOREIGN KEY (document_id) REFERENCES documents(id),
                UNIQUE(document_id, page_num)
            );
            
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
            
            CREATE INDEX IF NOT EXISTS idx_pages_doc_page 
                ON pages(document_id, page_num);
            CREATE INDEX IF NOT EXISTS idx_text_doc_page 
                ON text_content(document_id, page_num);
            CREATE INDEX IF NOT EXISTS idx_text_content 
                ON text_content(text_content);
        "#)?;
        Ok(())
    }
    
    fn check_memory_pressure(&mut self) -> Result<()> {
        // Simplified for SQLite
        Ok(())
    }
    
    fn get_available_memory_mb() -> usize {
        1000 // Simplified
    }
    
    pub fn sync_to_disk(&mut self) -> Result<()> {
        Ok(())
    }
    
    pub fn register_document(&mut self, path: &Path, total_pages: usize) -> Result<i64> {
        let path_str = path.to_string_lossy();
        let filename = path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown.pdf");
        
        let file_size = std::fs::metadata(path)
            .map(|m| m.len() as i64)
            .unwrap_or(0);
        
        self.conn.execute(
            r#"INSERT OR REPLACE INTO documents (path, filename, total_pages, file_size) 
               VALUES (?1, ?2, ?3, ?4)"#,
            params![path_str, filename, total_pages as i64, file_size],
        )?;
        
        let doc_id = self.conn.last_insert_rowid();
        
        self.dirty = true;
        self.check_memory_pressure()?;
        
        Ok(doc_id)
    }
    
    pub fn save_page(
        &mut self, 
        doc_id: i64, 
        page_num: usize, 
        grid: &[Vec<char>],
        width: usize,
        height: usize,
    ) -> Result<()> {
        let grid_json = serde_json::to_string(&grid)?;
        
        let char_count = grid.iter()
            .flat_map(|row| row.iter())
            .filter(|&&c| c != ' ')
            .count() as i64;
        
        self.conn.execute(
            r#"INSERT OR REPLACE INTO pages (document_id, page_num, width, height, grid_data, char_count)
               VALUES (?1, ?2, ?3, ?4, ?5, ?6)"#,
            params![doc_id, page_num as i64, width as i64, height as i64, grid_json, char_count],
        )?;
        
        self.save_text_content(doc_id, page_num, grid)?;
        
        self.dirty = true;
        self.check_memory_pressure()?;
        
        Ok(())
    }
    
    fn save_text_content(&mut self, doc_id: i64, page_num: usize, grid: &[Vec<char>]) -> Result<()> {
        self.conn.execute(
            "DELETE FROM text_content WHERE document_id = ?1 AND page_num = ?2",
            params![doc_id, page_num as i64],
        )?;
        
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
    
    pub fn load_page(&mut self, doc_id: i64, page_num: usize) -> Result<Vec<Vec<char>>> {
        let grid_json: String = self.conn.query_row(
            "SELECT grid_data FROM pages WHERE document_id = ?1 AND page_num = ?2",
            params![doc_id, page_num as i64],
            |row| row.get(0),
        ).context("Page not found in database")?;
        
        let grid: Vec<Vec<char>> = serde_json::from_str(&grid_json)?;
        Ok(grid)
    }
    
    pub fn search_text(&mut self, query: &str) -> Result<Vec<SearchResult>> {
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
    
    pub fn get_stats(&mut self) -> Result<StorageStats> {
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
    
    pub fn list_documents(&mut self) -> Result<Vec<DocumentInfo>> {
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
    
    pub fn query(&mut self, sql: &str) -> Result<Vec<Vec<String>>> {
        let mut stmt = self.conn.prepare(sql)?;
        let column_count = stmt.column_count();
        
        let rows = stmt.query_map([], |row| {
            let mut values = Vec::new();
            for i in 0..column_count {
                let value: String = row.get(i).unwrap_or_else(|_| "NULL".to_string());
                values.push(value);
            }
            Ok(values)
        })?;
        
        Ok(rows.collect::<std::result::Result<Vec<_>, _>>()?)
    }
    
    pub fn force_save(&mut self) -> Result<()> {
        Ok(())
    }
}

impl Drop for DuckDBStorage {
    fn drop(&mut self) {
        // SQLite auto-saves
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