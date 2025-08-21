#!/bin/bash
cd /Users/jack/chonker8

echo "🧹 NUCLEAR CLEANUP: Removing goofy stuff..."

# Clean dead files
rm -f src/pdf_extraction/ferrules_extraction.rs.backup_temp
rm -f tests/ferrules_integration.rs tests/extractous_integration_test.rs  
rm -f FERRULES_*.md test_vision.swift smart_discovery.rs test_convergence.rs
rm -f test.db mydata.db models/ch_PP-OCRv4_det_infer.onnx models/det.onnx
rm -f src/storage/legacy_storage.rs src/storage/sqlite_*.rs
rm -f src/storage/duckdb_storage.rs.bak

# Remove ALL storage modules (DuckDB dependency hell)
rm -f src/storage/duckdb_storage.rs src/storage/sqlite_compat.rs src/storage/legacy_storage.rs

# Clean fake/dummy model files
find models/ -name "*.onnx" -size -100c -delete  # Delete files < 100 bytes (fake)
find models/ -name "*.txt" -size -10c -delete    # Delete tiny text files

# Fix test scripts - replace OAR-OCR with OAR-OCR
find . -name "*.sh" -exec sed -i '' 's/OAR-OCR/OAR-OCR/g' {} \; 2>/dev/null || true

# Clean build artifacts
cargo clean
rm -rf target/ Cargo.lock

echo "✅ Nuclear cleanup complete!"
echo "📊 Files remaining:"
find src/ -name "*.rs" | wc -l | xargs echo "  Rust files:"
find models/ -name "*" -type f | wc -l | xargs echo "  Model files:"
ls -la target/ 2>/dev/null | wc -l | xargs echo "  Target files:" || echo "  Target files: 0 (clean!)"