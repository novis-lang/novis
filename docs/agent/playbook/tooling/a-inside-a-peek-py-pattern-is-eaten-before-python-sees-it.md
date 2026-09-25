- **A `"` inside a `peek.py` pattern is eaten before Python sees it, and the error names the *regex*
  rather than the quoting.** Windows re-quotes a native command's arguments after PowerShell has
  finished with them, so an embedded double quote opens a quoted region that swallows the next
  target too — `bad regex /name: [a-z] other:re:x/: multiple repeat`, a regex visibly two targets
  joined. Write the pattern without `"` (`name: .[a-z]` matches the same rows) rather than hunting
  for an escape that survives both layers. [until: gone tools/nv/cmd/peek.ts:peek]
