- **A tool's prose citing a crate file without its `crates/` prefix fails `bun nv links`, and the
  path it reports is a *suffix* of the one you wrote.** `MENTION_RE` in `tools/nv/cmd/links.ts`
  anchors a bare mention at a top-level directory, so `nvs-stdlib/src/tests/vectors.rs` matches from
  its inner `tests/` and is resolved as `tests/vectors.rs` against the repository root, where
  nothing is. Cite a crate file from the root, and read a `retired` finding whose path you cannot
  find in the line it names as the tail of a longer one.
  [until: gone tools/nv/cmd/links.ts:MENTION_TOPS]
