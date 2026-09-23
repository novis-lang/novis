- **Two sessions in one tree: a file both edit is committed by whichever stages it first, with the
  other's hunks inside.** The loop may hold the tree while you do, and staging a whole file takes
  the foreign hunks along. `git status --short` a file before editing it; if it is already dirty
  with hunks that are not yours, wait for that session's commit or stage your own hunks alone — `git
  show HEAD:<path>` plus your change through `git hash-object -w --stdin` and `git update-index
  --cacheinfo 100644,<blob>,<path>` stages a version the working tree never holds.
  [until: reviewed 2026-09-06]
