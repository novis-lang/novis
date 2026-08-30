# Handoff

## State

**Stage 0c — the reference findings — is the open stage and runs ahead of everything else in this
goal, stage 9 included.** `docs/agent/loop-goal.md` § *Stage 0c* is items 31–35, each naming the
section that is now the rule and the `file.rs:NN` where the binary breaks it;
[docs/reference/findings.md](../reference/findings.md) § *Triage* is each item's verdict and owner.

**Item 31 is closed. Item 32 has five findings left** — P6, P9, P11, P13 and P14; U18, U19, U20 and
P10 closed this session.

**All four of this session's refusals are in the `E02xx` rejected-PHP band, and that is where the
next one goes too.** `docs/adr/README.md` § *Decisions taken at project start* places them there by
name — including the bare `try`, which PHP refuses as well and which this parser accepted only by
omission — so the band's own doc comment in `crates/nvs-diagnostics/src/lib.rs` now says that a few
of its codes are shapes PHP refuses too. `E0126` was never spent and is still the next free `E01xx`;
`E0245` is the next free `E02xx`. **That README section, not an ADR, is the home of every decision
this session applied** — it also already decides P14 and the shape of item 32's remaining parser work.

**Each refusal recognises the construct and then parses it whole**, which is the recovery `===` and
`!==` already had: `<>` still pushes `BangEquals`, a braced `namespace` still yields its block, an
anonymous class still yields its members for `nvs_syntax::casing`, and a clause-less `try` is still a
`Try`. A file therefore reports the rest of its own problems in the same run.

**Stage 9 stands where it stood** — ADR 0119 accepted, items 21–23 written with their anchors,
nothing implemented — and resumes when stage 0c is green. **Stage 8**: conformance 1037, six of
eight named cases written.

**`orient.py` still prints four dead `[context] modules` patterns** —
`crates/nvs-stdlib/src/capability.rs` (it is `crates/nvs-config/src/capability.rs`),
`crates/nvs-host/src/budget.rs`, `crates/nvs-types/src/calls.rs` and
`crates/nvs-types/src/literals.rs`. This session also needed `crates/nvs-syntax/src/casing.rs` and
`crates/nvs-syntax/src/parser/decl.rs`, neither of which `[context] modules` names, and there is no
`[context]` field at all for `docs/adr/README.md` § *Decisions taken at project start*, which is
where every rule this session applied actually lives.

## Next group

**Item 32's `catch` half, then its constant.** The first two share `catch` and one file each —
`crates/nvs-syntax/src/parser/stmt.rs` for the clause's grammar and `crates/nvs-types/src/locals.rs`
for the binding's scope — plus the `E02xx` band (next free `E0245`) and the
`tests/conformance/reject/` cases `docs/agent/loop-goal.toml`'s item-32 check names. Read
`docs/adr/README.md` § *Decisions taken at project start* first: it already decides all three.

- [ ] **P14** — `catch (LogicError | IOError $e)` parses and checks, then fails in codegen. The
      README section makes it a **parse-time** refusal in the rejected-PHP band, not the checker's
      as the previous handoff said: `$e` carries one static type (ADR 0007 § 1's `catch` row), and
      the rewrite is two clauses or one common ancestor. It falls out of reusing the type grammar's
      union at `crates/nvs-syntax/src/parser/stmt.rs:688` (`parse_catch_clause`, whose doc comment
      says so). Case: `tests/conformance/reject/a-catch-clause-names-one-class.nvst`.
- [ ] **P13** — reading a `catch` binding after its clause panics as an undeclared local rather than
      being refused; the binding is the enclosing function's under ADR 0007 § 1's `catch` row, so the
      refusal is the reuse rule's (`crates/nvs-types/src/locals.rs:1106` is the `Try` arm that
      declares it, `crates/nvs-types/src/locals.rs:671` the rule it lands in). Case:
      `tests/conformance/reject/a-catch-binding-ends-with-its-clause.nvst`.
- [ ] **P6** — an untyped interface constant (`public const LIMIT = 9;`) parses and panics at use;
      ADR 0007 § 1 gives every binding a written type, so the `const` branch of the class body is
      where it is refused (`crates/nvs-syntax/src/parser/decl.rs:669`), and
      `crates/nvs-types/src/consts.rs:192` (`build_const_table`) is what panics today. Case:
      `tests/conformance/reject/a-constant-declares-its-type.nvst` — the one the driver's acceptance
      check names first.

## Backlog

- **P9** (`new $className()` with a `string` variable) and **P11** (an enum case as an array key) are
  item 32's two remaining lowering panics — `docs/reference/findings.md` § *Triage*.
- Stage 9's items 21–23, ADR 0119's expression `catch` — `docs/agent/loop-goal.md`.
- Stage 8: conformance 1037 against 1050, differential 206 against 210, two of eight named cases
  unwritten — `docs/agent/loop-goal.toml`.
- `[context] modules` misses `crates/nvs-syntax/src/casing.rs` and `.../parser/decl.rs`, and four of
  its patterns are dead — `docs/agent/loop-goal.toml`.
- `[context]` has no selector for `docs/adr/README.md` § *Decisions taken at project start*, which
  owns every rule stage 0c's parser half applies — `docs/agent/loop-goal.toml`.
