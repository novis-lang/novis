- **A session that died leaves its whole slice on disk, and the next pack hands you the item as if
  nothing had been done.** A session that exits without a commit and without `.loop/status.txt`
  leaves `orient.py` quoting the same handoff to the next one, whose item is the unverified work
  already in the tree; the tell is a `.loop/log.md` session line with `(no status written)`. `git
  status --short` **before the first edit** — then the owed work is `nv verify`, the commit
  messages, and the next slice, not a second implementation. [until: gone tools/nv/cmd/orient.ts:uncommitted changes]
