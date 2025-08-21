use rexpect::spawn;
use std::fs;
use tempfile::TempDir;

#[tokio::test]
async fn test_duckdb_integration_with_rexpect() {
    // Create temporary directory for test database
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let db_path = temp_dir.path().join("test_chonker.db");
    
    // Test 1: DuckDB Storage Creation
    println!("🦆 Testing DuckDB storage creation...");
    
    let mut p = spawn(&format!(
        "DYLD_LIBRARY_PATH=./lib timeout 30 ./target/release/chonker8 stats --db {}",
        db_path.display()
    ), Some(5000)).expect("Failed to spawn process");
    
    // Should create database and show stats
    p.exp_string("Database Statistics").unwrap_or_else(|_| {
        panic!("DuckDB failed to create database or show stats");
    });
    
    p.send_control('c').expect("Failed to send Ctrl+C");
    
    // Test 2: PDF Storage with DuckDB
    println!("📄 Testing PDF storage in DuckDB...");
    
    let mut p = spawn(&format!(
        "DYLD_LIBRARY_PATH=./lib timeout 30 ./target/release/chonker8 extract \"/Users/jack/Documents/test.pdf\" --page 1 --store --db {}",
        db_path.display()
    ), Some(10000)).expect("Failed to spawn extraction process");
    
    // Should extract and store in DuckDB
    p.exp_string("Extracted Text").unwrap_or_else(|_| {
        panic!("Failed to extract PDF text for DuckDB storage");
    });
    
    p.send_control('c').expect("Failed to send Ctrl+C");
    
    // Test 3: Search in DuckDB
    println!("🔍 Testing DuckDB search functionality...");
    
    let mut p = spawn(&format!(
        "DYLD_LIBRARY_PATH=./lib timeout 30 ./target/release/chonker8 search \"Table\" --db {}",
        db_path.display()
    ), Some(5000)).expect("Failed to spawn search process");
    
    // Should find the stored document
    p.exp_string("Found").unwrap_or_else(|_| {
        panic!("DuckDB search failed to find stored documents");
    });
    
    p.send_control('c').expect("Failed to send Ctrl+C");
    
    // Test 4: Vector Storage Capabilities
    println!("🧮 Testing DuckDB vector storage...");
    
    let mut p = spawn(&format!(
        "DYLD_LIBRARY_PATH=./lib timeout 30 ./target/release/chonker8 stats --db {}",
        db_path.display()
    ), Some(5000)).expect("Failed to spawn stats process");
    
    // Should show stored documents with vector capabilities
    p.exp_string("Documents stored").unwrap_or_else(|_| {
        panic!("DuckDB failed to show document storage stats");
    });
    
    p.send_control('c').expect("Failed to send Ctrl+C");
    
    // Verify database file was created
    assert!(db_path.exists(), "DuckDB database file was not created");
    
    println!("✅ DuckDB integration tests passed!");
}

#[tokio::test] 
async fn test_duckdb_memory_mode() {
    println!("🧠 Testing DuckDB in-memory mode...");
    
    let mut p = spawn(
        "DYLD_LIBRARY_PATH=./lib timeout 15 ./target/release/chonker8 extract \"/Users/jack/Documents/chonker_test.pdf\" --page 1 --store",
        Some(8000)
    ).expect("Failed to spawn in-memory test");
    
    // Should work without database file
    p.exp_string("Extracted Text").unwrap_or_else(|_| {
        panic!("DuckDB in-memory mode failed");
    });
    
    p.send_control('c').expect("Failed to send Ctrl+C");
    
    println!("✅ DuckDB in-memory mode test passed!");
}

#[tokio::test]
async fn test_duckdb_concurrent_access() {
    println!("⚡ Testing DuckDB concurrent access...");
    
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let db_path = temp_dir.path().join("concurrent_test.db");
    
    // Spawn multiple concurrent operations
    let handles: Vec<_> = (0..3).map(|i| {
        let db_path = db_path.clone();
        tokio::spawn(async move {
            let mut p = spawn(&format!(
                "DYLD_LIBRARY_PATH=./lib timeout 20 ./target/release/chonker8 extract \"/Users/jack/Documents/test.pdf\" --page 1 --store --db {} --raw",
                db_path.display()
            ), Some(8000)).expect(&format!("Failed to spawn concurrent process {}", i));
            
            // Should handle concurrent access gracefully
            p.exp_string("Table").unwrap_or_else(|_| {
                panic!("Concurrent access {} failed", i);
            });
            
            p.send_control('c').expect("Failed to send Ctrl+C");
        })
    }).collect();
    
    // Wait for all concurrent operations
    for handle in handles {
        handle.await.expect("Concurrent task failed");
    }
    
    println!("✅ DuckDB concurrent access test passed!");
}