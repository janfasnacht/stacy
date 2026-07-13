# Project Config (stacy.toml)

The `stacy.toml` file configures project-level settings. Created by [`stacy init`](../commands/init.md).

## Location

```
my-project/
├── stacy.toml    # Project config (this file)
├── stacy.lock    # Package lockfile
└── ...
```

## Full Reference

```toml
[project]
name = "my-analysis"
authors = ["Jane Doe <jane@university.edu>"]
description = "Economic analysis of market dynamics"
url = "https://github.com/user/my-analysis"

[run]
log_dir = "logs"
show_progress = true
progress_interval_seconds = 10
max_log_size_mb = 50

[paths]
ado = ["ado", "lib/custom"]

[packages.dependencies]
estout = "ssc"
reghdfe = "github:sergiocorreia/reghdfe"

[packages.dev]
assert = "ssc"

[scripts]
clean = "src/01_clean.do"
analyze = "src/02_analyze.do"
build = ["clean", "analyze"]
```

## Sections

### [project]

Project metadata. Optional but recommended for collaborative projects.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `name` | string | directory name | Project name |
| `authors` | array | `[]` | List of authors |
| `description` | string | none | Project description |
| `url` | string | none | Project URL |

### [run]

Settings for [`stacy run`](../commands/run.md) command.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `log_dir` | string | `"logs"` | Directory for kept log files, relative to the project root |
| `show_progress` | bool | `true` | Show progress during execution |
| `progress_interval_seconds` | int | `10` | Progress update interval |
| `max_log_size_mb` | int | `50` | Log size warning threshold |

Batch logs are internal: a script that succeeds leaves none behind. A script that
fails keeps its log, and `log_dir` is where it goes — for `stacy run` as well as
the scripts run by `stacy task`, `stacy test` and `stacy bench`. The directory is
created when the first log needs it. `stacy run --log <path>` overrides `log_dir`
for that run.

### [paths]

Local ado directories to prepend to S_ADO. Paths are relative to the project root and resolved to absolute paths at runtime. This lets strict mode work with project-local `.ado` programs without needing `adopath ++` boilerplate.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `ado` | array | `[]` | Local ado directories |

```toml
[paths]
ado = ["ado", "lib/custom"]
```

Directories are prepended to S_ADO in declared order, before package cache paths. Non-existent paths produce a warning in `stacy doctor` but are not a hard error.

### [packages.dependencies], [packages.dev], [packages.test]

Package dependencies by group. Format: `package_name = "source"`.

```toml
[packages.dependencies]
estout = "ssc"                              # From SSC
reghdfe = "github:sergiocorreia/reghdfe"    # From GitHub

[packages.dev]
assert = "ssc"

[packages.test]
mytest = "ssc"
```

Sources:
- `"ssc"` - Install from SSC
- `"github:user/repo"` - Install from GitHub (default branch)
- `"github:user/repo@tag"` - Install from GitHub at specific tag/branch

### [scripts]

Task definitions for [`stacy task`](../commands/task.md). Supports three formats:

```toml
[scripts]
# Simple: a task is a script path
clean = "src/01_clean.do"
analyze = "src/02_analyze.do"
tables = "src/03_tables.do"

# Sequential: an array runs tasks (or script paths) in order
build = ["clean", "analyze", "src/03_tables.do"]

# Parallel: run tasks (or script paths) concurrently
outputs = { parallel = ["analyze", "tables"] }
```

Array entries and `parallel` lists may name other tasks or point directly at script paths. The object form also supports `script`, `args`, and `description` keys:

```toml
analyze = { script = "src/02_analyze.do", description = "Main estimates" }
```

## Important Notes

### Stata Binary

stacy auto-detects Stata in common locations. If detection fails, configure manually:

```bash
# Environment variable (recommended)
export STATA_BINARY=/path/to/stata-mp

# Or per-command
stacy run --engine /path/to/stata-mp script.do

# Or user config file (see User Config docs)
stata_binary = "/path/to/stata-mp"
```

Run `stacy doctor` to verify detection.

### All Fields are Optional

An empty `stacy.toml` is valid:

```toml
# This file can be empty - all fields have defaults
```

### Paths are Relative

Paths in `stacy.toml` are relative to the project root (e.g., script paths in `[scripts]`).

### Global Package Cache

Packages are installed to a global cache at `~/.cache/stacy/packages/` and shared across all projects. Use `stacy cache packages list` to view cached packages.

## Examples

### Minimal

```toml
[project]
name = "analysis"
```

### With Packages

```toml
[project]
name = "analysis"

[packages.dependencies]
estout = "ssc"
reghdfe = "github:sergiocorreia/reghdfe"
```

### With Local Ado Paths

```toml
[project]
name = "analysis"

[paths]
ado = ["ado"]

[packages.dependencies]
estout = "ssc"
```

### With Tasks

```toml
[project]
name = "analysis"

[packages.dependencies]
estout = "ssc"

[scripts]
clean = "src/01_clean.do"
build = "src/02_build.do"
report = "src/03_report.do"
all = ["clean", "build", "report"]
```

### CI-Friendly

```toml
[project]
name = "analysis"

[run]
show_progress = false  # Cleaner CI logs
```

## Validation

stacy validates the config on load. Invalid TOML causes an error:

```
Error: Failed to parse stacy.toml: expected `=` at position 15-16
```

Use [`stacy env`](../commands/env.md) to verify your configuration is loaded correctly.

## See Also

- [User Config](./user.md) - Machine-specific settings (`~/.config/stacy/config.toml`)
- [stacy init](../commands/init.md) - Create stacy.toml
- [stacy env](../commands/env.md) - View loaded configuration
