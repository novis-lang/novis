- **rustfmt can stale a perf figure, because the trailing comma it adds when it breaks a signature
  is a token the impl hash reads.** A figure recorded over an unformatted file goes stale at the
  gate stage's `fmt` step, and `bun nv ci --check`'s proofs gate names every member of that file.
  Run `cargo fmt` before `bun nv proofs --record-perf`, never after. See
  `tools/nv/proofs/collect.ts`.
