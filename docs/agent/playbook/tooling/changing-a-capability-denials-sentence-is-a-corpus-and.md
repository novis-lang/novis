- **Changing a capability denial's sentence is a corpus *and* reference edit, not a one-line one.**
  `grep -rln "which is not granted" tests/ docs/reference/` is the blast radius: the conformance
  cases, some comparing `$e->message ==` in program logic, and the executed `output` fences in
  `docs/reference/lang/70-errors.md`, `docs/reference/lang/80-concurrency.md` and
  `docs/reference/tools/20-config.md`, which `nv verify`'s `reference` leg runs and
  which owe `bun nv render --website` for `website/public/llms-full.txt` in the same commit. Budget the pass as the slice itself. `until: gone crates/nvs-runtime/src/capability.rs:which is not granted`
