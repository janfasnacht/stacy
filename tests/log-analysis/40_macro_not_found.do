* Test: Macro not found
clear all
display "`undefined_macro'"
* Try to use undefined macro in command
gen x = `undefined_macro'
