- **A `cargo-named` check reports only the *first* of its missing names, so one "did not run" can be
  four tests of work.** The driver's line names one test; the check's `tests` list may hold several,
  all unwritten. Size the item from the check's whole list with one `grep -rn` over the names, which
  also separates a name the tree pins elsewhere from one nobody has written.
  [until: reviewed 2026-09-06]
