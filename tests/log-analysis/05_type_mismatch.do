* Test script: Type mismatch error (r(109))
* Should fail with type mismatch

clear all
set obs 100

gen str10 name = "test"

* This should trigger r(109) - type mismatch
* Cannot use string in mathematical operation
gen result = name + 10
