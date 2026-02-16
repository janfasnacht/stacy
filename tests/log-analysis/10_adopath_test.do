* Test script: Understanding adopath mechanism
* Show default adopath and test override

clear all

display "=== Default adopath ==="
adopath

display ""
display "=== S_ADO global macro ==="
display "$S_ADO"

display ""
display "=== Adding custom path with ++ (prepend) ==="
adopath ++ "/custom/ado/path"
adopath

display ""
display "=== S_ADO after prepend ==="
display "$S_ADO"

display ""
display "=== Removing custom path ==="
adopath - "/custom/ado/path"
adopath

display "Test complete"
