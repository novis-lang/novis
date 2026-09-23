- **A `peek.py` `re:` pattern carrying a double quote finds nothing under PowerShell, and the miss
  reads as the symbol being absent.** `powershell.exe` drops the embedded quotes out of a native
  command's argument unless they are backslash-escaped, so `'str.rs:re:name: "contains"'` reaches
  the tool as `name: contains` and reports no match over a file holding that exact line. Escape
  them — `'str.rs:re:name: \"contains\"'` — or search on the unquoted half alone, and never read a
  `re:` miss as evidence until the pattern is one bare word. [until: reviewed 2026-09-20]
