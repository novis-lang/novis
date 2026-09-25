- **A `peek.py` `re:` target over a wide glob prints every matching file's hits, and
  `tests/conformance/**/*.nvst` is over a thousand of them.** One probe for a spelling across the
  corpus comes back as tens of kilobytes, because every matching file prints its hits with context.
  Name one file, or ask a question whose answer is a handful of lines; when the question really is
  "what is the spelling for X across the corpus", send a subagent, whose findings are three lines
  and whose reading is enormous. [until: gone tools/nv/cmd/peek.ts:re:]
