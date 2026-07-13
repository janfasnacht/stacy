# stacy run

Execute a Stata script with error detection

## Synopsis

```
stacy run [SCRIPT] [OPTIONS]
```

## Description

Executes Stata scripts in batch mode and parses log files for errors. Unlike
`stata-mp -b`, returns proper exit codes that reflect whether the script
succeeded or failed—enabling integration with Make, Snakemake, and CI/CD.

The command runs Stata with `-b -q`, parses the log for error patterns, and
returns an appropriate exit code (0 for success, 1-10 for various errors).

Program output (boilerplate-stripped) streams to stdout live as Stata writes
it — like `Rscript` or `python` — so `stacy run foo.do > out.log` and pipes
behave as expected. stacy's own status and error messages go to stderr. On
failure, error details with official Stata documentation links and the log
file path are displayed. Use `-v` to stream the raw log instead, or `-q` to
suppress all output.

The batch log file is internal: removed on success, kept on failure with its
path printed in the failure output. Use `--log <path>` to keep the raw Stata
log as a durable artifact (`--quiet --log out.log` for a silent file-only run).

Multiple scripts can be run sequentially (default, fail-fast) or in parallel
(`--parallel`). Parallel mode runs all scripts regardless of failures.

To check a quick result without a script file, use `stacy run -c 'display ...'`.

In a project with a `stacy.lock`, run builds the ado-path from the lockfile and
checks it against the package cache before starting Stata. Every locked package
must be installed and must still hash to the checksum the lockfile records. A
package that is missing, or that has been modified since it was installed, fails
the run instead of executing. Every package in `stacy.lock` is on the ado-path,
so dev and test dependencies must be installed too (`stacy install --with dev,test`).

## Arguments

| Argument | Description |
|----------|-------------|
| `<SCRIPT>` | Script to execute |

## Options

| Option | Description |
|--------|-------------|
| `--allow-global` | Allow globally installed packages |
| `--cache` | Enable build cache (skip re-execution if script/deps unchanged) |
| `--cache-only` | Fail if not in cache (useful for CI) |
| `--cd` | Change to script's parent directory |
| `-c, --code` | Inline Stata code |
| `-C, --directory` | Run Stata in this directory |
| `--engine` | Stata engine to use (overrides config and auto-detection) |
| `--force` | Force rebuild even if cached |
| `-j, --jobs` | Max parallel jobs (default: CPU count) |
| `--log` | Write the raw Stata log to this path |
| `-P, --parallel` | Run scripts in parallel |
| `--profile` | Include execution metrics |
| `-q, --quiet` | Suppress output |
| `--timeout` | Kill script if it exceeds this many seconds |
| `--trace` | Enable execution tracing at given depth |
| `--verbose` | Extra output |

## Examples

### Run a script

```bash
stacy run analysis.do
```

### Multiple scripts (sequential)

Runs in order, stops on first failure

```bash
stacy run clean.do analyze.do report.do
```

### Parallel execution

Run all scripts concurrently for faster execution

```bash
stacy run --parallel *.do
stacy run --parallel -j4 a.do b.do c.do
```

### Inline code

Execute Stata code without creating a file

```bash
stacy run -c 'display 2+2'
stacy run -c 'sysuse auto, clear
summarize price'
```

### Working directory

Run in a specific directory (script paths resolved before cd)

```bash
stacy run -C reports/pilot/ table.do
stacy run --cd reports/pilot/table.do
```

### Verbose output

Stream log file in real-time

```bash
stacy run -v long_analysis.do
```

### JSON output

Machine-readable output for CI/CD

```bash
stacy run --format json analysis.do
```

### Execution tracing

Enable Stata's set trace on for debugging

```bash
stacy run --trace 2 analysis.do
stacy run --trace 2 -v analysis.do
```

### Timeout

Kill script if it takes longer than 60 seconds

```bash
stacy run --timeout 60 long_analysis.do
```

## Exit Codes

| Code | Meaning |
|------|--------|
| 0 | Success |
| 1 | Stata error (r() code detected) |
| 2 | Syntax error |
| 3 | File error (not found, permission denied) |
| 4 | Memory error |
| 5 | Internal stacy error |
| 6 | Statistical error (convergence, model problems) |
| 10 | Environment error (Stata not found) |

See [Exit Codes Reference](../reference/exit-codes.md) for details.

## See Also

- [stacy bench](./bench.md)
- [stacy task](./task.md)
- [Exit Codes](../reference/exit-codes.md)
- [Build Integration](../guides/build-integration.md)

