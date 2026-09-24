- **`nv splice` refuses an anchor that matches more than once, and a run of identical test call
  sites is exactly that.** Byte-identical argument lists give no unique anchor short of quoting out
  to each enclosing `#[test]` name. When an anchor is not unique, change the shape so the edit
  disappears — bundle the shared arguments into one struct a helper builds — rather than growing the
  anchor. [until: reviewed 2026-09-06]
