# Handoff

## State

**ADR 0126 is on disk and its type atom is landed as far as the parser and the interner.**
`property<T>` — ADR 0126's property key, whose values are `T`'s public declared property names —
parses in every type position (`crates/nvs-syntax/src/parser/ty.rs`, recognised by spelling in
front of a `<`, so `property` is a keyword there and nowhere else), interns as
`nvs_types::ty::Ty::PropertyKey`, and its argument is refused as **`E0799`** unless it names a
class. Nothing can *produce* a value yet: `as` has no row and `$obj->$key` is still `E0235`.

**`E0799` is the last code in the `E07xx` band, which is now full.** The next types diagnostic
opens a new band — `docs/adr/README.md` § *Decisions taken at project level* already owns that rule
generally, and `brief.py` derives the FULL line itself.

**The three decisions ADR 0014 § *Revisiting* left open are decided in 0126, not deferred again**:
the set is the whole public roster `Core\Reflect\ClassInfo::properties` walks (hooked properties
included, since the access is ADR 0036 § 4's erased one and nothing is copied), the argument is
**contravariant** (§ 3), and `readonly` is refused at the *write* as `E0782` rather than excluded
from the set.

The acceptance check `nvs-types (the property key)` still names three tests that do not exist; the
group below is what closes it. `Core\Cli::displayWidth` was outranked again and is now in the
backlog with its anchors.

## Next group

**The property key's checker half, over `crates/nvs-types/src/expr/operators.rs`,
`crates/nvs-types/src/expr/members.rs` and `crates/nvs-syntax/src/parser/expr.rs`.**

- [ ] **`as property<T>`** — ADR 0126 § 2's three rows: `string` → key (the run-time check),
      `property<U>` → `property<T>`, and the total key → `string`. The conversion table is
      `crates/nvs-types/src/expr/operators.rs:1803` (`conversion_kind`) and
      `crates/nvs-types/src/expr/operators.rs:1844` (`conversion_row_exists`);
      `crates/nvs-types/src/expr/operators.rs:141` is the shape for deciding a written-out operand
      where it is written, which for a key is `E_UNKNOWN_MEMBER` against the roster. The roster walk
      reads `crates/nvs-types/src/signatures.rs:403` and
      `crates/nvs-types/src/signatures.rs:411` (visibility) and belongs in one helper beside
      `class_ref_argument`, `crates/nvs-types/src/expr/members.rs:501`. Test:
      `a_property_key_ranges_over_the_public_properties_and_nothing_else`.
- [ ] **The empty-set half of `E0799`** — ADR 0126 § 1 also refuses a class declaring no public
      property, deliberately left out of `crates/nvs-types/src/lower.rs:196` so the roster is walked
      once; add it there once the helper above exists.
- [ ] **`$obj->$key`, and `E0235` moving to the checker** — ADR 0126 § 4. Delete the parser refusal
      at `crates/nvs-syntax/src/parser/expr.rs:976` (keep the node), report it from
      `crates/nvs-types/src/expr/members.rs:693` (`check_property_access`) for every operand that is
      not a key, and type the read as the union of the set. `E0235`'s own doc at
      `crates/nvs-diagnostics/src/lib.rs:356` says "exactly two ways" and needs the third.
      Tests: `a_read_through_a_property_key_types_as_the_union_of_the_set` and
      `a_computed_member_name_without_a_property_key_is_still_e0235`.
- [ ] **The write half** — ADR 0126 § 5: the erased store, and `E0782` at the write where the set
      holds a `readonly`, beside `reject_readonly_write` at
      `crates/nvs-types/src/expr/assign.rs:630`.

## Backlog

- `Core\Cli::displayWidth` — ADR 0086 § 3, the five edits at `crates/nvs-stdlib/src/cli.rs:213`,
  `:362`, `:1020`, `:1286`; owed since three sessions.
- The property key's three conformance cases (`loop-goal.toml`, stage 8) — they need the runtime
  half, which is `nvs-ir` lowering the access to the erased read/write.
- `Core\IO`'s `truncate` and `lock` — `docs/spec/01-core-library.md` § 14.
- `[log] target` reading — stage 7, `crates/nvs-stdlib/src/log.rs`.
- `orient.py` printed no section of ADR 0125 or 0014, both of which this work was written against:
  `[context] adrs` wants `0125:1`, `0125:2`, `0014:5` and `0014:Revisiting`, and still wants
  `0046:4`, `0046:5`, `0019:1`, `0019:2`, `0014:3`, `0086:1`, `0086:3`, `0033:4` and `0054:3`.
- `[context] spec` still misses `docs/spec/01-core-library.md` § 15.
