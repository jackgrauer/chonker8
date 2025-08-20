// DuckDB storage backend with advanced capabilities
use anyhow::{Result, Context};
use duckdb::{Connection, params, ToSql};
use std::path::{Path, PathBuf};
use std::time::Instant;
use serde_json;

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
        // DuckDB schema with advanced features
        conn.execute_batch(r#"
            CREATE TABLE IF NOT EXISTS documents (
                id INTEGER PRIMARY KEY,
                path VARCHAR NOT NULL UNIQUE,
                filename VARCHAR NOT NULL,
                total_pages INTEGER NOT NULL,
                file_size BIGINT,
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                last_accessed TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                extraction_metadata JSON
            );
            
            CREATE TABLE IF NOT EXISTS pages (
                id INTEGER PRIMARY KEY,
                document_id INTEGER NOT NULL,
                page_num INTEGER NOT NULL,
                width INTEGER NOT NULL,
                height INTEGER NOT NULL,
                grid_data JSON NOT NULL,
                char_count INTEGER NOT NULL,
                extracted_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                extraction_method VARCHAR DEFAULT 'basic',
                extraction_quality DOUBLE,
                FOREIGN KEY (document_id) REFERENCES documents(id),
                UNIQUE(document_id, page_num)
            );
            
            CREATE TABLE IF NOT EXISTS text_content (
                id INTEGER PRIMARY KEY,
                document_id INTEGER NOT NULL,
                page_num INTEGER NOT NULL,
                text_content VARCHAR NOT NULL,
                line_num INTEGER,
                x_pos INTEGER,
                y_pos INTEGER,
                confidence DOUBLE,
                FOREIGN KEY (document_id) REFERENCES documents(id)
            );
            
            -- DuckDB supports better indexing
            CREATE INDEX IF NOT EXISTS idx_pages_doc_page 
                ON pages(document_id, page_num);
            CREATE INDEX IF NOT EXISTS idx_text_doc_page 
                ON text_content(document_id, page_num);
            CREATE INDEX IF NOT EXISTS idx_text_content 
                ON text_content USING ART(text_content);
            
            -- Create a view for easy searching
            CREATE OR REPLACE VIEW search_view AS
            SELECT 
                d.filename,
                d.path,
                t.page_num,
                t.text_content,
                t.line_num,
                t.confidence
            FROM text_content t
            JOIN documents d ON t.document_id = d.id;
        "#)?;
        Ok(())
    }
    
    fn check_memory_pressure(&mut self) -> Result<()> {
        // DuckDB handles memory management internally
        if self.dirty && self.last_sync.elapsed().as_secs() > 30 {
            self.sync_to_disk()?;
        }
        Ok(())
    }
    
    pub fn sync_to_disk(&mut self) -> Result<()> {
        if let Some(_path) = &self.disk_path {
            // DuckDB automatically persists to disk
            self.conn.execute("CHECKPOINT", [])?;
            self.last_sync = Instant::now();
            self.dirty = false;
        }
        Ok(())
    }
    
    pub fn register_document(&mut self, path: &Path, total_pages: usize) -> Result<i64> {
        let path_str = path.to_string_lossy().to_string();
        let filename = path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown.pdf")
            .to_string();
        
        let file_size = std::fs::metadata(path)
            .map(|m| m.len() as i64)
            .unwrap_or(0);
        
        // Use RETURNING to get the inserted ID
        let doc_id: i64 = self.conn.query_row(
            r#"INSERT INTO documents (path, filename, total_pages, file_size) 
               VALUES (?, ?, ?, ?)
               ON CONFLICT (path) DO UPDATE SET
                   total_pages = EXCLUDED.total_pages,
                   last_accessed = CURRENT_TIMESTAMP
               RETURNING id"#,
            params![path_str, filename, total_pages as i64, file_size],
            |row| row.get(0),
        )?;
        
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
        // Convert grid to JSON
        let grid_json = serde_json::to_string(&grid)?;
        
        let char_count = grid.iter()
            .flat_map(|row| row.iter())
            .filter(|&&c| c != ' ')
            .count() as i64;
        
        self.conn.execute(
            r#"INSERT INTO pages (document_id, page_num, width, height, grid_data, char_count)
               VALUES (?, ?, ?, ?, ?::JSON, ?)
               ON CONFLICT (document_id, page_num) DO UPDATE SET
                   width = EXCLUDED.width,
                   height = EXCLUDED.height,
                   grid_data = EXCLUDED.grid_data,
                   char_count = EXCLUDED.char_count,
                   extracted_at = CURRENT_TIMESTAMP"#,
            params![doc_id, page_num as i64, width as i64, height as i64, grid_json, char_count],
        )?;
        
        self.save_text_content(doc_id, page_num, grid)?;
        
        self.dirty = true;
        self.check_memory_pressure()?;
        
        Ok(())
    }
    
    fn save_text_content(&mut self, doc_id: i64, page_num: usize, grid: &[Vec<char>]) -> Result<()> {
        // Clear existing content
        self.conn.execute(
            "DELETE FROM text_content WHERE document_id = ? AND page_num = ?",
            params![doc_id, page_num as i64],
        )?;
        
        // Batch insert for better performance
        let mut stmt = self.conn.prepare(
            r#"INSERT INTO text_content (document_id, page_num, text_content, line_num, y_pos)
               VALUES (?, ?, ?, ?, ?)"#
        )?;
        
        for (y, row) in grid.iter().enumerate() {
            let line: String = row.iter().collect();
            let trimmed = line.trim();
            
            if !trimmed.is_empty() {
                stmt.execute(params![doc_id, page_num as i64, trimmed, y as i64, y as i64])?;
            }
        }
        
        Ok(())
    }
    
    pub fn load_page(&mut self, doc_id: i64, page_num: usize) -> Result<Vec<Vec<char>>> {
        let grid_json: String = self.conn.query_row(
            "SELECT grid_data::VARCHAR FROM pages WHERE document_id = ? AND page_num = ?",
            params![doc_id, page_num as i64],
            |row| row.get(0),
        ).context("Page not found in database")?;
        
        let grid: Vec<Vec<char>> = serde_json::from_str(&grid_json)?;
        Ok(grid)
    }
    
    pub fn search_text(&mut self, query: &str) -> Result<Vec<SearchResult>> {
        // Use DuckDB's better text search
        let pattern = format!("%{}%", query);
        let mut stmt = self.conn.prepare(
            r#"SELECT filename, path, page_num, text_content, line_num
               FROM search_view
               WHERE text_content ILIKE ?
               ORDER BY filename, page_num, line_num
               LIMIT 100"#,
        )?;
        
        let results = stmt.query_map([pattern], |row| {
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
        // Use DuckDB's aggregation capabilities
        let (doc_count, page_count, total_chars, total_size): (i64, i64, i64, i64) = 
            self.conn.query_row(
                r#"SELECT 
                    COUNT(DISTINCT d.id),
                    COUNT(DISTINCT p.id),
                    COALESCE(SUM(p.char_count), 0),
                    COALESCE(SUM(d.file_size), 0)
                FROM documents d
                LEFT JOIN pages p ON d.id = p.document_id"#,
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
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
            r#"SELECT id, filename, path, total_pages, file_size, created_at::VARCHAR
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
        self.sync_to_disk()
    }
}

impl Drop for DuckDBStorage {
    fn drop(&mut self) {
        let _ = self.sync_to_disk();
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