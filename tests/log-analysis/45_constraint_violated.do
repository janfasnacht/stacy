* Test: Constraint violation
clear all
set obs 10
gen x = rnormal()
* Set impossible constraint
constraint 1 x = 0
constraint 2 x = 1
* Both cannot be true
