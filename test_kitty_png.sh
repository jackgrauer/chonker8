#!/bin/bash

echo "Testing Kitty with actual PNG file"

# Create a simple red PNG using ImageMagick if available, or use a base64 encoded one
if command -v convert &> /dev/null; then
    echo "Creating test.png with ImageMagick..."
    convert -size 100x100 xc:red test.png
else
    echo "Using pre-encoded red square..."
    # 10x10 red PNG
    base64 -d > test.png << 'EOF'
iVBORw0KGgoAAAANSUhEUgAAAAoAAAAKCAYAAACNMs+9AAAAFUlEQVR42mP8z8BQz0AEYBxVSF+FABJADveWkH6oAAAAAElFTkSuQmCC
EOF
fi

echo "Clearing existing images..."
printf '\033_Ga=d\033\\'

echo "Sending PNG with standard Kitty protocol..."
# Use icat to test if available
if command -v icat &> /dev/null; then
    echo "Using icat (Kitty's image tool)..."
    icat test.png
else
    echo "Using manual Kitty protocol..."
    # Encode the file
    BASE64_DATA=$(base64 < test.png | tr -d '\n')
    # Send with a=T (transmit), f=100 (PNG)
    printf '\033_Ga=T,f=100;%s\033\\' "$BASE64_DATA"
fi

echo ""
echo "You should see a red square above if Kitty graphics work."
echo ""
echo "Press Enter to test chonker8-hot..."
read

./target/release/chonker8-hot /Users/jack/Desktop/BERF-CERT.pdf