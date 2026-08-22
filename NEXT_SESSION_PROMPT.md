# Next session prompt

Continue MWL. M1 (front end) is done. M2 (HIR/types/IR) is close to done — run `python .claude/brief.py`
first, then read `docs/implementation-plan.md`'s M2 paragraph for exactly what landed and how (this file
only points at what's next; the plan is the one home for status detail, per CLAUDE.md's "state a fact
once").

**Last session landed `&&`/`||`/`!`/the ternary-elvis operator — ADR 0035's other four truthy positions,
beyond `if`/`while`, the nineteenth `mwl-ir` slice.** None of the four were lowered at all before this
session, not even for a plain `bool` operand:

- `&&`/`||` need genuine short-circuit control flow (PHP only evaluates the right operand when it can
  change the answer), not just a value computation — a real architectural addition, not a pure
  table-reuse: a new `Lowering::lower_expr_top` entry point exists specifically for the handful of
  positions that already own a mutable `cur: &mut BlockId` (a local declaration's initializer, `return`'s
  value, a plain reassignment's right-hand side — including a property/array-index target — and any
  condition under test via the now-`&mut BlockId` `Lowering::lower_truthy_cond`). Only those positions can
  redirect "the current block" mid-expression the way `&&`/`||`/a ternary need to; everywhere else (a call
  argument, an array-literal element, a `.`-operand, a nested arithmetic operand) still panics naming the
  gap, since those callers only ever own a fixed `cur: BlockId`. `Lowering::lower_and`/`Lowering::lower_or`
  reuse `lower_if`'s own branch/merge shape, joining the expression's own `Ty::Bool` value through a fresh
  `Phi` instead of merging named locals.
- `!` always produces `Ty::Bool` via the same truthy table (`Lowering::negate_truthy`, shared by a
  top-level `!` and a nested one), fixing a latent bug: it previously passed its operand's own type
  straight through to the result, silently correct only because the one pre-existing fixture happened to
  negate an already-`bool` local. A top-level `!` recurses through `lower_expr_top` for its own operand so
  `!($a && $b)` composes; a nested `!` (no `&mut BlockId` available) still applies the table but can't
  compose with a nested `&&`/`||`/ternary operand.
- The ternary/elvis operator (`Lowering::lower_ternary`) reuses `lower_if`'s shape once more, joining a
  `then`/`else` value through a `Phi`. Elvis (`then` omitted) reuses `cond`'s own value on the truthy path
  rather than retesting it (PHP evaluates a `?:` condition exactly once), needing one exception to every
  other truthy-tested position's usual release-after-test rule (`Lowering::truthy_convert`, the bare
  conversion factored out of `Lowering::truthy_value`) plus a retain when that reused `cond` is an aliasing
  read gaining a second owner. **The same retain-on-alias question turned out to apply to every ordinary
  `then`/`else` branch too** — caught by a dedicated test
  (`elvis_retains_an_aliased_refcounted_condition`), not assumed correct by inspection: nothing else treats
  a ternary's own result as anything but an ordinary fresh value, so a branch whose own expression is a
  bare variable/property/array-element read needs its own retain right there, or it would be double-released
  at the enclosing function's own exit sweep. A `then`/`else` pair lowering to two different `Ty`
  representations still panics naming the gap (the checker's own union has no IR fold yet — the same open
  question `Ty::Mixed` names). PHP's low-precedence `and`/`or`/`xor` keyword operators are out of scope by
  design — ADR 0035 names only `&&`/`||`/`!`.
- `while`'s own lowering needed one adjustment to host a branching condition at all: the loop header's phis
  still physically live in the fixed `header_block`, but the loop's own `Branch` terminator now seals onto
  `cond_end` — wherever condition lowering actually ends up — not `header_block` itself.
- **A pre-existing, previously-invisible gap surfaced and got fixed as a side effect:** `ExprKind::Paren` (a
  parenthesized `(expr)`) was never unwrapped anywhere in this crate's expression lowering at all, only in
  type position — invisible until now because no earlier slice's fixtures happened to need explicit parens.
  `!($a && $b)` does (`!` binds tighter than `&&`/`||` in the grammar), so both `Lowering::lower_expr` and
  `Lowering::lower_expr_top` now have their own transparent `Paren` arm.

Ten new tests cover: `&&`/`||` short-circuiting to a `Phi`, `!` fixing the type bug, `!` composing with a
short-circuit `&&`, an `if`/`while` condition itself short-circuiting, a plain matching-type ternary, elvis
with a non-refcounted condition (no retain needed), elvis retaining an aliased refcounted condition, elvis
transferring a fresh refcounted condition (no retain *or* release needed), a mismatched-branch-type ternary
still panicking, and a `&&` nested in a call argument still panicking (documents the scope boundary). `mwl-ir`
is now at 94 tests (was 82); `mwl-types` unchanged at 217. Built, tested, clippy- and fmt-clean, committed.
Two stale claims in the plan's M2 paragraph (both said `&&`/`||`/`!`/ternary were entirely unlowered) were
fixed in place rather than left to contradict the new paragraph, per CLAUDE.md's "state a fact once."

**With that, `mwl-ir`'s known-gap list (its own module docs in `lib.rs`) stands at:**

1. ~~Control flow (`if`/`while`).~~ **Done.**
2. ~~Safepoints.~~ **Done** (reserved shape only). Revisit once M3's codegen exists.
3. ~~`new`/a static call, an instance method call, a compile-time-known property access, and array-element
   access through a known `int`/`uint`/`string` key.~~ **Done.** What's left of this shape:
   - **`$a[]`/`$a[] = expr;` (PHP's append syntax).** Needs a "next available integer key" counter this
     crate has no representation for yet. Fully self-contained — doesn't touch `mixed` at all. **Recommended
     pick for next session — see below.**
   - **Array-element access through a `mixed`-erased base.** `Ty::Mixed` gives this a representation to
     fall back *to*, but wiring the fallback in still needs the runtime type-tag design question (item 5
     below) settled first — not independently actionable yet.
4. **Non-scalar *data* values and refcount operations** — two pieces remain, both independent of `mixed`:
   - **A `...spread` or `&value` array-literal element.** Still unsupported, still panics naming whichever
     is used. Each needs its own design: spread needs array-merge semantics, `&value` needs a
     reference-value representation this crate has none of anywhere yet.
   - **`Ty::Object` refcounting.** Still zero retain/release operations for an object reference — no
     allocation/field-layout story exists yet. Leave this for whenever an actual object layout/allocation
     design lands (expected around M3's codegen, not before).
   - **Qualified string/bytes types (`tainted`, `secret`, and their combination).** Still panics; likely
     wants to wait for ADR 0024 §4/0033's stdlib-dependent sinks anyway (M7/M8).
5. **`Ty::Mixed` exists, but nothing dispatches on a `mixed` value's *actual* runtime type yet.** What
   remains, all blocked on the same open design question:
   - **A runtime type-tag representation for `mixed`.** How a `mixed` value's actual runtime type (int?
     string? array? object?) is discoverable at runtime. Once picked, it unblocks arithmetic's `mixed`
     fallback, ADR 0035's `null`/`mixed` truthy case, and item 3's mixed-erased-array-base gap above.
     **You are authorized to design this yourself and proceed if you pick this up** — no need to stop and
     ask (standing user direction). Consider scoping the *first* slice to just one consumer rather than
     wiring all three at once.
   - The `.`-concatenation `Stringable`-object-operand gap is *not* primarily a `HelperCall` gap and is
     unrelated to `mixed`: it needs `.` to synthesize a resolved `toString()` call — see the "runtime-
     helper calls" session's design-choices writeup in `mwl-ir`'s module docs before picking this up.
6. **Virtual dispatch** — skip this one (per standing user direction, deferred until M3 starts) unless it
   turns out to be the only item left, in which case stop and report that instead of attempting it.
7. ~~`var` locals, multi-base integer-literal cooking, integer-literal magnitude range-checking.~~ **Done.**
8. ~~String-literal cooking completeness.~~ **Done.**
9. ~~`&&`/`||`/`!`/the ternary-elvis operator (ADR 0035's other four truthy positions).~~ **Done** (this
   session), at any position that already owns a mutable `cur` — see the module docs' nineteenth-slice
   paragraph for the narrower "nested inside a fixed-`cur` position" and "mismatched branch types" residual
   gaps.

**Recommended pick for next session:** `$a[]`/`$a[] = expr;` (PHP's append syntax, item 3's remaining
sub-bullet) — fully self-contained, doesn't touch `mixed` at all, and closes out array-element access to
"every shape except a `mixed`-erased base." Needs a "next available integer key" counter: this crate has no
representation for "the highest integer key used so far" (or, per the plan's own documented simplification,
"how many positional elements exist so far" — see the array-literal paragraph in the plan for why that
simpler rule was chosen over PHP's real one). Both `lower_array_key`'s read side and
`Lowering::lower_reassignment`'s `ExprKind::Index { index: None, .. }` write-side arm currently panic naming
this gap by name — both are the exact two places to wire up.

Other self-contained options, roughly in order of size:
- The `mixed` runtime type-tag representation (item 5 above) — the single biggest unblock left on this
  list, but real, non-mechanical design work rather than a narrow mechanical slice. Pick this up instead of
  append syntax if you'd rather tackle the bigger design question head-on; you're pre-authorized to design
  and proceed.
- A `...spread`/`&value` array-literal element (item 4 above) — each needs its own design (array-merge
  semantics; a reference-value representation), bigger and less mechanical than append syntax.

Once control flow, calls, and property/array access all lower, M2's own *Verify* bullet ("IR snapshot
tests; no program in the corpus produces an `Unknown` type") is worth revisiting for a real corpus-driven
snapshot suite, not just hand-written fixtures — at that point M2 as a whole should be closeable and M3
(baseline Cranelift backend, `Hello World`) can start.

Also still open from before (independent, low priority, unrelated to `mwl-ir`): a `set`-hooked property is
exempted from ADR 0022's constructor check entirely rather than verified against the hook's body; the
identical question now also applies to whether a `lateinit` + hooked property should discharge on the
hook's first commit (ADR 0038's own *Revisiting* names this, deferred to `docs/spec/`).

**Queued, independent of the `mwl-ir` work above: ADR 0043's code follow-up.** A separate concurrent session
landed [ADR 0043](docs/adr/0043-interface-default-methods-and-delegation-replace-traits.md) (docs-only,
`81448ce`): `trait`, class-body `use Trait, ...;`, and `insteadof` are removed from the language entirely,
replaced by an `interface` method with a `public`/`private` body and one `implements` entry carrying a
`by $field;` delegation suffix. The ADR's own *Consequences* and *Verification* sections are the one home
for the exact task list — don't re-derive it here — but the shape is: remove `mwl-syntax`'s
`TraitDecl`/`UseTraitMember`/adaptation AST and grammar (replaced by a parse-time `E_TRAIT_NOT_SUPPORTED`
diagnostic) and add default/private interface-method-body grammar plus `by $field` grammar; remove
`mwl-hir`'s entire trait-use/`insteadof` resolution machinery (`hierarchy.rs`) and add default/private
method resolution, `by`-delegation resolution, and the new `E_INTERFACE_MEMBER_CONFLICT`/
`E_INTERFACE_PRIVATE_METHOD_NOT_VISIBLE`/`E_DELEGATE_TYPE_MISMATCH` diagnostics. It's a large, separable
chunk — check `mwl-syntax`/`mwl-hir` first to confirm it hasn't already landed, then either fold it into a
session alongside the `mwl-ir` picks above or, better, give it its own dedicated session given its size.
`mwl-hir`'s trait-flattening code is stale (still matches the pre-ADR-0043 design) until this lands.

**FYI, no action needed:** [ADR 0044](docs/adr/0044-core-process-argv-only-no-shell.md) landed in a
concurrent session — `Core\Process::run()`/`::spawn()` replaces PHP's `exec`/`system`/`passthru`/
`shell_exec`/`proc_open` family with one argv-only API (no shell-string form at all, a Windows
batch/PowerShell-target refusal, coroutine-suspending waits), superseding ADR 0024 §4's placeholder bullet.
It's M8-scoped and `Core\Process` doesn't exist yet on disk, so there is no stale code to fix now — nothing
to pick up until M8 starts.

**Housekeeping note:** `python .claude/brief.py`'s "WHERE THE PLAN STANDS" section has been hitting its
4000-byte budget and truncating for several sessions now (the M2 paragraph in `docs/implementation-plan.md`
is the largest single contributor) — this is exactly the signal `DOC_CLEANUP_PROMPT.md` describes as
"overdue for a trim pass." The user runs that pass manually; flagging it again here since it's now a
recurring truncation, not a one-off.
