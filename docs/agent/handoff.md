# Handoff

## State

**M4 — item 18 is closed: a `&$x` argument's copy-back lands at the call, so
such a call lowers in any expression position.** `mwl-ir`'s known gap 10 is
deleted rather than reworded, and `examples/callable.mwl` prints all seven of
its stage-3 acceptance lines, `byref=7 then 7` included. The `mwl-ir (calls)`
acceptance list is complete: all eight names now exist.

- **The rule is a mark, not a boundary.** A call site takes
  `Lowering::pending_refs_mark` before it lowers its argument list
  (`crates/mwl-ir/src/lower/mod.rs:2074`) and hands it back to
  `flush_ref_writebacks` (`:2091`) after emitting its call, at the three sites
  that can stage one — `lower_new`, `lower_method_call`, `lower_static_call`
  (`crates/mwl-ir/src/lower/expr.rs:3664`, `:3819`, `:4008`). ADR 0063 R7
  leaves nothing by-reference in `Core`, so the three `CoreCall` sites need no
  mark. `pending_refs`' own doc comment (`mod.rs:1182`) is the rule's one home,
  including the one thing it does not buy — PHP's *operand* order, which the
  manual leaves undefined and which MWL takes left to right.
- **Subtracting the deferral uncovered an older leak**, fixed in the same
  slice: `return $s;` over a `&$x` parameter took `lower_stmt`'s `except`
  exemption and so retained nothing, while `release_all_locals` was never going
  to release a `Ty::Ref` cell anyway. The playbook's "Running things" bullet
  owns the recognition test. Valgrind-clean over fixtures that grow a borrowed
  and a freshly built string through a `&$x` parameter, and write back through
  a property holder, two hundred times.
- **The four statement-level flush sites in `stmt.rs` are gone.**
  `lower_stmts`' assertion stays as an internal-consistency check on the call
  sites (`crates/mwl-ir/src/lower/stmt.rs:61`), not as a refusal.

## Next group

**The two named `.mwlt` cases item 18 owes, and the divergence one of them must
not write.** The file set: `tests/conformance/lang/`, `tests/differential/lang/`
— no Rust. The rule they pin lives at `crates/mwl-ir/src/lower/mod.rs:1182`.

- [ ] **`tests/conformance/lang/a-reference-argument-is-written-back-before-the-next-read.mwlt`**
      — `python tools/loop.py --list` names it and stage 8 owes it. The shapes
      that run today, all checked by hand this session: a read to the right of
      the call in one statement, two calls in one statement where the second
      sees the first's write, a nested call
      (`Adder::sum(Adder::bump($n), $n)` answers `12`), a call in a condition,
      in an array-literal element, in a `while` body, and a property holder
      (`Box::grow($b->tag, "z")`). `crates/mwl-ir/src/lower/call.rs:846`
      (`stage_ref_arg`) is what the two holder kinds are.
- [ ] **`tests/differential/lang/a-reference-argument-matches-phps.mwlt`** — the
      oracle half. **Do not write `$n + Adder::bump($n)` into it**: PHP answers
      `7 + 7` and MWL answers `5 + 7`, PHP reading its left operand at the
      `ADD` after the call. Every other shape in the list above was checked
      against `php` this session and agrees. `mod.rs:1182` says why that one is
      not a divergence to pin.
- [ ] **`tests/conformance/error/a-finally-runs-when-its-catch-body-throws.mwlt`**
      — stage 8's next unwritten case, and the one nearest these two in the
      tree. `crates/mwl-ir/src/lower/exception.rs` owns the ladder.

## Backlog

- First-class callable syntax (`Class::method(...)`) still panics — `mwl-ir` gap
  1, at `lower/call.rs:85` and `:652`. Both are `CallArgs::FirstClassCallable`.
- ADR 0007 § 4's promotion table has 9 refusal sites left (`python
  tools/holes.py --item 1`), the largest remaining group.
- `crates/mwl-codegen/src/ty.rs:116` and `:121` are the two refusal sites no
  item anchors (`holes.py`'s UNATTRIBUTED section).
- An abandoned generator's `finally` — pre-authorized in
  `docs/agent/loop-goal.md` § *Standing decisions*, two named cases owed.
- Item 25: `object` as a declared type has 2 refusal sites left.
- `holes.py` counts 14 named cases still to write across stages 8 and 9.
