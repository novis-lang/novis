- **A `python - <<'PY'` heredoc eats a backslash, so a patch or probe script cannot carry a Rust,
  Novis or `Core\X` escape.** The quoted heredoc is the construct you reach for because it is
  supposed to pass bytes through, yet `\\x` still arrives as `\x` and `Core\Xml` as `CoreXml`: the
  script matches nothing (no error), dies with `SyntaxError: truncated \xXX escape`, or reports
  every class missing. Write the script or the text with Write to a file under `.agent-tmp/` and
  have the shell only *name* it, or use Edit and `bun nv splice --patch`, which never cross
  a shell. [until: reviewed 2026-09-06]
