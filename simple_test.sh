#!/bin/bash
export KITTY_WINDOW_ID=1
export TERM=xterm-kitty

# Run briefly and kill
timeout 1 ./target/release/chonker8-hot /Users/jack/Desktop/BERF-CERT.pdf 2>test.err 1>test.out

# Show error output (debug messages)
echo "=== Debug output ==="
cat test.err | head -100
