# stacy lock

Generate or verify lockfile

## Synopsis

```
stacy lock [OPTIONS]
```

## Description

Generates `stacy.lock` from `stacy.toml`, recording exact versions of all packages.
The lockfile ensures reproducible installs across machines. Use `--check` in CI
to verify the lockfile is up-to-date.

## Options

| Option | Description |
|--------|-------------|
| `--check` | Verify lockfile matches stacy.toml without updating |

## Examples

### Generate lockfile

```bash
stacy lock
```

### Verify lockfile (for CI)

```bash
stacy lock --check
```

## Exit Codes

| Code | Meaning |
|------|--------|
| 0 | Success / in sync |
| 1 | Out of sync (with --check) |

See [Exit Codes Reference](../reference/exit-codes.md) for details.

## See Also

- [stacy install](./install.md)
- [stacy update](./update.md)
- [Lockfile](../configuration/lockfile.md)

