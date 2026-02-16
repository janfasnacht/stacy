* Data cleaning script
* Called by main.do

* Load data
sysuse auto, clear

* Clean: drop if missing mpg
drop if missing(mpg)

* Create log of price
gen log_price = log(price)

* Label variables
label variable log_price "Log of Price"

display "Data cleaning complete: " _N " observations"
