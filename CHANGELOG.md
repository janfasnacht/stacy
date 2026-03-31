# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Fixed

- Non-UTF-8 characters in Stata log files no longer crash the log parser

## [1.2.0] - 2026-03-15

### Added

- `stacy run --timeout <seconds>` to kill long-running scripts
- `[paths]` config section for project-local ado directories in `S_ADO`
- Post-install dependency scanning: warn about missing implicit dependencies after `stacy add` and in `stacy doctor`
- Package naming hints: suggest correct SSC package on 404 (e.g. `labmask` → `labutil`)
- Stata wrappers now expose all CLI flags (`AllowGlobal`, `Trace`, `Timeout`, `Parallel`, `Cache`, etc.)

### Changed

- `stacy init` generates minimal config (no default values the user will delete)
- Dependencies in `stacy.toml` sorted alphabetically
- `stacy init` and `stacy add` show package cache location

### Fixed

- Sync `commands.toml` schema with CLI (missing flags, stale args, missing exit code 6)

## [1.1.0] - 2026-02-22

### Added

- `net:` source type for arbitrary URL packages (`stacy add grc1leg --source net:http://www.stata.com/users/vwiggins/`)
- `local:` source type for vendored/local packages (`stacy add myutils --source local:./lib/myutils/`)
- GitHub fallback: synthesize manifest from repository tree when `.pkg` file is missing
- Post-install hints for packages with known implicit dependencies (reghdfe, ivreghdfe, ppmlhdfe, grstyle, etc.)
- `stacy doctor` now surfaces available updates (reads version check cache)
- `stacy deps` now parses `require` statements as package dependencies (including `cap require` and `capture require`)

### Changed

- Improved SSC error messages: distinguish "package not found" from "mirror gap" from "server unreachable"

## [1.0.2] - 2026-02-17

### Fixed

- Fix SSC downloads always failing: use HTTP instead of HTTPS for `fmwww.bc.edu` (the server does not support TLS, causing every `stacy add` from SSC to fall back to the GitHub mirror)

## [1.0.1] - 2026-02-16

### Fixed

- Fix `net install` from Stata: remove phantom files from `stacy.pkg` that caused r(601)
- Fix `--format stata` output: wrong global prefix, wrong quoting, bare types in syntax
- Fix `stacy init` `--name` option and `stacy task` config section reference
- Fix installation docs command name

### Added

- 24 tests for `--format stata` output and codegen correctness

### Changed

- Regenerate all Stata wrappers with fixes
- Align docs and README with paper framing

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

[1.2.0]: https://github.com/janfasnacht/stacy/compare/v1.1.0...v1.2.0
[1.1.0]: https://github.com/janfasnacht/stacy/compare/v1.0.2...v1.1.0
[1.0.2]: https://github.com/janfasnacht/stacy/compare/v1.0.1...v1.0.2
[1.0.1]: https://github.com/janfasnacht/stacy/compare/v1.0.0...v1.0.1
[1.0.0]: https://github.com/janfasnacht/stacy/releases/tag/v1.0.0
