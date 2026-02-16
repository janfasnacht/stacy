* Basic Stata script that should succeed
* Used for testing successful execution

version 14
set more off

* Create simple dataset
clear
set obs 10
generate x = _n
generate y = x * 2

* Run simple regression
regress y x

* Save results
summarize

* End successfully
