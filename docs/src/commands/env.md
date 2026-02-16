# stacy env

Show environment configuration

## Synopsis

```
stacy env 
```

## Description

Displays the current stacy configuration: Stata binary location, project root,
path settings, and adopath order. Useful for debugging configuration issues.

## Examples

### Show environment

```bash
stacy env
```

## Exit Codes

| Code | Meaning |
|------|--------|
| 0 | Success |
| 10 | Environment error (Stata not found) |

See [Exit Codes Reference](../reference/exit-codes.md) for details.

## See Also

- [stacy doctor](./doctor.md)

