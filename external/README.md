# external/

Reference materials for understanding Stata internals.

**This directory is gitignored** - Stata documentation is copyrighted.

## Structure

```
external/
├── stata-docs/           # Stata PDF documentation
│   ├── p.pdf             # Programming Manual (CRITICAL)
│   ├── r.pdf             # Reference Manual (ALL commands)
│   ├── u.pdf             # User's Guide
│   └── ...
│
└── stata-ado-reference/  # System ado files for study
    ├── adopath.ado
    ├── ssc.ado
    ├── log.ado
    └── ...
```

## Critical Documents

### 1. Programming Manual (`p.pdf`)

**Why**: Contains:
- Complete r() error code reference
- adopath mechanics
- Batch mode behavior
- Environment variables
- Macro programming internals

**Location**: `/Applications/StataNow/docs/p.pdf` (4.9 MB)

**Key sections**:
- Error codes appendix
- Chapter on ado-files
- Chapter on programming

### 2. Base Reference Manual (`r.pdf`)

**Why**: Contains:
- ALL command syntax
- Error messages for each command
- Return codes by command
- Edge cases and examples

**Location**: `/Applications/StataNow/docs/r.pdf` (29 MB!)

**Use**: Reference for specific command error patterns

### 3. User's Guide (`u.pdf`)

**Why**: Contains:
- Log file mechanics
- Batch mode usage
- Path handling
- Installation structure

**Location**: `/Applications/StataNow/docs/u.pdf` (3.5 MB)

## System Ado Files

Study actual implementations:

```bash
# Location
/Applications/StataNow/ado/base/

# Critical files
a/adopath.ado        # How adopath actually works
s/ssc.ado            # How SSC installation works
l/log.ado            # Log file handling
p/program.ado        # Program definition
```

## Setup

### Copy Documentation (Local Only)

```bash
# From stacy project root
cd external/stata-docs/

# Copy critical PDFs
cp /Applications/StataNow/docs/p.pdf .    # Programming (5MB)
cp /Applications/StataNow/docs/r.pdf .    # Reference (29MB)
cp /Applications/StataNow/docs/u.pdf .    # User Guide (4MB)

# Optional: Copy specific topic manuals
cp /Applications/StataNow/docs/d.pdf .    # Data management
cp /Applications/StataNow/docs/m.pdf .    # Mata programming
```

### Copy Ado Files for Study

```bash
cd external/stata-ado-reference/

# Copy system ado files we need to understand
cp /Applications/StataNow/ado/base/a/adopath.ado .
cp /Applications/StataNow/ado/base/s/ssc.ado .
cp /Applications/StataNow/ado/base/l/log.ado .
```

## Machine-Readable Documentation Extraction

**See `EXTRACTION_GUIDE.md` for complete step-by-step instructions.**

### Quick Start: Error Codes (Pages 209-223 of p.pdf)

```bash
# 1. Copy PDFs
cd external/stata-docs/raw/
cp /Applications/StataNow/docs/{p,r,u}.pdf .

# 2. Extract error code pages
cd ../scripts/
./extract_pdf_pages.sh ../stata-docs/raw/p.pdf 209 223 ../stata-docs/raw/error-codes-raw.pdf

# 3. Convert to markdown (or use your preferred tool)
./pdf_to_markdown.sh ../stata-docs/raw/error-codes-raw.pdf ../stata-docs/markdown/error-codes-raw.md

# 4. Parse and generate structured formats
python parse_error_codes.py

# 5. Validate extraction
python validate_extraction.py

# 6. Build Rust project with generated code
cd ../..
cargo build
```

This generates:
- `external/stata-docs/markdown/error-codes.md` - Clean reference
- `external/stata-docs/structured/error-codes.{toml,json,csv}` - Machine-readable
- `src/error/official_codes.rs` - Generated Rust code

### Automation Scripts

Located in `external/scripts/`:
- `extract_pdf_pages.sh` - Extract specific PDF pages
- `pdf_to_markdown.sh` - Convert PDF to markdown
- `parse_error_codes.py` - Parse markdown into structured formats
- `validate_extraction.py` - Quality checks and validation

## Usage

### During Development

When implementing error detection:
1. **Extract `p.pdf` pages 209-223** - Get official error code list (see above)
2. **Search `r.pdf`** - Find exact error messages per command
3. **Use generated code** - Reference `src/error/official_codes.rs`
4. **Test against fixtures** - Verify we catch all documented errors

When implementing ado management:
1. **Read `adopath.ado`** - Understand precedence
2. **Read `ssc.ado`** - Understand SSC protocol
3. **Test behavior** - Verify our understanding

### Building Ground Truth

For each documented error code:
1. Extract from structured data: `external/stata-docs/structured/error-codes.json`
2. Create test fixture: `tests/fixtures/error-codes/r_XXX.do`
3. Run against real Stata
4. Verify stacy catches it

## Legal Note

**DO NOT commit Stata documentation to public repo**

Stata documentation is copyrighted by StataCorp LLC.

This directory is for **local reference only** during development:
- ✅ Keep on your machine
- ✅ Reference during development
- ✅ Extract patterns and codes
- ❌ Don't commit PDFs to git
- ❌ Don't distribute documentation

What we CAN do:
- Reference specific page numbers in comments
- Extract error code numbers (facts)
- Document behavior we observe
- Link to official Stata documentation online

## Stata Versions

Track version-specific behavior:

```
external/stata-versions/
├── stata14/
│   └── behavior-notes.md
├── stata15/
├── stata16/
├── stata17/
└── stata18/
```

Document differences in:
- Error messages
- Return codes
- Command syntax
- Ado file locations

## Next Steps

See `dev/notes/2025-12-01-stata-internals-strategy.md` for full strategy.

**Week 1 tasks**:
- [ ] Copy p.pdf, r.pdf, u.pdf
- [ ] Extract error code reference from p.pdf
- [ ] Copy critical ado files (adopath, ssc, log)
- [ ] Study adopath.ado for precedence rules
- [ ] Build error code test matrix
