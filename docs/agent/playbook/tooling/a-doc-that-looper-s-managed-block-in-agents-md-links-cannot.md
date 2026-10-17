- **A doc that looper's managed block in AGENTS.md links cannot be deleted, and the block cannot be
  edited to drop the link.** `looper lint` refuses any hand edit between the `looper:begin` markers,
  and `looper sync` restores the template's row, so deleting `docs/design.md` breaks `bun nv links`
  while repointing the row breaks `looper lint`. Keep the linked file as a short pointer, and change
  looper's template first. See `AGENTS.md`. `until: gone docs/design.md`
