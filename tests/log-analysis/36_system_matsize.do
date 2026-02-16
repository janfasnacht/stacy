* Test: Matrix size exceeded
* This is hard to trigger reliably
* Using a simpler approach
clear all
* Try to create too many variables (simplified)
set obs 1
forvalues i = 1/10000 {
    gen var`i' = `i'
}
