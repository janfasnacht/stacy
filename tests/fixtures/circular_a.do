* Circular dependency test: A depends on B

version 14
set more off

display "Script A starting"

* This creates a circular dependency with circular_b.do
do "tests/fixtures/circular_b.do"

display "Script A ending"
