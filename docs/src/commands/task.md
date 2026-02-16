# stacy task

Run tasks from stacy.toml

## Synopsis

```
stacy task <TASK> [OPTIONS]
```

## Description

Runs named tasks defined in `stacy.toml`. Tasks are like npm scripts—define
sequences of commands once and run them by name. Use `--list` to see available
tasks.

## Arguments

| Argument | Description |
|----------|-------------|
| `<TASK>` | Task name to run |

## Options

| Option | Description |
|--------|-------------|
| `--frozen` | Fail if lockfile doesn't match stacy.toml |
| `--list` | List available tasks |

## Examples

### Run a task

```bash
stacy task build
```

### List available tasks

```bash
stacy task --list
```

## Exit Codes

| Code | Meaning |
|------|--------|
| 0 | Success |
| 1 | Task failed |
| 5 | Task not found |

See [Exit Codes Reference](../reference/exit-codes.md) for details.

## See Also

- [stacy run](./run.md)
- [Project](../configuration/project.md)

