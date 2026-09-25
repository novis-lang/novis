- **An anchor window — `bun nv orient`'s inlined code, or a `bun nv peek` line range — carries no
  `impl` header, so the receiver type in it is a guess.** Methods of an inner type read as the outer
  type's, the tell is `error[E0599]: no method named ...` after a full rebuild, and `bun nv peek
  --locate` answers the symbol, not its owner. One `grep -n '^impl ' <file>` filtered to the lines
  around the anchor says which type you are adding a call to. [until: gone tools/nv/cmd/orient.ts:THE CODE YOUR ITEM ANCHORS]
