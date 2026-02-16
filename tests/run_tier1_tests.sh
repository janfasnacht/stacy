#!/bin/bash
# Run Tier 1 test scripts (51-60) through Stata

STATA="/Applications/StataNow/StataMP.app/Contents/MacOS/stata-mp"
TEST_DIR="tests/log-analysis"

cd "$TEST_DIR" || exit 1

echo "Running Tier 1 tests through Stata..."
for file in 51_*.do 52_*.do 53_*.do 54_*.do 55_*.do 56_*.do 57_*.do 58_*.do 59_*.do 60_*.do; do
  if [ -f "$file" ]; then
    echo "  Running $file..."
    $STATA -b -q do "$file" > /dev/null 2>&1 || true
  fi
done

echo ""
echo "Generated log files:"
ls -1 5*.log 2>/dev/null | wc -l
echo ""
echo "Total test suite:"
ls -1 *.do | wc -l
echo ".do files"
ls -1 *.log | wc -l
echo ".log files"
