# Introduction

[![Version](https://img.shields.io/badge/version-0.1.0-blue)](https://github.com/janfasnacht/stacy/releases)
[![License](https://img.shields.io/badge/license-MIT-green)](https://github.com/janfasnacht/stacy/blob/main/LICENSE)
[![GitHub](https://img.shields.io/badge/github-janfasnacht/stacy-black)](https://github.com/janfasnacht/stacy)

**stacy** is a modern workflow tool for Stata. It runs scripts with proper exit codes, manages packages with lockfiles, and integrates Stata into reproducible pipelines.

For those familiar with other ecosystems:

| If you know... | stacy is like... | Key similarity |
|----------------|----------------|----------------|
| Rust | Cargo | Manifest + lockfile + build orchestration |
| Python | uv or Poetry | Project dependencies + reproducible environments |
| JavaScript | npm | package.json / package-lock.json workflow |
| R | renv | Project-local library snapshots |
| Stata | (nothing existed) | This is what stacy provides |

## The Problem

Stata's defaults leave two critical things implicit:

**Dependencies are global and unversioned.** Packages install to a global path. Versions are whatever SSC has today. When a package updates, every project using it changes silently. A replication package that worked six months ago may fail today because `reghdfe` changed its defaults.

**Execution always returns success.** Stata's batch mode (`stata-mp -b do script.do`) exits with code 0 even when scripts fail. Errors are buried in logs. Build systems, CI pipelines, and orchestration tools cannot detect failure--they proceed as if nothing went wrong.

## The Solution

**stacy** makes both sides explicit:

```bash
# Execution: proper exit codes
stacy run analysis.do
echo $?  # 0 on success, 1-10 on various errors

# Environment: lockfile-based packages
stacy add estout reghdfe    # Adds to stacy.toml, creates stacy.lock
stacy install               # Installs exact versions from lockfile
```

Now your builds stop on errors and your environments reproduce:

```makefile
results/output.dta: analysis.do data/input.dta
    stacy run analysis.do   # Stops on failure
```

## Before and After

| Without stacy | With stacy |
|-------------|----------|
| `stata -b do script.do` returns 0 even on error | `stacy run script.do` returns 1-10 on error |
| Packages are global, unversioned | `stacy.lock` pins exact versions |
| Errors buried in log files | Errors displayed with documentation links |
| "It worked on my machine" | Same versions everywhere via lockfile |
| Manual `ssc install` in scripts | `stacy install` from lockfile |

## Key Features

| Feature | What it provides |
|---------|------------------|
| Proper exit codes | Maps 182 official Stata error codes to Unix exit codes |
| Lockfile management | `stacy.lock` pins exact versions with SHA256 checksums |
| Global package cache | Packages cached at `~/.cache/stacy/packages/`, shared across projects |
| Build system integration | Works with Make, Snakemake, CI/CD |
| Single binary | No runtime dependencies, easy to deploy |

> **Note:** Error detection uses [182 official Stata error codes](https://www.stata.com/manuals/perror.pdf) from the Stata Programming Reference Manual--not heuristics.

## Incremental Adoption

Even minimal usage restores critical functionality:

| Level | What you do | What you get | Who this is for |
|-------|-------------|--------------|-----------------|
| 1 | `stacy run script.do` | Exit codes work | Anyone using Make/CI |
| 2 | `stacy init` | Project configuration | Teams wanting standards |
| 3 | `stacy add pkg` | Locked dependencies | Reproducibility needs |
| 4 | Add `[scripts]` | Task automation | Complex workflows |
| 5 | Integrate with Make/CI | Full pipeline | Publication-ready research |

## Quick Example

```bash
# Run with error detection
stacy run analysis.do

# Initialize a project and add packages
stacy init
stacy add estout reghdfe

# Install all packages from lockfile (like npm install)
stacy install

# Check system configuration
stacy doctor
```

## How to Use These Docs

- **Just want to run scripts?** Start with [Quick Start](./quick-start.md)
- **Setting up a project?** See [Project Config](./configuration/project.md)
- **Integrating with build tools?** Read [Build Integration](./guides/build-integration.md)
- **Looking up a command?** Check the [Command Reference](./commands/README.md)
- **Something not working?** Try [Troubleshooting](./troubleshooting.md) or [FAQ](./faq.md)

## Next Steps

- [Installation](./installation.md) - Get stacy installed
- [Quick Start](./quick-start.md) - Run your first script
- [FAQ](./faq.md) - Common questions answered
