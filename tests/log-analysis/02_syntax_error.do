* Test script: Syntax error (r(198) or r(199))
* Should fail with unrecognized command

clear all
set obs 100

gen x = rnormal()

* This should trigger r(199) - unrecognized command
thisisnotacommand x y z
