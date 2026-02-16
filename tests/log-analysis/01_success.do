* Test script: Successful execution
* Should complete without errors

clear all
set obs 100

gen x = rnormal()
gen y = rnormal()

summarize x y
regress y x

display "Test completed successfully"
