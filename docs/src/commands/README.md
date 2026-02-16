# Commands Overview

stacy provides commands for Stata execution, package management, and project workflows.

## By Category

**Execution**
- [`stacy run`](./run.md) - Execute scripts with error detection
- [`stacy bench`](./bench.md) - Benchmark script performance
- [`stacy task`](./task.md) - Run tasks from stacy.toml
- [`stacy test`](./test.md) - Run tests

**Packages**
- [`stacy add`](./add.md) / [`remove`](./remove.md) / [`update`](./update.md) - Manage dependencies
- [`stacy install`](./install.md) - Install from lockfile
- [`stacy list`](./list.md) / [`outdated`](./outdated.md) - View package status
- [`stacy lock`](./lock.md) - Generate/verify lockfile

**Project**
- [`stacy init`](./init.md) - Initialize new project
- [`stacy deps`](./deps.md) - Analyze script dependencies

**Utility**
- [`stacy env`](./env.md) - Show configuration
- [`stacy doctor`](./doctor.md) - System diagnostics
- [`stacy explain`](./explain.md) - Look up error codes
- [`stacy cache`](./cache.md) - Manage build cache

## Getting Help

```bash
stacy --help          # General help
stacy run --help      # Command-specific help
```

See [Exit Codes](../reference/exit-codes.md) for exit code reference.
