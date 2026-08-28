# Handoff

## State

**M4's Stage 6 is the frontier, and ADR 0007 § 6's narrowing list is now
closed at all four spellings.** `nvs_types::locals`' `narrow` tries three
residue functions — `null_residue`, `instanceof_residue`, `literal_residue` —
and every site that writes a *condition* reaches it, the `match (true)` and
`switch (true)` labels included. That module's own narrowing section is the one
home for what each proves and on which edge; the plan's *Open now* paragraph
carries why each is shaped the way it is.

- **`ExprInfo::EnumCase` now carries the enum's `QName` and the case name**
  beside the backing value, so the guard row can name a case's *type*. `nvs-ir`
  reads the value and ignores both.
- **An `instanceof` against an interface narrows nothing**, deliberately —
  `instanceof_residue`'s doc comment owns why.
- **`nvs-ir` still panics on an instance method call through a `mixed` or a
  union receiver** (`crates/nvs-ir/src/lower/expr.rs:2389`), which is the next
  group's whole subject and why both new `.nvst` cases test their unnarrowed
  edges with a `?Dog` rather than a `mixed`.
- M4's acceptance still names *Verification* sections for ADRs 0023, 0028 and
  0069; 0014 and 0046 have theirs.

## Next group

**The receiver `nvs-ir` panics on.** One file set:
`crates/nvs-types/src/expr/calls.rs` (`infer_method_call`, `calls.rs:48`;
`report_method_on_erased_receiver`, `calls.rs:605`; the erased-receiver report
site, `calls.rs:93`) and `crates/nvs-ir/src/lower/expr.rs:2389`.

- [ ] **A method call through a union receiver is refused where it is
      written** — ADR 0036 § 4. `infer_method_call` resolves nothing for a
      receiver naming no single class, so the call reaches `nvs-ir` with no
      target recorded and panics at `expr.rs:2389` naming a compiler gap for a
      mistake in the program. `report_method_on_erased_receiver` (`calls.rs:605`)
      is the shape and `E0477` the code; the question is whether a union takes
      that code or one of its own.
- [ ] **A method call through a `mixed` receiver defers rather than refuses** —
      ADR 0036 § 4 makes `mixed` the widest of its three triggers, so the answer
      is the *property* half's one storage kind along: dispatch from the
      instance's own class and throw catchably where it has no such member.
      Decide it under the plan's *Open now* precedent for `ValueIndexGet` — a
      declared type that already answered does not get to ask again.
- [ ] **The case both halves owe**, over the refused union spelling and the
      deferred `mixed` one in one file each.

## Backlog

- A `require` whose path is not a string literal runs nothing, silently, in both
  forms — `nvs_hir::requires`' own known gap.
- A `switch` case falling through contributes nothing to what is live *within*
  the case it falls into — `nvs_types::locals`' module doc.
- ADR 0033's container axis: a `secret` array element and a shape-literal field
  carry no bit — `nvs_stdlib::debug`'s known gap 1.
- A user-declared class constant's value is unmodeled —
  `nvs_types::signatures`' own known gap.
- ADRs 0023, 0028 and 0069 still owe a *Verification* section — M4's acceptance.
