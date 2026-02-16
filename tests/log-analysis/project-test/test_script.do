* Test script: Use project-local ado file
* Should load mytest.ado from ./ado/

clear all

display "=== Testing project-local ado directory ==="

* Call the project-local command
mytest

display "Success! Project-local ado works"
