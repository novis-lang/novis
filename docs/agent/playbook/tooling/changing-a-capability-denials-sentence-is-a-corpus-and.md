- **Changing a capability denial's sentence is a corpus *and* reference edit, not a one-line one.**
  `grep -rln "which is not granted" tests/ docs/reference/` is the blast radius: the conformance
  cases, several of which compare `$e->message ==` inside program logic rather than echoing it
  (`tests/conformance/core/io-copy-names-two-grants-and-neither-one-alone-is-enough.nvst:23`) so an
  expectation bump never finds them, and the executed `output` fences in
  `docs/reference/lang/70-errors.md`, `docs/reference/lang/80-concurrency.md` and
  `docs/reference/tools/20-config.md`, which `nv verify`'s `reference` leg runs and
  which owe `python tools/reference.py --no-examples` for `docs/novis.md` in the same commit. Budget
  the pass as the slice itself, and price any design that leaves `$e->message` alone against it
  before choosing. [until: reviewed 2026-09-09]
