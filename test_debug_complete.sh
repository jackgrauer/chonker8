#!/bin/bash

echo "🧪 Complete DEBUG Screen Test"
echo "============================"
echo ""

# Clear debug log
echo "Clearing old debug log..."
rm -f /tmp/chonker8_debug.log

# Create a simple test file that will trigger a warning
echo "Creating test file with unused variable..."
cat > /tmp/test_build.rs << 'EOF'
fn main() {
    let unused = 5;
    println!("Test");
}
EOF

# Compile it to generate warnings and capture to our debug log
echo "Compiling test file to generate warnings..."
rustc /tmp/test_build.rs 2>&1 | while read line; do
    echo "[$(date '+%H:%M:%S.%3N')] [BUILD] $line" >> /tmp/chonker8_debug.log
done

# Add some fake build messages
echo "[$(date '+%H:%M:%S.%3N')] [BUILD] Starting build for chonker8-hot..." >> /tmp/chonker8_debug.log
echo "[$(date '+%H:%M:%S.%3N')] [BUILD] warning: unused variable: \`unused\`" >> /tmp/chonker8_debug.log
echo "[$(date '+%H:%M:%S.%3N')] [BUILD]  --> /tmp/test_build.rs:2:9" >> /tmp/chonker8_debug.log
echo "[$(date '+%H:%M:%S.%3N')] [BUILD]   |" >> /tmp/chonker8_debug.log
echo "[$(date '+%H:%M:%S.%3N')] [BUILD] 2 |     let unused = 5;" >> /tmp/chonker8_debug.log
echo "[$(date '+%H:%M:%S.%3N')] [BUILD]   |         ^^^^^^ help: if this is intentional, prefix it with an underscore: \`_unused\`" >> /tmp/chonker8_debug.log
echo "[$(date '+%H:%M:%S.%3N')] [BUILD] Build completed with warnings" >> /tmp/chonker8_debug.log

echo ""
echo "Debug log contents:"
echo "==================="
cat /tmp/chonker8_debug.log

echo ""
echo "✅ Debug log created with build warnings!"
echo ""
echo "Instructions to test:"
echo "1. Run: DYLD_LIBRARY_PATH=./lib ./target/release/chonker8-hot"
echo "2. Press Tab until you reach the DEBUG screen"
echo "3. You should see the build warnings in the debug output"
echo ""
echo "The DEBUG screen will load messages from /tmp/chonker8_debug.log"

# Clean up
rm -f /tmp/test_build.rs /tmp/test_build