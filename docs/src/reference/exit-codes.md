# Exit Codes

stacy uses consistent exit codes to indicate success or failure type.

## Exit Code Table

| Code | Name | Description |
|------|------|-------------|
| 0 | Success | Operation completed successfully |
| 1 | Stata Error | Stata r() code detected in log |
| 2 | Syntax Error | Invalid Stata syntax |
| 3 | File Error | File not found, permission denied, data errors |
| 4 | Memory Error | Insufficient memory |
| 5 | Internal Error | stacy itself failed (not Stata) |
| 10 | Environment Error | Stata not found or configuration invalid |

## Stata r() Code Mapping

stacy maps Stata's r() error codes to exit codes:

| Exit Code | Stata r() Codes |
|-----------|----------------|
| 1 | most r() codes not in other categories |
| 2 | r(198), r(199) |
| 3 | r(601), r(603), r(610), r(639), r(2000-2999) |
| 4 | r(950) |

## Usage

### Shell

```bash
stacy run analysis.do
echo $?  # 0 on success, 1-10 on failure
```

### Makefile

```makefile
results.dta: analysis.do
	stacy run analysis.do  # Stops on non-zero exit
```

## Stability

Exit codes 0-10 are stable and will not change meaning. New categories may be added with codes 11+.

## See Also

- [Error Detection](./errors.md)
- [stacy run](../commands/run.md)
