- **The Bash tool mangles backslashes and escapes inside a heredoc, so a spliced block silently
  fails to match or lands corrupted.** A `\\` in a `python - <<'PY'` heredoc arrives as `\`, a
  `"\n"` arrives as a real newline, and a Rust `\`-continuation comes back as a literal `\n` that
  still compiles. Use Write/Edit, or `bun nv splice <target> --patch <file>` with the patch
  written by the Write tool; if an edit genuinely must be scripted, build a backslash as `chr(92)`,
  and give a Rust string holding a `Core\Name` label `r"..."`. [until: reviewed 2026-09-06]
