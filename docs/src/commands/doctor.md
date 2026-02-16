# stacy doctor

Run system diagnostics

## Synopsis

```
stacy doctor 
```

## Description

Checks your system configuration and reports any issues. Verifies Stata
installation, project detection, and write permissions. Run this first
when troubleshooting.

## Examples

### Run diagnostics

```bash
stacy doctor
```

## Exit Codes

| Code | Meaning |
|------|--------|
| 0 | All checks passed |
| 1 | One or more checks failed |

See [Exit Codes Reference](../reference/exit-codes.md) for details.

## See Also

- [stacy env](./env.md)

