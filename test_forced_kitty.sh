#!/bin/bash

echo "Testing chonker8-hot with FORCED Kitty display..."
echo "================================================="

# Run and capture what happens
timeout 3 ./target/release/chonker8-hot /Users/jack/Desktop/BERF-CERT.pdf 2>&1 | tee forced_kitty.log | grep -E "FORCE|ACTIVE|display_image|SIMPLE_KITTY|Sent|Original:" | head -20

echo ""
echo "Key indicators:"
grep -E "FORCE-ENABLED|display_image called|Sent sized|Original:" forced_kitty.log | head -10

echo ""
echo "Check forced_kitty.log for full output"