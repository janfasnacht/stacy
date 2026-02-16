* Test script: Check environment variable access
* Test if Stata can read ENV vars set by parent process

clear all

* Try to access environment variables
* Stata uses global macros for this

display "Testing environment variable access:"
display "USER: ${USER}"
display "HOME: ${HOME}"
display "CUSTOM_VAR: ${CUSTOM_VAR}"
display "STACY_ARG: ${STACY_ARG}"

display "Environment test complete"
