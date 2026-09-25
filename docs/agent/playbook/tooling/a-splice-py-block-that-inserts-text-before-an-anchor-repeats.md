- **A `nv splice` block that inserts text before an anchor repeats that anchor on both sides, so
  fixing a stale `--- old` copy alone silently rewrites that line.** The tool matches the OLD side
  and writes the NEW one verbatim, so an "anchor" is an edit too, and a landed doc comment changes
  under an insertion that never meant to touch it. Correct both copies, and read the anchor lines
  out of `git diff` after a splice whose blocks were insertions.
  [until: gone tools/nv/cmd/splice.ts:<<<<<<< OLD]
