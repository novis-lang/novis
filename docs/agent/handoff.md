# Handoff

## State

**M4 — item 12 is closed: a `finally` runs when its own `catch` clause's body
throws**, which was the last exit out of a protected region that did not run
one. `mwl-ir`'s known gap 2 keeps only its throw-path-temporaries half, and
`mwl-codegen`'s gap 0 is deleted. Item 18's two owed `.mwlt` cases are on disk
with it, so `python tools/loop.py --list` is down to **10** unwritten
conformance cases and **1** differential.

- **The clause body is a protected region of its own.** `lower_catch_clauses`
  pushes a frame whose handler is a fresh finally-and-re-raise block
  (`crates/mwl-ir/src/lower/exception.rs:304`, `lower_finally_and_reraise`),
  built after the body is lowered from that frame's own landing edges: phis,
  `TakeThrown`, the clause binding released, the `finally` body, then
  `Terminator::Throw` of the taken reference. With no `finally` the frame still
  names no handler and the throw goes straight out, unchanged from before.
- **Two behaviours fall out rather than being written down.** A `finally` that
  throws replaces the exception in flight (its re-raise is never reached), and
  a `finally` cannot read the clause binding — the completing path has always
  released `$e` before lowering its copy, and the throwing path now matches it.
- Valgrind-clean over a fixture doing all four shapes two hundred times.
  `TryFrame::handler`'s doc (`crates/mwl-ir/src/lower/mod.rs:236`) is the one
  home for which block a frame's handler is and why.

## Next group

**Three of stage 8's unwritten conformance cases, none of which needs Rust —
unless one of them turns out to pin a hole, which is exactly what happened to
this session's third slice.** The file set: `tests/conformance/lang/`. Check
each shape against `php` in a scratch file before freezing `--EXPECT--`.

- [ ] **`tests/conformance/lang/a-mixed-value-answers-arithmetic-truth-and-a-subscript.mwlt`**
      — the `mixed` operand rows the plan's `Open now` says all lower now.
      `crates/mwl-ir/src/lower/convert.rs:60` (`convert`) and
      `crates/mwl-ir/src/lower/expr.rs:3142` (`lower_index`) are what a
      subscript through a tagged value reaches; ADR 0007 § 2's table is the
      arithmetic half, ADR 0035 the truth half.
- [ ] **`tests/conformance/lang/every-remaining-conversion-row-runs-or-throws.mwlt`**
      — ADR 0007 § 2's grid, both directions, plus the `E0708` refusals the
      plan lists for the pairs that name no row. Same `convert.rs:60`.
      `a-lossy-conversion-throws.mwlt` already owns the throwing scalar rows,
      so what this adds is the rows those do not reach.
- [ ] **`tests/conformance/lang/inline-html-at-file-scope-is-echoed-in-place.mwlt`**
      — `crates/mwl-ir/src/lower/expr.rs:467` (`lower_inline_html`), which
      lowers over the raw span. A multi-file case is fine but only the entry
      file's statements run (playbook).

## Backlog

- An abandoned generator's `finally` — goal item 13, pre-authorized in
  `docs/agent/loop-goal.md` § *Standing decisions*; two named cases owed, and
  `crates/mwl-ir/src/lower/generator.rs:99` is the anchor.
- First-class callable syntax (`Class::method(...)`) still panics — `mwl-ir` gap
  1, at `lower/call.rs:85` and `:652`.
- ADR 0007 § 4's promotion table has 9 refusal sites left (`python
  tools/holes.py --item 1`), the largest remaining group.
- `crates/mwl-codegen/src/ty.rs:116` and `:121` are the two refusal sites no
  item anchors (`holes.py`'s UNATTRIBUTED section).
- Item 25: `object` as a declared type has 2 refusal sites left.
- `tests/conformance/lang/a-required-file-runs-its-own-top-level-statements.mwlt`
  is owed but blocked on `mwl-ir` gap 22 (a `require`d file's own statements do
  not run).
