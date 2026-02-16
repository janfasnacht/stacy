# stacy add

Add packages to project

## Synopsis

```
stacy add <PACKAGES> [OPTIONS]
```

## Description

Adds packages to your project's `stacy.toml` and installs them. Supports SSC
(default) and GitHub sources. Packages are recorded with versions for
reproducible installs via `stacy install`.

## Arguments

| Argument | Description |
|----------|-------------|
| `<PACKAGES>` | Package names to add (required) |

## Options

| Option | Description |
|--------|-------------|
| `--dev` | Add as development dependency |
| `--source` | Package source: ssc or github:user/repo[@ref] |
| `--test` | Add as test dependency |

## Examples

### Add from SSC

```bash
stacy add estout
stacy add estout reghdfe
```

### Add from GitHub

```bash
stacy add --source github:sergiocorreia/ftools ftools
```

### Add as dev dependency

```bash
stacy add --dev assert
```

## Exit Codes

| Code | Meaning |
|------|--------|
| 0 | Success |
| 1 | All packages failed to add |

See [Exit Codes Reference](../reference/exit-codes.md) for details.

## See Also

- [stacy remove](./remove.md)
- [stacy install](./install.md)
- [stacy update](./update.md)

