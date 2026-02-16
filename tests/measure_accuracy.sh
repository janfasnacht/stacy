#!/bin/bash
# Measure stacy error detection accuracy
# Target: 95%+ accuracy

TEST_DIR="tests/log-analysis"
STACY="./target/debug/stacy"

# Counters
total_with_errors=0
correctly_detected=0
missed_errors=0

total_without_errors=0
false_positives=0
correct_success=0

echo "Measuring stacy error detection accuracy..."
echo "=========================================="
echo ""

# Test all .do files that have .log files
for script in "$TEST_DIR"/*.do; do
    [ -e "$script" ] || continue

    basename=$(basename "$script" .do)
    log_file="$TEST_DIR/${basename}.log"

    # Skip if no log file
    [ -f "$log_file" ] || continue

    # Check if log has error (r() code after "end of do-file")
    has_error=false
    if grep -q "end of do-file" "$log_file" 2>/dev/null; then
        if tail -5 "$log_file" | grep -q "^r([0-9]\+);"; then
            has_error=true
        fi
    fi

    # Run stacy and check exit code
    $STACY run "$script" --quiet > /dev/null 2>&1
    exit_code=$?

    # Count results
    if $has_error; then
        total_with_errors=$((total_with_errors + 1))
        if [ $exit_code -ne 0 ]; then
            correctly_detected=$((correctly_detected + 1))
            echo "✓ $basename: Error detected (exit $exit_code)"
        else
            missed_errors=$((missed_errors + 1))
            echo "✗ $basename: ERROR MISSED (exit $exit_code)"
        fi
    else
        total_without_errors=$((total_without_errors + 1))
        if [ $exit_code -eq 0 ]; then
            correct_success=$((correct_success + 1))
            echo "✓ $basename: Success detected correctly"
        else
            false_positives=$((false_positives + 1))
            echo "! $basename: FALSE POSITIVE (exit $exit_code)"
        fi
    fi
done

echo ""
echo "=========================================="
echo "Results"
echo "=========================================="
echo ""
echo "Scripts with errors: $total_with_errors"
echo "  Correctly detected: $correctly_detected"
echo "  Missed: $missed_errors"
echo ""
echo "Scripts without errors: $total_without_errors"
echo "  Correct (exit 0): $correct_success"
echo "  False positives: $false_positives"
echo ""

# Calculate overall accuracy
total=$((total_with_errors + total_without_errors))
correct=$((correctly_detected + correct_success))

if [ $total -gt 0 ]; then
    accuracy=$(( (correct * 100) / total ))
    echo "Overall accuracy: $correct / $total = $accuracy%"
    echo ""

    if [ $accuracy -ge 95 ]; then
        echo "✓ SUCCESS: Met 95% accuracy target!"
    else
        echo "⚠ WARNING: Below 95% accuracy target"
    fi
else
    echo "No tests found"
fi

# Detection rate for errors only
if [ $total_with_errors -gt 0 ]; then
    error_detection_rate=$(( (correctly_detected * 100) / total_with_errors ))
    echo ""
    echo "Error detection rate: $correctly_detected / $total_with_errors = $error_detection_rate%"
fi
