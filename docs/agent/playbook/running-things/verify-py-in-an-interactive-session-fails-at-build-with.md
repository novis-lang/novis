- **`verify.py` in an interactive session fails at build with `failed to remove file … nvs.exe` (os
  error 5) while the unattended loop is mid-session.** The loop's own test run holds the binary and
  cargo cannot replace a running exe on Windows — contention, not a broken tree. Check first
  (`Get-CimInstance Win32_Process` shows `loop.py` and a `target\debug\nvs.exe`), commit finished
  slices before verifying so the loop's wrap cannot sweep them, and retry when the binary frees;
  never kill the loop's `nvs.exe` to win the race. [until: reviewed 2026-09-06]
