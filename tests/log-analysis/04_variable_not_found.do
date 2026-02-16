* Test script: Variable not found error (r(111))
* Should fail with variable not found

clear all
set obs 100

gen x = rnormal()

* This should trigger r(111) - variable not found
summarize nonexistent_variable
