#!/bin/bash

echo "🧪 Testing Build Output in DEBUG Screen"
echo "========================================"
echo ""

# Clear the debug log first
rm -f /tmp/chonker8_debug.log

# Start chonker8-hot in background
echo "Starting chonker8-hot..."
DYLD_LIBRARY_PATH=./lib timeout 3 ./target/release/chonker8-hot 2>&1 &
PID=$!

# Wait a moment for it to start
sleep 1

# Trigger a rebuild by touching a source file
echo "Triggering rebuild by modifying source..."
touch src/main_hotreload.rs

# Wait for build to happen
sleep 2

# Check if debug log has build output
echo ""
echo "Debug log contents:"
echo "-------------------"
if [ -f /tmp/chonker8_debug.log ]; then
    cat /tmp/chonker8_debug.log
    echo ""
    echo "✅ Build output is being logged to debug file!"
else
    echo "❌ No debug log found"
fi

# Kill the background process
kill $PID 2>/dev/null

echo ""
echo "Test complete. Build warnings should now appear in DEBUG screen!"