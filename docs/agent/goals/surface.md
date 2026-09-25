---
milestone: M1
---
# Loop goal 15 — the last two front-end items, before anything colours them

Land the two M1 items that were scheduled after M4 and never taken:
`rule:expressions/pipeline-substitution`'s **pipeline
operator** and `rule:php-migration/a-deprecation-is-a-refusal`'s **PHP
8.6 refusals**. Both are written and accepted; this goal implements them and reopens neither.
[docs/plan/m1.md](../../plan/m1.md) items 5 and 6 are the scope.

**Why here, between the tree and the server.** Goal `editor`'s TextMate grammar must colour `|>` and `let`/`is`
as things Novis **rejects** (`rule:ide/highlighting-is-two-layers`),
and `|>` stops being one the moment `rule:expressions/pipeline-substitution` lands. A grammar written against a surface that changes two
goals later is written twice, and its snapshots are re-frozen by a session that has no idea why. Landing
both now means the grammar, the semantic legend and every `.lspt` case see the final surface once.

Both items are confined to the front end. `|>` substitutes in the parser, so what reaches `nvs-hir` and
everything after it is the `ExprKind` the nested spelling already produces; the refusals name code no
fixture writes. **Neither blocks anything else**, which is exactly why they are still open — and why this
is the smallest goal on the chain after goal `program-id`.

## Stage 0 — the catch-up: three diagnostic numbers are already taken

`rule:expressions/pipeline-substitution`'s table assigns `E0124`, `E0125` and `E0126`. **All three were allocated to other diagnostics
after it was written** — `E_FOR_INIT_MIXES_DECL_AND_EXPR` and `E_FOR_INIT_TWO_DECLARATIONS`
(`rule:iteration/for-init-clause`) and
`E_CATCH_ARM_NOT_AN_EXPRESSION` (`rule:expressions/catch-expression`),
in `crates/nvs-diagnostics/src/lib.rs`. The registry is the allocator and it wins.

So the first slice **reassigns the pipeline's three codes to the next free numbers in the parser band —
`E0129`, `E0130`, `E0131` — and rewrites `rule:expressions/pipeline-hole-once`'s table to match**, in the rule's own fragment, because a
later decision rewrites the rule it changes rather than being left as an overlay. This is stage 0 and not a
detail of stage 2: every conformance case written against the old numbers is a case written twice.

## Stage 1 — the floor

Goal `resilient-tree`'s whole acceptance list — the parity program, goals `temp-sweep` through `doc-comments`, and the resilient tree. Never traded.

## Stage 2 — the pipeline operator

The `|>` token, the `$_` hole, § 2's precedence level and associativity, and the substitution itself: the
right side is an ordinary expression containing `$_` **exactly once**, and the parser replaces that hole
with the left side. The result is the AST the nested spelling already produces, so no later pass changes —
that identity is the design and the acceptance list asserts it directly rather than by inspection.

Three diagnostics, at their reassigned numbers: no hole on the right side (with the second half of its
message, the one that names PHP 8.5's callable shape and is what makes sharing the spelling affordable),
`$_` more than once, and `$_` outside a right side. `tests/conformance/reject/` carries all three,
including the PHP-callable form, and a positive case carries the identity claim.

## Stage 3 — the PHP 8.6 refusals

`let` and `is` join the reserved set in `crates/nvs-syntax/src/token.rs`, and three front-end checks land
with codes from the **`E02xx` rejected-PHP-constructs band** — the next free numbers are `E0247` onward,
and `E0213`/`E0214` are retired holes that are never reused:

1. A `return` — or an escaping `break`/`continue` — inside a `finally` block (`rule:php-migration/no-return-leaves-a-finally`).
2. `return $value;` in a constructor (§ 2).
3. A default on a `readonly` property (§ 4).

Each gets its `--EXPECTF-ERROR--` case under `tests/conformance/reject/`. Partial function application is
**not** adopted and nothing is built for it: `fn` already spells it, and the ADR says so.

## Stage 4 — the reference follows

`|>` is a new operator, so it gets its own heading in `docs/reference/lang/30-expressions.md` and owes
what a language feature owes (`rule:testing/feature-proofs`,
`POLICY["lang"]`): two tests, three examples under `docs/examples/`, one program under `tests/hostile/`,
and one measured figure in `benches/members/`. **That last one is not a formality here** — the figure
worth recording is the pipeline spelling against the nested spelling, which is the claim of the whole
design: identical AST, therefore identical cost.

The three refusals need no new heading. They are rows in
`docs/reference/tools/30-php-differences.md`'s existing sections and in the lang chapters that already
own `finally`, constructors and `readonly`, and `docs/reference/lang/30-expressions.md`'s
*Operators PHP has that do not parse* gains PHP 8.5's `|>` shape. `python tools/reference.py --check`
is in the acceptance list.

## Standing decisions — pre-authorized, do not stop the loop for these

- **The two ADRs are implemented, not reopened.** `rule:expressions/pipeline-substitution`'s `?|>` stays deferred with its trigger named,
  and `rule:php-migration/a-deprecation-is-a-refusal`'s non-adoptions stay non-adopted. A session that thinks either is wrong records the
  thought in the handoff's `## Backlog` and implements what is written.
- **The renumbering is mechanical and it is the ADR's body that moves.** No new ADR number, no overlay
  note, no "see also" — `docs/agent/conventions.md` § *An ADR* is the shape and the registry's own rule
  about a retired code leaving a hole is why the old numbers are not shuffled.
- **No ADR slots.** Both designs are written. Anything smaller is decided-and-recorded in the crate's
  module doc, never `BLOCKED`.
- **`|>` is parse-time substitution and never a run-time application.** If lowering or the type checker
  appears to need to know about the operator at all, that is a bug in the substitution — the whole point
  is that nothing downstream can tell.
