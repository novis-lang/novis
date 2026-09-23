- **A `file:line` cited by the goal prose can point into a case's `--EXPECT--` section, where it is
  output rather than source.** The `//// ____` at
  `tests/conformance/core/encoding-every-encoder-agrees-with-its-own-decoder-over-a-table.nvst:114` is
  an encoder's expected stdout, not a divider comment, and the corpus holds no `////` comment anywhere —
  the only `////` in source sits inside the text of a `//` comment at line 47 of that same file. Read
  which section a cited line falls in before writing the test that pins it: a case file is several
  languages stacked, and only the `--FILE--` one is Novis. [until: reviewed 2026-09-07]
