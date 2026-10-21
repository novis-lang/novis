- **Renaming a `compile-error` example fails `bun nv reference` while its `.out` looks untouched.**
  The diagnostic's `-->` line names the example's file, so `git mv` of the `.nvs` and `.out` leaves the
  old name in the pinned output, and `docs/reference/lang/*.md` renders it. Re-bless each renamed
  example with `bun nv proofs --bless`, then `bun nv render`. `until: gone tools/nv/cmd/reference.ts:does not contain the`
