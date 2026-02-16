* Test: File already exists (when using replace)
clear all
set obs 10
gen x = 1
* Try to save without replace (after it exists)
save "temp_exists.dta"
save "temp_exists.dta"
