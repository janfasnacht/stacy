# stacy remove

Remove packages from project

## Synopsis

```
stacy remove <PACKAGES> 
```

## Description

Removes packages from `stacy.toml` and deletes them from the local `ado/`
directory. Does not affect globally installed packages.

## Arguments

| Argument | Description |
|----------|-------------|
| `<PACKAGES>` | Package names to remove (required) |

## Examples

### Remove a package

```bash
stacy remove estout
```

### Remove multiple packages

```bash
stacy remove estout reghdfe
```

## Exit Codes

| Code | Meaning |
|------|--------|
| 0 | Success |
| 1 | No packages removed |

See [Exit Codes Reference](../reference/exit-codes.md) for details.

## See Also

- [stacy add](./add.md)
- [stacy list](./list.md)

