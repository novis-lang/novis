- **A `.nvs` file can be CRLF in the working tree and LF in the index, and then every line of a
  diff against it reads as changed.** `.gitattributes` pins `*.nvs text eol=lf`, so a fresh
  checkout is LF, but `core.autocrlf=true` left `examples/match.nvs` and a dozen more CRLF on disk
  with `git status` still clean, because the commit-side normalization makes them equal. Run `git
  ls-files --eol <path>` before believing such a diff, and build a frozen fixture with the Write
  tool rather than `cp`, because `nvs fmt` writes LF. [until: reviewed 2026-10-12]
