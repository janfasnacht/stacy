* Test: Type mismatch
clear all
set obs 10
gen str_var = "hello"
* Try to do numeric operation on string
gen result = str_var + 1
