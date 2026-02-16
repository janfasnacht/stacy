* Script that tries to use a file that doesn't exist
* Should generate r(601) error

version 14
set more off

* Try to use non-existent dataset
use "nonexistent_data_file.dta", clear

* Should not reach here
display "This should not execute"
