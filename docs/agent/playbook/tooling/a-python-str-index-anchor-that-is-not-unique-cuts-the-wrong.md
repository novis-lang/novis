- **A Python `str.index` anchor that is not unique cuts the wrong region and *duplicates* the file,
  with no error.** A script splicing with `s[:a] + s[b:]` that finds an earlier match for `b` has `b
  < a`, removes nothing, and hands the file back longer than it went in — only `git diff --stat`
  notices. Use the Edit tool to remove a block, since it refuses a non-unique `old_string`; if a
  script really must cut, `assert a < b` and print the slice length, and recover with `git checkout
  -- <file>` plus re-applying the edits. [until: reviewed 2026-09-06]
