# How It Works

What actually happens when stacy runs a script or installs a package. stacy has four moving parts: an execution engine, an error detector, package management, and a build cache. This page walks through each, then covers the machine interface and stacy's boundaries.

## Contents

- [Script Execution](#script-execution)
- [Error Detection](#error-detection)
- [Package Isolation](#package-isolation)
- [Build Cache](#build-cache)
- [Output Streaming](#output-streaming)
- [Machine Interface](#machine-interface)
- [What stacy Does Not Do](#what-stacy-does-not-do)

---

## Script Execution

`stacy run script.do` does four things:

```
┌───────────┐     ┌───────────┐     ┌─────────┐     ┌──────────┐
│ Build     │ ──▶ │ Stata     │ ──▶ │ Log     │ ──▶ │ Exit     │
│ S_ADO     │     │ -b -q     │     │ Parser  │     │ Code 0-10│
└───────────┘     └───────────┘     └─────────┘     └──────────┘
 from lockfile     fresh process     from the end    translated
```

1. **Build the environment.** If a lockfile is present, stacy constructs the `S_ADO` search path from it, so Stata sees exactly the locked packages (see [Package Isolation](#package-isolation)).
2. **Run Stata.** The script runs in a fresh batch-mode process (`-b -q`). The `-q` flag skips your `profile.do`, so execution doesn't depend on machine-specific startup configuration.
3. **Parse the log.** stacy reads the log Stata produced and determines whether the script succeeded (see [Error Detection](#error-detection)).
4. **Translate the outcome.** Stata error codes become standard shell exit codes that any build tool understands.

Extras that matter in practice:

- `--timeout <seconds>` kills a hung script (SIGTERM, then SIGKILL after a grace period) -- useful for convergence loops on shared clusters.
- `--parallel` runs multiple scripts concurrently, each in its own Stata process; output prints as a grouped block per script on completion, and internal logs are uniquely named, so `make -j` and Snakemake can run same-named scripts safely.
- `-c 'display ...'` runs inline code without a script file.

---

## Error Detection

### The Problem

Stata's batch mode always exits with code 0, even when scripts fail. Errors are only visible in log files.

### stacy's Solution

stacy parses the log **from the end**. It locates the last `end of do-file` marker -- which corresponds to the outermost do-file in nested execution -- and scans the lines after it for a return-code pattern (`r(N);`). Found: the script failed with that code. Not found: it succeeded.

This design handles the edge cases correctly:

| Scenario | Behavior |
|----------|----------|
| Uncaptured error, e.g. `r(601)` | `r(N);` appears after the final marker -- failure |
| `capture`d error | Doesn't propagate past `end of do-file` -- success |
| Error in a nested do-file | Propagates to the outermost marker -- failure |
| Script *prints* `"r(199);"` | Appears before the marker -- ignored |
| Stata killed / crashed | No final marker at all -- reported as error, never as success |

The error-detection logic is exercised by a test suite of 250+ cases covering nested do-files, captured errors, false positives from display output, and incomplete logs.

### Error Descriptions

To describe an error rather than just number it, stacy extracts Stata's own error descriptions from your installation at first run (`stacy doctor --refresh` re-extracts after a Stata upgrade). Where no description is available, it falls back to the documented range categories -- see [Exit Codes](./exit-codes.md) for the mapping.

Failures print a human-readable description plus a link to the official manual page, so you can diagnose a remote job from the error output alone:

```
FAIL  broken.do  (0.8s)

   Error: r(199) - unrecognized command

   See: https://www.stata.com/manuals/perror.pdf#r199
```

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

Multiple versions coexist in the cache, projects share it (disk-efficient), and cached packages install offline.

### Runtime Isolation

When you run `stacy run script.do`, stacy reads `stacy.lock`, builds an `S_ADO` search path pointing at the exact cached versions, and launches Stata with it. Each project's path is built from its own lockfile:

```
Project A (stacy.lock):          Project B (stacy.lock):
  estout = 2024.01.15            estout = 2024.03.15
  reghdfe = 6.12.3               reghdfe = 6.12.3

Both use the same cache, but see different versions.
```

Two modes:

- **Strict (default):** only locked packages and Stata's built-ins (`BASE`) are visible. Nothing leaks in from your global `PLUS` or `PERSONAL` directories, so "works because of something installed on my machine" cannot happen.
- **Allow-global (`--allow-global`):** locked packages take precedence, but globally installed packages remain available. Useful during development or incremental migration.

Project-local `.ado` directories can be added to the path via the [`[paths]` config section](../configuration/project.md#paths).

### Lockfile Verification

The lockfile includes SHA256 checksums:

```toml
[packages.estout]
version = "2024.03.15"
checksum = "sha256:14af94e03edd..."
```

On `stacy install`, checksums are verified to ensure downloaded files match expected content, cached packages haven't been modified, and SSC hasn't silently updated the package. See [Lockfile Format](./lockfile.md).

---

## Build Cache

Pipelines often re-run scripts that haven't changed. `stacy run --cache` skips that work:

1. stacy hashes the script and every do-file it depends on (`do`, `run`, and `include` statements, traced recursively -- the same parser behind `stacy deps`).
2. If nothing changed since the last successful run, stacy replays the previous result (exit code, log path, duration) without launching Stata.

The cache is project-local (`.stacy/cache/build.json`) and opt-in. `--force` re-runs regardless; `--cache-only` fails when no cached result exists, letting CI require a pre-populated cache. Files outside the do-file graph -- datasets, environment variables -- are not tracked; use `--force` when they change.

---

## Output Streaming

### Real-time Output

Program output streams to stdout live by default -- boilerplate-stripped (command echoes removed, blank runs collapsed), both in a terminal and when piped. Use `-v` (verbose) to stream the raw, unstripped log instead:

```bash
stacy run -v long_analysis.do
```

Streaming stops when the Stata process exits, so killed or timed-out runs terminate cleanly, and closed pipes (`stacy run foo.do | head`) end the stream without error.

### Log Files

The batch log is internal: it gets a unique name per invocation (so concurrent runs never collide), is removed on success, and is kept on failure — with its path printed in the failure output so you can inspect it. `--log <path>` writes the raw log to a chosen location regardless of outcome. Machine-readable formats keep the log and report its path.

### Progress Reporting

Without verbose mode, stacy shows periodic progress:

```
⠋ Running: analysis.do (45s elapsed)
```

Configure the interval in `stacy.toml`:

```toml
[run]
progress_interval_seconds = 30
```

### Structured Logging

For automated pipelines, use `--format json`. Machine-readable formats imply quiet execution (no streaming); the JSON result's `log_file` field points to the kept Stata log:

```bash
stacy run --format json analysis.do
```

---

## Machine Interface

stacy is designed to be a good Unix citizen: standard exit codes, machine-readable output, and no interactive surprises (update checks and colors are suppressed automatically in CI and piped output).

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
| 6 | Statistical error |
| 10 | Environment error |

See [Exit Codes](./exit-codes.md) for mapping details.

### Build System Integration

stacy's exit codes work with any tool that respects Unix conventions:

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

## What stacy Does Not Do

Knowing the boundaries is as useful as knowing the features:

- **It is not a build system.** stacy decides whether a Stata step *succeeded*; Make, Snakemake, or statacons decide *which* steps run and in what order. They compose: point your build tool's Stata rule at `stacy run`. See [Build Integration](../guides/build-integration.md).
- **It does not manage the Stata version, data files, or other languages.** The lockfile pins Stata *packages*. For full-stack reproducibility (OS, Stata itself, Python/R), use Docker -- stacy works the same inside a container.
- **It does not resolve transitive dependencies.** SSC packages declare dependencies inconsistently, as free text, so automatic resolution would guess. If package A needs package B, add both: `stacy add A B`. `stacy doctor` and post-install scanning warn about likely missing dependencies.
- **It does not replace interactive Stata.** stacy wraps batch execution. For exploratory work, use Stata as usual -- and run the finished script through `stacy run` to verify it stands on its own.

---

## See Also

- [Exit Codes](./exit-codes.md) - Exit code reference
- [JSON Output](./json-output.md) - JSON schemas
- [Lockfile Format](./lockfile.md) - Lockfile specification
- [Stata Error Manual](https://www.stata.com/manuals/perror.pdf) - Official documentation
