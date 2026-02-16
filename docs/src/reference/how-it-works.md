# How It Works

Technical details on stacy's architecture.

## Contents

- [Error Detection](#error-detection)
- [Package Isolation](#package-isolation)
- [Output Streaming](#output-streaming)
- [Machine Interface](#machine-interface)

---

## Error Detection

### The Problem

Stata's batch mode (`stata -b do script.do`) always exits with code 0, even when scripts fail. Errors are only visible in log files.

### stacy's Solution

1. **Execute**: stacy runs Stata with `-b -q` flags
2. **Parse**: After Stata exits, stacy parses the log file
3. **Detect**: Matches against 182 official error patterns
4. **Report**: Returns appropriate exit code (1-10)

```
┌───────────┐     ┌───────┐     ┌─────────┐     ┌──────────┐
│ stacy run │ ──▶ │ Stata │ ──▶ │ Log     │ ──▶ │ Exit     │
│           │     │ -b -q │     │ Parser  │     │ Code 0-10│
└───────────┘     └───────┘     └─────────┘     └──────────┘
```

### Error Patterns

stacy recognizes errors in two forms:

**r() codes** (primary):
```
r(199);
```

**Error messages** (secondary):
```
unrecognized command: foobar
file myfile.dta not found
```

### Official Stata Error Codes

stacy includes all 182 error codes from the [Stata Programming Reference Manual](https://www.stata.com/manuals/perror.pdf):

| Range | Category | Examples |
|-------|----------|----------|
| r(1-99) | General errors | r(1) generic error |
| r(100-199) | Syntax/variable errors | r(111) not found, r(199) unrecognized command |
| r(400-499) | System limits | r(459) not sorted |
| r(600-699) | File errors | r(601) file not found, r(603) file exists |
| r(900-999) | Resource errors | r(950) insufficient memory |

### 10 Most Common Errors

| Code | Name | Typical Cause |
|------|------|---------------|
| r(100) | varlist required | Command needs variable list |
| r(109) | type mismatch | String operation on numeric or vice versa |
| r(111) | not found | Variable doesn't exist in dataset |
| r(198) | invalid syntax | Malformed command |
| r(199) | unrecognized command | Command doesn't exist or package missing |
| r(459) | not sorted | Data must be sorted |
| r(601) | file not found | File doesn't exist |
| r(603) | file already exists | Need `replace` option |
| r(950) | insufficient memory | Operation too large |

### Accuracy

Error detection achieves 97% accuracy on common patterns. Edge cases:

- **`capture` blocks**: Intentionally suppressed errors are not reported
  (captured errors don't produce `r(N);` after the "end of do-file" marker)
- **Custom programs**: User-written error messages may not match patterns
- **Unusual formatting**: Some packages produce non-standard log output

---

## Package Isolation

### The Problem

Stata packages install globally to `~/ado/plus/`. Every project shares the same versions. When SSC updates a package, all projects change silently.

### stacy's Solution

stacy uses a **global cache** with **per-project isolation**:

```
~/.cache/stacy/packages/
├── estout/
│   ├── 2024.01.15/
│   │   └── estout.ado
│   └── 2024.03.15/
│       └── estout.ado
└── reghdfe/
    └── 6.12.3/
        └── reghdfe.ado
```

**Cache benefits:**
- Multiple versions coexist
- Shared across projects (disk efficient)
- Offline installation from cache

### Runtime Isolation

When you run `stacy run script.do`, stacy:

1. Reads `stacy.lock` to determine required packages
2. Builds a custom `S_ADO` path pointing to cached versions
3. Launches Stata with this adopath

```
Project A (stacy.lock):          Project B (stacy.lock):
  estout = 2024.01.15            estout = 2024.03.15
  reghdfe = 6.12.3               reghdfe = 6.12.3

Both use the same cache, but see different versions.
```

### Lockfile Verification

The lockfile includes SHA256 checksums:

```toml
[packages.estout]
version = "2024.03.15"
checksum = "sha256:14af94e03edd..."
```

On `stacy install`, checksums are verified to ensure:
- Downloaded files match expected content
- Cached packages haven't been modified
- SSC hasn't silently updated the package

---

## Output Streaming

### Real-time Logs

Use `-v` (verbose) to stream Stata's log output as it runs:

```bash
stacy run -v long_analysis.do
```

This displays log lines in real-time, useful for:
- Monitoring long-running scripts
- Debugging interactively
- Seeing progress indicators

### Progress Reporting

For scripts without verbose mode, stacy shows periodic progress:

```
⠋ Running: analysis.do (45s elapsed)
```

Configure the interval in `stacy.toml`:

```toml
[run]
progress_interval_seconds = 30
```

### Structured Logging

For automated pipelines, combine `--format json` with verbose output:

```bash
stacy run --format json -v analysis.do 2>log.txt
```

- stdout: JSON result
- stderr: Real-time log stream

---

## Machine Interface

stacy is designed for integration with other tools.

### JSON Output

Every command supports `--format json`:

```bash
stacy run --format json analysis.do
stacy install --format json
stacy doctor --format json
```

See [JSON Output](./json-output.md) for complete schemas.

### Exit Codes

Stable, semantic exit codes for scripting:

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | Stata error |
| 2 | Syntax error |
| 3 | File error |
| 4 | Memory error |
| 5 | Internal error |
| 10 | Environment error |

See [Exit Codes](./exit-codes.md) for mapping details.

### Build System Integration

stacy's exit codes enable integration with any tool that respects Unix conventions:

**Make:**
```makefile
results.dta: analysis.do
	stacy run analysis.do  # Stops on non-zero
```

**Shell scripts:**
```bash
stacy run analysis.do || exit 1
```

**CI pipelines:**
```yaml
- run: stacy run analysis.do  # Fails job on error
```

### Programmatic Usage

**Python:**
```python
import subprocess, json

result = subprocess.run(
    ['stacy', 'run', '--format', 'json', 'analysis.do'],
    capture_output=True
)
data = json.loads(result.stdout)
if not data['success']:
    print(f"Failed: {data['errors']}")
```

**R:**
```r
result <- system2("stacy", c("run", "--format", "json", "analysis.do"),
                  stdout = TRUE)
data <- jsonlite::fromJSON(paste(result, collapse = "\n"))
```

---

## See Also

- [Exit Codes](./exit-codes.md) - Exit code reference
- [JSON Output](./json-output.md) - JSON schemas
- [Stata Error Manual](https://www.stata.com/manuals/perror.pdf) - Official documentation
