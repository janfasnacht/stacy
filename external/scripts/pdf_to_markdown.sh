#!/usr/bin/env bash
# Convert PDF to markdown for easier parsing
#
# Usage:
#   ./pdf_to_markdown.sh input.pdf output.md
#
# This script tries multiple tools in order of preference:
#   1. marker (best quality, AI-powered)
#   2. pandoc (good for text-heavy PDFs)
#   3. pdftotext (fallback, basic text extraction)

set -e

PDF_INPUT="$1"
MD_OUTPUT="$2"

if [ -z "$PDF_INPUT" ] || [ -z "$MD_OUTPUT" ]; then
    echo "Usage: $0 INPUT.pdf OUTPUT.md"
    echo ""
    echo "Example:"
    echo "  $0 ../stata-docs/raw/error-codes-raw.pdf ../stata-docs/markdown/error-codes-raw.md"
    exit 1
fi

if [ ! -f "$PDF_INPUT" ]; then
    echo "Error: Input file not found: $PDF_INPUT"
    exit 1
fi

echo "Converting $PDF_INPUT to markdown..."

# Try marker-pdf first (if user installed it)
if command -v marker_single &> /dev/null; then
    echo "Using marker (AI-powered extraction)..."
    marker_single "$PDF_INPUT" "$MD_OUTPUT"
    echo "✅ Converted with marker: $MD_OUTPUT"

# Try pandoc
elif command -v pandoc &> /dev/null; then
    echo "Using pandoc..."
    pandoc -f pdf -t markdown "$PDF_INPUT" -o "$MD_OUTPUT"
    echo "✅ Converted with pandoc: $MD_OUTPUT"

# Fall back to pdftotext
elif command -v pdftotext &> /dev/null; then
    echo "Using pdftotext (basic extraction)..."
    # Extract to temporary text file
    TMP_TXT="${MD_OUTPUT%.md}.txt"
    pdftotext -layout "$PDF_INPUT" "$TMP_TXT"

    # Convert to basic markdown format
    {
        echo "# Extracted from $(basename "$PDF_INPUT")"
        echo ""
        cat "$TMP_TXT"
    } > "$MD_OUTPUT"

    rm -f "$TMP_TXT"
    echo "✅ Converted with pdftotext: $MD_OUTPUT"
    echo "⚠️  Note: pdftotext provides basic extraction - you may need to clean up the markdown manually"

else
    echo "❌ Error: No PDF conversion tool found"
    echo ""
    echo "Install one of:"
    echo "  pip install marker-pdf    # Best quality (AI-powered)"
    echo "  brew install pandoc       # Good for text PDFs"
    echo "  brew install poppler      # Provides pdftotext (basic)"
    echo ""
    echo "Or use Adobe Acrobat / other tool to export to markdown manually"
    exit 1
fi

# Show file size
if [ -f "$MD_OUTPUT" ]; then
    LINES=$(wc -l < "$MD_OUTPUT")
    SIZE=$(du -h "$MD_OUTPUT" | cut -f1)
    echo "   Lines: $LINES"
    echo "   File size: $SIZE"
fi
