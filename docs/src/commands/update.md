# stacy update

Update packages to latest versions

## Synopsis

```
stacy update <PACKAGES> [OPTIONS]
```

## Description

Checks for newer versions of installed packages and updates them. Updates both
`stacy.toml` and `stacy.lock` to reflect new versions. Use `--dry-run` to preview
changes without applying them.

## Arguments

| Argument | Description |
|----------|-------------|
| `<PACKAGES>` | Package names to update (default: all) |

## Options

| Option | Description |
|--------|-------------|
| `--dry_run` | Show what would be updated without making changes |

## Examples

### Update all packages

```bash
stacy update
```

### Update specific package

```bash
stacy update estout
```

### Preview updates

```bash
stacy update --dry-run
```

## Exit Codes

| Code | Meaning |
|------|--------|
| 0 | Success |
| 1 | All updates failed |

See [Exit Codes Reference](../reference/exit-codes.md) for details.

## See Also

- [stacy outdated](./outdated.md)
- [stacy install](./install.md)
- [stacy lock](./lock.md)

