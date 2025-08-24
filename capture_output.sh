#!/bin/bash

echo "🔍 Capturing chonker8-hot output with Kitty environment"
echo "========================================================="

# Set Kitty environment
export KITTY_WINDOW_ID=1
export TERM=xterm-kitty

# Run the app and capture EVERYTHING (stdout and stderr)
echo "Starting app..."
timeout 2 script -q /dev/null ./target/release/chonker8-hot /Users/jack/Desktop/BERF-CERT.pdf 2>&1 | tee output.log

echo ""
echo "📊 Analysis of captured output:"
echo "========================================================="

# Check for Kitty escape sequences
if grep -q $'\x1b_Ga=d' output.log; then
    echo "✅ Found Kitty clear command"
else
    echo "❌ No Kitty clear command found"
fi

if grep -q $'\x1b_Ga=T' output.log; then
    echo "✅ Found Kitty transmit command"
else
    echo "❌ No Kitty transmit command found"
fi

# Check for debug messages
echo ""
echo "Debug messages:"
grep -E "\[DEBUG\]|\[KITTY\]|\[SIMPLE_KITTY\]" output.log | head -20

# Show hex dump of Kitty sequences
echo ""
echo "🔬 Hex dump of Kitty sequences:"
hexdump -C output.log | grep -A2 -B2 "1b 5f 47" | head -30

# Show the actual escape sequences
echo ""
echo "📜 Raw Kitty escape sequences found:"
grep -ao $'\x1b_G[^'$'\x1b'']*'$'\x1b''\\' output.log | head -10

echo ""
echo "✅ Output saved to output.log for analysis"