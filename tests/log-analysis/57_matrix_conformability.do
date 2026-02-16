* Test: r(503) - Matrix conformability error
* Matrix operations require compatible dimensions
clear all
matrix A = (1, 2 \ 3, 4)
matrix B = (1, 2, 3)
* Cannot add 2x2 and 1x3 matrices
matrix C = A + B
