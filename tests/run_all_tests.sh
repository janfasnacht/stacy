#!/bin/bash
# Run all test scripts through Stata to generate .log files
# Then test stacy detection accuracy

set -e

STATA="/Applications/StataNow/StataMP.app/Contents/MacOS/stata-mp"
TEST_DIR="tests/log-analysis"
RESULTS_FILE="tests/accuracy_results.txt"

# Colors for output
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo "Running comprehensive test suite..."
echo "=================================="
echo ""

# Step 1: Generate .log files for new tests (20-50)
echo "Step 1: Generating log files for test scripts 20-50..."
cd "$TEST_DIR" || exit 1

for i in {20..50}; do
    script="${i}_*.do"
    if ls $script 1> /dev/null 2>&1; then
        for file in $script; do
            if [ -f "$file" ]; then
                echo "  Running $file..."
                $STATA -b -q do "$file" 2>&1 > /dev/null || true
            fi
        done
    fi
done

cd ../..

echo ""
echo "Step 2: Testing stacy error detection on all scripts..."
echo ""

# Initialize counters
total=0
detected=0
missed=0
false_positive=0

> "$RESULTS_FILE"

echo "Test Results" >> "$RESULTS_FILE"
echo "============" >> "$RESULTS_FILE"
echo "" >> "$RESULTS_FILE"

# Test all scripts
for script in "$TEST_DIR"/*.do; do
    [ -e "$script" ] || continue

    basename=$(basename "$script" .do)
    log_file="$TEST_DIR/${basename}.log"

    # Skip if no log file
    [ -f "$log_file" ] || continue

    total=$((total + 1))

    # Check if log has error (look for r() code after "end of do-file")
    has_error=false
    if grep -q "end of do-file" "$log_file"; then
        # Check for r() after last "end of do-file"
        if tail -5 "$log_file" | grep -q "^r([0-9]\+);"; then
            has_error=true
        fi
    fi

    # Run stacy and check exit code
    ./target/debug/stacy run "$script" --quiet > /dev/null 2>&1
    exit_code=$?

    # Determine if stacy detected the error
    stacy_detected=false
    if [ $exit_code -ne 0 ]; then
        stacy_detected=true
    fi

    # Compare results
    if $has_error; then
        if $stacy_detected; then
            detected=$((detected + 1))
            echo -e "${GREEN}✓${NC} $basename: Error detected correctly"
            echo "✓ $basename: Error detected (exit $exit_code)" >> "$RESULTS_FILE"
        else
            missed=$((missed + 1))
            echo -e "${RED}✗${NC} $basename: Error MISSED"
            echo "✗ $basename: Error MISSED (exit $exit_code)" >> "$RESULTS_FILE"
        fi
    else
        if $stacy_detected; then
            false_positive=$((false_positive + 1))
            echo -e "${YELLOW}!${NC} $basename: False positive"
            echo "! $basename: False positive (exit $exit_code)" >> "$RESULTS_FILE"
        else
            detected=$((detected + 1))
            echo -e "${GREEN}✓${NC} $basename: Success detected correctly"
            echo "✓ $basename: Success detected" >> "$RESULTS_FILE"
        fi
    fi
done

echo ""
echo "=================================="
echo "Results Summary"
echo "=================================="
echo ""
echo "Total scripts: $total"
echo "Correctly detected: $detected"
echo "Missed errors: $missed"
echo "False positives: $false_positive"
echo ""

# Calculate accuracy
if [ $total -gt 0 ]; then
    accuracy=$(( (detected * 100) / total ))
    echo "Accuracy: ${accuracy}%"
    echo ""

    if [ $accuracy -ge 95 ]; then
        echo -e "${GREEN}✓ SUCCESS: Met 95% accuracy target!${NC}"
    else
        echo -e "${YELLOW}⚠ WARNING: Below 95% accuracy target${NC}"
        echo "  Need to investigate missed errors and false positives"
    fi
else
    echo "No tests found"
fi

echo ""
echo "Detailed results written to: $RESULTS_FILE"
