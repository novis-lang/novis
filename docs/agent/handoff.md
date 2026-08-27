# Handoff

## State

**M4 — language completeness**, and **Stage 0 (the operator table) is closed.** Every
operator row runs and the four cargo guards items 1 and 2 owed are written.
[loop-goal.md](loop-goal.md) is the target, the 43 items and the standing decisions;
[loop-goal.toml](loop-goal.toml) is every check. `python tools/holes.py` is the worklist
and no session re-derives it — it is down to **11 items and 42 sites**; `--item N` prints
one item's anchors, refusal sites and cases, and `python tools/loop.py --list` prints the
named `.mwlt` cases and which are not written yet.

**Items 1 through 10 are done.** Item 10 landed this session: `break N` and `continue N`
lower at **any** level. The counting rule is PHP's — every enclosing loop *and* every
`switch` is a level, for both keywords — and a `continue` level landing on a `switch`
frame then walks outward to the nearest loop, which is MWL's already-settled bare-
`continue` reading generalized. `docs/adr/README.md` § *Decisions taken at project start*
is that decision's home; `mwl_ir::lower::Lowering::lower_break`/`lower_continue` carry the
mechanism, and one `end_iteration_at` against the **outermost** frame being left is what
releases every intervening loop's bindings.

**A level with no statically-known target is `E0475`, never a panic.** Four shapes:
a non-literal level, a `0`, a level past the enclosing depth (which subsumes the bare
`break;` outside every loop, closing `mwl-ir`'s old defensive panics), and `continue`
with no loop at or outside its frame. `mwl_types::locals::check_exit_level` owns all four,
counting off the new `Env::exit_targets` stack — one entry per enclosing frame, `true` for
a loop — which is saved/emptied/restored across a closure literal's body.

Conformance 562, differential 162, `verify.py` 6 of 6 green. No valgrind leg: a multi-level
jump adds no *new* refcount edge, it re-aims existing ones, and the conformance case's
nested-`string` block exits 0 rather than 127.

## Next group

**Item 11 — a ternary or `match` whose arms lower to two representations widens to one.**
All three refusal sites are in one file, `crates/mwl-ir/src/lower/expr.rs`, and the shape
is the one `lower_binary` already uses next door: widen at the lowering, never in
`mwl-codegen`.

- [ ] **A ternary's two arms widen to one representation** —
      `crates/mwl-ir/src/lower/expr.rs:1581` (*"only lowers a ternary whose branches share
      the same IR-level type"*). ADR 0007 § 4's promotion rows plus § 2's implicit
      `int`→`float` widening; `Lowering::widen_to_float` at `expr.rs:2649` is the helper,
      and `coerce` is what already reconciles a `Ty::Tagged`. The named case owed is
      `tests/conformance/lang/a-ternarys-two-arms-widen-to-one-value.mwlt`.
- [ ] **A `match`'s arms widen the same way** — `crates/mwl-ir/src/lower/expr.rs:1751`
      (*"only lowers arms that share one IR-level type"*). Same helper, same file; take it
      with the ternary rather than alone, since the two share the arm-merge path.
- [ ] **An arm-less `match` is a diagnostic, not a panic** —
      `crates/mwl-ir/src/lower/expr.rs:1643`. It is an expression that can only throw, so
      there is no value to produce; refuse it in `mwl_types` with the next free `E04xx`
      and name the rule, exactly as `check_exit_level` does for a `break` level.

## Backlog

- `$a < $b` over two enum cases still type-checks and fails in `emit_binop` — a refusal to
  write, not a lowering to add (ADR 0090 § 2 keeps the domains apart).
- Item 22's second site, an element write through an ADR 0014 § 1 hooked property, is a
  **decided** compile error (loop-goal.md § *Standing decisions*) and needs a new `E`-code.
- Two unattributed sites in `crates/mwl-codegen/src/ty.rs:116,121` that no item anchors.
- 22 named `.mwlt` cases still to write (`python tools/holes.py --cases`).
- `docs/spec/02-php-migration.md` is 31% classified (`python tools/check-migration.py`).
