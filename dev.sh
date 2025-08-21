#!/bin/bash

# Development script for testing hot-reload
echo "🔥 Chonker8 Hot-Reload Development Script"
echo "=========================================="

# Build the hot-reload version
echo "Building chonker8-hot..."
DYLD_LIBRARY_PATH=./lib cargo build --bin chonker8-hot

if [ $? -ne 0 ]; then
    echo "❌ Build failed!"
    exit 1
fi

echo "✅ Build successful!"

# Start the app in background
echo ""
echo "Starting chonker8-hot in background..."
DYLD_LIBRARY_PATH=./lib ./target/debug/chonker8-hot test.pdf &
APP_PID=$!

echo "PID: $APP_PID"
sleep 2

# Test hot-reload by modifying ui.toml
echo ""
echo "Testing hot-reload..."
echo "Original highlight color: $(grep 'highlight' ui.toml)"

# Change highlight to green
sed -i.bak 's/highlight = "yellow"/highlight = "green"/' ui.toml
echo "Changed to: $(grep 'highlight' ui.toml)"
sleep 1

# Change border style
sed -i.bak 's/border = "rounded"/border = "sharp"/' ui.toml
echo "Changed border to: $(grep 'border' ui.toml | head -1)"
sleep 1

# Change mode
sed -i.bak 's/mode = "split"/mode = "full"/' ui.toml
echo "Changed mode to: $(grep 'mode' ui.toml)"
sleep 1

# Restore original
echo ""
echo "Restoring original config..."
mv ui.toml.bak ui.toml

echo ""
echo "Test complete! Press any key to stop the app..."
read -n 1

# Kill the app
kill $APP_PID 2>/dev/null
echo "App stopped."