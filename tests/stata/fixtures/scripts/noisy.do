* noisy.do - Prints output that breaks `do` if it leaks into the result channel (#84)
* Lines starting with "(" are not valid Stata commands; if the CLI streams them
* into --format stata stdout, _stacy_exec's `do` fails with r(199).
display "( 42 observations deleted )"
display "( note: free-form script output )"
