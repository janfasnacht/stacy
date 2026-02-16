# stacy bench

Benchmark script execution

## Synopsis

```
stacy bench <SCRIPT> [OPTIONS]
```

## Description

Runs a Stata script multiple times and reports timing statistics (mean, median,
min, max, stddev). Includes warmup runs by default to account for JIT and
caching effects.

## Arguments

| Argument | Description |
|----------|-------------|
| `<SCRIPT>` | Stata script to benchmark (required) |

## Options

| Option | Description |
|--------|-------------|
| `--no_warmup` | Skip warmup runs |
| `-q, --quiet` | Suppress progress output |
| `-n, --runs` | Number of measured runs |
| `-w, --warmup` | Number of warmup runs |

## Examples

### Benchmark a script

```bash
stacy bench analysis.do
```

### Custom run count

```bash
stacy bench -n 20 analysis.do
```

## Exit Codes

| Code | Meaning |
|------|--------|
| 0 | Success |
| 1 | Script failed during benchmark |
| 3 | Script not found |

See [Exit Codes Reference](../reference/exit-codes.md) for details.

## See Also

- [stacy run](./run.md)

