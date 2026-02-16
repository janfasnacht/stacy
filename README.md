<p align="center">
  <img src="assets/wordmark-dark.svg" alt="stacy" height="48">
  <br>
  <em>A modern workflow tool for Stata</em>
</p>

<p align="center">
  <a href="https://github.com/janfasnacht/stacy/actions/workflows/ci.yml"><img src="https://github.com/janfasnacht/stacy/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
  <a href="https://github.com/janfasnacht/stacy/releases"><img src="https://img.shields.io/github/v/release/janfasnacht/stacy" alt="Release"></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/License-MIT-blue.svg" alt="License: MIT"></a>
</p>

<p align="center">
  <a href="https://stacy.janfasnacht.com"><strong>Documentation →</strong></a>
</p>

---

**stacy** runs Stata scripts with proper exit codes, manages packages with lockfiles, and integrates Stata into reproducible pipelines.

| If you know... | stacy is like... |
|----------------|----------------|
| Rust | Cargo |
| Python | uv / Poetry |
| JavaScript | npm |
| R | renv |

## The Problem

Stata's batch mode always returns success, even when scripts fail:

```bash
stata-mp -b do analysis.do
echo $?  # Always 0, even on error
```

This breaks Make, Snakemake, CI pipelines, and coding agents that depend on exit codes.

## The Solution

```bash
stacy run analysis.do
echo $?  # 0 on success, 1-10 on error
```

Now your builds stop on failure and your environments reproduce.

## Installation

```bash
# macOS / Linux
curl -fsSL https://raw.githubusercontent.com/janfasnacht/stacy/main/install.sh | bash

# Homebrew
brew install janfasnacht/stacy/stacy

# From source
cargo install --git https://github.com/janfasnacht/stacy.git
```

## Quick Start

```bash
stacy run analysis.do          # Run with error detection
stacy init                     # Initialize project
stacy add estout reghdfe       # Add packages (creates lockfile)
stacy install                  # Install from lockfile
stacy doctor                   # Check setup
```

## Learn More

- **[Quick Start](https://stacy.janfasnacht.com/quick-start.html)** — Run your first script
- **[Commands](https://stacy.janfasnacht.com/commands/)** — Full reference
- **[Build Integration](https://stacy.janfasnacht.com/guides/build-integration.html)** — Make, Snakemake, CI
- **[FAQ](https://stacy.janfasnacht.com/faq.html)** — Common questions

## License

[MIT](LICENSE)
