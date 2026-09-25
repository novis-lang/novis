- **Widening a member's declared type in `nvs_stdlib::registry` moves four expectations, none naming
  the constant you edited.** Adding to `router.rs`'s `CAPTURE` union changed a rendering pinned in
  `crates/nvs-types/src/core_lib.rs`'s tainted-answer roster, two `--EXPECTF-ERROR--` cases under
  `tests/conformance/core/`, and the generated `docs/novis.md`. One `grep -rn` for the old rendering
  across `crates docs tests` finds all four, and since the rendered union order is not the
  declaration order, `python tools/try.py <case>` prints the real one. [until: gone crates/nvs-types/src/core_lib.rs:nvs_stdlib::registry]
