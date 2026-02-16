#!/usr/bin/env bash
# Extract specific pages from Stata PDF documentation
#
# Usage:
#   ./extract_pdf_pages.sh p.pdf 209 223 error-codes-raw.pdf
#
# Requirements:
#   - pdftk (install via: brew install pdftk-java)
#   OR
#   - qpdf (install via: brew install qpdf)

set -e

PDF_INPUT="$1"
START_PAGE="$2"
END_PAGE="$3"
PDF_OUTPUT="$4"

if [ -z "$PDF_INPUT" ] || [ -z "$START_PAGE" ] || [ -z "$END_PAGE" ] || [ -z "$PDF_OUTPUT" ]; then
    echo "Usage: $0 INPUT.pdf START_PAGE END_PAGE OUTPUT.pdf"
    echo ""
    echo "Example:"
    echo "  $0 ../stata-docs/raw/p.pdf 209 223 ../stata-docs/raw/error-codes-raw.pdf"
    exit 1
fi

if [ ! -f "$PDF_INPUT" ]; then
    echo "Error: Input file not found: $PDF_INPUT"
    exit 1
fi

echo "Extracting pages $START_PAGE-$END_PAGE from $PDF_INPUT..."

# Try qpdf first (more reliable)
if command -v qpdf &> /dev/null; then
    echo "Using qpdf..."
    qpdf "$PDF_INPUT" --pages . "$START_PAGE-$END_PAGE" -- "$PDF_OUTPUT"
    echo "✅ Extracted to: $PDF_OUTPUT"

# Fall back to pdftk
elif command -v pdftk &> /dev/null; then
    echo "Using pdftk..."
    pdftk "$PDF_INPUT" cat "$START_PAGE-$END_PAGE" output "$PDF_OUTPUT"
    echo "✅ Extracted to: $PDF_OUTPUT"

else
    echo "❌ Error: Neither qpdf nor pdftk found"
    echo ""
    echo "Install one of:"
    echo "  brew install qpdf"
    echo "  brew install pdftk-java"
    exit 1
fi

# Show file size
if [ -f "$PDF_OUTPUT" ]; then
    SIZE=$(du -h "$PDF_OUTPUT" | cut -f1)
    echo "   File size: $SIZE"
fi
