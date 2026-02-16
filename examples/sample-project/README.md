# Sample stacy Project

This is a minimal example demonstrating stacy usage with Make.

## Quick Start

```bash
# Initialize stacy project (optional - creates stacy.toml)
stacy init

# Run the analysis
stacy run main.do

# Or use Make
make all
```

## Files

- `main.do` - Main orchestration script
- `clean_data.do` - Data cleaning
- `analysis.do` - Statistical analysis
- `error_example.do` - Demonstrates error detection
- `Makefile` - Build automation

## Testing Error Detection

```bash
# This will fail with proper exit code
stacy run error_example.do
echo "Exit code: $?"  # Should be non-zero

# Compare with traditional Stata (always exits 0)
stata-mp -b do error_example.do
echo "Exit code: $?"  # Always 0, even on error!
```

## Using with Make

The key benefit of stacy is proper exit codes. This means Make will stop
when Stata encounters an error:

```bash
# With stacy: Make stops on Stata error
make all  # Stops if any .do file fails

# Without stacy: Make continues despite errors
# (because stata-mp always exits 0)
```

## Dependency Analysis

```bash
# Show dependency tree
stacy deps main.do

# Output:
# main.do
# ├── clean_data.do
# └── analysis.do
```
