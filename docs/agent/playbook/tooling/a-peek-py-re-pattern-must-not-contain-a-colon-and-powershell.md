- **A `peek.py` `re:` pattern must not contain a colon, and PowerShell must not see its `|`
  unquoted.** `peek.py 'file.rs:re:name: "co:6'` reads everything after the last colon as the
  context count and answers `NO SUCH FILE`, and a double-quoted argument holding `(a|b)` is split
  by the shell before Python is started at all. Single-quote the whole target and keep colons out
  of the pattern; a pattern that needs one is a `Grep` call instead. [until: reviewed 2026-09-18]
