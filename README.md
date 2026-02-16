<p align="center">
  <img src="assets/wordmark-dark.svg" alt="stacy" height="48">
  <br>
  <em>Reproducible Stata projects through lockfiles and exit codes</em>
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

Stata projects need to compose: with build systems that expect exit codes, with environments that must be reconstructed, with pipelines that mix languages. But Stata leaves two things implicit that composition requires to be explicit.

**The environment is implicit.** Packages install to a global path — no manifest, no lockfile, no isolation between projects. Each `ssc install` retrieves whatever version exists today; a collaborator installing later gets a different version entirely.

**The outcome is implicit.** Batch mode (`stata-mp -b do script.do`) returns exit code 0 even when scripts fail. Build systems, CI pipelines, and coding agents cannot detect failure — they proceed as if nothing went wrong.

**stacy** makes both explicit:

```bash
# Proper exit codes
stacy run analysis.do
echo $?  # 0 on success, 1-10 on error

# Lockfile-based dependencies
stacy add estout reghdfe    # Declares in stacy.toml, locks in stacy.lock
stacy install               # Installs exact versions from lockfile
```

| If you know... | stacy is like... |
|----------------|----------------|
| Rust | Cargo |
| Python | uv / Poetry |
| JavaScript | npm |
| R | renv |

## Installation

```bash
# macOS / Linux
curl -fsSL https://raw.githubusercontent.com/janfasnacht/stacy/main/install.sh | bash

# Homebrew
brew install janfasnacht/stacy/stacy

# From within Stata
net install stacy, from("https://raw.githubusercontent.com/janfasnacht/stacy/main/stata/")
stacy_setup

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
