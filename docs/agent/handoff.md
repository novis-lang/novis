# Handoff

## State

**Goal 19 — a class a string names, at one contract. Stage 2's `nvs-hir` half has landed; the
`nvs-types` half is next, and that is where the contract's semantics are.**

`Parses` is on `nvs_hir::interfaces::RESERVED` taking no type parameters, with `PARSES` beside
`COMPARABLE` and `STRINGABLE`. Nothing seeds its members yet, so `implements Parses` compiles and owes
nothing — `crates/nvs-types/src/iter_lib.rs` is what closes that, and the next group is it.

A second refusal landed with it: a program may no longer declare its own copy of either compiler-owned
roster's names. `interface Stringable {}` and `class Throwable {}` were both accepted before this, and
every resolver in `nvs-hir` short-circuits on those names before the symbol table, so such a
declaration was dead text rather than a shadow. **No rule states this yet** — the goal's one record
should name `rule:core-api/reserved-namespace` in its `changes.modifies`, which is where `Core` already
gets the same treatment for the same reason.

The narrowing in the goal's *Standing decisions* is unchanged and is not to be re-derived: this goal
gives `as` no class-building meaning.

## Next group

**Stage 2: the contract, the `nvs-types` half** — one file set: `crates/nvs-types/src/iter_lib.rs`,
`crates/nvs-types/src/signatures.rs`, `crates/nvs-types/src/conformance.rs`.

- [ ] **`Parses::parse` is seeded** — `crates/nvs-types/src/iter_lib.rs:69` is `seed`, and its
      `bodiless` helper at `crates/nvs-types/src/iter_lib.rs:164` hardcodes `is_static: false`,
      `returns_static: false`, `has_body: false` and an empty `param_quals`. `parse(tainted string $s):
      static` needs the first two flipped and `interner.tainted_string()`
      (`crates/nvs-types/src/ty.rs:714`) for the parameter; `rule:expressions/try-parse` is the contract
      it encodes, and the goal's *Standing decisions* hold the `Qual::Contagious` fallback for a plain
      `string` argument that will not assign to it.
- [ ] **`Parses::tryParse` is seeded with a body, and `?static` is the wrinkle** —
      `crates/nvs-types/src/signatures.rs:1560`'s `writes_static_return` answers `false` for `?static`
      deliberately, so this seed interns a nullable over the interface's own class type instead of
      setting `returns_static`. Conformance needs no edit for it: `crates/nvs-types/src/conformance.rs:82`
      discharges a member whose resolved declaration `has_body`, and `collect_obligations` never owes a
      member that has one — so the default is inherited and overridable exactly as stage 2's item 3 asks.
- [ ] **`a_parse_returning_something_other_than_static_is_refused`** — named by the stage-2 check at
      `docs/agent/loop-goal.toml:5828`. The override-compatibility site that would report it is not
      located yet, and locating it is the first half of this slice.

## Backlog

- `crates/nvs-types/src/layout.rs:214`'s seeding comment says every roster member is bodiless, which
  `Parses::tryParse` makes false — a default body may owe a descriptor method-table entry.
- `class Core {}` is still accepted: `crates/nvs-syntax/src/parser/decl.rs:194` guards the `Core\…`
  spelling and nothing guards the bare one. `rule:core-api/reserved-namespace` owns it.
- The goal's one new record is unwritten; `docs/agent/loop-goal.md:130`'s stage 5 is where its prose
  half is scheduled.
- Stages 3, 4 and 5 as `docs/agent/loop-goal.md` lists them.
