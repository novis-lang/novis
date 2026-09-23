- **A goal switch orphans whatever a carried check's green depended on, and `cargo test -p nvs-ir
  --test refusals` is where you find out.** `goal-switch.py` carries the outgoing goal's `[[check]]`
  blocks forward and its unclosed items not at all, so an `nvs-ir (no refusal left)` check arrives
  without the item list that attributed its sites — a red in a crate the session never touched.
  `python tools/holes.py` reads `docs/agent/carried-refusals.md` as a second item source (`--item
  901` and up); before writing anything, `python tools/holes.py --unattributed` says whether the
  sites are new. [until: gone tools/holes.py:carried-refusals]
