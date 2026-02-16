#!/bin/bash
# Extract r() codes from all log files

TEST_DIR="tests/log-analysis"

for log in "$TEST_DIR"/*.log; do
    [ -f "$log" ] || continue
    basename=$(basename "$log" .log)

    # Look for r() code after "end of do-file"
    if grep -q "end of do-file" "$log" 2>/dev/null; then
        code=$(tail -5 "$log" | grep -o "^r([0-9]\+);" | head -1 | grep -o "[0-9]\+")
        if [ -n "$code" ]; then
            echo "$basename: r($code)"
        else
            echo "$basename: success"
        fi
    else
        echo "$basename: incomplete"
    fi
done
