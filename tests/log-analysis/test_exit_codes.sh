#!/bin/bash
# Test exit code behavior for all scenarios
# This confirms the core problem stacy solves

set -u

STATA="/Applications/StataNow/StataMP.app/Contents/MacOS/stata-mp"
TEST_DIR="$(cd "$(dirname "$0")" && pwd)"

echo "=== Exit Code Behavior Test ==="
echo ""

# Test 1: Success
echo "Test 1: Success (01_success.do)"
"$STATA" -b -q do "$TEST_DIR/01_success.do" >/dev/null 2>&1
EXIT_CODE=$?
echo "  Exit code: $EXIT_CODE (expected: 0)"
echo ""

# Test 2: Stata error (syntax)
echo "Test 2: Syntax error (02_syntax_error.do)"
"$STATA" -b -q do "$TEST_DIR/02_syntax_error.do" >/dev/null 2>&1
EXIT_CODE=$?
echo "  Exit code: $EXIT_CODE (expected: 0 - PROBLEM!)"
echo ""

# Test 3: Stata error (file not found)
echo "Test 3: File not found (03_file_not_found.do)"
"$STATA" -b -q do "$TEST_DIR/03_file_not_found.do" >/dev/null 2>&1
EXIT_CODE=$?
echo "  Exit code: $EXIT_CODE (expected: 0 - PROBLEM!)"
echo ""

# Test 4: Stata error (variable not found)
echo "Test 4: Variable not found (04_variable_not_found.do)"
"$STATA" -b -q do "$TEST_DIR/04_variable_not_found.do" >/dev/null 2>&1
EXIT_CODE=$?
echo "  Exit code: $EXIT_CODE (expected: 0 - PROBLEM!)"
echo ""

# Test 5: SIGTERM (kill -15)
echo "Test 5: SIGTERM (kill -15)"
"$STATA" -b -q do "$TEST_DIR/07_infinite_loop.do" >/dev/null 2>&1 &
PID=$!
sleep 1
kill -15 $PID 2>/dev/null
wait $PID 2>/dev/null
EXIT_CODE=$?
echo "  Exit code: $EXIT_CODE (expected: 143 = 128 + 15)"
echo ""

# Test 6: SIGINT (kill -2, simulates Ctrl-C)
echo "Test 6: SIGINT (kill -2, simulates Ctrl-C)"
"$STATA" -b -q do "$TEST_DIR/07_infinite_loop.do" >/dev/null 2>&1 &
PID=$!
sleep 1
kill -2 $PID 2>/dev/null
wait $PID 2>/dev/null
EXIT_CODE=$?
echo "  Exit code: $EXIT_CODE (expected: 130 = 128 + 2)"
echo ""

# Test 7: SIGKILL (kill -9, cannot be caught)
echo "Test 7: SIGKILL (kill -9)"
"$STATA" -b -q do "$TEST_DIR/07_infinite_loop.do" >/dev/null 2>&1 &
PID=$!
sleep 1
kill -9 $PID 2>/dev/null
wait $PID 2>/dev/null
EXIT_CODE=$?
echo "  Exit code: $EXIT_CODE (expected: 137 = 128 + 9)"
echo ""

echo "=== Summary ==="
echo ""
echo "The PROBLEM confirmed:"
echo "  - ALL Stata errors exit with code 0"
echo "  - Only signals return non-zero codes"
echo "  - This breaks Make/Snakemake/CI workflows"
echo ""
echo "The SOLUTION:"
echo "  - stacy must parse log files to detect errors"
echo "  - stacy returns proper exit codes based on r() codes"
echo ""
