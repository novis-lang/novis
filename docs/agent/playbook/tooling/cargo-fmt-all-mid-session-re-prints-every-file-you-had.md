- **`cargo fmt --all` mid-session re-prints every file you had already read.** The harness sees each
  file the formatter rewrote as changed on disk and pastes it back into context, so a formatting
  pass costs far more than its own output. `bun nv verify` formats as step 1 and its
  `formatted N file(s)` line is the tell; write formatted code and there is nothing to re-print, and
  never run a rewriting tool over open files except `session.py --wrap`, where the session ends.
  [until: gone tools/nv/cmd/verify.ts:formatted]
