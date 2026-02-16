* Circular dependency test: B depends on A

version 14
set more off

display "Script B starting"

* This creates a circular dependency with circular_a.do
do "tests/fixtures/circular_a.do"

display "Script B ending"
