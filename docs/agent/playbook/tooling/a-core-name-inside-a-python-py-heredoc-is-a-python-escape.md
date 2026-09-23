- **A `Core\Name` inside a `python - <<'PY'` heredoc is a Python escape error, not just a Bash
  one.** `"""… Core\Uri …"""` reaches Python intact and *Python* then rejects `\U` as a truncated
  `\UXXXXXXXX`, so a patch script quoting any `Core\U…`/`Core\N…` path dies at parse time with
  nothing about the real edit in the message. Use the Edit tool for a targeted replacement, or
  `splice.py` with a Write-tool patch file. [until: reviewed 2026-09-06]
