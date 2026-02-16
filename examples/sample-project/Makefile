# Sample Makefile demonstrating stacy integration
#
# stacy provides proper exit codes, enabling Make to detect Stata errors
# and stop the build when something goes wrong.
#
# Usage:
#   make all        # Run full analysis
#   make clean      # Remove generated files
#   make analysis   # Run just the analysis

# Use stacy for Stata execution (falls back to stata-mp if stacy not found)
STATA := stacy run
# STATA := stata-mp -b do  # Traditional approach (doesn't detect errors!)

# Default target
all: results/analysis.log

# Run the main analysis pipeline
results/analysis.log: main.do clean_data.do analysis.do
	@mkdir -p results
	$(STATA) main.do
	@mv main.log results/analysis.log
	@echo "Analysis complete. Log saved to results/analysis.log"

# Run just the analysis script
analysis: analysis.do
	$(STATA) analysis.do

# Clean generated files
clean:
	rm -rf results/
	rm -f *.log

# Show dependencies
deps:
	stacy deps main.do

.PHONY: all analysis clean deps
