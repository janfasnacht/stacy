* Main analysis script
* Orchestrates the full analysis pipeline

display "Starting analysis pipeline..."

* Step 1: Clean data
do "clean_data.do"

* Step 2: Run analysis
do "analysis.do"

display "Pipeline complete!"
