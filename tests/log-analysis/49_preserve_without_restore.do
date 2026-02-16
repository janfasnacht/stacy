* Test: Preserve without restore (not always error)
clear all
set obs 10
preserve
* Exit without restore - may or may not error
* Let's force an error another way
use "nonexistent.dta"
