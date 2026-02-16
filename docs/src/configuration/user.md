# User Config (~/.config/stacy/config.toml)

Machine-specific settings that should **not** be committed to version control.

This is separate from [project config](./project.md) (`stacy.toml`), which lives in the project directory and is shared with collaborators.

## Location

| Platform | Path |
|----------|------|
| macOS / Linux | `~/.config/stacy/config.toml` |
| Windows | `%APPDATA%\stacy\config.toml` |

Created automatically by `stacy init` if it doesn't exist, or create it manually.

## Full Reference

```toml
# Stata binary path (overrides auto-detection)
stata_binary = "/Applications/StataNow/StataMP.app/Contents/MacOS/stata-mp"

# Check for updates on startup (default: true)
# update_check = false
```

## Fields

### stata_binary

Override Stata auto-detection with an explicit path.

```toml
# macOS
stata_binary = "/Applications/StataNow/StataMP.app/Contents/MacOS/stata-mp"

# Linux
stata_binary = "/usr/local/stata18/stata-mp"

# Windows
stata_binary = "C:\\Program Files\\Stata18\\StataMP-64.exe"
```

stacy validates the path on load. If the file doesn't exist, you'll get an error with a hint to fix it.

Precedence for Stata binary resolution (highest first):

1. `--engine` CLI flag
2. `STATA_BINARY` environment variable
3. `stata_binary` in user config
4. Auto-detection from common install paths

### update_check

Controls whether stacy checks for new releases on startup. Enabled by default.

```toml
# Disable update notifications
update_check = false
```

When enabled, stacy:

1. Reads a local cache (`~/.cache/stacy/version-check.json`) on startup
2. Prints a notification to stderr if a newer version is available
3. Refreshes the cache in the background (every 24 hours)

The check never blocks or slows down commands. The background refresh uses the [GitHub Releases API](https://docs.github.com/en/rest/releases) with a 3-second timeout.

## Environment Variables

These environment variables affect stacy behavior independently of the config file:

| Variable | Effect |
|----------|--------|
| `STATA_BINARY` | Stata binary path (overrides config file) |
| `STACY_NO_UPDATE_CHECK` | Suppress update notifications (set to any value) |
| `CI` | Suppresses update notifications automatically |
| `GITHUB_ACTIONS` | Suppresses update notifications automatically |

## Update Notification Suppression

Update notifications are automatically suppressed when:

- `update_check = false` in user config
- `STACY_NO_UPDATE_CHECK` environment variable is set
- `CI` or `GITHUB_ACTIONS` environment variable is set
- stderr is not a terminal (piped output, cron jobs, etc.)

The notification looks like:

```
Update available: v1.0.0 → v1.0.1
Run `brew upgrade stacy` to update
```

The upgrade instruction adapts to your install method (Homebrew, Cargo, or manual download).

## Cache Directory

stacy stores cached data at:

| Platform | Path |
|----------|------|
| macOS / Linux | `~/.cache/stacy/` |
| Windows | `%LOCALAPPDATA%\stacy\cache\` |

Contents:

| File | Purpose |
|------|---------|
| `packages/` | Global package cache (shared across projects) |
| `version-check.json` | Last update check result |
| `update-available` | Flag file for Stata-side notifications |

To clean everything: `rm -rf ~/.cache/stacy` (packages will be re-downloaded on next `stacy install`).

## Examples

### Minimal (just Stata path)

```toml
stata_binary = "/usr/local/stata18/stata-mp"
```

### CI / headless server

```toml
stata_binary = "/usr/local/stata18/stata-mp"
update_check = false
```

### Default (auto-detect everything)

An empty file or no file at all is valid. stacy auto-detects Stata and enables update checks.

## See Also

- [Project Config](./project.md) - Per-project settings (`stacy.toml`)
- [stacy env](../commands/env.md) - View resolved configuration
- [stacy doctor](../commands/doctor.md) - Diagnose configuration issues
- [Installation](../installation.md) - Install methods and Stata detection
