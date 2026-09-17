# Handoff

## State

**Goal `class-scoped-types`, stage 2: the member and its key have landed; the checker half has not.**

- The parser takes `type Name = TypeExpr;` in a class, an interface and an enum body as
  `ClassMemberKind::TypeAlias`, from the same production the file-scope form uses.
- `nvs-hir`'s alias table files a body's alias under its owner plus the member name, expands both
  spellings inside another expansion (a bare `Name` first against the owner, `Owner::Name` from
  anywhere), and `E_TYPE_ALIAS_CYCLE` covers a chain through either.
  `rule:types/alias-is-never-a-bare-class` runs on a member unchanged.
- **Nothing in `nvs-types` reads the member yet**, so a class-scoped alias resolves nowhere in a
  program: that is the next group, and it is what makes the goal's own conformance cases possible.
- Two shapes stage 3 must answer, both discovered here: a modifier or an attribute group written in
  front of a body's `type` parses and is dropped silently, and an alias in an **anonymous** class
  body is never collected, since there is no owner name to key it by.

## Next group

**Stage 2: the two spellings, in the checker** — one file set: `crates/nvs-types/src/lower.rs`.

- [ ] **`Owner::Name`** — `crates/nvs-types/src/lower.rs:339` (`lower_member_type`) asks
      `AliasTable::get_member(owner, name)` before the enum case and before the constant fold, and
      re-lowers the expansion it gets. `rule:types/type-alias`, against
      `rule:types/enum-case-type` and `rule:types/constant-in-type-position` for the order.
- [ ] **A bare `Name` inside the owner's body** — `crates/nvs-types/src/lower.rs:654`
      (`resolve_name_type`) tries the current class's own aliases, through `Ctx::current_class`,
      before the namespace's. `rule:types/type-alias`.
- [ ] **The left of `::` resolves through the alias table first** —
      `crates/nvs-types/src/lower.rs:339` again, where the owner is resolved: an alias
      whose expansion is a single name atom stands in for that name before the `::` is read, and any
      other expansion left of `::` is `E_UNKNOWN_MEMBER` with a help naming what it expands to.
- [ ] **Stage 0's two sentences** — rewrite `lower_member_type`'s doc comment at
      `crates/nvs-types/src/lower.rs:339` as a whole once the three above are in.

## Backlog

- Stage 3's refusals, plus the two shapes above — `docs/agent/loop-goal.md` § *Stage 3*.
- Stage 4: `nvs meta`, the four `nvs-lsp` passes and `nvs-fmt` all skip the new member today, through
  a `#[non_exhaustive]` wildcard — `docs/agent/loop-goal.md` § *Stage 4*.
- The `[context] modules` manifest does not name `crates/nvs-hir/src/hierarchy.rs`, whose
  `resolve_ref` the alias collector resolves every name through.
