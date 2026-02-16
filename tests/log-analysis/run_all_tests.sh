#!/bin/bash
# Run all test scripts in batch mode and capture logs

STATA="/Applications/StataNow/StataMP.app/Contents/MacOS/stata-mp"
TEST_DIR="$(cd "$(dirname "$0")" && pwd)"

echo "Running Stata batch mode tests..."
echo "Test directory: $TEST_DIR"
echo "Stata binary: $STATA"
echo ""

cd "$TEST_DIR"

# Run each test script
for script in *.do; do
    logfile="${script%.do}.log"

    echo "Running: $script"
    "$STATA" -b do "$script" "$logfile"
    exit_code=$?
    echo "  Exit code: $exit_code"
    echo "  Log: $logfile"

    # Show last few lines of log
    if [ -f "$logfile" ]; then
        echo "  Last 5 lines:"
        tail -5 "$logfile" | sed 's/^/    /'
    fi
    echo ""
done

echo "All tests complete!"
echo ""
echo "Log files created:"
ls -lh *.log 2>/dev/null || echo "  No log files found"
