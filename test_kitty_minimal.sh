#!/bin/bash

echo "Testing minimal Kitty graphics protocol..."
echo "This will try to display a small red square"

# Create a tiny 10x10 red PNG using ImageMagick or base64 encoded data
# This is a pre-encoded 10x10 red PNG
RED_SQUARE="iVBORw0KGgoAAAANSUhEUgAAAAoAAAAKCAYAAACNMs+9AAAAFUlEQVR42mP8z8BQz0AEYBxVSF+FABJADveWkH6oAAAAAElFTkSuQmCC"

# Clear any existing images
printf '\x1b_Ga=d\x1b\\'

# Send the image
printf '\x1b_Ga=T,f=100,s=10,v=10;%s\x1b\\' "$RED_SQUARE"

echo ""
echo "If you see a red square above, Kitty graphics are working!"
echo ""
echo "Now testing with your PDF:"

# Test with actual chonker8-hot
DYLD_LIBRARY_PATH=./lib timeout 3 ./target/release/chonker8-hot /Users/jack/Desktop/BERF-CERT.pdf 2>&1 | head -20