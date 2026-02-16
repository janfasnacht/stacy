# stacy explain

Look up Stata error code details

## Synopsis

```
stacy explain <CODE> 
```

## Description

Displays detailed information about Stata error codes. Includes the error
name, category, full description from the Stata Programming Manual, and
link to official documentation. Useful for understanding r() return codes.

## Arguments

| Argument | Description |
|----------|-------------|
| `<CODE>` | Error code (e.g., 199 or r(199)) (required) |

## Examples

### Look up error code

```bash
stacy explain 199
```

### Using r() syntax

```bash
stacy explain r(601)
```

## Exit Codes

| Code | Meaning |
|------|--------|
| 0 | Error code found |
| 1 | Unknown error code |

See [Exit Codes Reference](../reference/exit-codes.md) for details.

## See Also

- [stacy run](./run.md)
- [Exit Codes](../reference/exit-codes.md)

