* Test script: Long-running script to test timeouts
* Should run for ~5 seconds

clear all

forvalues i = 1/5 {
    display "Iteration `i' of 5"
    sleep 1000  // Sleep for 1 second (1000 ms)
}

display "Long-running script completed successfully"
