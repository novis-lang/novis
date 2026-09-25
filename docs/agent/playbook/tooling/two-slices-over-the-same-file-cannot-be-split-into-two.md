- **Two slices over the same file cannot be split into two commits, so commit the first before
  starting the second.** `bun nv session --wrap` stages a `## commit:` by *pathspec*, so if two slices
  both edit `routes.rs` the first section sweeps both slices' changes and the second commits nothing
  — and a "next group" shares a file set by definition, so this is the normal case. Either accept
  one commit with two clauses, which house style allows, or `git commit` slice 1 by hand once its
  own crate's tests are green and let the wrap commit the rest. [until: gone tools/nv/cmd/session.ts:## commit:]
