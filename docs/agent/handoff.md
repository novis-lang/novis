# Handoff

## State

**ADR 0126's `property<T>` reaches `nvs-ir`, and § 2's three conversions run end to end.** A key is a
name, so the type erases to `Ty::Str` (`crates/nvs-ir/src/lower/mod.rs`'s `erase_checked_ty` and
`lower_decl_type`) and a parameter, a return and a local hold one for nothing. `property<T> as
string` is the free `from == to` row; the two rows *into* a key are the membership chain
`lower_literal_membership` already emits, under `as ?property<T>` too — the roster travels from the
checker as `ExprInfo::PropertyKey`, keyed by the annotation's span, because `nvs-ir` has no class
table to re-derive it from. A written-out operand still reaches no chain: § 2 decides it where it is
written.

**What this opens is the one thing still missing: `$obj->$key` does not lower.** It now *panics*
where before no program could reach it — the type did not lower, so nothing could hold a key. The
catch-all is `crates/nvs-ir/src/lower/expr.rs:3670`; it is `nvs-ir`'s known gap 21, which carries
what closing it needs, and `check_property_member`'s doc comment says the proof it used to carry is
owed rather than held. The panel's message deliberately does not name the gap: `nvs-ir`'s
`tests/refusals.rs` ceiling is a one-way ratchet over messages claiming one, so the claim lands in
the slice that removes the site. That slice is the next group's first.

Owed on the item beyond that: § 5's write half, and the empty-set half of `E0799` (siting still
undecided — the playbook bullet a previous session left says why the obvious home cannot work).

## Next group

**The keyed access, both directions, over `crates/nvs-types/src/expr/members.rs`,
`crates/nvs-types/src/expr_table.rs` and `crates/nvs-ir/src/lower/expr.rs`.**

- [ ] **The read lowers** — ADR 0126 § 5's union. `check_keyed_property` returns § 5's union today
      and records nothing (`crates/nvs-types/src/expr/members.rs:633`), so the access reaches
      `lower_property_access`'s catch-all at `crates/nvs-ir/src/lower/expr.rs:3670` and panics. It
      wants the dynamic-name counterpart of `ExprInfo::Property`
      (`crates/nvs-types/src/expr_table.rs:444`) — the bounding class and the union, with the name
      arriving as a value. **ADR 0036 § 4's erased read is not reusable as it stands**:
      `crates/nvs-ir/src/ir.rs:734`'s `SlotGet` carries the name as a `String` and
      `crates/nvs-codegen/src/emit.rs:2437` hands the runtime a constant byte range, so this is
      either a new instruction taking a `ValueId` name or a closed-set chain over the same roster
      the conversion tests. `nvs-ir`'s known gap 21 weighs the two; the choice is § 5's to record.
      Closing this is also what lets the panic say so — the refusal ceiling forbids the claim
      before the site is gone.
- [ ] **The write half** — ADR 0126 § 5's checked erased store, at
      `crates/nvs-runtime/src/object.rs:2445`'s `write_erased_property`, which is the landing point
      and closes for all of its callers at once. The store's own arm is `lower_store`'s
      `PropertyAccess`; `E0782` at the write where `T`'s public set holds a `readonly` property is
      the checker's, beside `crates/nvs-types/src/expr/members.rs:633`.
- [ ] **The case the acceptance check names** —
      `tests/conformance/class/a-property-key-reads-and-writes-the-field-it-names.nvst`, which the
      two slices above make writable. The conversion half is already pinned by
      `tests/conformance/class/a-property-key-is-checked-where-the-name-arrives.nvst:1`, which is
      the shape to follow; the arm it cannot reach yet is
      `crates/nvs-ir/src/lower/expr.rs:3670`.

## Backlog

- The empty-set half of `E0799` — ADR 0126 § 1, `crates/nvs-types/src/lower.rs:210`.
- `Core\Cli::displayWidth` — docs/plan/m8.md, stage 3.
- `Core\IO\File::truncate` and `::lock` — docs/plan/m8.md, stage 2.
- Reading `[log] target` — ADR 0020 § 6, stage 7.
- `lower_class_reference`'s message names the bound, not the name that failed —
  `crates/nvs-ir/src/lower/convert.rs`'s own *Known gaps*. `property<T>`'s throw does name both, so
  the two siblings now disagree.
