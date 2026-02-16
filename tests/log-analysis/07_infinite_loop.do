* Test script: Infinite loop to test interruption
* Will be killed externally

clear all

display "Starting infinite loop..."

local counter = 1
while 1 {
    display "Loop iteration: `counter'"
    sleep 500
    local counter = `counter' + 1
}

display "This should never execute"
