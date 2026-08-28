# Handoff

## State

**M4's Stage 8, with the refusal ceiling at 4, and the acceptance gate green.** The tree is at 868
conformance plus 189 differential, and 1758 Rust tests across 81 suites. Nothing is blocked.

The last session's work is unchanged and still green. What changed since is an audit pass the
user fired by hand, and one of its findings moves this file's next group:

**The refusal gate counted arms, not shapes, and item 25 was about the wrong thing.**
`refusals.rs` attributes a site to the item whose anchors sit in the same file, and one catch-all
arm is one site however many shapes fall into it. `lower_checked_ty` was exactly that: item 25
claimed it because item 25 anchors `lower/mod.rs`, item 25 was about `object`, and `object` has
had a representation arm for some time — a program that declares one, passes one and returns one
runs. What actually reaches that arm is **`never`, `iterable` and an intersection**, each in both
declaration positions, all six type-checking and then panicking.

`crates/nvs-ir/tests/type_atoms.rs` is the half that names shapes: every atom ADR 0007 § 3 spells,
in a parameter and in a return, through parse/resolve/check/lower with the unwind caught.
`KNOWN_ICE` ratchets exactly like `CEILING` — it may not grow, and a row that stops panicking fails
until it is deleted. Its name is in `loop-goal.toml`'s Stage 8 block beside
`every_refusal_is_a_diagnostic_or_decided`.

`verify.py` now has a seventh step, `doc`: `cargo doc` with rustdoc's broken-link lint denied. The
workspace had 391 intra-doc warnings and has none; a stale `[`Foo::bar`]` is a failed step now
rather than something nothing ran. It costs four seconds on a warm tree.

## Next group

**Item 25, and it is three shapes rather than one.** File set:
`crates/nvs-ir/src/lower/mod.rs`, `crates/nvs-types/src/expr/assign.rs`,
`crates/nvs-ir/tests/type_atoms.rs`, and cases under `tests/conformance/lang/`. Read the item with
`python tools/holes.py --item 25`.

- [ ] **`never` first, because it is the smallest and half of it is the checker's.** ADR 0007 § 3
      makes `void` and `never` return-only; `void` in a parameter is diagnosed and `never` is not,
      so the `("never", Position::Param)` row closes with a diagnostic naming § 3, not with a
      representation. The return row then needs `erase_checked_ty`
      (`crates/nvs-ir/src/lower/mod.rs:2206`) to answer for `CheckedTy::Never` — a body that
      declares it cannot return, so what the arm owes is a representation for a value no path
      produces.
- [ ] **`iterable` and the intersection are two halves each, and the checker's half comes first.**
      Nothing is assignable to either in `nvs_types::expr::assign` — not an `array<int>`, not a
      `Core\Generator`, not a class implementing every member of the intersection — so both are
      types no value can inhabit and an IR arm alone would not make either usable. Decide the
      assignability rule, then give `erase_checked_ty` the arm.
- [ ] **Delete each row from `KNOWN_ICE` in the slice that closes it**, and pin the result in a
      `tests/conformance/lang/` case. `refusals.rs`'s `CEILING` ratchets from 4 in the same slice
      if the arm itself goes.

## Backlog

- Item 16's lowering half (`crates/nvs-ir/src/lower/call.rs:773`) — confirmed reachable: `callable
  $g = $f(...);` on a value already typed `callable` panics there. Item 4's `stmt.rs:268`
  catch-all is the other of the four sites, and its own comment argues no shape reaches it, which
  is the goal's "a panic naming the roster that proves nothing reaches it" case.
- **`tests/conformance/diag/` has no case for `new Core\Error("x")`**, which panics
  `lower/expr.rs:2368` naming `nvs-types`' own zero-arity gap. Tracked in that crate's known gaps,
  not here, but it is one `.nvst` case away from being a diagnostic.
- **[ADR 0109](../adr/0109-a-for-header-declares-its-own-counter.md) is `Proposed` and waiting on
  the user.** `for (int $i = 0; …)` does not parse; accepting it obliges the row in 0007 § 1's
  binding-site table, that ADR's `Amended by:`, and 0109's `Amends:`, all three in the commit that
  flips the status.
- **`website/` has 8 stale pages** (`cd website && python site.py check`), each an authored page
  whose source ADR has changed since it was blessed. Not a gate — that job is `|| true` on
  purpose — but it is the public surface.
- **At the M4 → M4B boundary, not before:** re-run `python tools/playbook.py --goal --min 3`
  against the current module list, then re-measure with `loop-stats.py` and only then revisit the
  120k slice gate in AGENTS.md.
- `python tools/gaps.py` ranks the thinnest `Core` classes once the refusal sites are gone.
