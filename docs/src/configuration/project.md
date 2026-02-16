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

[packages.dependencies]
estout = "ssc"
reghdfe = "github:sergiocorreia/reghdfe"

[packages.dev]
assert = "ssc"

[scripts]
clean = "src/01_clean.do"
build = ["clean", "src/02_build.do", "src/03_analyze.do"]
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
| `log_dir` | string | `"logs"` | Directory for log files |
| `show_progress` | bool | `true` | Show progress during execution |
| `progress_interval_seconds` | int | `10` | Progress update interval |
| `max_log_size_mb` | int | `50` | Log size warning threshold |

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
# Simple: run a single script
clean = "src/01_clean.do"

# Sequential: run tasks/scripts in order
build = ["clean", "src/02_build.do", "src/03_analyze.do"]

# Parallel: run scripts concurrently
test = { parallel = ["test/test1.do", "test/test2.do"] }
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

### With Tasks

```toml
[project]
name = "analysis"

[packages.dependencies]
estout = "ssc"

[scripts]
clean = "src/01_clean.do"
build = ["clean", "src/02_build.do"]
all = ["build", "src/03_report.do"]
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
