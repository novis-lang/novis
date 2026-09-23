- **Writing a tree file through Python's text mode rewrites every line ending on this checkout.**
  `Path.read_text` maps `\r\n` to `\n` and `write_text` writes back what it was handed, so a
  one-word change reports every touched file modified with an empty `git diff`, buries the real
  change in `git status`, and makes a "restore the original bytes" rollback a lie. Open with
  `newline=""` in both directions. [until: reviewed 2026-09-06]
