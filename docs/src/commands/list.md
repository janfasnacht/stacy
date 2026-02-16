# stacy list

List installed packages

## Synopsis

```
stacy list [OPTIONS]
```

## Description

Shows all packages installed in the current project with their versions and
sources. Use `--tree` to group by dependency type (production, dev, test).

## Options

| Option | Description |
|--------|-------------|
| `--tree` | Group packages by dependency type |

## Examples

### List packages

```bash
stacy list
```

### List by dependency group

```bash
stacy list --tree
```

## Exit Codes

| Code | Meaning |
|------|--------|
| 0 | Success |

See [Exit Codes Reference](../reference/exit-codes.md) for details.

## See Also

- [stacy outdated](./outdated.md)
- [stacy add](./add.md)

