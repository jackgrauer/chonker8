#!/bin/bash

echo "Testing Kitty Protocol Integration for chonker8"
echo "================================================"

# Create a simple test PDF
echo "Creating test PDF..."
cat > /tmp/test_kitty.tex << 'EOF'
\documentclass{article}
\usepackage{graphicx}
\begin{document}
\title{Kitty Protocol Test PDF}
\author{chonker8}
\maketitle

\section{Test Section}
This is a test PDF to verify that the Kitty graphics protocol is working correctly.

\begin{itemize}
\item The PDF image should appear in the left panel
\item The extracted text should appear in the right panel
\item If Kitty is not supported, text fallback should work
\end{itemize}

\section{Features}
\begin{enumerate}
\item Native image display
\item Hardware acceleration
\item Better quality than ASCII
\end{enumerate}

\end{document}
EOF

# Convert to PDF if pdflatex is available
if command -v pdflatex &> /dev/null; then
    cd /tmp && pdflatex -interaction=nonstopmode test_kitty.tex > /dev/null 2>&1
    PDF_FILE="/tmp/test_kitty.pdf"
else
    # Use existing test PDF or create simple one with text
    echo "Test PDF Content" | ps2pdf - /tmp/test_kitty.pdf 2>/dev/null || true
    PDF_FILE="/tmp/test_kitty.pdf"
fi

if [ -f "$PDF_FILE" ]; then
    echo "Test PDF created at: $PDF_FILE"
    echo ""
    echo "Running chonker8-hot with Kitty protocol..."
    echo "============================================"
    
    # Check if we're in Kitty terminal
    if [ -n "$KITTY_WINDOW_ID" ]; then
        echo "✓ Kitty terminal detected (ID: $KITTY_WINDOW_ID)"
    else
        echo "✗ Not running in Kitty terminal - will use fallback rendering"
    fi
    
    echo ""
    echo "Launching chonker8-hot..."
    echo "Press 'q' to quit, arrow keys to navigate"
    echo ""
    
    # Run chonker8-hot with the test PDF
    DYLD_LIBRARY_PATH=./lib ./target/release/chonker8-hot "$PDF_FILE"
else
    echo "Error: Could not create test PDF"
    exit 1
fi