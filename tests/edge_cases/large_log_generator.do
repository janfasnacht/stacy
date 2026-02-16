* Generate a large log file to test memory-efficient parsing
* This will create a log with many lines to verify we don't load entire file into memory

display "Generating large log file..."
display "This will produce many lines of output"

* Generate 50,000 lines of output (should be ~5-10 MB)
forvalues i = 1/50000 {
    display "Line `i': This is a test line with some content to make it longer and increase log file size"
}

display "Large log generation complete"
display "Total lines: 50,000"
