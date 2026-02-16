* Test script: Access environment variables (correct syntax)
* Stata uses extended macro functions for this

clear all

display "Testing environment variable access (correct syntax):"

* Method 1: environment() function
display "USER: " _c
display "`c(username)'"

* Method 2: Extended macro function
local user : environment USER
display "ENV USER: `user'"

local home : environment HOME
display "ENV HOME: `home'"

local custom : environment CUSTOM_VAR
display "ENV CUSTOM_VAR: `custom'"

local stacy_arg : environment STACY_ARG
display "ENV STACY_ARG: `stacy_arg'"

display "Environment test complete"
