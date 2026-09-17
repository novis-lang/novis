# Handoff

## State

**Goal `class-scoped-types`, stage 3: the two declaration refusals are landed, three of the five
`reject` cases the stage check names are on disk and green.**

- `E0133` refuses a modifier run or an attribute group written in front of a body's `type`, in a
  class, an interface and an enum body alike, and keeps the declaration
  (`crates/nvs-syntax/src/parser/decl.rs:@refuse_decoration_on_a_type_alias`). A modifier run in an
  enum body reaches the member path, so that path no longer adds a second "not a case" refusal when
  everything it parsed was an alias.
- `type` written in a method body, a block or a closure body was **already** refused by name as
  `E0233` before this session — only its `reject` case was missing, and it is written now.
- Green on disk: `a-class-scoped-type-alias-takes-no-modifier.nvst`,
  `a-type-alias-declared-inside-a-body-is-refused-by-name.nvst`,
  `a-class-scoped-type-alias-of-a-bare-class-is-refused.nvst`.
- **The two cases left each need code first**, which is why they are not written:
  - *Not inherited* — the refusal works (`Sub::Id` and `Implementor::Handle` are `E0405`), but it is
    reported **twice for one span**; the same miss in expression position is `E0309` once. Pinning
    the duplicate would freeze it.
  - *No collision* — a name shared with a constant or an enum case is **not refused at all** today:
    `enum Colour: int { Red = 1, type Red = int; }` and
    `class Holder { public const int ID = 1; type ID = int; }` both compile clean.
- The pack did not print `rule:types/alias-is-never-a-bare-class`, which the bare-class case pins —
  `[context] rules` is missing it.

## Next group

**Stage 3: the collision refusal and the duplicated unknown-member report** — one file set:
`crates/nvs-hir/src/aliases.rs`, `crates/nvs-types/src/lower.rs`, `tests/conformance/reject/`.

- [ ] **A name is a constant, a case or an alias, never two** — `crates/nvs-hir/src/aliases.rs:227`
      (`record_members`) already holds the whole `&[ClassMember]` slice, so a constant sharing the
      alias's name is visible there; an enum case is not, and has to come from that function's
      caller at `crates/nvs-hir/src/aliases.rs:169`. A new `E0134`.
      `rule:types/type-alias` and the goal's *Standing decisions* § *Resolution order at
      `Owner::Name`*, which says the collision is refused at the declaration so no compiling
      program depends on the lookup order.
- [ ] **One `E0405` per site** — `crates/nvs-types/src/lower.rs:552` reports the unknown member for
      `Owner::Name` in type position, and one parameter type produces it twice; find whether the
      site runs twice or the report is not deduped, and fix whichever it is.
      `rule:types/type-alias`.
- [ ] **The last two `reject` cases**, written once the two above land, beside
      `tests/conformance/reject/a-class-scoped-type-alias-of-a-bare-class-is-refused.nvst:1` —
      `a-class-scoped-type-alias-is-not-inherited.nvst` and
      `a-class-scoped-type-alias-does-not-share-a-name-with-a-constant-or-a-case.nvst`, the two
      names the stage's check still reports missing. The house style for a `reject` expectation is
      the `error[...]` line plus `%A`, which is what keeps the snippet's widening indentation out
      of the file.

## Backlog

- `parent::Name` in a parameter type position does not parse and cascades; not a refusal this goal
  owns (`rule:types/grammar`).
- Stage 4 and stage 5 of `docs/agent/loop-goal.md` are untouched.
- The alias's `[context] rules` needs `types/alias-is-never-a-bare-class` added.
