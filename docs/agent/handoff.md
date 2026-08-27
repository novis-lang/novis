# Handoff

## State

**M4 — language completeness.** Both `mwl-ir` *operator* catch-alls now have no reachable
target, on the same evidence the expression dispatch's did one session ago: the roster is
subtracted in the arm's own comment and the panic message is explicitly not the proof.

- **Unary.** `UnaryOp`'s five variants are four arms and one the parser never constructs.
  `+` is the identity over `int`/`uint`/`float`/`decimal` and lowers to its operand — no
  instruction, so no overflow edge, which is the asymmetry
  `unary-plus-is-the-identity-that-negation-is-not.mwlt` pins at `i64::MIN` and at
  `uint`'s top, where `-` throws. What makes that safe rather than a silent divergence is
  one refusal a phase up: `mwl_types::expr::operators::reject_unary_arith_operand` (the
  widened `reject_arithmetic_on_object`) turns away every operand ADR 0007 § 4 has no row
  for as **`E0705`**, because PHP's `+"5"`/`-true`/`~"ab"` all *convert* first and ADR 0007
  § 2 has no implicit conversion for that to be. An object still parts from them with its
  own `E0401` "MWL has no operator overloading" sentence.
- **`@` is `E0236`, refused at the parser.** ADR 0020 makes every runtime failure a
  `Throwable` propagated by checked return, never a diagnostic printed beside a value, so
  there is no channel to mute; ADR 0063 § 3 already listed `@` among what that decision
  closes. `UnaryOp::Suppress` is now a variant nothing constructs — kept only so the
  operator has a name to be refused under. The parser hands the **operand** back in place
  of the whole `@expr`, so each site reports exactly once (see the new playbook bullet).
- **Binary.** `BinaryOp`'s 22 are the scalar table's eighteen rows plus `.`, `&&`, `||`
  and `??`, every one of which `lower_expr` takes before the general `Binary` arm that is
  `lower_binary`'s only caller — compound assignment included, since `AssignOp::to_binary_op`
  re-enters `lower_expr`. All four were run rather than assumed (`.=`, `??=`, `+=` too);
  no new case, because `logical-connectives-short-circuit.mwlt`,
  `a-coalesce-guards-*.mwlt` and `concatenation-*.mwlt` already pin them.
- **One code commit for two slices, deliberately.** Both edit
  `crates/mwl-ir/src/lower/expr.rs` and a commit stages whole files, so a path-split would
  have been a lie about what each carries.

## Next group

**The three dispatch catch-alls one level down** — the same "enumerate the roster, do not
trust the message" job, all three in the file this session had open. The file set:
`crates/mwl-ir/src/lower/expr.rs`, `tests/conformance/lang/`.

- [ ] **`expr.rs:629` — the `decimal` operator table's catch-all.** Subtract `BinaryOp`
      against ADR 0054 § 3's rows: `Add`/`Sub`/`Mul`/`Div`/`Mod`/`Eq`/`NotEq`/`Lt`/`Gt`/
      `LtEq`/`GtEq`/`Cmp` are there, so the survivors are `Pow` (whose refusal
      `mwl_types::expr::operators::power_result` already reports — confirm it, do not
      quote the panic), `Concat`, `And`, `Or` and `Coalesce`, which `lower_expr` takes
      first exactly as it does for the scalar table. One scratch `.mwl` per survivor.
      Anchors: `crates/mwl-ir/src/lower/expr.rs:629`, `crates/mwl-syntax/src/ast.rs:251`
      (`BinaryOp`).
- [ ] **`expr.rs:762` — `concat_operand`'s representation catch-all.** A `Ty` roster, not
      an AST one: which `mwl_ir::ir::Ty` can reach a `.` operand, and whether
      `mwl_types::expr::require_stringable` plus ADR 0007 § 2's "anything → `string`" row
      already refuse the rest where they are written. Anchors:
      `crates/mwl-ir/src/lower/expr.rs:762`, and `Ty`'s own definition in
      `crates/mwl-ir/src/ir.rs`.
- [ ] **`expr.rs:384` — the `lower_expr` dispatch's message.** The *arm* was proven
      unreachable last session and its comment carries the subtraction; what is stale is
      the message below it, which still enumerates twenty shapes as though it were the
      proof. Rewrite it the way the two operator arms now read. Comment-only, so it rides
      with whichever of the two above lands first.

## Backlog

- `holes.py` item 1 — 10 sites, ADR 0007 § 4's promotion table in `mwl-codegen`.
- `holes.py` item 16 — 3 sites, named and spread arguments; the `mwl_types` checker half
  lands first (`docs/agent/loop-goal.md` § *Standing decisions*).
- `holes.py` item 25 — 2 sites, `object` as a declared type.
- 2 unattributed sites, `crates/mwl-codegen/src/ty.rs:116` and `:121` — no item anchors
  the file.
- 14 named `.mwlt` cases still owed; `python tools/holes.py --cases` is the list.
- `orient.py` printed everything this session needed. The one thing greped for outside it
  was ADR 0063 § 3's rejected-construct table, which is where `@` was already closed —
  worth adding to `[context] adrs` as `0063` § 3 for the sessions that reach the rest of
  that list.
