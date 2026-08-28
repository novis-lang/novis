# Handoff

## State

**M4's Stage 5 is closed, and Stage 6 — the checker and the front end — is the
frontier.** Item 32 is done end to end: ADR 0046 §§ 1-6 attach, refuse, retrieve and
fold, the synthesized-`constructor` reference resolves, and that ADR now carries the
*Verification* section M4's acceptance names for it. The plan's *Open now* paragraph is
the one home for why each landed piece is shaped the way it is.

- **The constructor exemption is guarded at the call site, not inside
  `check_member_ref`** (`crates/nvs-hir/src/members.rs:710`): only a
  `CallArgs::FirstClassCallable` reference is exempt, so a written `Foo::constructor()`
  keeps its `E0309` and `nvs-ir` is never handed a static call with no target.
- **A static first-class callable still panics `nvs-ir` wherever it is not folded** —
  `nvs-ir` gap 1, unchanged and not widened by the above: it is the same panic for every
  method name, and a retrieval target never reaches lowering.
- M4's acceptance still names *Verification* sections for ADRs 0023, 0028 and 0069;
  0014 and 0046 have theirs.

## Next group

**ADR 0007 § 6's three remaining narrowing spellings — goal item 37.** One file set:
`crates/nvs-types/src/locals.rs` (`narrow`, `locals.rs:331`; the module doc's narrowing
section, `locals.rs:29-63`) and `crates/nvs-types/src/expr/mod.rs`
(`check_condition`, `expr/mod.rs:144`).

- [ ] **`instanceof` narrows its subject on the true edge** — ADR 0007 § 6. `narrow`
      (`crates/nvs-types/src/locals.rs:331`) reads `== null`/`!= null` and nothing else;
      the residue is restricted to a class, which is what makes the narrowed read
      lowerable as `nvs_ir::ir::InstKind::Untag`.
- [ ] **A comparison against a literal-typed value narrows** — ADR 0047 § 5's case
      membership under ADR 0007 § 6, same function, same recording
      (`ExprInfo::NarrowedRead`).
- [ ] **`match (true)` narrows per arm**, which is the spelling a ported program writes
      instead of an `if` ladder. `crates/nvs-types/src/expr/mod.rs:144` is where a
      condition's type is asked for once.

## Backlog

- Item 38 — exhaustive control-flow reachability, and `switch`/`try` contributing to
  definite assignment (`docs/agent/loop-goal.md` § *Stage 6*).
- Item 40 — one equality-operand compatibility pass, beside
  `reject_disjoint_equality` (`crates/nvs-types/src/expr/operators.rs:220`).
- Item 41 — an `inout` parameter and its argument declaring the same type
  (`docs/agent/loop-goal.md` § *Stage 6*).
- Item 42 — the unparsed front-end constructs, `crates/nvs-syntax/src/lib.rs`
  § *Known gaps*.
- A `require` whose path is not a string literal runs nothing, silently, in both forms
  (`nvs_hir::requires`' own known gap).
- ADRs 0023, 0028 and 0069 still owe the *Verification* sections M4 names
  (`docs/plan/m4.md`).
