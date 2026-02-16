* Sample analysis script for stacy
* Demonstrates basic Stata workflow

* Load built-in dataset
sysuse auto, clear

* Summary statistics
summarize price mpg weight

* Simple regression
regress price mpg weight

* Display results
display "Analysis complete!"
display "Number of observations: " _N
