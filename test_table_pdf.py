#!/usr/bin/env python3
import subprocess
import sys

# Common PDFs that might have tables
test_pdfs = [
    "/Users/jack/Desktop/BERF-CERT.pdf",
    "/Users/jack/Desktop/17.pdf",
    "/Users/jack/Desktop/Testing_the_waters_for_floating_class_7.5M___Philadelphia_Daily_News_PA___February_17_2025__pX10.pdf",
]

print("Looking for PDFs with tables...\n")

for pdf in test_pdfs:
    print(f"Testing: {pdf}")
    result = subprocess.run(
        ["./target/release/chonker8-hot", pdf],
        env={"DYLD_LIBRARY_PATH": "./lib"},
        capture_output=True,
        text=True,
        timeout=5
    )
    
    # Check if LayoutAnalysis was triggered
    if "LayoutAnalysis" in result.stderr or "has_tables=true" in result.stderr:
        print(f"  ✓ Tables detected! Method: LayoutAnalysis")
        for line in result.stderr.split('\n'):
            if 'has_tables' in line or 'LayoutAnalysis' in line or 'LayoutLM' in line:
                print(f"    {line}")
    else:
        # Show what was detected
        for line in result.stderr.split('\n'):
            if 'has_tables=' in line:
                print(f"    {line}")
                break
    print()