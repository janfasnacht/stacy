* Test script: File not found error (r(601))
* Should fail trying to open non-existent file

clear all

* This should trigger r(601) - file not found
use "this_file_does_not_exist.dta", clear
