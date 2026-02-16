* Script with syntax error for testing error detection

version 14
set more off

* This command doesn't exist - should trigger unrecognized command error
foobar this is not a valid command

* Should not reach here
display "This should not execute"
