# Handoff

## State

**M4 — language completeness.** An increment is a **write**, which is the half
`mwl_types::expr`'s `PreIncDec`/`PostIncDec` arm used to leave out: it now marks its target's
subscript levels and runs `assign::check_write_target`, so `$a?->b++`, `$g->hooked["0"]++` and
`$erased->rows["0"]++` take the same `E0479`/`E0478`/`E0480` the `=` and `⊕=` spellings have taken
since `c5a8761`, instead of two `mwl-ir` panics and one `E0482` blaming the subscript for the
`?->` above it. The numeric refusal is skipped when the write refusal fired, so a refused target
is one diagnostic and not two. Item 29's remaining half — the `mwl-ir` assert at
`lower/stmt.rs`'s `PropertyAccess` target arm — no longer advertises a gap it does not have; it
asserts the checker's answer and names `E0479`.

`mwl-ir`'s statement slice also lowers **a typed declaration with no initializer** (`int $x;`) and
**the empty statement** `;`. The declaration binds nothing — `mwl_types::locals`' definite
assignment is what makes that safe — but it does fix the representation, in the new
`Lowering::declared_tys`, which the reassignment arm reads when `Env` has no entry yet. Without
that the *first* assignment would take its representation from the right-hand side, and
`?string $s;` assigned a string on one branch and `null` on the other would merge two shapes.
Nothing that compiles today can reach the new map, since every program that fills it panicked
before.

Two conformance cases under `tests/conformance/lang/`; `tools/leak-check.sh` clean over both new
paths; `verify.py` green — conformance 626, differential 173. `python tools/holes.py` is at 25
sites.

**Gap in the pack:** the item's own prose was stale (see the playbook bullet), and `orient.py`
printed no map line or window for `crates/mwl-types/src/expr/assign.rs`, which is where the three
write-target refusals actually live. `[context] modules` wants an `mwl-types/src/expr/assign.rs`
pattern on any item about an assignment target.

## Next group

**The three shapes still reaching the statement slice's catch-all.** The file set:
`crates/mwl-ir/src/lower/stmt.rs`, `crates/mwl-syntax/src/parser/stmt.rs`,
`crates/mwl-diagnostics/src/lib.rs`, `tests/conformance/lang/`. All three panic at the one site
`crates/mwl-ir/src/lower/stmt.rs:230`, and a scratch `.mwl` per shape reproduces each in one call.

- [ ] **Inline HTML at file scope lowers** — `docs/agent/loop-goal.md` item 34, which already names
      the lowering: the `Helper::EchoStr` call `echo` emits, at `crates/mwl-ir/src/lower/expr.rs:351`
      (`lower_echo`). `StmtKind::InlineHtml` carries the span of the text after `?>`; the run is a
      string literal like any other.
- [ ] **A class, interface or enum declared inside a function body is refused by name.** No item
      owns it and no decision has been taken: PHP declares such a class when the statement *runs*,
      and a static class table has no reading of that, so the choice is a diagnostic rather than a
      silent hoist. Next free `E02xx` is **E0233**; its siblings are at
      `crates/mwl-diagnostics/src/lib.rs:297` and the refusal that reads most like it is
      `E_LIST_DESTRUCTURING_UNSUPPORTED` at `crates/mwl-syntax/src/parser/stmt.rs:773`. Take the
      decision in `docs/agent/loop-goal.md` § *Standing decisions* in the same session.
- [ ] **ADR 0050's `[$a, $b] = $pair` destructuring lowers.** The largest of the three and the one
      that is a feature rather than a decision — `StmtKind::Destructure` carries a
      `DestructureTarget` of typed leaves, each of which is a `bind_local` against the element the
      key names. `E0230` already refuses the `list(...)` spelling, so `[...]` is the only one.

## Backlog

- `Core\Reflect::typeOf` does not exist yet (`E0405`), so no case can assert a binding's *type* by
  observation — `docs/spec/01-core-library.md` owns when it arrives.
- Item 7's four remaining sites in `lower/stmt.rs` are the nested-index and `unset` internal
  asserts, not increment work — `python tools/holes.py --item 7`.
- Items 1, 4, 6, 16 and 25 still hold refusal sites; `python tools/holes.py` ranks them.
- 14 of 32 named `.mwlt` cases still to write — `python tools/holes.py --cases`.
