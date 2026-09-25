- **A `bun nv peek` window's first printed line is a bad `nv splice` anchor when it lands inside a
  doc comment.** The window starts at the line you asked for, and a wrapped `///` sentence almost
  always began on the line above, so the anchor you copy starts mid-sentence, reads perfectly, and
  `nv splice` refuses it with "the anchor matches for its first 7 character(s)" — the tiny prefix
  length against a verbatim-looking block is the tell. Ask for one line more than you think you need
  whenever the region is prose. [until: gone tools/nv/cmd/splice.ts:the anchor matches for its first]
