# Handoff

## State

**M4 — language completeness.** A closure's declared parameter types are checked at the call
(`check_param_tags`, `crates/mwl-runtime/src/closure.rs:325`) and ADR 0007 § 2's one widening is
applied there. Both rules are now pinned twice: from MWL in `tests/conformance/lang/`, and from
native code in `crates/mwl-codegen/tests/closures.rs`, which asserts the three halves a `.mwlt`
case cannot see — the thrown *class* (`LogicError` for a mismatch, `ArithmeticError` past 2^53,
each caught by name rather than as `Throwable`), the uncaught `THROWN` status and message left on
the context, and that the partial result `Core\Arr::filter` abandons mid-walk is freed.

That leak guard measures live bytes through its own `#[global_allocator]` in the test binary — the
pattern `crates/mwl-codegen/tests/arrays.rs:130` already uses — and asserts a *slope*: 200 and
2,000 iterations must hold the same bytes. It is calibrated rather than guessed, and the numbers
are in its comment.

`mwl_ir::lower::param_tag_nibble` (`crates/mwl-ir/src/lower/mod.rs:2705`) is already the "a new
`Ty` row has to be decided" guard the previous handoff pointed at; the playbook says why it cannot
be reproduced in `mwl-codegen`. Nothing in this area is open below the front end.

`verify.py` green — conformance 619, differential 173, unchanged: both slices are Rust tests.

## Next group

**Decide what a callback's *key* argument carries, then pin it.** The file set:
`crates/mwl-stdlib/src/arr.rs`, `docs/spec/01-core-library.md`, `tests/conformance/core/`.

- [ ] **Decide it and record it.** Every member renders the key as a `string` before the call —
      `crates/mwl-stdlib/src/arr.rs:913` (`filter`), `:1002` (`map`), `:1068`, `:1158`, `:2568`,
      `:2670` (`reduce`, where the carry is argument 1 and the key argument 3) — so a
      two-parameter callback over a *list* that declares `int $k` now throws where before the tag
      check it read an integer key's payload as an `MwlStr`. PHP hands the callback the native
      key, and priority 2 outranks the simplicity of one rendering, so handing the key in its own
      form is the likely answer; `docs/spec/01-core-library.md:313` (R9) is the home for whichever
      it is, and `crates/mwl-stdlib/src/arr.rs:40` § *A callback that does not want a key is never
      handed one* owns the mechanism it changes.
- [ ] **A `.mwlt` case in the agreement shape**, asking the same two-parameter callback of `filter`
      and `map` over a list and over a string-keyed array, so a member that grew its own key
      rendering fails rather than printing plausibly on its own line.
- [ ] **`reduce`'s three-argument callback**, same file: the tag check counts positions, so a
      declared-type reducer is the one shape where an off-by-one in the nibble word shows up.
      `crates/mwl-runtime/src/closure.rs:346` reads the nibbles.

## Backlog

- `mwl_codegen::ty`'s `clif_ty` and `tag_of` both end in `_ =>`, so a new `Ty` defaults silently
  there — `crates/mwl-codegen/src/ty.rs:56` and `:119`.
- `crate::helpers`' `does_not_fit` still cannot raise `ArithmeticError`; its own `# Known gap`
  owns it.
- The rest of M4's holes: `python tools/holes.py`, and `python tools/loop.py --list` for the
  named cases each stage still owes.
