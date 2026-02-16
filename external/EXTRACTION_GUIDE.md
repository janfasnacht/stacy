# Stata Documentation Extraction Guide

This guide walks through extracting machine-readable data from Stata's official documentation.

## Overview

We extract Stata documentation into machine-readable formats to:
1. Build comprehensive error code reference
2. Generate Rust source code from official specs
3. Create test fixtures for every documented error
4. Future-proof against Stata version changes
5. Provide user reassurance ("we match the official spec")

## Quick Start: Error Codes Extraction

### Step 1: Copy Stata PDFs

```bash
cd external/stata-docs/raw/

# Copy critical documentation
cp /Applications/StataNow/docs/p.pdf .    # Programming Manual (5MB)
cp /Applications/StataNow/docs/r.pdf .    # Reference Manual (29MB)
cp /Applications/StataNow/docs/u.pdf .    # User Guide (4MB)
```

### Step 2: Extract Error Code Pages (209-223)

**Option A: Using extraction script**

```bash
cd external/scripts/

# Extract pages 209-223 from Programming Manual
./extract_pdf_pages.sh \
  ../stata-docs/raw/p.pdf \
  209 223 \
  ../stata-docs/raw/error-codes-raw.pdf
```

**Option B: Manual extraction**
- Open `p.pdf` in Adobe Acrobat / Preview
- Print pages 209-223 to PDF
- Save as `external/stata-docs/raw/error-codes-raw.pdf`

### Step 3: Convert PDF to Markdown

**Option A: Using conversion script**

```bash
cd external/scripts/

# Convert extracted pages to markdown
./pdf_to_markdown.sh \
  ../stata-docs/raw/error-codes-raw.pdf \
  ../stata-docs/markdown/error-codes-raw.md
```

**Option B: Manual conversion**

User mentioned "I have tools" - use whatever tool you prefer:
- Adobe Acrobat: File → Export To → Markdown
- Pandoc: `pandoc -f pdf -t markdown error-codes-raw.pdf -o error-codes-raw.md`
- Marker.ai: `marker_single error-codes-raw.pdf error-codes-raw.md`
- Other PDF to markdown converters

Save the output as: `external/stata-docs/markdown/error-codes-raw.md`

### Step 4: Parse and Generate Structured Formats

```bash
cd external/scripts/

# Parse markdown and generate all outputs
python parse_error_codes.py
```

This will generate:
- `external/stata-docs/markdown/error-codes.md` - Clean, structured markdown
- `external/stata-docs/structured/error-codes.toml` - For Rust inclusion
- `external/stata-docs/structured/error-codes.json` - Machine-readable
- `external/stata-docs/structured/error-codes.csv` - Spreadsheet format
- `src/error/official_codes.rs` - Generated Rust code

### Step 5: Validate Extraction

```bash
cd external/scripts/

# Run validation checks
python validate_extraction.py
```

This checks:
- Required error codes are present (r(1), r(198), r(199), etc.)
- No duplicates
- All codes have descriptions
- JSON/CSV consistency
- Code range sanity checks

### Step 6: Build and Test

```bash
cd ../..  # Back to project root

# Build Rust project (includes generated code)
cargo build

# Run tests (TODO: will test error code detection)
cargo test
```

## Expected Format of error-codes-raw.md

The parser is flexible and can handle several formats:

### Format 1: Header-based (Preferred)

```markdown
## r(1) - Generic error

Catchall error code when no specific error applies.

---

## r(198) - Invalid syntax

Range invalid or option invalid.

---
```

### Format 2: Table format

```markdown
| Code | Name | Description |
|------|------|-------------|
| 1 | Generic error | Catchall error code |
| 198 | Invalid syntax | Range invalid |
```

### Format 3: Simple list

```markdown
r(1): Generic error. Catchall error code when no specific error applies.

r(198): Invalid syntax. Range invalid or option invalid.
```

The parser will detect and handle whichever format your PDF conversion produces.

## Output Formats Explained

### 1. Markdown (Human-readable reference)

**File**: `external/stata-docs/markdown/error-codes.md`

**Purpose**: Canonical human-readable reference

**Format**:
```markdown
## r(1) - Generic error

**Category**: General

Catchall error code when no specific error applies.

**See also**: Pages 209-210
```

### 2. TOML (Rust integration)

**File**: `external/stata-docs/structured/error-codes.toml`

**Purpose**: Embedded in Rust binary via `include_str!`

**Format**:
```toml
[[error]]
code = 1
name = "Generic error"
category = "General"
description = "Catchall error code"
```

### 3. JSON (Machine consumption)

**File**: `external/stata-docs/structured/error-codes.json`

**Purpose**: CI/testing, version comparison, API integration

**Format**:
```json
{
  "source": "Stata Programming Manual v18",
  "pages": "209-223",
  "errors": [
    {
      "code": 1,
      "name": "Generic error",
      "description": "Catchall error code"
    }
  ]
}
```

### 4. CSV (Spreadsheet analysis)

**File**: `external/stata-docs/structured/error-codes.csv`

**Purpose**: Manual review, spreadsheet analysis, quick reference

**Format**:
```csv
Code,Name,Category,Description
1,Generic error,General,Catchall error code
198,Invalid syntax,Syntax,Range invalid
```

### 5. Rust Source Code (Generated)

**File**: `src/error/official_codes.rs`

**Purpose**: Compile-time inclusion in stacy binary

**Format**:
```rust
pub struct OfficialErrorCode {
    pub code: u32,
    pub name: &'static str,
    pub description: &'static str,
}

pub const OFFICIAL_ERROR_CODES: &[OfficialErrorCode] = &[
    OfficialErrorCode {
        code: 1,
        name: "Generic error",
        description: "Catchall error code",
    },
    // ... all codes
];
```

## Troubleshooting

### "No error codes found in input file"

**Problem**: Parser couldn't detect error codes in markdown

**Solutions**:
1. Check `external/stata-docs/markdown/error-codes-raw.md` format
2. Look for patterns like `r(1)`, `r(198)` in the file
3. Try manual cleanup of the markdown
4. Adjust parser in `parse_error_codes.py` if needed

### "Missing required error codes"

**Problem**: Validation found missing critical error codes

**Solutions**:
1. Check if pages 209-223 fully captured in PDF extraction
2. Verify PDF to markdown conversion didn't skip content
3. Manually review `p.pdf` pages 209-223 for complete list
4. Add missing codes manually to markdown if needed

### "Large gaps in error codes"

**Warning**: Not necessarily an error - Stata has gaps in error code ranges

**Action**:
1. Note the gaps
2. Cross-reference with `p.pdf` to verify they're expected
3. Document any version-specific gaps in metadata

## Tools Installation

### Required Tools

```bash
# Python 3 (for parsing scripts)
python3 --version  # Should be 3.8+

# Git (for version control)
git --version

# Rust (for building stacy)
cargo --version
```

### Optional Tools (for PDF extraction)

```bash
# For PDF page extraction
brew install qpdf          # Preferred
# OR
brew install pdftk-java    # Alternative

# For PDF to markdown conversion
pip install marker-pdf     # Best quality (AI-powered)
# OR
brew install pandoc        # Good for text PDFs
# OR
brew install poppler       # Provides pdftotext (basic)
```

If you have Adobe Acrobat or other PDF tools, you can use those instead.

## Next Steps After Error Code Extraction

### 1. Extract Command Documentation (Week 2)

```bash
# Extract specific command pages from r.pdf
# Focus on commands we care about:
# - do, run, include
# - log, adopath, ssc
# - use, save, merge, append
```

### 2. Study Ado Files (Week 2)

```bash
# Copy critical ado files
cp /Applications/StataNow/ado/base/a/adopath.ado external/stata-ado-reference/
cp /Applications/StataNow/ado/base/s/ssc.ado external/stata-ado-reference/
cp /Applications/StataNow/ado/base/l/log.ado external/stata-ado-reference/

# Annotate with our understanding
# Create *-annotated.md files
```

### 3. Build Test Fixtures (Week 2-3)

```bash
# Create test fixtures for each error code
# tests/fixtures/error-codes/r_001.do
# tests/fixtures/error-codes/r_198.do
# etc.
```

### 4. Cross-version Testing (Week 3)

```bash
# Extract from older Stata versions
# Compare error codes across versions
# Document differences
```

## File Layout After Extraction

```
external/
├── scripts/
│   ├── extract_pdf_pages.sh        ✅ Extract PDF pages
│   ├── pdf_to_markdown.sh          ✅ Convert to markdown
│   ├── parse_error_codes.py        ✅ Parse and structure
│   └── validate_extraction.py      ✅ Validate completeness
│
├── stata-docs/
│   ├── raw/
│   │   ├── p.pdf                   📄 Programming Manual
│   │   ├── r.pdf                   📄 Reference Manual
│   │   ├── u.pdf                   📄 User Guide
│   │   └── error-codes-raw.pdf     📄 Extracted pages 209-223
│   │
│   ├── markdown/
│   │   ├── error-codes-raw.md      📝 Raw conversion
│   │   └── error-codes.md          📝 Clean, structured
│   │
│   ├── structured/
│   │   ├── error-codes.toml        🔧 For Rust
│   │   ├── error-codes.json        🔧 For machines
│   │   └── error-codes.csv         🔧 For spreadsheets
│   │
│   └── metadata.toml               📋 Extraction tracking
│
└── stata-ado-reference/
    └── (ado files copied here later)
```

## Git Strategy

### What to Commit

✅ **Commit these**:
- Extraction scripts
- Structured data (TOML, JSON, CSV)
- Markdown documentation
- Generated Rust code
- Metadata tracking

❌ **Don't commit these** (gitignored):
- Original PDFs (copyrighted)
- Raw extracted PDFs
- Large binary files

### Why This Strategy

- **Scripts are reproducible**: Anyone can re-run extraction
- **Structured data are facts**: Error codes are factual information
- **Markdown is transformative**: Our analysis and structuring
- **Generated code is ours**: Derived work for our project
- **PDFs are copyrighted**: Respect Stata's copyright

## Legal Note

Stata documentation is copyrighted by StataCorp LLC.

**What we CAN do**:
- Extract factual information (error code numbers)
- Reference specific page numbers
- Create derived works (structured data)
- Document behavior we observe

**What we CANNOT do**:
- Redistribute Stata PDFs
- Copy substantial documentation text verbatim
- Include PDFs in public repository

Our approach: Extract facts and create transformative structured data.

## Questions?

See:
- `dev/notes/2025-12-01-documentation-extraction-strategy.md` - Full strategy
- `dev/notes/2025-12-01-stata-internals-strategy.md` - Overall approach
- `dev/notes/2025-12-01-action-items.md` - Week 1-2 tasks
