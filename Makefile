# stacy - Modern Stata Workflow Tool
# Common development tasks

.PHONY: build test clean lint fmt check release help

# Default target
help:
	@echo "stacy development commands:"
	@echo ""
	@echo "  make build      - Build debug binary"
	@echo "  make release    - Build optimized release binary"
	@echo "  make test       - Run all tests"
	@echo "  make lint       - Run clippy lints"
	@echo "  make fmt        - Format code"
	@echo "  make check      - Run all checks (fmt, lint, test)"
	@echo "  make clean      - Remove build artifacts and logs"
	@echo "  make clean-logs - Remove .log files only"
	@echo ""

# Build targets
build:
	cargo build

release:
	cargo build --release

# Testing
test:
	cargo test

test-verbose:
	cargo test -- --nocapture

# Code quality
lint:
	cargo clippy -- -D warnings

fmt:
	cargo fmt

fmt-check:
	cargo fmt --all -- --check

# Run all checks (what CI runs)
check: fmt-check lint test
	@echo "All checks passed!"

# Cleanup
clean:
	cargo clean
	rm -f *.log
	rm -f tests/log-analysis/*.log
	rm -rf tests/edge_cases/*.log

clean-logs:
	rm -f *.log
	rm -f tests/log-analysis/*.log
	rm -rf tests/edge_cases/*.log
	@echo "Removed all .log files"

# Development helpers
watch:
	cargo watch -x check -x test

bench:
	cargo bench

doc:
	cargo doc --open
