# Handoff

## State

**Goal `unowned-closures`, stage 6 — the register.** `python tools/owners.py` reports `unowned: 38`,
`untagged: 0`, `broken-tag: 0`, `unreasoned: 0`, `retired-owner: 0` and `past-milestone: 8`; stage 6
wants the first at 0, and the stage's other check — `python tools/owners.py --deferrals` — is green.
`crates/nvs-lsp/src/index.rs` now records no `# Known gaps` at all: its two occurrence gaps closed
together, and `docs/agent/carried-gaps.md`'s entry for the enum-case one went with them. Nothing is
blocked.

**Reading a class constant and reading an enum case are each an occurrence of what they name.** The
checker records `nvs_types::ExprInfo::ClassConst` (`crates/nvs-types/src/expr_table.rs:898`) at a
`Class::CONST` — the class that **declares** it, from the new
`nvs_types::signatures::resolve_const_owned`, the constant's name, and the value `nvs-ir` already
read out of `CoreConst`, which now means only a fold that named no member. `crate::definition`'s
`Target::Constant` carries both reads, so `symbol_of` spells `C::NAME` the way the declaration side
already does and `site` reaches an enum's case list through `declared_case`.

**One read leaves two occurrences where the entry proves both names.** `Status::Draft` is a use of
the case *and* of the enum, because no enum extends another and the qualifier is therefore the enum
the entry carries (`case_qualifier`, `crates/nvs-lsp/src/index.rs:891`). `Cart::LIMIT` leaves one:
the entry names the declaring class, so it cannot say whether the source wrote `Cart` or the parent
it inherits from. Two answers moved with that — a case with no `///` of its own hovers as its enum
(`enclosing_run`, `crates/nvs-lsp/src/hover.rs:281`), and an enum case now carries a code lens like
every other declaration the index holds.

## Next group

**Stage 6: the cursor side of the reference index, which answers a narrower question than the index
now holds** — one file set: `crates/nvs-lsp/src/definition.rs`, `crates/nvs-lsp/src/index.rs`, and
the `.lspt` cases under `tests/lsp/`.

- [ ] **A cursor on a clause name resolves to the name the clause resolved to** —
      `crates/nvs-lsp/src/index.rs:514` (`declared_at`), `rule:ide/the-index-answers-the-cursor`.
      `declared_at` already reaches the declaration's node through `declared_type` for a member name;
      a clause name is a name that declaration writes too, and pairing it against `ClassLinks` is
      `supertype_names` (`crates/nvs-lsp/src/index.rs:826`) plus the same length test `clause_uses`
      makes. Index-side only, and it is what makes go-to-definition work from inside `extends Base`,
      where the occurrence side has answered since the clause slice landed.
- [ ] **A cursor on the enum written in front of a case asks about the case, not the enum** —
      `crates/nvs-lsp/src/definition.rs:437` (`named_at`), `rule:ide/five-features-are-one-reference-index`.
      The index records a use of `Status` at that very span now, and the cursor cannot reach it:
      `named_at` answers the innermost node the checker recorded an entry for, and the qualifier is
      not one. The fix is a step before that walk — an offset inside the class-side child of a
      `ClassConstAccess` whose entry is an `ExprInfo::EnumCase` answers `Target::Type(enum_)` — and
      the case that pins it today is
      `tests/lsp/highlight/an-enum-case-read-highlights-the-case-and-not-the-enum-beside-it.lspt`,
      whose cursor is on the qualifier and whose answer would move.

## Backlog

- `crates/nvs-lsp/src/hints.rs:64` gap 1 — a parameter hint is drawn only for `ExprInfo::Call`; the
  decision it needs is in `docs/agent/carried-gaps.md`.
- `crates/nvs-types/src/lib.rs:149` and `:156` — no equality-operand compatibility check, and no
  exhaustive control-flow reachability; both that crate's module doc.
- `crates/nvs-types/src/signatures.rs:24` and `:33` — a promoted constructor property and a variadic
  parameter's declared type; same doc.
- `crates/nvs-types/src/intrinsics.rs:94` gap 6 — the host check reaches one call shape only.
