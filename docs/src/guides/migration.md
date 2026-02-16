# Migration Guide

How to adopt stacy in existing Stata projects.

## Overview

stacy works with existing Stata scripts unchanged. Migration is incremental--start with error detection, add package management when ready.

| Current workflow | stacy equivalent |
|------------------|----------------|
| `stata -b do script.do` | `stacy run script.do` |
| `ssc install pkg` | `stacy add pkg` |
| `master.do` | `[scripts]` section |

## From Batch Mode to stacy run

### Before

```bash
stata-mp -b do analysis.do
# Always exits 0, even on error
# Must manually check analysis.log
```

### After

```bash
stacy run analysis.do
# Exits 1-10 on error
# Shows error with documentation link
```

**What changes:**
- Exit codes now reflect success/failure
- Errors display with Stata documentation links
- Build systems (Make, Snakemake) can detect failures

**What stays the same:**
- Your `.do` files work unchanged
- Output goes to the same log file
- Stata runs in the background

### Updating scripts

For shell scripts that check logs:

```bash
# Before: parse log for errors
stata-mp -b do analysis.do
if grep -q "^r(" analysis.log; then
    echo "Error!"
    exit 1
fi

# After: just check exit code
stacy run analysis.do
if [ $? -ne 0 ]; then
    echo "Error!"
    exit 1
fi

# Or more simply
stacy run analysis.do || exit 1
```

## From ssc install to Lockfiles

### Before

```stata
* At the top of master.do or a setup script
ssc install estout
ssc install reghdfe
```

Problems:
- Different versions on different machines
- "It worked last month" failures
- No record of what's actually installed

### After

```bash
# One-time setup
stacy init
stacy add estout reghdfe

# Creates stacy.toml (what you want) and stacy.lock (what you have)
git add stacy.toml stacy.lock
git commit -m "Add stacy package management"
```

For collaborators:

```bash
git pull
stacy install  # Gets exact same versions
```

### Step-by-step migration

1. **List current packages**
   ```stata
   ado dir
   ```

2. **Initialize stacy**
   ```bash
   stacy init
   ```

3. **Add each package**
   ```bash
   stacy add estout reghdfe ftools
   ```

4. **Remove ssc install lines from scripts**
   Delete or comment out `ssc install` commands--stacy handles this now.

5. **Commit both files**
   ```bash
   git add stacy.toml stacy.lock
   git commit -m "Switch to stacy package management"
   ```

### Handling GitHub packages

If you install from GitHub:

```stata
* Before
net install reghdfe, from("https://raw.githubusercontent.com/sergiocorreia/reghdfe/master/src/")
```

```bash
# After
stacy add github:sergiocorreia/reghdfe
```

## From master.do to [scripts]

### Before

```stata
* master.do
do "01_clean_data.do"
do "02_analysis.do"
do "03_tables.do"
```

Problems:
- Running one script requires editing master.do
- No parallelization
- Error in script 2 still runs script 3 (unless you add `capture` logic)

### After

Add to `stacy.toml`:

```toml
[scripts]
clean = "01_clean_data.do"
analysis = "02_analysis.do"
tables = "03_tables.do"
all = ["clean", "analysis", "tables"]
```

Run individual tasks or sequences:

```bash
stacy task clean      # Run just cleaning
stacy task analysis   # Run just analysis
stacy task all        # Run all in order
```

Benefits:
- Named tasks are self-documenting
- Each task stops on error by default
- Can run tasks individually for debugging

### Keeping master.do

You don't have to remove master.do. Both can coexist:

```bash
# Using stacy tasks
stacy task all

# Or using master.do through stacy (still get exit codes)
stacy run master.do
```

## From Make to Make + stacy

If you already use Make:

### Before

```makefile
%.log: %.do
    stata-mp -b do $<
```

### After

```makefile
%.log: %.do
    stacy run $<
```

That's it. Make now stops on Stata errors.

### Adding package management

```makefile
# Ensure packages are installed before running
.PHONY: install
install:
    stacy install

results/analysis.dta: analysis.do install
    stacy run analysis.do
```

## Checklist

### Minimal migration (exit codes only)

- [ ] Install stacy
- [ ] Replace `stata -b do` with `stacy run` in scripts/Makefile
- [ ] Verify `stacy doctor` passes

### Full migration (packages + tasks)

- [ ] Run `stacy init`
- [ ] Add packages with `stacy add`
- [ ] Remove `ssc install` lines from scripts
- [ ] Add `[scripts]` section for common tasks
- [ ] Commit `stacy.toml` and `stacy.lock`
- [ ] Update CI to run `stacy install` before tests
- [ ] Tell collaborators to run `stacy install` after pulling

## See Also

- [Quick Start](../quick-start.md) - Getting started guide
- [Project Config](../configuration/project.md) - stacy.toml reference
- [Build Integration](./build-integration.md) - Make, Snakemake, CI/CD
