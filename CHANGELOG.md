# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [1.0.1] - 2026-02-15

### Fixed

- Fix `net install` from Stata: remove 5 phantom files from `stacy.pkg` that caused r(601)

### Changed

- Align project description, README, and docs intro with paper framing
- Add Stata installation instructions to README and docs

## [1.0.0] - 2026-02-15

Initial public release.

### Added

- `stacy run` — Execute Stata scripts with proper error detection and exit codes
- `stacy run -c` — Run inline Stata code
- `stacy run --parallel` — Parallel execution of multiple scripts
- `stacy init` — Initialize project with `stacy.toml`
- `stacy add` / `stacy remove` — Manage dependencies
- `stacy install` — Reproducible installs from lockfile
- `stacy update` / `stacy outdated` — Keep packages current
- `stacy lock` — Generate and verify lockfile
- `stacy task` — Task runner (npm-style scripts in `stacy.toml`)
- `stacy deps` — Script dependency analysis
- `stacy env` / `stacy doctor` — Environment diagnostics
- `stacy explain` — Error code lookup
- Error codes dynamically extracted from user's Stata installation
- SSC and GitHub package sources (`github:user/repo@tag`)
- Global package cache at `~/.cache/stacy/packages/`
- `--format json` and `--format stata` output modes
- Cross-platform support: macOS, Linux, Windows

[Unreleased]: https://github.com/janfasnacht/stacy/compare/v1.0.0...HEAD
[1.0.0]: https://github.com/janfasnacht/stacy/releases/tag/v1.0.0
