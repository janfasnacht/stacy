#!/bin/bash
# Quick check of all Tier 1 tests

cd "$(dirname "$0")/.."

echo "Checking all 10 Tier 1 tests..."
echo ""

for i in 51 52 53 54 55 56 57 58 59 60; do
  script=$(ls tests/log-analysis/${i}_*.do 2>/dev/null | head -1)
  if [ -f "$script" ]; then
    ./target/debug/stacy run "$script" --quiet > /dev/null 2>&1
    exit_code=$?
    name=$(basename "$script" .do | cut -d_ -f2-)
    if [ $exit_code -eq 0 ]; then
      echo "⚠️  Test $i ($name): Exit 0 (NO ERROR DETECTED)"
    else
      echo "✅ Test $i ($name): Exit $exit_code"
    fi
  fi
done
