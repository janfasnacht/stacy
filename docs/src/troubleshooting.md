# Troubleshooting

## Installation

### stacy: command not found

Add `~/.local/bin` to PATH:

```bash
export PATH="$HOME/.local/bin:$PATH"  # add to ~/.zshrc
```

Restart terminal.

### Installation script fails

```bash
mkdir -p ~/.local/bin
curl -fsSL https://raw.githubusercontent.com/janfasnacht/stacy/main/install.sh | bash
```

### macOS blocks the binary

```bash
xattr -d com.apple.quarantine ~/.local/bin/stacy
```

## Stata Detection

### Stata not found

Set path explicitly:

```bash
export STATA_BINARY=/path/to/stata-mp
```

Or create `~/.config/stacy/config.toml`:

```toml
stata_binary = "/path/to/stata-mp"
```

Run `stacy doctor` to verify.

### Wrong Stata version detected

```bash
stacy env  # see what was found
export STATA_BINARY=/path/to/correct/stata-mp
```

### Permission denied running Stata

```bash
chmod +x /path/to/stata-mp
```

## Runtime

### Exit code doesn't match error

Possible causes:
- Script uses `capture` to suppress errors
- stacy only catches `r()` errors, not warnings
- Custom program writes non-standard messages

[Report missed errors](https://github.com/janfasnacht/stacy/issues).

### Script works in GUI but fails with stacy

**Working directory:** stacy runs from current shell directory.

**Missing packages:** Run `stacy add packagename`.

**Profile.do:** stacy uses `-q` flag, skipping profile.do.

### Log file not found

Check write permissions. Try `stacy run -v script.do` for verbose output.

## Packages

### Failed to download from SSC

Check network: `curl -I https://www.stata.com`

Retry: `stacy add packagename`

### Checksum mismatch

SSC updated the package: `stacy update packagename`

Or clear cache: `stacy cache packages clean && stacy install`

### Package installed but Stata can't find it

```bash
stacy list  # verify it's listed
```

Check package docs for unlisted dependencies.

## Lockfile

### Conflicts after git merge

Resolve `stacy.toml` conflicts first, then:

```bash
stacy lock
```

### Lockfile out of sync

```bash
stacy lock          # regenerate
stacy lock --check  # verify
```

### Different results on teammate's machine

1. Verify lockfile committed: `git status stacy.lock`
2. Both have same lockfile: `git diff origin/main -- stacy.lock`
3. Reinstall: `stacy install`
4. Check Stata versions: `stacy env`

## Update Notifications

### How do I disable update notifications?

Set `update_check = false` in `~/.config/stacy/config.toml`:

```toml
update_check = false
```

Or set an environment variable:

```bash
export STACY_NO_UPDATE_CHECK=1
```

### Notifications don't appear

Update notifications are suppressed when:

- stderr is not a terminal (piped output, cron, scripts)
- `CI` or `GITHUB_ACTIONS` environment variable is set
- `STACY_NO_UPDATE_CHECK` environment variable is set
- `update_check = false` in user config

If you want to check manually: `stacy --version` and compare with the [releases page](https://github.com/janfasnacht/stacy/releases).

### Notification shows wrong upgrade command

stacy detects the install method from the binary path. If detection is wrong (e.g., after moving the binary), the fallback shows the GitHub releases URL.

## Getting Help

1. Run `stacy doctor`
2. Run failing command with `-v`
3. [Open an issue](https://github.com/janfasnacht/stacy/issues)
