# Next session prompt

Continue MWL. M1 (front end) is done. M2 (HIR/types/IR) is close to done — run `python .claude/brief.py`
first, then read `docs/implementation-plan.md`'s M2 paragraph for exactly what landed and how (this file
only points at what's next; the plan is the one home for status detail, per CLAUDE.md's "state a fact
once").

**Last session landed `Ty::Mixed` — the eighteenth slice, and the first slice to widen `mwl-ir`'s own IR
type lattice rather than what it lowers.** ADR 0007 § 3's `mixed` now has an opaque IR representation:

- `Ty::Mixed`, a new `#[non_exhaustive] Ty` variant, bare and opaque like `Ty::Object`/`Ty::Array`.
  `lower_decl_type` gained a `TypeAtom::Mixed => Ty::Mixed` arm and `lower_checked_ty` a
  `CheckedTy::Mixed => Ty::Mixed` one, mirroring exactly how both already erase a class/enum name to
  `Ty::Object` — no new `Lowering` insertion point was needed at all, since `bind_local`,
  `lower_call_args`, `release_all_locals` and `StmtKind::Return`'s own arm all key off
  `Ty::is_refcounted`/`is_aliasing_read` rather than naming a concrete `Ty` variant directly.
- `Ty::is_refcounted` deliberately does **not** include `Ty::Mixed`: a `mixed` value's actual runtime
  shape might be refcounted (a `string`, an array, an object) or not (a scalar), and nothing decides that
  runtime type tag yet — so there's no way to know *whether* a retain/release is even needed today, only
  that skipping one doesn't break the round-trip this slice promises.
- Scoped deliberately narrow, per the standing "narrow slice first" discipline: enough representation for
  a `mixed`-typed local, parameter, return value or call argument to exist and round-trip. Nothing that
  needs to know a `mixed` value's *actual* runtime type — arithmetic, `.` concatenation, ADR 0035's truthy
  table, array-element access through a `mixed`-erased base — was wired up; all four still panic naming the
  gap, now reachable (a `mixed`-typed value can exist as input) rather than theoretical. A dedicated test
  (`a_mixed_condition_still_panics_naming_the_gap`) pins that a `mixed`-typed `if` condition reaches
  `lower_truthy_cond` and panics there rather than being unreachable input.
- The real design question the milestone text poses — **how a `mixed` value's runtime type tag is
  represented**, needed before any code can branch on what a `mixed` value actually holds — is still open.
  It's the single remaining unblock for arithmetic's `mixed` fallback, ADR 0035's `null`/`mixed` truthy
  case, and the mixed-erased-array-base gap, but is real design work, not a mechanical follow-on, so it was
  deliberately left rather than guessed at in the same session that added the representation.

Four new snapshot tests cover a `mixed` parameter round-tripping through `return`, an explicitly
`mixed`-typed local, `var $y = $x;` inferring `mixed` from a `mixed` initializer, and a `mixed` local
crossing a call-argument boundary; a fifth (`should_panic`) test pins the still-open truthy-conversion gap.
`mwl-ir` is now at 82 tests (was 77); `mwl-types` is unchanged at 217 (no checker-side change — `mixed`
already parsed and checked before this session). Built, tested, clippy- and fmt-clean, committed in two
commits (code+tests+`mwl-ir` module docs, then the plan's M2 paragraph). Two stale claims left over from
the truthy-conversion session (both said no `Ty::Mixed` IR representation existed yet) were fixed in place
rather than left to contradict the new paragraph.

**With that, `mwl-ir`'s known-gap list (its own module docs in `lib.rs`) stands at:**

1. ~~Control flow (`if`/`while`).~~ **Done.**
2. ~~Safepoints.~~ **Done** (reserved shape only). Revisit once M3's codegen exists.
3. ~~`new`/a static call, an instance method call, a compile-time-known property access, and array-element
   access through a known `int`/`uint`/`string` key.~~ **Done.** What's left of this shape:
   - **`$a[]`/`$a[] = expr;` (PHP's append syntax).** Needs a "next available integer key" counter this
     crate has no representation for yet. Fully self-contained — doesn't touch `mixed` at all.
   - **Array-element access through a `mixed`-erased base.** `Ty::Mixed` now gives this a representation to
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
5. **`Ty::Mixed` now exists** (the eighteenth slice, above), but nothing dispatches on a `mixed` value's
   *actual* runtime type yet. What remains, all blocked on the same open design question:
   - **A runtime type-tag representation for `mixed`.** This is the real remaining design work — how a
     `mixed` value's actual runtime type (int? string? array? object?) is discoverable at runtime. Once
     picked, it unblocks all three of: arithmetic's `mixed` fallback (a `BinOp`/`UnOp` operand that erased
     to `mixed`), ADR 0035's `null`/`mixed` truthy case (a `mixed`-typed `if`/`while` condition — see
     `lower_truthy_cond`'s own doc comment), and item 3's mixed-erased-array-base gap above.
     **You are authorized to design this yourself and proceed if you pick this up** — no need to stop and
     ask (standing user direction, carried over from two sessions ago). Consider scoping the *first* slice
     to just one consumer (e.g. only the truthy case, or only one arithmetic operator) rather than wiring
     all three at once.
   - The `.`-concatenation `Stringable`-object-operand gap is *not* primarily a `HelperCall` gap and is
     unrelated to `mixed`: it needs `.` to synthesize a resolved `toString()` call — see the "runtime-
     helper calls" session's design-choices writeup in `mwl-ir`'s module docs before picking this up.
6. **Virtual dispatch** — skip this one (per standing user direction, deferred until M3 starts) unless it
   turns out to be the only item left, in which case stop and report that instead of attempting it.
7. ~~`var` locals, multi-base integer-literal cooking, integer-literal magnitude range-checking.~~ **Done.**
8. ~~String-literal cooking completeness.~~ **Done.**

**Recommended pick for next session:** the `&&`/`||`/`!`/ternary-elvis truthy positions (ADR 0035's other
four truthy positions, beyond `if`/`while`) — currently *none* of the four are lowered by this crate at all,
not even for a plain `bool` operand. This is fully self-contained (no design decision needed — it's a
mechanical reuse of `lower_truthy_cond`'s existing `bool`/scalar/`Ty::Array`/`Ty::Object` table at three new
syntax positions, the same way `if`/`while` reused it) and doesn't touch `mixed` at all, unlike almost
everything else left on this list. It also closes out ADR 0035's checker-side-already-passing truthy story
to "every position lowers except `null`/`mixed`," a clean, nameable stopping point.

Other self-contained options, roughly in order of size:
- `$a[]`/`$a[] = expr;` append syntax (needs the "next available integer key" counter design — item 3
  above). Bigger than the truthy positions, but still doesn't touch `mixed`.
- The `mixed` runtime type-tag representation (item 5 above) — the single biggest unblock left on this
  list, but real, non-mechanical design work rather than a narrow mechanical slice. Pick this up instead of
  the truthy positions if you'd rather tackle the bigger design question head-on; you're pre-authorized to
  design and proceed.

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
session alongside the `mwl-ir` picks below or, better, give it its own dedicated session given its size.
`mwl-hir`'s trait-flattening code is stale (still matches the pre-ADR-0043 design) until this lands.

**Housekeeping note:** `python .claude/brief.py`'s "WHERE THE PLAN STANDS" section has been hitting its
4000-byte budget and truncating for at least three sessions now (the M2 paragraph in
`docs/implementation-plan.md` is the largest single contributor) — this is exactly the signal
`DOC_CLEANUP_PROMPT.md` describes as "overdue for a trim pass." The user runs that pass manually; flagging
it again here since it's now a recurring truncation, not a one-off.
