- **`grep -rn <pattern> .` from the repo root walks `target/` and times out.** It is minutes of I/O
  over build artifacts for an answer the source tree gives in under a second, and the shell call
  comes back with nothing at all. Name the roots (`grep -rn <pat> crates docs tests`), use the
  harness `Grep` tool, which respects the ignore file, or `bun nv peek
  "crates/**/*.rs:re:pattern"` for several such questions in one call. [until: gone tools/nv/cmd/peek.ts:re:]
