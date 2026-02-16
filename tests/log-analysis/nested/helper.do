* Helper script called by main script
display "=== Inside helper.do ==="

gen y = x + rnormal()

display "Helper completed"
