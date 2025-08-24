#!/bin/bash
export KITTY_WINDOW_ID=1 
export TERM=xterm-kitty

# Run in background
./target/release/chonker8-hot /Users/jack/Desktop/BERF-CERT.pdf 2>debug.log &
PID=$!

# Wait briefly for UI to render
sleep 1

# Send tab key to switch screens
echo -e "\t" | ./target/release/chonker8-hot /Users/jack/Desktop/BERF-CERT.pdf 2>>debug.log

# Kill after a moment
sleep 1
kill $PID 2>/dev/null

# Show debug log
echo "=== Debug output ==="
grep -E "(render_pdf_screen|KITTY|display_image|transmit)" debug.log | head -50
