# Handoff

## State

**Stage 9 is closed — ADR 0119 is on disk end to end.** `Lowering::lower_catch`
(`crates/nvs-ir/src/lower/exception.rs:446`) is § 6: `lower_try`'s region push, handler block,
`TakeThrown` and `instanceof` dispatch, with the guard and every completing arm carried into one phi
through the `join_representations` a `match` joins its arms with. Its doc comment owns the three ways
the expression form differs from the block form.

**A `throw` arm goes through `lower_throw` directly rather than lowering as an expression.**
`ExprKind::Throw` in expression position hands back a placeholder out of a fresh dead block
(`crates/nvs-ir/src/lower/expr.rs:339`), and letting that block into the phi would feed the
placeholder's representation to `join_representations` — a `string` guard beside a `throw` arm would
join at `Ty::Tagged`, which is not what the checker typed. `lower_throw` seals the arm's own block and
opens no successor, so the arm has no edge, which is exactly what § 4's union already does with a
`never` arm. `lower_ternary` has the same shape and does *not* do this, so `$c ? "a" : throw …` still
joins at `Ty::Tagged`; that is a latent inefficiency, not a wrong answer, and it is in `## Backlog`.

**The driver's standing failure was a name, not a claim** — the behaviour was pinned as
`a_following_catch_is_the_next_arm_of_the_same_guard`, which is now the check's own
`an_expression_catch_is_arms_on_one_guarded_expression`, and `a_statement_keyword_in_a_catch_arm_is_e0126`
is new beside it. `tests/conformance/reject/a-catch-arm-holds-an-expression-so-return-is-refused.nvst`
was renamed the same way. The playbook bullet at `docs/agent/playbook.md:907` already owns this trap;
what it does not say is that an `nvs-suite` check's `cases` list names **paths**, so the rename is a
`git mv` rather than an edit.

**Valgrind is clean** over `.agent-tmp/catch-expr.nvs` and `catch-expr2.nvs`, which exercise a
refcounted guard, a bound arm, an unbound arm, a rethrow past every arm and a `throw` arm under an
enclosing `finally`.

**`orient.py`'s `[context]` gaps.** New: the pack prints the goal item but never the `[[check]]` it is
graded by, so a session closing a check re-reads `docs/agent/loop-goal.toml` for its `tests`/`cases`
list — that stage's check block belongs in the manifest. Standing, each proven again or earlier: no
field selects `docs/reference/lang/*.md` (this session wrote two of them) or `docs/reference/core/*.md`;
`docs/adr/divergences.md`; `docs/reference/README.md` § *Examples: the fence grammar*;
`docs/spec/01-core-library.md`'s Part II class table; and in `modules` `crates/nvs-stdlib/src/arr.rs`.
In `adrs`: **0007 §§ 2-4**, **0079 §§ 4 and 24**, **0072 §§ 6-7**, **0012 § 6**, **0013 §§ 2-4**,
**0046 §§ 2, 5**, **0053 §§ 1-3**, **0027 § 1**, **0031 § 3**, **0033 §§ 3-4**, **0047 § 2**, **0011**,
**0086 § 6**, **0090 § 3**, **0057 § 1**, **0096 §§ 1-1a**, **0117 § 1**. `orient.py` still warns that
`crates/nvs-host/src/budget.rs` matches nothing, the forward anchor its own comment describes.

## Next group

**Stage 8's differential floor — four oracle cases, over `tests/differential/`.** It is the one red
check left in the goal (`min_passing = 210`, 206 on disk); `python tools/gaps.py --differential` ranks
the candidates and names the anchor for each. An oracle case never goes in `tests/conformance/`
(conventions.md), and PHP is on `PATH` on both legs, so the expectation is computed rather than frozen.

- [ ] **`Core\Math::fdiv` against `fdiv`.** The one arithmetic twin with no oracle case: `±INF`, `NAN`,
      and division by both signed zeros, which is the whole reason the member exists.
      `crates/nvs-stdlib/src/math.rs:1821`.
- [ ] **Two `Core\Str` or `Core\Arr` edges against their twins.** Both classes are 200+ cases deep in
      conformance and thin in oracle cases, so the value is in PHP's own boundary answers rather than in
      another row: `crates/nvs-stdlib/src/str.rs:1`, `crates/nvs-stdlib/src/arr.rs:1`.
- [ ] **`Core\Task::afterResponse` against `fastcgi_finish_request`.** Take it last and drop it if the
      oracle needs a server the differential runner cannot start — say so in the handoff rather than
      freezing an expectation. `crates/nvs-stdlib/src/task.rs:561`.

## Backlog

- Stage 10 — the class reference, `class<T>`, item 36's ADR first; it runs last (`docs/agent/loop-goal.md`).
- `lower_ternary` joins a `throw` branch at `Ty::Tagged` where `lower_catch` now drops it
  (`crates/nvs-ir/src/lower/expr.rs:1475`); the same treatment would suit `lower_match`'s arms.
- `caught_class_label`'s known gap: a class named inside a `namespace` block is not resolved
  (`crates/nvs-ir/src/lower/exception.rs:597`).
- Item 18's adversarial suite, m6.md's *Verify* list (`docs/plan/m6.md`).
- `docs/agent/doc-cleanup.md`'s pass is user-fired, never automatic.
