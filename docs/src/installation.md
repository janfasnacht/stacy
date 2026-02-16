# Installation

stacy is a single static binary with no runtime dependencies.

## Quick Install

**macOS / Linux:**
```bash
curl -fsSL https://raw.githubusercontent.com/janfasnacht/stacy/main/install.sh | bash
```

**Windows (PowerShell):**
```powershell
irm https://raw.githubusercontent.com/janfasnacht/stacy/main/install.ps1 | iex
```

Both install to `~/.local/bin/` (or equivalent). Ensure this directory is in your PATH.

## From within Stata

If you prefer not to leave Stata, you can install stacy directly from the Stata console:

```stata
net install stacy, from("https://raw.githubusercontent.com/janfasnacht/stacy/main/stata/")
stacy_setup
```

This downloads the Stata wrappers and installs the stacy binary to `~/.local/bin/`. After setup, all stacy commands are available as native Stata commands (e.g., `stacy run analysis.do`, `stacy add estout`).

## Other Methods

**Homebrew:**
```bash
brew install janfasnacht/stacy/stacy
```

**Cargo (from source):**
```bash
cargo install --git https://github.com/janfasnacht/stacy.git
```

**Manual download:** Get binaries from the [releases page](https://github.com/janfasnacht/stacy/releases).

## Verify Installation

```bash
stacy --version    # Check version
stacy doctor       # Check Stata detection and configuration
```

## Stata Detection

stacy automatically finds Stata in common locations:

| Platform | Searched paths |
|----------|---------------|
| macOS | `/Applications/Stata*/`, `/Applications/StataNow/` |
| Linux | `/usr/local/stata*`, `~/stata*` |
| Windows | `C:\Program Files\Stata*\` |

If Stata is elsewhere, configure via (in precedence order):

1. CLI flag: `stacy run --engine /path/to/stata-mp script.do`
2. Environment: `export STATA_BINARY=/path/to/stata-mp`
3. Config file: `~/.config/stacy/config.toml`

## Troubleshooting

Common installation issues:

| Problem | Solution |
|---------|----------|
| `stacy: command not found` | Add `~/.local/bin` to PATH, restart terminal |
| macOS blocks binary | Run `xattr -d com.apple.quarantine ~/.local/bin/stacy` |
| Stata not found | Set `STATA_BINARY` env var or config file |

See [Troubleshooting](./troubleshooting.md#installation) for detailed solutions.

## Updating

Re-run the install command, or:
```bash
brew upgrade stacy              # Homebrew
cargo install ... --force     # Cargo
```

### Update Notifications

stacy checks for new releases on startup and prints a notification to stderr if one is available:

```
Update available: v0.1.0 → v0.2.0
Run `brew upgrade stacy` to update
```

The check uses a local cache refreshed every 24 hours in the background. It never slows down commands.

Update notifications are automatically suppressed in CI environments, non-interactive sessions, and piped output. To disable globally, set `update_check = false` in [`~/.config/stacy/config.toml`](./configuration/user.md) or set the `STACY_NO_UPDATE_CHECK` environment variable.

## Uninstalling

```bash
rm ~/.local/bin/stacy           # Remove binary
rm -rf ~/.config/stacy          # Remove config (optional)
rm -rf ~/.cache/stacy           # Remove package cache (optional)
```

## Next Steps

- [Quick Start](./quick-start.md) - Run your first script
- [stacy doctor](./commands/doctor.md) - Troubleshoot detection issues
