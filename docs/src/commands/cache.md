# stacy cache info

Show cache statistics

## Synopsis

```
stacy cache info 
```

## Description

Displays information about the build cache used by `stacy run --cache`. Shows
number of cached entries and approximate size. The cache stores results to
skip re-execution of unchanged scripts.

Use `stacy cache clean` to remove old entries.

## Examples

### Show cache info

```bash
stacy cache info
```

### Clean old entries

```bash
stacy cache clean
stacy cache clean --older-than 7
```

## Exit Codes

| Code | Meaning |
|------|--------|
| 0 | Success |
| 10 | Not in project |

See [Exit Codes Reference](../reference/exit-codes.md) for details.

## See Also

- [stacy run](./run.md)

