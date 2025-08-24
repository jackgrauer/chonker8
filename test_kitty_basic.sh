#!/bin/bash

echo "Testing Kitty Graphics Protocol"
echo "================================"

# Check if running in Kitty
if [ -n "$KITTY_WINDOW_ID" ]; then
    echo "✅ Kitty detected! KITTY_WINDOW_ID=$KITTY_WINDOW_ID"
else
    echo "⚠️  Not running in Kitty terminal"
fi

# Create a simple test image using ImageMagick or base64
# This is a tiny 1x1 red pixel PNG encoded in base64
RED_PIXEL="iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8DwHwAFBQIAX8jx0gAAAABJRU5ErkJggg=="

echo ""
echo "Clearing any existing images..."
printf '\033_Ga=d,d=a\033\\'

echo "Sending a red pixel to Kitty..."
# Send image: a=T (transmit), f=100 (PNG), i=1 (id), s=50,v=50 (size)
printf '\033_Ga=T,f=100,i=1,s=50,v=50;%s\033\\' "$RED_PIXEL"

echo ""
echo "If Kitty graphics work, you should see a red square above."
echo ""
echo "Now let's test chonker8-hot..."
echo "Press any key to continue..."
read -n 1

# Run chonker8-hot
./target/release/chonker8-hot /Users/jack/Desktop/BERF-CERT.pdf