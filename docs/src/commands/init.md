# stacy init

Initialize new stacy project

## Synopsis

```
stacy init <PATH> [OPTIONS]
```

## Description

Creates a new stacy project with standard directory structure and configuration.
This sets up `stacy.toml` for project settings and `ado/` for local packages.

Run this in an existing directory or specify a path to create a new one.

## Arguments

| Argument | Description |
|----------|-------------|
| `<PATH>` | Project directory (default: current) |

## Options

| Option | Description |
|--------|-------------|
| `--force` | Overwrite existing files |
| `-i, --interactive` | Interactive mode: prompt for project details and packages |

## Examples

### Initialize in current directory

```bash
stacy init
```

### Initialize in new directory

```bash
stacy init my-project
```

## Exit Codes

| Code | Meaning |
|------|--------|
| 0 | Project created successfully |
| 1 | Initialization failed |

See [Exit Codes Reference](../reference/exit-codes.md) for details.

## See Also

- [stacy add](./add.md)
- [stacy install](./install.md)
- [Project](../configuration/project.md)

