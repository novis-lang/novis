- **Prefer `re:pattern` to `/pattern/` in a `peek.py` target on Windows.** Git Bash rewrites any
  argument that *starts* with a slash into a Win32 path before the process sees it, so `file.rs:/fn
  foo/` arrives as `file.rs;C:/Program Files/Git/fn foo/` and the tool reports no such file; quoting
  does not help, because the conversion happens in argv handling. The same trap catches any tool
  argument spelled as a leading-slash path. [until: reviewed 2026-09-06]
