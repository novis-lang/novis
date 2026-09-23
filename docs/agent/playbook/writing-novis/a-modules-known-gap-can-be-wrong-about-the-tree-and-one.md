- **A module's "known gap" can be wrong about the tree, and one stale field comment is how it gets
  that way.** A gap saying a mechanism is missing, against a rule saying it exists, can rest on a
  single doc line that stopped being true when the mechanism landed, and the gap then travels from
  handoff to handoff. When a doc and a rule disagree about what exists, `grep -n` the enum, or probe
  a four-line scratch file with `target/debug/nvs.exe check`, before believing either.
  [until: reviewed 2026-09-06]
