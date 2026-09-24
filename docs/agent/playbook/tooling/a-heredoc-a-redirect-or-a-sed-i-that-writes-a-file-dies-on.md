- **A heredoc, a `>` redirect or a `sed -i` that writes a file dies on the first apostrophe or
  backtick, and a doc comment is made of both.** The shell expands `` `rule:...` `` and eats the
  quoting before the tool it feeds ever runs, so it lands as mangled content on disk rather than as
  an error you can see. Write and Edit carry file content into the tree and
  `bun nv splice --patch <file>` does three or more edits in one call; `python
  tools/loop-stats.py` counts the sessions that reached for the shell instead.
  [until: reviewed 2026-09-10]
