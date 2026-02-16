* Test: Operations with missing values (usually just a warning)
clear all
set obs 10
gen x = .
* This might not error, but good to test
summarize x
* Force an error by using missing in context that requires real value
generate y = 1/x
