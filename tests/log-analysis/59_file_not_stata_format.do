* Test: r(610) - File not Stata format
* Try to load a non-Stata file as dataset
* First create a non-Stata file
clear all
set obs 1
gen x = 1
* Create a text file
outfile using "not_stata.txt", replace
* Now try to load it as Stata format
use "not_stata.txt", clear
