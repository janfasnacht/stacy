# Introduction

[![Version](https://img.shields.io/badge/version-1.0.1-blue)](https://github.com/janfasnacht/stacy/releases)
[![License](https://img.shields.io/badge/license-MIT-green)](https://github.com/janfasnacht/stacy/blob/main/LICENSE)
[![GitHub](https://img.shields.io/badge/github-janfasnacht/stacy-black)](https://github.com/janfasnacht/stacy)

Stata projects need to compose: with build systems that expect exit codes, with environments that must be reconstructed, with pipelines that mix languages. But Stata leaves two things implicit that composition requires to be explicit: the environment — what packages the project needs — and the outcome — whether execution succeeded.

**stacy** makes both explicit. Dependencies get a manifest and lockfile; execution gets proper exit codes. With these primitives, Stata projects can be versioned, automated, and reproduced.

| If you know... | stacy is like... | Key similarity |
|----------------|----------------|----------------|
| Rust | Cargo | Manifest + lockfile + build orchestration |
| Python | uv or Poetry | Project dependencies + reproducible environments |
| JavaScript | npm | package.json / package-lock.json workflow |
| R | renv | Project-local library snapshots |
| Stata | (nothing existed) | This is what stacy provides |

## The Problem

**The environment is implicit.** User-written packages install to a global path — no manifest, no lockfile, no isolation between projects. There is no way to declare dependencies and install from that declaration. Each `ssc install` retrieves whatever version exists at that moment; a collaborator installing later gets a different version entirely.

**The outcome is implicit.** Stata's batch mode (`stata-mp -b do script.do`) exits with code 0 even when scripts fail. Errors are buried in logs. Build systems, CI pipelines, and orchestration tools cannot detect failure — they proceed as if nothing went wrong.

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

A project that declares its dependencies can be installed identically elsewhere. A project that signals failure can be automated reliably:

- Journals can verify that replication packages run
- Cluster jobs fail fast instead of silently producing garbage
- Coding agents detect when their changes cause errors
- Collaborators work from the same locked environment rather than debugging "it worked on my machine"

```makefile
results/output.dta: analysis.do data/input.dta
    stacy run analysis.do   # Stops on failure
```

## At a Glance

| Without stacy | With stacy |
|-------------|----------|
| `stata -b do script.do` returns 0 even on error | `stacy run script.do` returns 1-10 on error |
| Packages are global, unversioned | `stacy.lock` pins exact versions with SHA256 checksums |
| Errors buried in log files | Errors displayed with documentation links |
| "It worked on my machine" | Same versions everywhere via lockfile |
| Manual `ssc install` in scripts | `stacy install` from lockfile |

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
