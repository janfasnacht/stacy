# Build Integration

stacy integrates with build systems through standard Unix exit codes. Any tool that stops on non-zero exit works with stacy.

## Quick Examples

**Make:**
```makefile
results.dta: analysis.do data.dta
	stacy run analysis.do
```

**Snakemake:**
```python
rule analysis:
    input: "analysis.do", "data.dta"
    output: "results.dta"
    shell: "stacy run {input[0]}"
```

**Shell:**
```bash
stacy run step1.do && stacy run step2.do
```

---

## GNU Make

### Basic Makefile

```makefile
STATA := stacy run

# Final output depends on analysis
results/tables.tex: src/03_tables.do results/estimates.dta
	$(STATA) $<

# Estimates depend on clean data
results/estimates.dta: src/02_analysis.do data/clean.dta
	$(STATA) $<

# Clean data depends on raw data
data/clean.dta: src/01_clean.do data/raw.dta
	$(STATA) $<

.PHONY: all clean
all: results/tables.tex
clean:
	rm -f data/clean.dta results/*.dta results/*.tex
```

### With Package Installation

```makefile
.PHONY: install
install:
	stacy install

results.dta: analysis.do | install
	stacy run $<
```

See [GNU Make documentation](https://www.gnu.org/software/make/manual/) for more patterns.

---

## Snakemake

### Basic Snakefile

```python
rule all:
    input: "results/tables.tex"

rule clean:
    input: "src/01_clean.do", "data/raw.dta"
    output: "data/clean.dta"
    shell: "stacy run {input[0]}"

rule analysis:
    input: "src/02_analysis.do", "data/clean.dta"
    output: "results/estimates.dta"
    shell: "stacy run {input[0]}"

rule tables:
    input: "src/03_tables.do", "results/estimates.dta"
    output: "results/tables.tex"
    shell: "stacy run {input[0]}"
```

### Parallel Execution

```bash
snakemake --cores 4
```

See [Snakemake documentation](https://snakemake.readthedocs.io/) for workflows, clusters, and more.

---

## CI/CD

### GitHub Actions

```yaml
# .github/workflows/analysis.yml
name: Analysis
on: [push, pull_request]

jobs:
  build:
    runs-on: self-hosted  # With Stata installed
    steps:
      - uses: actions/checkout@v4

      - name: Install stacy
        run: |
          curl -fsSL https://raw.githubusercontent.com/janfasnacht/stacy/main/install.sh | bash
          echo "$HOME/.local/bin" >> $GITHUB_PATH

      - name: Install packages
        run: stacy install --frozen

      - name: Run analysis
        run: stacy run analysis.do

      - name: Upload results
        uses: actions/upload-artifact@v4
        with:
          name: results
          path: output/
```

> **Note:** `--frozen` fails if lockfile doesn't match stacy.toml, catching uncommitted dependency changes.

### GitLab CI

```yaml
# .gitlab-ci.yml
analysis:
  stage: build
  before_script:
    - curl -fsSL https://raw.githubusercontent.com/janfasnacht/stacy/main/install.sh | bash
    - export PATH="$HOME/.local/bin:$PATH"
    - stacy install
  script:
    - stacy run analysis.do
  artifacts:
    paths: [output/]
```

### Caching Packages

```yaml
- uses: actions/cache@v4
  with:
    path: ~/.cache/stacy/packages/
    key: stata-packages-${{ hashFiles('stacy.lock') }}
```

### Stata Licensing in CI

Stata requires a license. Options:

1. **Self-hosted runner** with Stata installed
2. **Docker container** with Stata
3. **Skip Stata steps** in CI (validate config only)

See [GitHub Actions docs](https://docs.github.com/en/actions) or [GitLab CI docs](https://docs.gitlab.com/ee/ci/) for more.

---

## Best Practices

1. **Use `--frozen` in CI** to catch lockfile drift
2. **Commit `stacy.lock`** for reproducibility
3. **Cache packages** to speed up builds
4. **Use JSON output** for programmatic checks: `stacy run --format json`
5. **Upload artifacts** on failure for debugging

## See Also

- [How It Works](../reference/how-it-works.md#machine-interface) - Exit codes and JSON
- [Exit Codes](../reference/exit-codes.md) - Code meanings
