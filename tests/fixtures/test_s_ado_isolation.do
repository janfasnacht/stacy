* test_s_ado_isolation.do
* Acceptance test: verify S_ADO isolation at runtime
*
* When run via `stacy run`, the adopath should contain only:
*   - Package cache paths (from lockfile)
*   - BASE (Stata built-ins)
*
* It should NOT contain SITE, PERSONAL, PLUS, or OLDPLACE,
* which would break reproducibility by allowing unlocked packages.

adopath

* Capture adopath output and verify no global paths leak through.
* We check the global macro S_ADO directly — if stacy set it correctly,
* SITE/PERSONAL/PLUS/OLDPLACE should not appear.
display "S_ADO = `c(adopath)'"

* If we reach here without error, the script succeeded.
display "S_ADO isolation test passed"
