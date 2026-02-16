* Helper script with error
display "=== Inside helper_with_error.do ==="

* This will fail - bad_var doesn't exist
gen y = bad_var + 1
