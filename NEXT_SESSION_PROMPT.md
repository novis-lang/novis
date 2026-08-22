# Next session prompt

Continue MWL. M1 (front end) is done. M2 (HIR/types/IR) is close to done — run `python .claude/brief.py`
first, then read `docs/implementation-plan.md`'s M2 paragraph for exactly what landed and how (this file
only points at what's next; the plan is the one home for status detail, per CLAUDE.md's "state a fact
once").

**Last session landed `$a[] = expr;` — PHP's array append syntax, write side — the twentieth `mwl-ir`
slice.** A new `InstKind::ArrayAppend { array, value }` instruction carries no key at all, unlike
`InstKind::ArraySet`: PHP's real "next available integer key" rule tracks the highest `int` key ever used
as part of the array's own runtime state (surviving earlier explicit-`int`-keyed inserts, removals and
appends alike), which a lowering pass genuinely can't compute from the source text the way a literal's
positional index or an explicit `key =>` already can — its storage and increment are left entirely to
whatever `mwl-codegen`'s own array representation does with them, the same "shape now, functional once a
backend exists" deferral `InstKind::Safepoint` already gets.

`Lowering::lower_reassignment`'s `Index`-target arm now matches on the subscript itself: `None` lowers the
base and the new value only (no `lower_array_key` call at all, since there is no key to normalize) and
emits `ArrayAppend`, retaining the value first under the same `Ty::is_refcounted`/`is_aliasing_read` policy
`ArraySet`'s own value already gets; `Some(index)` is the unchanged pre-existing `ArraySet` path. `$a[]` as
a *read* (no subscript, no assignment) is untouched by this session and stays a permanent panic in
`Lowering::lower_expr`'s `Index` arm — it has no PHP meaning at all (PHP itself rejects it as "cannot use
`[]` for reading"), a closed design question rather than a gap waiting to be filled, even though
`mwl-syntax` parses it in any expression position and `mwl_types::expr::check_expr`'s `Index` arm doesn't
reject it as a read either.

The prior `should_panic` write-side test was converted into two real lowering tests: a fresh `int` value
needs no retain, and an aliasing `string` local value gets retained before `ArrayAppend` runs — both with
`insta` snapshots. `mwl-ir` is now at 95 tests (was 94). Built, tested, clippy- and fmt-clean, committed.
The crate's own module docs (`lib.rs`) and `docs/implementation-plan.md`'s M2 paragraph were both updated
in place, including fixing a couple of already-stale claims in `lib.rs`'s top summary paragraph (it still
listed the append-syntax gap and an already-landed "no explicit `key =>`" item from several slices back).

**With that, `mwl-ir`'s array-access shape (item 3 in its own known-gap list) is now closed except for one
sub-bullet:**

1. ~~Control flow (`if`/`while`).~~ **Done.**
2. ~~Safepoints.~~ **Done** (reserved shape only). Revisit once M3's codegen exists.
3. ~~`new`/a static call, an instance method call, a compile-time-known property access, array-element
   access through a known `int`/`uint`/`string` key, an array literal's explicit `key =>` element, and
   `$a[] = expr;` append syntax.~~ **All done.** What's left of this shape:
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
9. ~~`&&`/`||`/`!`/the ternary-elvis operator (ADR 0035's other four truthy positions).~~ **Done**, at any
   position that already owns a mutable `cur` — a nested one inside a fixed-`cur` position and mismatched
   ternary branch types are the narrower residual gaps (see `mwl-ir`'s own module docs).
10. ~~`$a[] = expr;` append syntax (write side).~~ **Done** (this session). `$a[]` as a read stays a
    permanent, deliberate panic — not a gap — since it has no PHP meaning at all.

**No self-contained mechanical mwl-ir slice is obviously next anymore** — every remaining item above is
either blocked on the `mixed` runtime type-tag design question (item 5), needs its own real design
(`...spread`/`&value`, item 4), is explicitly deferred (virtual dispatch, item 6), or waits on stdlib/M3
work that doesn't exist yet. Recommended options for next session, roughly in order of how self-contained
they are:

- **`for`/`switch`/`break`/`continue`.** Not named in the numbered list above (it's its own bullet in the
  known-gaps section) but likely the most mechanical remaining pick: `Terminator::Branch`/`ids::EdgeId` are
  already exercised by `if`/`while`, so this is expected to reuse the same shapes rather than invent new
  ones — see `lower`'s own module docs for the SSA merge-block precedent (`Lowering::merge_envs` for a
  fixed set of incoming edges, the seed-then-patch phi dance in `Lowering::lower_while` for a
  not-yet-known back edge). `break`/`continue` need a way to track the enclosing loop's merge/exit blocks
  through nested lowering — worth scoping deliberately rather than assuming it's free.
- **The `mixed` runtime type-tag representation (item 5 above)** — the single biggest unblock left on this
  list, but real, non-mechanical design work rather than a narrow mechanical slice. You're pre-authorized
  to design and proceed if you pick this up.
- **A `...spread`/`&value` array-literal element (item 4 above)** — each needs its own design (array-merge
  semantics; a reference-value representation), bigger and less mechanical than append syntax was.

Once control flow, calls, and property/array access all lower, M2's own *Verify* bullet ("IR snapshot
tests; no program in the corpus produces an `Unknown` type") is worth revisiting for a real corpus-driven
snapshot suite, not just hand-written fixtures — at that point M2 as a whole should be closeable and M3
(baseline Cranelift backend, `Hello World`) can start. `for`/`switch`/`break`/`continue` landing would be a
natural trigger to reassess whether that point has been reached.

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

**FYI, check for concurrent work before picking anything above:** at the end of this session, another
concurrent session had uncommitted, in-progress changes in the shared working tree touching `CLAUDE.md`,
`crates/mwl-diagnostics/src/lib.rs`, `crates/mwl-syntax/src/{ast,parser}.rs`, `crates/mwl-types/src/expr.rs`
and `docs/adr/README.md`, plus a new untracked `docs/adr/0045-and-or-xor-keyword-operators-rejected.md`
(status: Accepted) — PHP's `and`/`or`/`xor` keyword operators are being removed from the grammar entirely
(parse-time diagnostic naming `&&`/`||` as the replacement, or naming none for `xor`), since ADR 0035 never
included them and nothing past `mwl-types` ever lowered one anyway. This is unrelated to any `mwl-ir` pick
above and needs no action from you — it just means `git status`/`git diff` may show files you didn't touch
when you start; investigate before assuming corruption, per CLAUDE.md's own guidance, and don't disturb it
unless it's actually finished/committed by the time you look.

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
