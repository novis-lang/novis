# Handoff

## State

**M4's Stage 8, with the refusal ceiling at 4, and the acceptance gate green again.** The tree is
at 868 conformance plus 189 differential. Nothing is blocked.

`abi-probe [9 guards]`, red after sessions 0055-0057, is closed: all 12 tests in
`benches/abi-probe/tests/perf_guards.rs` pass under `cargo test --release -p nvs-abi-probe`. The
guard was wrong, not the lowering, exactly as the previous session judged. `emitted_calls` became
`path_calls` (`perf_guards.rs:808`): it parses the section's `blockN:` bodies out of the VCode
text, walks them from the entry, and refuses to follow the taken edge of a `test`-then-`jnz` pair,
which is the one shape every status check compiles to. Three more unhooked accesses now measure 0
extra calls against the hooked pair's 12, and the probe/safepoint correction the slope used to
carry is gone with it — those calls sit behind a status test too, so they were never on the path.
The playbook bullet under *Writing Novis itself* owns the recognition rules.

ADR 0014 § 4 now records the judgement itself: "direct field load or store" is a claim about the
path that runs, and a landing block's `Release` belongs to ADR 0002's checked return and ADR 0007
§ 4's overflow throw rather than to this ADR. Its *Verification* bullet was quoting the old
"beyond the probe sites they add" wording and now quotes the new one.

## Next group

**Item 25, `object` as a declared type — both remaining `nvs-ir` refusal sites are in one file.**
File set: `crates/nvs-ir/src/lower/mod.rs`, `crates/nvs-ir/tests/refusals.rs`, and one new case
under `tests/conformance/lang/`. Read the item with `python tools/holes.py --item 25`.

- [ ] **Judge the two sites before writing anything.** Both messages already list `object` among
      what they lower (`crates/nvs-ir/src/lower/mod.rs:2701` for a declared type,
      `crates/nvs-ir/src/lower/mod.rs:2784` for a resolved call's parameter or return), so the
      refusal that is left may be the catch-all rather than the feature. If it is, this is the
      goal's § *Standing decisions* "a panic naming the roster that proves nothing reaches it"
      case and the slice is a doc comment plus the ceiling, not a lowering.
- [ ] **Close whichever half is real**, anchored at `erase_checked_ty`
      (`crates/nvs-ir/src/lower/mod.rs:2206` is item 25's own anchor). The playbook's "teaching
      `nvs-ir` a new receiver or value shape is two edits" bullet is about exactly this function.
- [ ] **Ratchet `crates/nvs-ir/tests/refusals.rs`'s `CEILING`** from 4 by whatever this closed,
      and pin the result in a `tests/conformance/lang/` case that declares an `object`, passes one
      as a parameter and returns one.

## Backlog

- Item 16's lowering half (`crates/nvs-ir/src/lower/call.rs:773`, named/spread arguments through a
  `callable`) and item 4's `stmt.rs` declaration catch-all — the other two of the four sites.
- **At the M4 → M4B boundary, not before:** re-run `python tools/playbook.py --goal --min 3`
  against the current module list, then re-measure with `loop-stats.py` and only then revisit the
  120k slice gate in AGENTS.md.
- `python tools/gaps.py` ranks the thinnest `Core` classes once the refusal sites are gone.
