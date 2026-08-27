# Handoff

## State

**M4 — language completeness.** Item 20 is closed: `foreach (… as &$v)` lowers as a
write-through — each rebinding of `$v` stores the entry it came from where the write is
written, so `break`, `return` and a throw all leave standing what the body already wrote.
Every row is byte-identical to PHP. `python tools/holes.py` is at **32 sites**; the five it
still lists under item 7 and the eleven under item 1 are other items' catch-alls, not their
own.

`verify.py` 6 of 6 green — conformance **590**, differential **165**, 1629 unit tests.
`tools/leak-check.sh` green over a by-reference loop with refcounted elements, a nested one,
a `&$x` parameter subject, a generator yielding inside one, and a throw with a written entry
live.

Three facts recorded where they belong rather than here: the write-through rule and why it
is not a copy-back are `lower_foreach`'s own doc comment; the two refusals are
`mwl_types::expr::iteration::check_foreach_by_ref` (E0490/E0491), which states them as
`check_by_ref_arg`'s two obligations arrived at from the same direction; and the trap that
cost this session its one wrong answer is the new playbook bullet on `env.insert`.

## Next group

**Item 19 — a closure or a generator written where a `&$x` parameter is in scope.** One
file set: `crates/mwl-ir/src/lower/generator.rs` and `crates/mwl-ir/src/lower/closure.rs`,
with `Lowering::ref_locals`/`pointee_of` (`crates/mwl-ir/src/lower/mod.rs:1902`) the side
table all three read. `python tools/holes.py --item 19` is the item; ADR 0031 § 2 gives the
language no by-reference *capture*, so what closes here is the parameter's **value**.

- [ ] **A generator method with a `&$x` parameter** — `crates/mwl-ir/src/lower/generator.rs:469`
      (`lower_generator`'s parameter loop, anchor `:312`). The panic's own reasoning is that
      the cell is caller-staged and stops existing when the factory returns, which is a
      *rule*, not a missing lowering: take the decision (§ *Standing decisions* pre-authorizes
      it), refuse it with a new `E04xx` naming the frame lifetime, and record the paragraph in
      `docs/adr/README.md` § *Decisions taken at project start*.
- [ ] **A `&$x` binding live across a `yield`** — `crates/mwl-ir/src/lower/generator.rs:154`
      (the spill loop). The refusal above closes this one at the parameter, so this becomes an
      internal assert naming E04xx rather than a hole — check that a generator *body* has no
      other way to bind a `Ty::Ref` before assuming it.
- [ ] **A closure with a `&$x` parameter** — `crates/mwl-ir/src/lower/closure.rs:159`. Nothing
      calls a closure through a signature yet (`mwl-ir` gap 1), so this is the same wall as
      calling one through the variable holding it; if it is still blocked, refuse it beside the
      generator's rule and say so, rather than leaving a panic.

## Backlog
- `mwl-ir` gap 1, `Class::method(...)` — `crates/mwl-ir/src/lower/call.rs:85`; blocks a
  callback named once and handed to several members (playbook bullet).
- Item 25's two sites are `lower_decl_type`/`lower_checked_ty` catch-alls whose uncovered
  shapes (`decimal`, `never`, `iterable`, `self`/`static`/`parent`, a shape, an intersection
  as a *declared* type) no item names — `docs/agent/loop-goal.md` item 25.
- `mwl-codegen/src/ty.rs:116`/`:121` are unattributed by `holes.py` and no item names them.
- 17 named `.mwlt` cases still to write — `python tools/holes.py --cases`.
- ADR 0007 § 5's remaining keyed-literal divergence: a positional element after an explicit
  `int`-looking key numbers from its own position.
- `docs/spec/02-php-migration.md` is 31% classified (`python tools/check-migration.py`).
