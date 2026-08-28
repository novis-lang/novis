# Handoff

## State

**M4, Stage 00, is closed entirely.** Item 49 was the last of it and landed as three edits in three
files this session; `python tools/verify.py` is 7 of 7 green (1782 unit, 878 conformance, 189
differential).

`crates/nvs-ir/tests/type_atoms.rs` holds both halves of one gate now — the type atoms it opened with,
and `every_spellable_expression_reaches_a_diagnostic_or_an_ir`, a roster of source shapes read off
`nvs_syntax::ast`'s `ExprKind`/`StmtKind` rather than assembled from memory. Six tables, three slots,
and each row asserts **which** of a diagnostic or an IR it reaches, so a row that stops compiling for a
reason nobody intended fails instead of passing. The file's own module doc owns the design; do not
restate it elsewhere.

**`tools/holes.py` read 4 refusal sites where there are 17**, and `refusals.rs`'s `CEILING` is 17 now.
Nothing was added to the tree — the recognizer was matching three fixed phrasings and is keyed on the
construct plus the claim's shape instead. All 17 are attributed to open items and 0 are unattributed.
The playbook bullet under *Tooling* owns why a construct-only match is wrong, and it is worth reading
before touching that regex again.

`docs/agent/guard-name-debt.md` § *Stage 00's remaining names* is empty: no `loop-goal.toml` name is
deliberately unwritten today.

## Next group

**Item 25 — `never`, `iterable` and an intersection reach a representation or a diagnostic.** It is
the item the type table's whole `KNOWN_ICE` list points at (six rows, three shapes × two positions),
so closing it empties that ratchet. One file set: `crates/nvs-ir/src/lower/mod.rs`,
`crates/nvs-ir/tests/type_atoms.rs`, `crates/nvs-ir/tests/refusals.rs`, plus `nvs-types` for the first
slice's diagnostic.

- [ ] **`never` in a parameter position is a diagnostic** (ADR 0007 § 3 makes `void` and `never`
      return-only; `void` there is already refused and `never` is not). Next free code is `E0742`
      (`crates/nvs-diagnostics/src/lib.rs`). The table row that found it is
      `crates/nvs-ir/tests/type_atoms.rs:114`.
- [ ] **`iterable` and an intersection get a representation**, both erasing to `Ty::Object` as
      `Ty::Shape` already does. `crates/nvs-ir/src/lower/mod.rs:2206` (item 25's anchor), with the
      two catch-alls at `:2705` (`lower_checked_ty`) and `:2792` (a resolved call's types).
- [ ] **Delete the closed rows and lower the ratchets in the same slice**:
      `crates/nvs-ir/tests/type_atoms.rs:105` (`KNOWN_ICE`, which fails on a row that has *stopped*
      panicking) and `crates/nvs-ir/tests/refusals.rs:58` (`CEILING`, 17 minus whatever closes).

**If the driver's acceptance check names a failure, that outranks this** — a check naming a test that
"did not run" is an open item, anything else is a regression.

## Backlog

- Item 16, ADR 0027's `Class::method(...)`: 4 refusal sites, the only `SHAPE_ICE` row, and the two
  `expr.rs` panics read as a checker/lowering mismatch (`docs/agent/loop-goal.md` item 16).
- Make an `nvs-ir` invariant panic spell itself as one, so `holes.py` need not judge by wording — the
  `CodegenError::Internal`/`Unsupported` split is the precedent (`tools/holes.py`, `CONSTRUCT`).
- `nvs-codegen`'s `class_desc_const`/`field_offset` raise `Unsupported` for a descriptor above
  `i64::MAX`; `loop-goal.md` § *Standing decisions* says an unreachable `CodegenError` is `Internal`.
- Item 20's `foreach (… as inout $v)`, 3 sites, and item 17's spread element, 2 (`holes.py --item N`).
