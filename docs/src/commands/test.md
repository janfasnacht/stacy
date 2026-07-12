# stacy test

Run tests

## Synopsis

```
stacy test <TEST> [OPTIONS]
```

## Description

Discovers and runs test scripts from the `tests/` or `test/` directory. Tests
are Stata scripts that use assertion commands. Supports filtering, parallel
execution, and verbose output for debugging failures.

Each test runs with the project root as the working directory, so relative
paths in tests resolve from the project root regardless of where `stacy test`
is invoked. Use `--directory <dir>` to run tests in a specific directory, or
`--cd` to run each test in its own parent directory.

## Arguments

| Argument | Description |
|----------|-------------|
| `<TEST>` | Specific test to run |

## Options

| Option | Description |
|--------|-------------|
| `--cd` | Run each test in its own parent directory |
| `-C, --directory` | Run tests in this directory |
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

### Run each test in its own directory

```bash
stacy test --cd
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

