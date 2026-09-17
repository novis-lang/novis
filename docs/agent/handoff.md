# Handoff

## State

**Goal `class-scoped-types`, stage 2 is complete — all four of its checks are green on disk.**

- The parser takes `type Name = TypeExpr;` in a class, an interface and an enum body; `nvs-hir` keys a
  body's alias by its owner plus the member name and refuses a cycle or a bare class through it.
- `nvs-types` now resolves both spellings: `Owner::Name` asks `AliasTable::get_member` before the enum
  case and the constant fold (`crates/nvs-types/src/lower.rs:343`), and a bare `Name` inside the
  owner's body asks it before the namespace (`lower.rs:@resolve_name_type`). The stage 0 gap in
  `lower_member_type`'s doc comment is closed, not carried.
- **A tension stage 3 must not break:** `rule:types/alias-is-never-a-bare-class` refuses the *fully
  bare* atom only, so the single-name expansion the left of a `::` is read through can only be written
  wrapped — `type M = (Mode);`, which is what
  `tests/conformance/lang/an-alias-left-of-a-double-colon-names-its-enum.nvst` pins. Tightening the
  refusal through parentheses rewrites that case, and stage 5's rule fragment has to state which it is.
- Two shapes stage 3 still owes, both found in stage 2: a modifier or an attribute group written in
  front of a body's `type` parses and is dropped silently, and an alias in an **anonymous** class body
  is never collected, since there is no owner name to key it by.

## Next group

**Stage 3: the two declaration refusals** — one file set: `crates/nvs-syntax/src/parser/decl.rs`,
`crates/nvs-diagnostics/src/lib.rs`, `crates/nvs-types/src/locals.rs`.

- [ ] **No modifier, no attribute group** — `crates/nvs-syntax/src/parser/decl.rs:685`
      (`parse_class_member`) reports a new `E0133` where a `public`/`private`/`static`/`final` run or
      an attribute group precedes a body's `type`, instead of parsing it and dropping it.
      `rule:types/type-alias` and the goal's *Standing decisions* § *No visibility, ever*.
- [ ] **Not in a body** — `type` inside a method, a closure or a block is refused by name at
      `crates/nvs-types/src/locals.rs:1476`, the way a nested class is `E0233`, rather than falling
      through as a statement error. `rule:types/type-alias`, guarded today by
      `tests/conformance/lang/a-type-declared-inside-a-body-is-a-compile-error.nvst`.
- [ ] **One `reject` case per refusal**, each pinning the diagnostic's own text, beside
      `tests/conformance/reject/an-attribute-name-is-a-shape-typed-type-alias.nvst:1` — the
      `rule:types/type-alias` case already there. The `--EXPECTF-ERROR--` indentation widens with the
      line number (`docs/agent/conventions.md` § *A `.nvst` test case*).

## Backlog

- Stage 3's other half — no collision between an alias and a constant or an enum case under one owner
  (`crates/nvs-hir/src/aliases.rs:227`), and `Child::Meta` not inherited from `Parent`
  (`crates/nvs-types/src/lower.rs:343`). Same stage, different file set.
- An alias in an anonymous class body is silently not collected — `crates/nvs-hir/src/aliases.rs:227`
  keys by the owner's written name, and there is none. Refuse it or key it; stage 3 decides.
- Stage 4: the formatter, `nvs meta --json` and the four LSP positions, per
  `docs/agent/loop-goal.md` § *Stage 4*.
- Stage 5: `rule:types/type-alias`'s fragment gains the member, and the decision record that owns the
  reasoning — including which spellings reach a single name (see *State*).
