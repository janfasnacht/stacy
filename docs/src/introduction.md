# Introduction

[![Version](https://img.shields.io/github/v/release/janfasnacht/stacy?label=version&color=blue)](https://github.com/janfasnacht/stacy/releases)
[![License](https://img.shields.io/badge/license-MIT-green)](https://github.com/janfasnacht/stacy/blob/main/LICENSE)
[![GitHub](https://img.shields.io/badge/github-janfasnacht/stacy-black)](https://github.com/janfasnacht/stacy)

Stata projects increasingly run inside larger workflows — a Makefile that rebuilds results when inputs change, a CI service that reruns an analysis on every commit, a replication package that must run unattended on a stranger's machine. Integration like this rests on two things Stata leaves implicit: whether a step succeeded, and what the project needs in order to run.

**stacy** makes both explicit. It is a task runner and package manager for Stata: `stacy run` executes a script, parses the log, and returns a proper exit code, while `stacy add` and `stacy install` maintain a manifest and lockfile for dependencies. Every command works from both the terminal and the Stata console. With these primitives, Stata projects can be automated, versioned, and reproduced.

| If you know... | stacy is like... | Key similarity |
|----------------|----------------|----------------|
| Rust | Cargo | Manifest + lockfile + build orchestration |
| Python | uv or Poetry | Project dependencies + reproducible environments |
| JavaScript | npm | package.json / package-lock.json workflow |
| R | renv | Project-local library snapshots |
| Stata | (nothing existed) | This is what stacy provides |

## The Problem

**The outcome is implicit.** Stata's batch mode (`stata-mp -b do script.do`) exits with code 0 even when scripts fail. Errors are buried in logs. Build systems, CI pipelines, and downstream scripts cannot detect failure — they proceed as if nothing went wrong.

**The environment is implicit.** User-written packages install to a global path — no manifest, no lockfile, no isolation between projects. There is no way to declare dependencies and install from that declaration. Each `ssc install` retrieves whatever version exists at that moment; a collaborator installing later gets a different version entirely.

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
- Collaborators work from the same locked environment rather than debugging "it worked on my machine"

```makefile
results/output.dta: analysis.do data/input.dta
    stacy run analysis.do   # Stops on failure
```

## One Tool, Two Interfaces

stacy is a single binary you can drive from the terminal or from inside Stata:

```bash
# Terminal
$ stacy run analysis.do --timeout 600
```

```stata
. stacy run analysis.do, timeout(600)
```

The Stata commands are thin wrappers around the same binary, generated from the same command schema as the command-line interface, so the two never drift apart. `help stacy` works as for any other Stata package. See [Installation](./installation.md#from-within-stata) for setup.

## What stacy Manages (and What It Doesn't)

stacy makes two things explicit -- execution outcomes and the package environment -- and stays out of everything else:

| stacy manages | stacy does not manage |
|---------------|----------------------|
| Whether a script succeeded (exit codes) | Orchestrating large pipelines (use Make/Snakemake on top) |
| Which packages a project needs (manifest) | The Stata version itself (use Docker for full-stack pinning) |
| Which exact versions are installed (lockfile + checksums) | Data files or other languages' environments |
| Where Stata looks for packages at runtime (`S_ADO`) | Transitive dependencies (Stata packages don't declare them reliably) |

This makes stacy a small, composable piece of infrastructure rather than a framework: it slots under whatever build system, scheduler, or CI service you already use.

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
