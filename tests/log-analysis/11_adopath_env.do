* Test script: Can we set adopath via environment variable?

clear all

display "=== Check if S_ADO can be set externally ==="

* Check current S_ADO
display "Current S_ADO:"
adopath

* Try to read from environment
local env_ado : environment S_ADO
display "Environment S_ADO: `env_ado'"

* Try to override manually
display ""
display "=== Manual override test ==="
global S_ADO "PROJECT_ADO;BASE;SITE;PERSONAL;PLUS;OLDPLACE"
adopath

display "Test complete"
