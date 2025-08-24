#!/bin/bash

echo "Testing various terminal image protocols..."
echo "Terminal: $TERM"
echo ""

# Test 1: Kitty Graphics Protocol
echo "1. Testing Kitty protocol..."
printf '\x1b_Ga=d\x1b\\'
printf '\x1b_Ga=T,f=100,s=10,v=10;iVBORw0KGgoAAAANSUhEUgAAAAoAAAAKCAYAAACNMs+9AAAAFUlEQVR42mP8z8BQz0AEYBxVSF+FABJADveWkH6oAAAAAElFTkSuQmCC\x1b\\'
echo " (Red square if Kitty works)"

# Test 2: Sixel (common in many terminals)
echo ""
echo "2. Testing Sixel protocol..."
printf '\x1bPq#0;2;0;0;0#1;2;100;0;0#2;2;0;100;0#1~~@@vv@@~~@@~~$#2??}}GG}}??}}??-\x1b\\'
echo " (Red/green pattern if Sixel works)"

# Test 3: iTerm2 Inline Images Protocol  
echo ""
echo "3. Testing iTerm2 protocol..."
printf '\x1b]1337;File=inline=1;width=10;height=10:'
printf 'iVBORw0KGgoAAAANSUhEUgAAAAoAAAAKCAYAAACNMs+9AAAAFUlEQVR42mP8z8BQz0AEYBxVSF+FABJADveWkH6oAAAAAElFTkSuQmCC' | base64 -d | base64
printf '\x07'
echo " (Red square if iTerm2 works)"

echo ""
echo "---"
echo "If none of these work, your terminal may not support inline images."
echo "Ghostty is still in development and may have limited image support."
echo ""
echo "Options:"
echo "1. Use Kitty terminal: kitty ./target/release/chonker8-hot file.pdf"
echo "2. Use iTerm2 on macOS"
echo "3. Use a terminal with Sixel support"
echo "4. Wait for Ghostty to add image support (it's planned)"