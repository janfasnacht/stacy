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
  <a href="https://janfasnacht.com/assets/pdfs/stacy-paper.pdf"><strong>Paper (preprint) →</strong></a> · <a href="https://stacy.janfasnacht.com"><strong>Documentation →</strong></a>
</p>

---

Stata projects increasingly run inside larger workflows — a Makefile that rebuilds results when inputs change, a CI service that reruns an analysis on every commit, a replication package that must run unattended on a stranger's machine. Integration like this rests on two things Stata leaves implicit: whether a step succeeded, and what the project needs in order to run.

**The outcome is implicit.** In batch mode, Stata returns exit code 0 whether the script succeeded or failed — errors are recorded in the log and nowhere else. Build systems, schedulers, and downstream scripts proceed as if nothing went wrong.

**The environment is implicit.** User-written packages install into one global directory shared by every project; no file records what a project depends on. Each `ssc install` retrieves whatever version exists that day, so a collaborator installing months later may get a different program — and different results.

**stacy** makes both explicit. It is a task runner and package manager for Stata — one program, used from the terminal or the Stata console:

```bash
stacy run analysis.do       # exits 0 on success, 1-10 on error
stacy add estout reghdfe    # declares in stacy.toml, locks in stacy.lock
stacy install               # installs exact versions from the lockfile
```

The Stata wrappers are generated from the same schema as the CLI, so commands work unchanged in both interfaces — `. stacy run analysis.do, timeout(600)` — and `help stacy` works as expected.

| If you know... | stacy is like... |
|----------------|----------------|
| Rust | Cargo |
| Python | uv / Poetry |
| JavaScript | npm |
| R | renv |

## Installation

```bash
# macOS / Linux
curl -fsSL https://stacy.janfasnacht.com/install.sh | bash

# Homebrew
brew install janfasnacht/stacy/stacy

# From within Stata
net install stacy, from("https://stacy.janfasnacht.com/stata")
stacy setup

# From source
cargo install --git https://github.com/janfasnacht/stacy.git
```

## Quick Start

```bash
stacy init                     # Scaffold stacy.toml
stacy add estout reghdfe       # Declare and lock packages
stacy install                  # Reproduce from the lockfile
stacy doctor                   # Check your setup
```

## Learn More

- **[Quick Start](https://stacy.janfasnacht.com/quick-start.html)** — Run your first script
- **[Commands](https://stacy.janfasnacht.com/commands/)** — Full reference
- **[Build Integration](https://stacy.janfasnacht.com/guides/build-integration.html)** — Make, Snakemake, CI
- **[FAQ](https://stacy.janfasnacht.com/faq.html)** — Common questions

## License

[MIT](LICENSE)
