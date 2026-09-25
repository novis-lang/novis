- **A `## commit:` section cannot name a path the session renamed away, so `git mv`'s staged deletion
  is left out and the rename lands half-committed.** `bun nv session --wrap` refuses a path that does not
  exist, and it commits by pathspec, so the old filename stays staged in the index while the new one
  goes in — a fresh checkout of that commit carries both. Name only the new path in the section, then
  commit the leftover deletion straight after the wrap. [until: gone tools/nv/cmd/session.ts:## commit:]
