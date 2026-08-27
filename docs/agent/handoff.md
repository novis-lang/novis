# Handoff

## State

**M4 — language completeness.** `instanceof` is closed on both sides. A subject whose tag nothing
proved — a `mixed`, a `?Box` no test narrowed — travels as a whole `Value` by address and
`mwl_runtime::mwl_value_instanceof` reads its tag; every subject whose *declared* type can hold no
object is **E0497** (ADR 0007 § 7 **row 14**, new); every right-hand side naming no declared class
is **E0496**, except a name resolving to nothing, which keeps the ordinary **E0303**.
`python tools/holes.py` is at **25 sites, 7 items**.

The one fact worth carrying: `holes.py` read **26** sites at this session's head, not the 24 the
previous handoff quoted — the tool's inventory is derived live, so treat a handoff's count as
stale and re-run it rather than subtracting from it.

`verify.py` 6 of 6 green — conformance **598**, differential **167**, 1631 unit tests.
`tools/leak-check.sh` green over three fixtures: a temporary subject, a tagged subject holding each
non-object tag, and a loop declaring a refcounted `mixed` per round.

Facts recorded where they belong rather than here: ADR 0007 § 7 row 14 owns the subject-side
divergence; `E0496`/`E0497`'s reasoning is each one's own `Code::new` doc comment;
`mwl_types::expr::members`' module doc says how the two sides split; `ExprInfo::InstanceOf`'s doc
comment says nothing unrecorded reaches `mwl-ir` any more; `mwl_runtime::mwl_value_instanceof` owns
the by-address subject.

## Next group

**The three remaining panics in `crates/mwl-ir/src/lower/expr.rs`** — the file this session already
had open, with `crates/mwl-types/src/expr/mod.rs` (the `infer` dispatch each one is the mirror of)
and `crates/mwl-diagnostics/src/lib.rs` (next free code is **E0498**).

- [ ] **An `as` whose target `closed_literal_set` cannot build** —
      `crates/mwl-ir/src/lower/expr.rs:4167`, the `other => panic!` inside the atom map, against
      ADR 0047 § 3's three atoms (`StringLiteral`, `IntLiteral`, `EnumCase`). Measure which target
      spellings actually reach it — `$x as true`, `$x as 1|2.5`, a `?T` over literals — with one
      scratch `.agent-tmp/*.mwl` before deciding diagnostic vs lowering. The neighbouring `:4159`
      panic is an internal-consistency check between two tables and is owed no case.
- [ ] **The `lower_expr` dispatch catch-all** — `crates/mwl-ir/src/lower/expr.rs:258`. Its message
      lists what *is* covered, so the work is a subtraction against `mwl_syntax::ast::ExprKind`'s
      roster; each survivor is then either a lowering or an `E04xx` at the checker. Do this second:
      it names the group's remaining scope.
- [ ] **A `Class::CONST` on a user-declared class** — `crates/mwl-ir/src/lower/expr.rs:246`, whose
      fix is on the `mwl_types` side (nothing collects a user class constant into a signature
      table, `mwl_types`' own known gaps). Bigger than the two above; take it only with the file set
      already loaded and the context to spare.

## Backlog

- `emit_instanceof`'s `CodegenError::Unsupported` for a class with no descriptor is unreachable from
  source now that every right-hand side is checked — `crates/mwl-codegen/src/emit.rs:2097`.
- A `Core` class has no `ClassDesc`, so `$x instanceof Core\Cli\Text` is `E0496` rather than an
  answer; revisit when `Core` classes become real instances at M7/M8 — `E0496`'s own doc comment.
- Item 25's two sites stay misattributed catch-alls (`lower_decl_type`/`lower_checked_ty`): `decimal`,
  `never`, `iterable`, `self`/`static`/`parent`, a shape and an intersection as a *declared* type —
  `docs/agent/loop-goal.md`.
- ADRs 0091, 0093, 0097 and 0100 § 3 are decided and unbuilt, at M6/M7/M8/M10 — their own ADRs.
- `docs/spec/02-php-migration.md` is 31% classified — `python tools/check-migration.py`.
- 17 of the 32 named `.mwlt` cases each stage owes are still to write — `python tools/loop.py --list`.
