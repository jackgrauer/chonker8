#!/bin/bash
export KITTY_WINDOW_ID=1
export TERM=xterm-kitty

# Create a named pipe for input
mkfifo /tmp/test_input 2>/dev/null || true

# Run the app with input from pipe, capturing stderr
(echo "" | ./target/release/chonker8-hot /Users/jack/Desktop/BERF-CERT.pdf) 2>&1 &
PID=$!

# Wait for app to start
sleep 1

# Send escape to exit
echo -e "\x1b" > /tmp/test_input

# Wait for clean exit
wait $PID 2>/dev/null

# Clean up
rm -f /tmp/test_input
