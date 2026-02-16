# stacy test

Run tests

## Synopsis

```
stacy test <TEST> [OPTIONS]
```

## Description

Discovers and runs test scripts from the `test/` directory. Tests are Stata
scripts that use assertion commands. Supports filtering, parallel execution,
and verbose output for debugging failures.

## Arguments

| Argument | Description |
|----------|-------------|
| `<TEST>` | Specific test to run |

## Options

| Option | Description |
|--------|-------------|
| `-f, --filter` | Filter tests by pattern |
| `--list` | List tests without running |
| `--parallel` | Run tests in parallel |
| `-q, --quiet` | Suppress progress output |
| `-V, --verbose` | Show full log context for failures |

## Examples

### Run all tests

```bash
stacy test
```

### Run specific test

```bash
stacy test test_regression
```

### Filter tests

```bash
stacy test -f 'regression*'
```

## Exit Codes

| Code | Meaning |
|------|--------|
| 0 | All tests passed |
| 1 | One or more tests failed |
| 5 | Test not found |

See [Exit Codes Reference](../reference/exit-codes.md) for details.

## See Also

- [stacy run](./run.md)

