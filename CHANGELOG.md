# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Fixed

- Schema/CLI divergence: sync `commands.toml` with actual CLI implementation
  - Add missing `run` flags: `--parallel`, `-j/--jobs`, `--cache`, `--force`, `--cache-only`, `--engine`
  - Add missing exit code 6 (statistical error, r(400)-r(499)) to schema and generated docs
  - Remove stale `install` args (`package`, `from`) that no longer exist in CLI
  - Replace removed `init --yes` with `--interactive` in schema
  - Add missing `doctor --refresh` to schema

### Added

- `stacy run --timeout <seconds>` flag to kill long-running scripts (SIGTERM → SIGKILL escalation)
- Stata wrappers: `Timeout(integer)` option for `stacy_run`
- Stata wrappers: `AllowGlobal` and `Trace(integer)` options for `stacy_run`
- Stata wrappers: `With(string)`, `FROZEN`, and `NOVerify` options for `stacy_install`
- Stata wrappers: `PARALLEL`, `Jobs(integer)`, `Cache`, `Force`, `CacheOnly`, `Engine(string)` options for `stacy_run`
- `[paths]` config section for project-local ado directories prepended to `S_ADO`
- Post-install dependency scanning: after `stacy add`, scans `.ado` files for `require`/`which`/`findfile` patterns and warns about missing dependencies
- `stacy doctor` now checks all installed packages for missing implicit dependencies
- `stacy init` and `stacy add` now show the package cache location in output

### Changed

- `stacy init` template trimmed to essentials (`[project]` and `[packages.dependencies]` examples only)
- Package dependencies in `stacy.toml` are now written in alphabetical order

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

[Unreleased]: https://github.com/janfasnacht/stacy/compare/v1.1.0...HEAD
[1.1.0]: https://github.com/janfasnacht/stacy/compare/v1.0.2...v1.1.0
[1.0.2]: https://github.com/janfasnacht/stacy/compare/v1.0.1...v1.0.2
[1.0.1]: https://github.com/janfasnacht/stacy/compare/v1.0.0...v1.0.1
[1.0.0]: https://github.com/janfasnacht/stacy/releases/tag/v1.0.0
