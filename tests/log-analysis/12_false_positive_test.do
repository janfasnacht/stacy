* Test false positives: Can we be fooled by r() codes in output?
* This script succeeds but outputs text that looks like error codes

clear all

display "=== False Positive Test ==="

* Test 1: Display text with r() code
display "This command returns r(199)"

* Test 2: Display with semicolon
display "Error code r(601);"

* Test 3: Multiple potential false positives
display "r(111);"
display "r(601);"
display "r(950);"

* Test 4: In a comment
* This should trigger r(199) according to docs

display "All tests completed successfully"

* Script should succeed (exit 0) despite r() codes in output
