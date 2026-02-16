* Test fixture: error after long output (tests truncation)
* Expected: stacy shows "... (N lines omitted)" before error context
sysuse auto, clear
forvalues i = 1/50 {
    display "Line `i' of regression output"
}
summarize price mpg weight
badcmd
