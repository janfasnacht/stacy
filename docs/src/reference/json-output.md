# JSON Output

stacy supports JSON output for machine-readable results.

## Usage

Add `--format json` to any command:

```bash
stacy run --format json analysis.do
stacy install --format json
stacy env --format json
```

## Output Schemas

### stacy run

**Success:**
```json
{
  "success": true,
  "script": "analysis.do",
  "duration_secs": 12.45,
  "exit_code": 0,
  "log_file": "analysis.log"
}
```

**Failure:**
```json
{
  "success": false,
  "script": "analysis.do",
  "duration_secs": 0.45,
  "exit_code": 2,
  "log_file": "analysis.log",
  "errors": [
    {
      "type": "StataCode",
      "r_code": 199,
      "name": "unrecognized command",
      "line": 15,
      "context": "reghdfe price mpg, absorb(make)"
    }
  ]
}
```

| Field | Type | Description |
|-------|------|-------------|
| `success` | bool | Whether script completed without errors |
| `script` | string | Path to script that was run |
| `duration_secs` | float | Execution time in seconds |
| `exit_code` | int | stacy exit code (0-10) |
| `log_file` | string | Path to Stata log file |
| `errors` | array | Error details (only on failure) |
| `errors[].type` | string | Error type (`StataCode`, `Syntax`, `File`) |
| `errors[].r_code` | int | Stata r() code if applicable |
| `errors[].name` | string | Human-readable error name |
| `errors[].line` | int | Line number if detected |
| `errors[].context` | string | Code that caused the error |

### stacy install

```json
{
  "success": true,
  "installed": [
    {
      "name": "estout",
      "version": "2024.03.15",
      "source": "SSC"
    },
    {
      "name": "reghdfe",
      "version": "6.12.3",
      "source": "GitHub"
    }
  ],
  "already_cached": [
    {
      "name": "ftools",
      "version": "2.49.0",
      "source": "SSC"
    }
  ],
  "failed": []
}
```

| Field | Type | Description |
|-------|------|-------------|
| `success` | bool | Whether all packages installed |
| `installed` | array | Packages downloaded this run |
| `already_cached` | array | Packages found in cache |
| `failed` | array | Packages that failed to install |

### stacy list

```json
{
  "packages": [
    {
      "name": "estout",
      "version": "2024.03.15",
      "source": "SSC",
      "locked": true
    },
    {
      "name": "reghdfe",
      "version": "6.12.3",
      "source": "GitHub",
      "locked": true
    }
  ]
}
```

### stacy env

```json
{
  "stata": {
    "binary": "/Applications/StataNow/StataMP.app/Contents/MacOS/stata-mp",
    "version": "18.0",
    "flavor": "MP",
    "source": "user config"
  },
  "project": {
    "root": "/Users/user/projects/analysis",
    "has_config": true,
    "has_lockfile": true
  },
  "cache": {
    "path": "/Users/user/.cache/stacy/packages",
    "package_count": 12
  }
}
```

### stacy doctor

```json
{
  "ready": true,
  "checks": [
    {
      "name": "Stata binary",
      "status": "ok",
      "message": "Found at /Applications/StataNow/StataMP.app/Contents/MacOS/stata-mp"
    },
    {
      "name": "Stata version",
      "status": "ok",
      "message": "Stata 18.0 MP"
    },
    {
      "name": "Project config",
      "status": "ok",
      "message": "Found stacy.toml"
    },
    {
      "name": "Lockfile",
      "status": "warning",
      "message": "No stacy.lock found"
    }
  ]
}
```

| Status | Meaning |
|--------|---------|
| `ok` | Check passed |
| `warning` | Non-blocking issue |
| `error` | Blocking issue |

### stacy deps

```json
{
  "script": "master.do",
  "dependencies": {
    "path": "master.do",
    "type": null,
    "exists": true,
    "is_circular": false,
    "line_number": null,
    "children": [
      {
        "path": "config/settings.do",
        "type": "do",
        "exists": true,
        "is_circular": false,
        "line_number": 3,
        "children": []
      },
      {
        "path": "reghdfe",
        "type": "require",
        "exists": true,
        "is_circular": false,
        "line_number": 5,
        "children": []
      }
    ]
  },
  "summary": {
    "unique_count": 2,
    "has_circular": false,
    "has_missing": false,
    "circular_paths": [],
    "missing_paths": [],
    "circular_count": 0,
    "missing_count": 0
  }
}
```

## jq Examples

### Check if a run succeeded

```bash
stacy run --format json analysis.do | jq '.success'
```

### Get exit code

```bash
stacy run --format json analysis.do | jq '.exit_code'
```

### Extract error codes

```bash
stacy run --format json analysis.do | jq '.errors[]?.r_code'
```

### List installed package names

```bash
stacy list --format json | jq -r '.packages[].name'
```

### Get Stata binary path

```bash
stacy env --format json | jq -r '.stata.binary'
```

### Check if project has lockfile

```bash
stacy env --format json | jq '.project.has_lockfile'
```

### Find failed doctor checks

```bash
stacy doctor --format json | jq '.checks[] | select(.status == "error")'
```

### Count dependencies

```bash
stacy deps --format json master.do | jq '.files | length'
```

### Get packages that need downloading

```bash
stacy install --format json | jq '.installed[].name'
```

## Using JSON in Scripts

### Shell

```bash
#!/bin/bash
result=$(stacy run --format json analysis.do)
if echo "$result" | jq -e '.success' > /dev/null; then
    echo "Success!"
else
    echo "Failed with errors:"
    echo "$result" | jq -r '.errors[].name'
    exit 1
fi
```

### Python

```python
import subprocess
import json

result = subprocess.run(
    ['stacy', 'run', '--format', 'json', 'analysis.do'],
    capture_output=True,
    text=True
)
data = json.loads(result.stdout)

if data['success']:
    print(f"Completed in {data['duration_secs']:.2f}s")
else:
    for error in data.get('errors', []):
        print(f"Error r({error['r_code']}): {error['name']}")
```

### R

```r
library(jsonlite)

result <- system2("stacy", c("run", "--format", "json", "analysis.do"),
                  stdout = TRUE, stderr = TRUE)
data <- fromJSON(paste(result, collapse = "\n"))

if (data$success) {
  cat(sprintf("Completed in %.2fs\n", data$duration_secs))
} else {
  cat("Errors:\n")
  print(data$errors)
}
```

## Stability

The JSON schema follows semantic versioning:

- Core fields (`success`, `exit_code`, `errors`) are **stable** from v1.0
- New fields may be added in minor versions (backward compatible)
- Field removal or type changes only in major versions

> **Tip:** Use jq's `-e` flag to handle missing fields gracefully in scripts.

## See Also

- [stacy run](../commands/run.md) - Running scripts
- [Build Integration](../guides/build-integration.md) - CI/CD and build tools
- [Exit Codes](./exit-codes.md) - Exit code meanings
