# Handoff

## State

**ADR 0126's property key converts, and `as` is its only source as far as the checker.** § 2's three
rows are in the conversion table (`crates/nvs-types/src/expr/operators.rs`): `string` → key (the
run-time check), `property<U>` → `property<T>` (the narrowing), and the total key → `string`.
`property<T>` is its own `ConvKind`, so nothing else produces one and a key converts to nothing but
a `string` and a `bool`. A **written-out** operand is decided where it is written, per § 2's second
paragraph: `"email" as property<User>` is a compile-time yes and `"token"`/`"emial"` is `E0405`
(`E_UNKNOWN_MEMBER`, the diagnostic `$user->emial` already gets), with the help listing the roster.
A `private` name and a name that names nothing are one failure with one message, which is § 2's
sentence that visibility is decided at the conversion, once.

The roster is one helper, `expr::members::public_property_names` — the class's own public
declarations and its ancestors', walked exactly as `signatures::resolve_property_owned` walks for
one name (`implements` chained for that function's reason: the two must agree, or a name refused at
the conversion would still resolve at the access).

**Nothing produces a key at run time yet** — `nvs-ir` has no lowering for the three rows — and
`$obj->$key` is still the parser's `E0235`.

**The empty-set half of `E0799` is blocked on a siting decision, not on effort.** ADR 0126 § 1 also
refuses a class declaring no public property, and the obvious home — beside the class-kind refusal
in `lower_property_key` — cannot work: see the playbook bullet added this session. Decide between
re-lowering at check time and a separate post-table walk over written types before writing it; the
second is the only one that reaches a method parameter's annotation.

## Next group

**The property key's remaining three slices, over `crates/nvs-types/src/expr/members.rs`,
`crates/nvs-syntax/src/parser/expr.rs` and `crates/nvs-types/src/lower.rs`.**

- [ ] **`$obj->$key`, and `E0235` moving to the checker** — ADR 0126 § 4. Delete the parser refusal
      for the `TokenKind::Variable` arm at `crates/nvs-syntax/src/parser/expr.rs:951` (the
      `TokenKind::LBrace` arm at `crates/nvs-syntax/src/parser/expr.rs:959` keeps it: `->{expr}` is
      no key), and report `E0235` from the checker instead, where the operand's *type* is what
      decides it — `crates/nvs-types/src/expr/members.rs:894` is the arm that already documents the
      parser as owning this, and `crates/nvs-types/src/expr/members.rs:501`'s new
      `property_key_argument` answers "is this a key". Grep the reject corpus first: every
      `--EXPECTF-ERROR--` case pinning `E0235` moves phase with it. Tests:
      `a_computed_member_name_without_a_property_key_is_still_e0235`.
- [ ] **The read types as the union of the set** — ADR 0126 § 4's other half: `$obj->$key` at a
      `property<T>` key is the union of the declared types of `T`'s public properties, built off
      `crates/nvs-types/src/expr/members.rs:507`'s roster walk plus
      `crates/nvs-types/src/signatures.rs:403`. The access records an
      `ExprInfo::ShapeProperty`-shaped entry keyed on the *name*, which is ADR 0036 § 4's erased
      access — `crates/nvs-types/src/expr/members.rs:875` is the comment that owns which of the four
      arms this becomes. Test: `a_read_through_a_property_key_types_as_the_union_of_the_set`.
- [ ] **The empty-set half of `E0799`** — ADR 0126 § 1, once the siting above is decided. The
      refusal's text and shape are at `crates/nvs-types/src/lower.rs:225`; the roster is
      `crates/nvs-types/src/expr/members.rs:507`.
- [ ] **The write half** — ADR 0126 § 5: the erased store, and `E0782` at the write where the set's
      member is `readonly`. `crates/nvs-types/src/expr/members.rs:952` is where the write's
      visibility check already stands.

## Backlog

- `Core\Cli::displayWidth` — outranked three sessions running; ADR 0086 § 1, `crates/nvs-stdlib/src/cli.rs`.
- ADR 0126's run-time half: `nvs-ir` lowers no row of § 2 yet, so the three conformance cases in
  `loop-goal.toml`'s *conformance (the property key)* check cannot pass — `docs/plan/m8.md`.
- `Core\IO`'s `truncate` and `lock` — `docs/implementation-plan.md` § *Open now*.
- `[log] target` is read by nothing — `docs/implementation-plan.md` § *Open now*.
- `E07xx` is full; the next types diagnostic opens a new band — `docs/adr/README.md`.
