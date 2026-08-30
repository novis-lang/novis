# Handoff

## State

**ADR 0119's checker is on disk — stage 9 items 21 and 22 are done, item 23 is next.**
`ExprKind::Catch`'s arm in `crates/nvs-types/src/expr/mod.rs:802` is § 4's rule: `make_union` over
the guard and every arm, the same call `ExprKind::Match` makes, with both sides checked against
`None` so the union alone meets the position. A `throw` arm is `never` and joins neither the union
nor the live-set join (§ 3).

**An arm's `$e` needed a mechanism the crate did not have.** `check_expr` threads a `&LocalScope`,
so `declare_binding` — which takes `&mut` — is out of reach, and an arm is the only place in the
language where an *expression* introduces a binding. `LocalScope::arm_bound`
(`crates/nvs-types/src/locals.rs:150`) is a `RefCell` side table on the same footing as `narrowed`,
consulted by `declared_ty` and `visible`; `bind_catch_arm` (`:735`) is `declare_binding`'s
non-strict path against it, and the `ArmBinding` it returns is the boundary that ends the binding,
since an expression has no statement boundary after it to do the job.

**The advisory is `W1006`, not the ADR's `E0778`** — that code is taken, and a warning belongs in
the `W1xxx` band. ADR 0119 § 5, `loop-goal.md` item 22 and the `[[check]]` test name in
`loop-goal.toml` are all folded to `W1006`; the playbook bullet is the trap. Nine fixtures in
`crates/nvs-types/tests/catch_expression.rs`.

**The driver's standing failure is closed and was not a regression.** `check-migration at 36%` was
a floor no session had reached: `docs/spec/02-php-migration.md` gained *Runtime configuration, the
environment and the process*, 34 rows over `ini_*`, `gc_*`, `opcache_*`, `memory_*`, the env pair
and PHP's own introspection. Coverage 34% → 37%.

**`crates/nvs-ir/src/lower/expr.rs:107` is still the `panic!` naming § 6**, so the two `.nvst` cases
for the shape stay `--EXPECTF-ERROR--`; both now declare `uint` where they said `mixed`, which is
what the checker answers for them.

**`orient.py`'s `[context]` gaps.** New this session, in `modules`: **`crates/nvs-ir/src/lower/control.rs`**
for item 23. In `adrs`: **0119 § 5** was printed, but the *warning band* it needs is
`crates/nvs-diagnostics/src/lib.rs`'s tail, which no field selects — the pack's own next-free-number
block stops at `E09xx` and should list `W1xxx`. Standing, each proven earlier: no field selects
`docs/reference/lang/*.md` or `docs/reference/core/*.md`; `docs/adr/divergences.md`;
`docs/reference/README.md` § *Examples: the fence grammar*; `docs/spec/01-core-library.md`'s Part II
class table; and in `modules` `crates/nvs-stdlib/src/arr.rs`. In `adrs`: **0007 §§ 2-4**,
**0079 §§ 4 and 24**, **0072 §§ 6-7**, **0012 § 6**, **0013 §§ 2-4**, **0046 §§ 2, 5**,
**0053 §§ 1-3**, **0027 § 1**, **0031 § 3**, **0033 §§ 3-4**, **0047 § 2**, **0011**, **0086 § 6**,
**0090 § 3**, **0057 § 1**, **0096 §§ 1-1a**, **0117 § 1**. `orient.py` still warns that
`crates/nvs-host/src/budget.rs` matches nothing, the forward anchor its own comment describes.

## Next group

**Stage 9's lowering and the corpus it unblocks — item 23, over `crates/nvs-ir/src/lower/`.**
ADR 0119 § 6 is the whole design, and its *Verification* names the cases.

- [ ] **Item 23 — the lowering.** ADR 0119 § 6. Replace the `panic!` at
      `crates/nvs-ir/src/lower/expr.rs:107` with a value-producing twin of `lower_try`: the region
      push, the handler block, `TakeThrown` and the clause dispatch in
      `crates/nvs-ir/src/lower/exception.rs`, joined into one phi whose incoming edges are the
      guard's value and each arm's. A `throw` arm has no edge. `crates/nvs-ir/src/lower/control.rs:2272`
      already walks the node for reassignment collection and needs nothing.
- [ ] **The corpus and the reference chapter.** Stage 9's third `[[check]]` names three passing
      cases the lowering unblocks and none of them exists yet — the value where the guard threw,
      arms tried in order with the rest rethrown, and a `throw` arm leaving through the enclosing
      `finally`. The shape to copy is the reject case already on disk at
      `tests/conformance/reject/a-catch-arm-holds-an-expression-so-return-is-refused.nvst:9`, with
      `--EXPECT--` in place of `--EXPECTF-ERROR--`; author against `python tools/try.py` on a
      scratch under `.agent-tmp/` first. Then `docs/reference/lang/30-expressions.md`'s `throw expr`
      bullet (line 393, under *`??`, `?:`, `?->` and the ternary* at line 295) gains the arm form
      beside it — the `reference` verify leg runs that chapter's fences.

## Backlog

- Stage 10 — the class reference, `class<T>`, items 36-39; the goal's § *Standing decisions* holds the design.
- Stage 8's remaining differential gap: 206 of 210 (`docs/agent/loop-goal.toml`, stage 8's second check).
- The migration table's next domain: files and streams (`docs/spec/02-php-migration.md` § *Not yet classified*).
- `docs/reference/findings.md` § *Triage* item 35's remainder, absent on time.
- `crates/nvs-host/src/budget.rs` is a forward anchor in `[context] modules` and will stay a warning until item 12 creates it.
