- **One call reads many places: `bun nv peek A.rs:120-160 B.rs:@sym C.md:"## 4" D.rs:re:pat:3`.**
  Locators are `120-160`, `120+30`, `@symbol`, `re:pattern` — which is the matching line and nothing
  else, so `re:pattern:3`, or `--context 3` for the whole call, is what brings the block with it —
  `"## Heading"`, or nothing for a whole file under 400 lines, and the path may be a glob, so one target
  can sweep a crate. `--locate <symbol> ...` answers with `file:line` and no bodies, which is what a
  handoff's anchors are made of; reach for it instead of a `grep`, then a `sed`, then another
  `grep`. [until: gone tools/nv/cmd/peek.ts:re:]
