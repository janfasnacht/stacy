# Quick Start

## 1. Verify Setup

```bash
stacy doctor
```

This checks that stacy can find Stata. If it fails, see [Troubleshooting](./troubleshooting.md).

## 2. Run a Script

```bash
stacy run analysis.do
```

On success: exit code 0.
On failure: non-zero exit code with error details.

```
❌ Failed: analysis.do (0.08s)

   Error: r(199) - unrecognized command
   See: https://www.stata.com/manuals/perror.pdf#r199
```

That's it. Your existing scripts work unchanged.

## 3. Initialize a Project

```bash
stacy init
```

Creates `stacy.toml` (project configuration) and `.gitignore`.

## 4. Add Packages

```bash
stacy add estout reghdfe
```

This adds packages to `stacy.toml`, downloads them to the cache, and creates `stacy.lock` with exact versions.

To install from an existing lockfile:

```bash
stacy install
```

Your dependencies are now pinned. Anyone running `stacy install` gets the same versions.

## 5. Define Tasks

Add a `[scripts]` section to `stacy.toml`:

```toml
[scripts]
clean = "clean_data.do"
analysis = "run_analysis.do"
all = ["clean", "analysis"]
```

Run tasks by name:

```bash
stacy task clean       # Run one script
stacy task all         # Run sequence
```

This replaces `master.do` with explicit, named tasks that stop on error.

For complex pipelines with dependency tracking, see [Build Integration](./guides/build-integration.md).

## Common Options

```bash
stacy run -v analysis.do       # Stream log output
stacy run -c 'display 2+2'     # Run inline code
stacy run --format json ...    # Machine-readable output
```

## Next Steps

- [Commands](./commands/README.md) - Full reference
- [Configuration](./configuration/project.md) - Project settings
- [FAQ](./faq.md) - Common questions
