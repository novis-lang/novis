- **A handoff calling a stage "landed, end to end" claims behaviour, not that stage's acceptance
  artefacts.** Goal `core-class-tests` stage 2's checker, IR and codegen were all on disk and the
  class test answered, while none of its three `cargo-named` tests and neither named `.nvst` path
  existed. Grep a red check's `tests` against `crates/` and its `cases` against `tests/conformance/`: a
  whole list missing while the behaviour runs means the artefacts are the work, not a re-point.
  [until: reviewed 2026-09-17]
