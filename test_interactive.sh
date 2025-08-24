#!/bin/bash
export KITTY_WINDOW_ID=1
export TERM=xterm-kitty
echo "Testing in simulated Kitty environment..."
timeout 3 ./target/release/chonker8-hot /Users/jack/Desktop/BERF-CERT.pdf 2>&1 | grep -E "(KITTY|DEBUG|ERROR)" | head -100
