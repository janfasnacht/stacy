#!/bin/bash
# Generate comprehensive test suite for error detection
# Target: 50 scripts covering 182 official error codes
# Goal: 95%+ detection accuracy

set -e

TEST_DIR="tests/log-analysis"
mkdir -p "$TEST_DIR"

echo "Generating comprehensive test suite..."
echo "Target: 50 scripts covering major error categories"
echo ""

# Category 1: Syntax Errors (r(198), r(199))
echo "Creating syntax error tests..."

cat > "$TEST_DIR/20_syntax_invalid_option.do" << 'EOF'
* Test: Invalid option
clear all
summarize, invalidoption
EOF

cat > "$TEST_DIR/21_syntax_too_few_vars.do" << 'EOF'
* Test: Too few variables specified
clear all
set obs 100
gen x = rnormal()
* regress needs at least 2 variables
regress x
EOF

cat > "$TEST_DIR/22_syntax_invalid_name.do" << 'EOF'
* Test: Invalid variable name
clear all
gen 1invalid = 1
EOF

# Category 2: File I/O Errors (r(601-699))
echo "Creating file I/O error tests..."

cat > "$TEST_DIR/23_file_permission_denied.do" << 'EOF'
* Test: File permission denied (if we can create one)
* For now, just file not found variant
use "/nonexistent/path/data.dta", clear
EOF

cat > "$TEST_DIR/24_file_already_exists.do" << 'EOF'
* Test: File already exists (when using replace)
clear all
set obs 10
gen x = 1
* Try to save without replace (after it exists)
save "temp_exists.dta"
save "temp_exists.dta"
EOF

cat > "$TEST_DIR/25_file_read_only.do" << 'EOF'
* Test: Cannot write to file
* This is hard to test portably
* Use another file not found variant
use "missing_data.dta"
EOF

# Category 3: Variable Errors (r(100-199))
echo "Creating variable error tests..."

cat > "$TEST_DIR/26_var_type_mismatch.do" << 'EOF'
* Test: Type mismatch
clear all
set obs 10
gen str_var = "hello"
* Try to do numeric operation on string
gen result = str_var + 1
EOF

cat > "$TEST_DIR/27_var_already_defined.do" << 'EOF'
* Test: Variable already defined
clear all
set obs 10
gen x = 1
gen x = 2
EOF

cat > "$TEST_DIR/28_var_no_observations.do" << 'EOF'
* Test: No observations
clear all
* Try to generate variable with no obs
gen x = 1
EOF

cat > "$TEST_DIR/29_var_ambiguous_abbrev.do" << 'EOF'
* Test: Ambiguous abbreviation
clear all
set obs 10
gen variable1 = 1
gen variable2 = 2
* 'var' is ambiguous
summarize var
EOF

# Category 4: Data Errors (r(400-499))
echo "Creating data error tests..."

cat > "$TEST_DIR/30_data_invalid_numlist.do" << 'EOF'
* Test: Invalid numlist
clear all
forvalues i = 1/invalid {
    display `i'
}
EOF

cat > "$TEST_DIR/31_data_matrix_not_found.do" << 'EOF'
* Test: Matrix not found
matrix list nonexistent_matrix
EOF

cat > "$TEST_DIR/32_data_invalid_expression.do" << 'EOF'
* Test: Invalid expression
clear all
set obs 10
gen x = 1
gen y = x + + 1
EOF

# Category 5: Estimation Errors (r(2000-2999))
echo "Creating estimation error tests..."

cat > "$TEST_DIR/33_estimation_no_variance.do" << 'EOF'
* Test: No variance in variable
clear all
set obs 100
gen x = 1
gen y = rnormal()
* x has no variance
regress y x
EOF

cat > "$TEST_DIR/34_estimation_collinearity.do" << 'EOF'
* Test: Collinearity
clear all
set obs 100
gen x = rnormal()
gen x2 = x
gen y = rnormal()
* x and x2 are perfectly collinear
regress y x x2
EOF

cat > "$TEST_DIR/35_estimation_too_few_obs.do" << 'EOF'
* Test: Too few observations
clear all
set obs 2
gen x = rnormal()
gen y = rnormal()
* Need more obs for regression
regress y x
EOF

# Category 6: Memory/System Errors (r(900-999))
echo "Creating system error tests..."

cat > "$TEST_DIR/36_system_matsize.do" << 'EOF'
* Test: Matrix size exceeded
* This is hard to trigger reliably
* Using a simpler approach
clear all
* Try to create too many variables (simplified)
set obs 1
forvalues i = 1/10000 {
    gen var`i' = `i'
}
EOF

# Category 7: Specific common errors
echo "Creating specific common error tests..."

cat > "$TEST_DIR/37_missing_values.do" << 'EOF'
* Test: Operations with missing values (usually just a warning)
clear all
set obs 10
gen x = .
* This might not error, but good to test
summarize x
* Force an error by using missing in context that requires real value
generate y = 1/x
EOF

cat > "$TEST_DIR/38_string_number_mismatch.do" << 'EOF'
* Test: String where number expected
clear all
set obs 10
gen str_var = "not a number"
* Try to use string in numeric context
summarize str_var
EOF

cat > "$TEST_DIR/39_invalid_if_condition.do" << 'EOF'
* Test: Invalid if condition
clear all
set obs 10
gen x = rnormal()
* Invalid if syntax
summarize x if invalid_var
EOF

cat > "$TEST_DIR/40_macro_not_found.do" << 'EOF'
* Test: Macro not found
clear all
display "`undefined_macro'"
* Try to use undefined macro in command
gen x = `undefined_macro'
EOF

# Category 8: Edge cases and special scenarios
echo "Creating edge case tests..."

cat > "$TEST_DIR/41_empty_dataset.do" << 'EOF'
* Test: Operations on empty dataset
clear all
* No set obs - dataset is empty
describe
* Some commands may fail with empty dataset
summarize
EOF

cat > "$TEST_DIR/42_invalid_format.do" << 'EOF'
* Test: Invalid format specification
clear all
set obs 10
gen x = 1
format x %invalid
EOF

cat > "$TEST_DIR/43_divide_by_zero.do" << 'EOF'
* Test: Division by zero (may produce missing, not error)
clear all
set obs 10
gen x = 1
gen y = 0
gen z = x / y
* Force error context
assert z != .
EOF

cat > "$TEST_DIR/44_assert_failure.do" << 'EOF'
* Test: Assert failure
clear all
set obs 10
gen x = rnormal()
* This will fail
assert x > 0
EOF

cat > "$TEST_DIR/45_constraint_violated.do" << 'EOF'
* Test: Constraint violation
clear all
set obs 10
gen x = rnormal()
* Set impossible constraint
constraint 1 x = 0
constraint 2 x = 1
* Both cannot be true
EOF

# Category 9: Additional syntax variations
echo "Creating additional syntax tests..."

cat > "$TEST_DIR/46_unmatched_quotes.do" << 'EOF'
* Test: Unmatched quotes
clear all
display "This is unmatched
EOF

cat > "$TEST_DIR/47_invalid_weight.do" << 'EOF'
* Test: Invalid weight specification
clear all
set obs 10
gen x = rnormal()
gen y = rnormal()
gen wt = -1
* Negative weights
summarize x [pw=wt]
EOF

cat > "$TEST_DIR/48_label_too_long.do" << 'EOF'
* Test: Label too long
clear all
set obs 10
gen x = 1
* Create extremely long label
label variable x "This is a very long label that exceeds the maximum allowed length for variable labels in Stata which has specific limits on how many characters can be used"
EOF

cat > "$TEST_DIR/49_preserve_without_restore.do" << 'EOF'
* Test: Preserve without restore (not always error)
clear all
set obs 10
preserve
* Exit without restore - may or may not error
* Let's force an error another way
use "nonexistent.dta"
EOF

cat > "$TEST_DIR/50_nested_error_propagation.do" << 'EOF'
* Test: Error propagation through multiple nested files
* Similar to 14 but different error
clear all
set obs 10
gen x = rnormal()
* Call nested file that will error
do tests/log-analysis/nested/helper_with_error.do
EOF

# Category 10: Tier 1 - Critical Untested Codes (Week 2)
echo "Creating Tier 1 critical error tests..."

cat > "$TEST_DIR/51_too_few_variables.do" << 'EOF'
* Test: r(102) - Too few variables specified
* regress requires at least dependent and one independent variable
clear all
set obs 100
gen y = rnormal()
* This should fail - regress needs at least 2 variables
regress y
EOF

cat > "$TEST_DIR/52_too_many_variables.do" << 'EOF'
* Test: r(103) - Too many variables specified
* tabulate has a limit on number of variables
clear all
set obs 10
gen x1 = 1
gen x2 = 2
gen x3 = 3
* tabulate only accepts 1 or 2 variables, not 3
tabulate x1 x2 x3
EOF

cat > "$TEST_DIR/53_invalid_numlist.do" << 'EOF'
* Test: r(121) - Invalid numlist
* Numlist syntax must be valid
clear all
* "invalid" is not a number
forvalues i = 1/invalid {
    display `i'
}
EOF

cat > "$TEST_DIR/54_negative_weights.do" << 'EOF'
* Test: r(402) - Negative weights encountered
* Weights must be non-negative
clear all
set obs 10
gen x = rnormal()
gen wt = -1
* Negative weight should trigger error
summarize x [pw=wt]
EOF

cat > "$TEST_DIR/55_missing_values_strict.do" << 'EOF'
* Test: r(416) - Missing values in critical context
* Some commands require non-missing values
clear all
set obs 10
gen x = .
* ttest requires non-missing values
ttest x == 0
EOF

cat > "$TEST_DIR/56_convergence_failure.do" << 'EOF'
* Test: r(430) - Convergence not achieved
* Maximum likelihood estimation with impossible constraints
clear all
set obs 100
gen y = rnormal()
gen x = rnormal()
* Force convergence failure with bad model
constraint 1 x = 999999
cnsreg y, constraints(1)
EOF

cat > "$TEST_DIR/57_matrix_conformability.do" << 'EOF'
* Test: r(503) - Matrix conformability error
* Matrix operations require compatible dimensions
clear all
matrix A = (1, 2 \ 3, 4)
matrix B = (1, 2, 3)
* Cannot add 2x2 and 1x3 matrices
matrix C = A + B
EOF

cat > "$TEST_DIR/58_file_cannot_open.do" << 'EOF'
* Test: r(603) - File could not be opened
* Attempt to open a file that exists but is inaccessible
* We'll use a directory path instead of a file
use "/tmp/", clear
EOF

cat > "$TEST_DIR/59_file_not_stata_format.do" << 'EOF'
* Test: r(610) - File not Stata format
* Try to load a non-Stata file as dataset
* First create a non-Stata file
clear all
set obs 1
gen x = 1
* Create a text file
outfile using "not_stata.txt", replace
* Now try to load it as Stata format
use "not_stata.txt", clear
EOF

cat > "$TEST_DIR/60_too_many_values.do" << 'EOF'
* Test: r(1001) - Too many values
* tabulate has a limit on unique value combinations
clear all
set obs 1000
* Create variables with many unique combinations
gen x = _n
gen y = mod(_n, 500)
* This should exceed tabulate's limit
tabulate x y
EOF

echo ""
echo "Generated test scripts. Summary:"
echo "  Syntax errors: 20-22"
echo "  File I/O: 23-25"
echo "  Variable errors: 26-29"
echo "  Data errors: 30-32"
echo "  Estimation: 33-35"
echo "  System: 36"
echo "  Common errors: 37-40"
echo "  Edge cases: 41-45"
echo "  Additional: 46-50"
echo "  Tier 1 (Week 2): 51-60"
echo ""
echo "Next steps:"
echo "1. Run each script through Stata to generate .log files"
echo "2. Run: ./tests/run_all_tests.sh"
echo "3. Measure detection accuracy"
echo ""
