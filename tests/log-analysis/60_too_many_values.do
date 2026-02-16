* Test: r(1001) - Too many values (system limit)
* Exceed tabulate limits with massive table
clear all
* Create a huge two-way table that exceeds limits
set obs 5000
gen x = _n  // 5000 unique values
gen y = mod(_n, 500)  // 500 unique values
* This should definitely exceed limits
tabulate x y
