# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed

- `stacy run` streams program output to stdout live in piped mode, matching `Rscript`/`python`, instead of printing it after execution (#24). Output content is unchanged (boilerplate-stripped); stacy's status and error messages stay on stderr. Note: on failure, partial output now appears on stdout before the failure is reported.
- The batch log file is now internal: removed on success, kept on failure (path shown in the failure output). Machine-readable formats (`--format json|stata`) keep the log since their output references it. Use `--log <path>` for a durable log artifact.
- `--parallel` prints each script's output as a grouped block on completion (`==> script.do <==` headers on stdout, status lines on stderr) instead of discarding it.
- Raw `-v` streaming runs to process exit instead of stopping a few lines after the end-of-do-file marker.
- `stacy task` output streams live as well (tasks run with the same piped-mode default).

### Added

- `stacy run --log <path>`: write the raw Stata log to a chosen path (works with `--quiet` for silent file-artifact mode).

### Removed

- Dead `log_reader::is_successful_completion` (unused since the streaming rework, #65).

### Fixed

- Post-install dependency hints now comma-separate package names (`appears to need ftools, require`) instead of running them together (#63).
- Failure context no longer loads the entire log into memory to number its last 20 lines (#66).
- Log streaming (`-v`, `-vv`, TTY default) no longer hangs forever when Stata is killed (`--timeout` watchdog, external SIGKILL) or fails to launch without creating a log (#24).
- `stacy run foo.do | head` no longer panics when the downstream pipe closes.
- Log streaming now recovers from a truncated/recreated log file and no longer emits partially-written lines.
- `--trace` no longer leaks its temporary traced script and log file on exit.

## [1.2.1] - 2026-05-06

### Added

- Stata wrappers now verify that the `stacy` binary they invoke is version-compatible (#35). On mismatch, `_stacy_exec` aborts with a clear error and a `stacy_setup, force` hint instead of silently running against a stale binary. The check shells out once per Stata session (cached in `$stacy_version_checked`).

### Fixed

- Surface Stata's stderr on launch failures instead of the misleading "Log file incomplete" (#21). Distinguish "no log produced" (launch failure) from "log truncated" (killed mid-run).
- `stacy install --format stata` (and `--format json`) no longer emits a success-shaped block when checksum verification fails (#38). Status is now computed before output, so wrappers see `global stacy_status "error"` plus `global stacy_error "<msg>"` (and JSON gets matching `status`/`error`/`failed` fields) instead of a stale success preceding the non-zero exit.
- Stata wrappers failed with `command _stacy_check_version is unrecognized` because the generated `_stacy_compat.ado` defined programs that didn't match its filename and wasn't listed in `stacy.pkg` (#37). Split into one file per program.
- Parallel `stacy run` invocations on scripts that share a basename no longer collide on the log file (#20). Each run writes to a uniquely-named log in the working directory (`<stem>_<pid>_<nanos>_<n>.log`), so build orchestrators like Make `-j` and Snakemake can run same-stemmed scripts from a shared cwd safely. The path is reported in JSON output's `log_file` field.
- Non-UTF-8 characters in Stata log files no longer crash the log parser
- Update check now compares against the running binary version, not a stale cached value
- Cargo upgrade instruction now shows correct crate name (`stacy`, not `stata-cli`)

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
