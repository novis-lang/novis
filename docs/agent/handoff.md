# Handoff

## State

**Goal `unowned-closures`, stage 6 — the register.** `python tools/owners.py` reports `unowned: 40`,
`untagged: 0`, `broken-tag: 0`, `unreasoned: 0`, `retired-owner: 0` and `past-milestone: 8`; stage 6
wants the first of those at 0, and the stage's other check — `python tools/owners.py --deferrals` — is
green. Nothing is blocked.

**A name in an `extends` or `implements` clause is now an occurrence of what it resolved to.**
`nvs_lsp::index::occurrences` (`crates/nvs-lsp/src/index.rs:785`) runs a second walk after the
expression one: per symbol declared in the file, the written names come off the declaration's own node
(`supertype_names`, `crates/nvs-lsp/src/index.rs:834`) and the resolved ones off
`nvs_hir::ClassLinks`, the same graph `supertypes_of` reads the declaration side from. The two are
paired by position and only when the lengths match (`clause_uses`, `crates/nvs-lsp/src/index.rs:862`),
because `nvs_hir::HierarchyResolver::resolve` drops a name that did not resolve and a clause paired off
by one would record a use at the wrong name. The module doc's third known gap is gone with it.

The cursor half of that edge is **not** here: `symbol_at` answers a declaration's own name and an
expression, and a clause name is neither, so go-to-definition and a reference list *from* inside
`implements Greets` still answer nothing.

## Next group

**Stage 6: the reference index's two remaining occurrence gaps, both of which are a change in the
checker's table before they are one here** — one file set: `crates/nvs-types/src/expr/mod.rs`,
`crates/nvs-types/src/expr_table.rs`, `crates/nvs-lsp/src/definition.rs`,
`crates/nvs-lsp/src/index.rs`, and the `.lspt` cases under `tests/lsp/`.

- [ ] **Reading a class constant is an occurrence of it** — `crates/nvs-lsp/src/index.rs:68`,
      `rule:ide/five-features-are-one-reference-index`. `Class::CONST` is checked at
      `crates/nvs-types/src/expr/mod.rs:537` through `crates/nvs-types/src/expr/members.rs:65` and
      records **no** `ExprInfo` at all, so the variant is the slice's first half — beside
      `ExprInfo::CoreConst` (`crates/nvs-types/src/expr_table.rs:898`), carrying the class `QName` and
      the constant's name. Then `target_of` needs the arm
      (`crates/nvs-lsp/src/definition.rs:380`) and `symbol_of` the `C::NAME` spelling the declaration
      side already writes (`member_symbol`, no `$` sigil). **`named` is the trap**:
      `crates/nvs-lsp/src/index.rs:896`'s `ClassConstAccess` arm answers the node's **class** side,
      because the only thing recorded on that production today is an enum case, whose symbol is its
      enum — a constant's name is the second child, so the arm has to know which target it is answering
      for. Check what a new `Target` variant costs `crate::hover`'s card in `crates/nvs-lsp/src/render.rs`
      before starting. Four cases move: `tests/lsp/references/a-class-constant-read-answers-nothing.lspt`,
      `tests/lsp/references/a-class-constant-declaration-answers-itself-alone.lspt`,
      `tests/lsp/highlight/a-class-constant-read-highlights-nothing.lspt`,
      `tests/lsp/highlight/a-class-constant-declaration-highlights-nothing.lspt`.
- [ ] **An enum case occurrence is recorded against the case** — `crates/nvs-lsp/src/index.rs:61`,
      same rule. `ExprInfo::EnumCase` (`crates/nvs-types/src/expr_table.rs:873`) names the enum and
      nothing else, so this is that table's widening and not this walk's:
      `crates/nvs-lsp/src/definition.rs:387` reads the field, and moving it moves what `definition` and
      `hover` answer on `Suit::Hearts` too (`docs/agent/carried-gaps.md:621`). The cases that move are
      `tests/lsp/references/an-enum-case-read-answers-the-enum-it-was-recorded-against.lspt` and
      `tests/lsp/highlight/an-enum-case-read-highlights-the-read-it-was-recorded-against.lspt`.
- [ ] **A cursor on a clause name resolves to the name the clause resolved to** —
      `crates/nvs-lsp/src/index.rs:516`, same rule. `declared_at` already reaches the declaration's node
      through `declared_type` for a member name; a clause name is a name that declaration writes too,
      and pairing it against `ClassLinks` is `supertype_names` plus the same length test
      `clause_uses` makes. Index-side only, and it is what makes go-to-definition work from inside
      `extends Base`.

## Backlog

- `crates/nvs-lsp/src/index.rs` gap 1 is also `docs/agent/carried-gaps.md:621` — closing one closes both.
- `crates/nvs-lsp/src/hints.rs` gap 1 (a parameter hint needs a wider `ResolvedCall`),
  `docs/agent/carried-gaps.md:628`.
- `python tools/owners.py`'s `past-milestone: 8` is unexamined this goal; the stage's checks do not read it.
