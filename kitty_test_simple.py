#!/usr/bin/env python3

import base64
import sys

# First, render the PDF with our Rust code
import subprocess
subprocess.run(["./target/release/test_vello"], capture_output=True)

# Read the rendered image
with open("vello_render_test.png", "rb") as f:
    img_data = f.read()

# Send via Kitty graphics protocol
encoded = base64.b64encode(img_data).decode('ascii')

# Kitty protocol: a=T (transmit), f=100 (PNG)
sys.stdout.write(f'\x1b_Ga=T,f=100;{encoded}\x1b\\')
sys.stdout.flush()

print("\n\nPDF should be visible above in Kitty terminal")
print("The birth certificate has been rendered via Vello (GPU-accelerated)")