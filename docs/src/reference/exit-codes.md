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
| 6 | Statistical Error | Convergence failure, model problems |
| 10 | Environment Error | Stata not found or configuration invalid |

## Stata r() Code Mapping

The number inside `r(N)` is preserved in stacy's output (JSON field `r_code`, stored result `r(exit_code)` in Stata). The *shell* exit code is a category derived from it, in two steps:

1. **Error database lookup.** stacy extracts error descriptions and categories from your local Stata installation (run `stacy doctor --refresh` after a Stata upgrade). If the code is found there, its category decides the exit code.
2. **Range fallback.** Otherwise, the documented ranges from Stata's Programming Reference Manual apply:

| Exit Code | Stata r() Codes |
|-----------|----------------|
| 1 | all r() codes not in other categories |
| 2 | r(100)-r(199), e.g. r(198), r(199) |
| 3 | r(600)-r(699), e.g. r(601), r(603) |
| 4 | r(900)-r(999), e.g. r(950) |
| 6 | r(400)-r(499) |
| 10 | r(800)-r(899) |

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

This mapping is many-to-one by design: it compresses Stata's hundreds of return codes into a small, stable set that build tools can branch on.

## Stability

Exit codes 0-10 are stable and will not change meaning. New categories may be added with codes 11+.

## See Also

- [Error Detection](./how-it-works.md#error-detection)
- [stacy run](../commands/run.md)
