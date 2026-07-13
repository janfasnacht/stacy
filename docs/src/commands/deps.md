# stacy deps

Show dependency tree for Stata scripts

## Synopsis

```
stacy deps <SCRIPT> [OPTIONS]
```

## Description

Analyzes a Stata script to find all files it depends on (via `do`, `run`,
`include`) and package dependencies (via `require`). Shows a tree view of the
dependency graph, detects circular dependencies, and identifies missing files.

`require` statements (including `cap require` and `capture require`) are
recognized as package dependencies and shown as leaf nodes in the tree.

A path that holds a Stata macro, such as `do "$root/prep.do"`, only points
somewhere once Stata expands the macro. stacy reads the script but does not run
it, so it lists the path as written and marks it as resolved at run time. Such a
path is not a missing file and does not fail the command.

## Arguments

| Argument | Description |
|----------|-------------|
| `<SCRIPT>` | Script to analyze (required) |

## Options

| Option | Description |
|--------|-------------|
| `--flat` | Show flat list instead of tree |

## Examples

### Show dependency tree

```bash
stacy deps main.do
```

### Show flat list

```bash
stacy deps --flat main.do
```

## Exit Codes

| Code | Meaning |
|------|--------|
| 0 | Analysis complete |
| 1 | Circular dependencies detected |
| 3 | Script not found, or a dependency is missing |

See [Exit Codes Reference](../reference/exit-codes.md) for details.

## See Also

- [stacy run](./run.md)

