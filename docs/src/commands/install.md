# stacy install

Install packages from lockfile or SSC/GitHub

## Synopsis

```
stacy install <PACKAGE> [OPTIONS]
```

## Description

Installs packages defined in `stacy.lock` (or `stacy.toml` if no lockfile exists).
This ensures reproducible environments by installing exact versions from the
lockfile. Can also install individual packages directly from SSC or GitHub.

## Arguments

| Argument | Description |
|----------|-------------|
| `<PACKAGE>` | Package name (optional) |

## Options

| Option | Description |
|--------|-------------|
| `--from` | Source: ssc or github:user/repo |

## Examples

### Install from lockfile

Install all packages at locked versions

```bash
stacy install
```

### Install specific package

```bash
stacy install estout
```

## Exit Codes

| Code | Meaning |
|------|--------|
| 0 | Success |
| 1 | Installation failed |
| 3 | Package not found |

See [Exit Codes Reference](../reference/exit-codes.md) for details.

## See Also

- [stacy add](./add.md)
- [stacy lock](./lock.md)
- [stacy list](./list.md)

