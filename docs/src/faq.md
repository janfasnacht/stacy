# FAQ

## Getting Started

### Do I need to change my Stata scripts?

No. stacy runs your existing `.do` files unchanged.

### Can I use stacy with existing projects?

Yes. Run `stacy init` in any directory to create `stacy.toml`. Existing scripts work as before.

### What if I don't use Make or Snakemake?

stacy works standalone. You get error detection, lockfile packages, and the task runner:

```toml
[scripts]
clean = "clean_data.do"
all = ["clean", "analysis.do"]
```

```bash
stacy task all
```

### How is stacy different from batch mode?

| Batch mode | stacy |
|------------|-----|
| Always exits 0 | Exits 1-10 on errors |
| Errors in log only | Errors shown with docs link |
| No package management | Lockfile pins versions |

## Package Management

### How is the lockfile different from net install?

`net install` gets whatever version exists today. The lockfile records exact versions with checksums. `stacy install` reproduces those exact versions.

### Can I use packages not on SSC?

Yes:

```bash
stacy add github:sergiocorreia/reghdfe
stacy add github:user/repo@v1.2
```

### What happens if SSC is down?

Packages are cached at `~/.cache/stacy/packages/`. Cached packages work offline.

### Where are packages stored?

Global cache at `~/.cache/stacy/packages/`, organized by name/version. stacy sets `S_ADO` at runtime for project isolation.

## Technical

### How does stacy detect errors?

Runs Stata with `-b -q`, parses the log for `r()` patterns, returns appropriate exit code. See [How It Works](./reference/how-it-works.md#error-detection).

### Does stacy modify my Stata installation?

No. stacy manages packages in its own cache and sets `S_ADO` at runtime.

### What are the exit codes?

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | Stata error |
| 2 | Syntax error |
| 3 | File error |
| 4 | Memory error |
| 5 | Internal error |
| 10 | Environment error |

## Updates

### Does stacy check for updates automatically?

Yes. On startup, stacy checks a local cache for available updates and prints a notification to stderr. The cache is refreshed in the background every 24 hours via the GitHub Releases API. This never blocks or slows down commands.

### How do I disable update checks?

Either set `update_check = false` in `~/.config/stacy/config.toml`, or set the `STACY_NO_UPDATE_CHECK` environment variable. Update checks are also suppressed automatically in CI and non-interactive environments. See [User Config](./configuration/user.md#update-notification-suppression) for details.

## Compatibility

### Does stacy work on Windows?

Yes. Windows, macOS, and Linux.

### Which Stata versions are supported?

Stata 14+ (MP, SE, BE, StataNow).

### Can I use stacy in Docker?

Yes. Set `STATA_BINARY` environment variable.

### Does stacy work with Stata GUI?

stacy is for command-line workflows (batch mode). Use Stata directly for interactive work.
