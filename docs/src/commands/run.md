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

By default, clean output is shown after execution (boilerplate stripped). On
failure, error details with official Stata documentation links and the log file
path are displayed. Use `-v` to stream the raw log in real-time instead, or
`-q` to suppress all output.

Multiple scripts can be run sequentially (default, fail-fast) or in parallel
(`--parallel`). Parallel mode runs all scripts regardless of failures.

For interactive use where you want to quickly check a result, see `stacy eval`.

## Arguments

| Argument | Description |
|----------|-------------|
| `<SCRIPT>` | Script to execute |

## Options

| Option | Description |
|--------|-------------|
| `--cd` | Change to script's parent directory |
| `-c, --code` | Inline Stata code |
| `-C, --directory` | Run Stata in this directory |
| `--profile` | Include execution metrics |
| `-q, --quiet` | Suppress output |
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
stt run --trace 2 analysis.do
stt run --trace 2 -v analysis.do
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
| 10 | Environment error (Stata not found) |

See [Exit Codes Reference](../reference/exit-codes.md) for details.

## See Also

- [stacy eval](./eval.md)
- [stacy bench](./bench.md)
- [stacy task](./task.md)
- [Exit Codes](../reference/exit-codes.md)
- [Build Integration](../guides/build-integration.md)

