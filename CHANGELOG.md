# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Fixed

- A task that defines no work — a table without `script` or `parallel` (e.g. a typo'd key, which TOML parsing silently drops) or an empty array — now fails with a config error instead of succeeding as a no-op (#92).

## [1.4.0] - 2026-07-13

### Added

- Post-install dependency hints now also read the `Requires:` line of a package's `.pkg` manifest, catching author-declared SSC dependencies that static `.ado` scanning misses (#78).
- `stacy add` warns when a package's manifest declares a newer minimum Stata version than the one stacy last detected (#82).
- `stacy test -C <dir>`/`--directory` and `--cd`: control the test working directory, matching `stacy run` (#85).

### Changed

- `stacy test` runs tests with the project root as the working directory regardless of where it's invoked (previously the inherited directory) (#85).

### Fixed

- `.pkg` manifests with bare `\r` (classic-Mac) line endings no longer parse as a title with no files (#79).
- `stacy task` from the Stata console no longer fails with `r(199)`: machine-readable formats no longer stream script output to stdout (#84).
- Line breaks in `--format stata` string values are replaced with spaces.

## [1.3.1] - 2026-07-10

### Added

- `stacy lock --refresh`: recompute lockfile checksums from the installed cache.

### Fixed

- Packages whose `.pkg` manifest lists a file twice (e.g. reghdfe) no longer fail checksum verification (#68). Run `stacy lock --refresh` to repair lockfile entries recorded by older versions.

## [1.3.0] - 2026-07-09

### Changed

- `stacy run` streams program output to stdout live in piped mode, like `Rscript` (#24). Same content as before, just live; status and errors stay on stderr. `stacy task` streams too.
- The log file is now internal: removed on success, kept on failure. Machine-readable formats keep it. Use `--log <path>` for a durable artifact.
- `--parallel` prints each script's output as a grouped block on completion instead of discarding it.

### Added

- `stacy run --log <path>`: write the raw Stata log to a chosen path (works with `--quiet` for silent file-artifact mode).

### Removed

- Dead `log_reader::is_successful_completion` (unused since the streaming rework, #65).

### Fixed

- Task arrays accept script paths: `all = ["clean", "src/02_analyze.do"]` (#64).
- Post-install hints comma-separate package names (#63).
- Failure context no longer loads the entire log into memory (#66).
- Log streaming no longer hangs when Stata is killed or fails to launch, recovers from truncated logs, and survives closed pipes (`| head`).
- `--trace` no longer leaks its temp script and log.

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
