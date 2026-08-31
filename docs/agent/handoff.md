# Handoff

## State

**ADR 0126 §§ 4-5' read half is on disk.** `$obj->$key` and `$obj->{$expr}` are admitted where the
operand's type is a `property<T>` whose argument the receiver satisfies, and the access reads as the
**union of `T`'s public roster's declared types** — `crates/nvs-types/src/expr/members.rs`'s
`check_keyed_property`, which is also where every other operand becomes `E0235`. The parser refuses
no computed name at all now (`crates/nvs-syntax/src/parser/expr.rs`'s `parse_member_name`), the help
names `as property<T>`, a call keeps the refusal unconditionally at both sites in
`crates/nvs-types/src/expr/calls.rs`, and `unset($obj->$key)` is `E_UNSET_ON_PROPERTY`.

**`->{expr}` moved with `->$name`, against the last handoff's item and with ADR 0126 § 4's body**,
whose first sentence admits both spellings. Every checker site already matched
`MemberName::Variable | MemberName::Expr` as one pattern, so one rule was less code than two. The
caret for the brace form now sits on the operand rather than on the braces, since the checker holds
the inner expression's span; `tests/conformance/lang/a-computed-member-name-is-a-diagnostic.nvst`
pins that and the four refusals.

**An admitted keyed access records no `ExprInfo`, and no program can reach the arm that would want
one:** `nvs-ir` lowers no `property<T>` parameter, return or local, and says so with the known-gap
panic at `crates/nvs-ir/src/lower/mod.rs:2943`. The proof is on `check_property_member`'s doc
comment, which is the only home for it; § 5's erased store is the slice that records the entry and
retires the proof.

Owed on the item: the empty-set half of `E0799` (siting still undecided — the playbook bullet the
previous session left says why the obvious home cannot work) and § 5's write half.

## Next group

**The property key's last two slices plus the lowering they both need, over
`crates/nvs-types/src/lower.rs`, `crates/nvs-types/src/expr/members.rs` and
`crates/nvs-ir/src/lower/`.**

- [ ] **`property<T>` reaches `nvs-ir` at all** — ADR 0126 § 5. The known-gap panic at
      `crates/nvs-ir/src/lower/mod.rs:2943` refuses the type in a parameter, a return and a local,
      so no key-holding program lowers and neither of the two slices below can be tested end to
      end. A key is a name, so the representation to give it is the one `Ty::String` already has;
      the three § 2 conversions then lower at `crates/nvs-ir/src/lower/expr.rs:343`'s
      `Conversion` arm. Do this one first.
- [ ] **The write half** — ADR 0126 § 5: the erased store, and `E0782` at the write where the set's
      union is not assignable to the value. The read arm is
      `crates/nvs-types/src/expr/members.rs:576`'s `check_keyed_property` and the write goes
      through the same function (`is_unset` is already its third case); the entry it must record
      is what `crates/nvs-ir/src/lower/expr.rs:3638` reads back, beside `ExprInfo::ShapeProperty`.
- [ ] **The empty-set half of `E0799`** — ADR 0126 § 1, a class declaring no public property.
      `crates/nvs-types/src/lower.rs:126` is where `lower_property_key` is reached from and
      `crates/nvs-types/src/expr/members.rs:536`'s `public_property_names` is the roster to ask.
      The siting decision is still open and the playbook says why the obvious home fails: prefer a
      post-table walk over written types, which is the only one that reaches a method parameter's
      annotation.

## Backlog

- `Core\IO::truncate` and `::lock` — docs/spec/01-core-library.md § 13, stage 2's handle half.
- `Core\Cli::displayWidth` — ADR 0086 § 1, stage 3.
- Reading `[log] target` — ADR 0020 § 6, stage 7.
- Stage 6's shared store needs a reachable Docker daemon (`tests/db/compose.yaml`).
