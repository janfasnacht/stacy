# Contributing

## Setup

```bash
git clone https://github.com/janfasnacht/stacy.git && cd stacy
cargo build
cargo test
```

## Code Style

- `cargo fmt` before committing
- `cargo clippy` with no warnings
- Tests for new functionality

## Commit Messages

Conventional commits: `feat:`, `fix:`, `docs:`, `test:`, `refactor:`

```
feat: add parallel execution to stacy run
fix: handle spaces in script paths
docs: update installation guide
```

## Pull Requests

1. Branch from `main`
2. Make changes with tests
3. Run: `cargo fmt && cargo clippy && cargo test`
4. Update docs if user-facing (`docs/src/`, `README.md`)
5. Submit PR

## Project Structure

| Directory | Purpose |
|-----------|---------|
| `src/` | Rust source |
| `tests/` | Integration tests |
| `docs/src/` | mdBook documentation |
| `schema/` | Command schema (Stata wrapper codegen) |
| `xtask/` | Dev tooling (`cargo xtask codegen`) |

## Stata Wrapper Codegen

`.ado` and `.sthlp` files are generated from `schema/commands.toml`:

```bash
cargo xtask codegen        # Generate
cargo xtask codegen --check  # Verify (CI)
```

See `CLAUDE.md` for project context and design decisions.

## Releasing

### Prerequisites

- All tests passing: `cargo test`
- No clippy warnings: `cargo clippy`
- Code formatted: `cargo fmt --check`
- Documentation builds: `mdbook build docs`

### Release Checklist

1. **Update version** in `Cargo.toml`
2. **Update CHANGELOG.md**: Move `[Unreleased]` to `[X.Y.Z]` with date
3. **Commit and tag**:
   ```bash
   git add Cargo.toml Cargo.lock CHANGELOG.md
   git commit -m "Release vX.Y.Z"
   git tag -a vX.Y.Z -m "Release vX.Y.Z"
   git push origin main --tags
   ```
4. **Verify**: GitHub Actions builds binaries and creates the release automatically

### Versioning

[Semantic Versioning](https://semver.org/):
- **MAJOR**: Breaking changes to CLI or exit codes
- **MINOR**: New features, backward compatible
- **PATCH**: Bug fixes

The exit code contract (0-5, 10) is stable.

### Release Artifacts

| File | Platform |
|------|----------|
| `stacy-vX.Y.Z-x86_64-unknown-linux-gnu.tar.gz` | Linux (x86_64) |
| `stacy-vX.Y.Z-x86_64-apple-darwin.tar.gz` | macOS (Intel) |
| `stacy-vX.Y.Z-aarch64-apple-darwin.tar.gz` | macOS (Apple Silicon) |
| `stacy-vX.Y.Z-x86_64-pc-windows-msvc.zip` | Windows |
| `checksums.txt` | SHA256 checksums |
