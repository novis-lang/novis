# Next session prompt

Continue MWL. M1 (front end) is done. M2 (HIR/types/IR) is close to done — run `python .claude/brief.py`
first, then read `docs/implementation-plan.md`'s M2 paragraph for exactly what landed and how (this file
only points at what's next; the plan is the one home for status detail, per CLAUDE.md's "state a fact
once").

**Last session landed ADR 0035's truthy-conversion partial slice in `mwl-ir` — the seventeenth slice.** An
`if`/`while` condition no longer needs to be statically `bool`:

- Five new `ir::Helper` variants (`IntTruthy`/`UintTruthy`/`FloatTruthy`/`StrTruthy`/`ArrayTruthy`) reuse
  the existing `InstKind::HelperCall` shape the `.`-concatenation helpers already established. A new
  `lower::Lowering::lower_truthy_cond` converts a `bool` condition straight through, a scalar
  (`int`/`uint`/`float`/`string`) or `Ty::Array` condition through the matching helper, and a `Ty::Object`
  condition (a class instance or enum case) to a fresh `const.bool true` with **no** helper call at all —
  ADR 0035 § 4 makes either always truthy, so there's nothing to inspect at runtime.
- `lower_if`/`lower_while` both now call `lower_truthy_cond` instead of asserting the condition is already
  `bool`. A refcounted condition (`Ty::Str`/`Ty::Array`) that isn't `is_aliasing_read` — a fresh
  call/`new`/literal result whose only use is the truthy test — is released right after the helper reads
  it, the same precedent `concat_operand`'s own caller already set for `.` concatenation; verified with a
  dedicated snapshot test (`a_fresh_array_condition_is_released_after_the_truthy_check`) showing the
  `release` lands right after the `helper.array_truthy` call and before the branch.
- Deliberately still out of scope, and unreachable today: `null` (no nullable-type IR representation
  exists yet) and `mixed`/a union (same `Ty::Mixed` gap named below). `&&`/`||`/`!` and the ternary/elvis
  condition — ADR 0035's other four truthy positions — are untouched: this crate doesn't lower any of the
  three yet at all, so only `if`/`while`'s own condition changed this slice.

Five new snapshot tests cover an `int` condition, a `string` `while` condition, an `array<int>` condition
(aliasing — no extra release), a fresh array-returning-call condition (release verified), and an
always-truthy object condition. `mwl-ir` is now at 77 tests (was 72); `mwl-types` is unchanged at 217 (its
own ADR 0035 checker-side fixture already existed). Built, tested, clippy- and fmt-clean, committed.
`docs/implementation-plan.md`'s M2 paragraph and `crates/mwl-ir/src/lib.rs`'s module docs (both the
seventeenth-slice paragraph and the known-gaps list) were updated in place — including fixing a stale
contradiction in the plan's "deliberately out of scope" list left over from an earlier session (it still
described the array-literal explicit `key =>` element as unsupported, which had actually landed two
sessions ago).

**With that, `mwl-ir`'s known-gap list (its own module docs in `lib.rs`) is down to:**

1. ~~Control flow (`if`/`while`).~~ **Done.**
2. ~~Safepoints.~~ **Done** (reserved shape only). Revisit once M3's codegen exists.
3. ~~`new`/a static call, an instance method call, a compile-time-known property access, and array-element
   access through a known `int`/`uint`/`string` key.~~ **Done.** What's left of this shape:
   - **`$a[]`/`$a[] = expr;` (PHP's append syntax).** Needs a "next available integer key" counter this
     crate has no representation for yet.
   - **Array-element access through a `mixed`-erased base.** Still blocked on the same "no `Ty::Mixed`
     representation" gap as item 5 below — not independently actionable.
4. **Non-scalar *data* values and refcount operations** — two pieces remain:
   - **A `...spread` or `&value` array-literal element.** Still unsupported, still panics naming whichever
     is used. Each needs its own design: spread needs array-merge semantics, `&value` needs a
     reference-value representation this crate has none of anywhere yet.
   - **`Ty::Object` refcounting.** Still zero retain/release operations for an object reference — no
     allocation/field-layout story exists yet for `Ty::Object` to attach retain/release to. Leave this for
     whenever an actual object layout/allocation design lands (expected around M3's codegen, not before).
   - **Qualified string/bytes types (`tainted`, `secret`, and their combination).** Still panics; likely
     wants to wait for ADR 0024 §4/0033's stdlib-dependent sinks anyway (M7/M8).
5. **Runtime-helper calls are landed** for `.`'s scalar-to-`string` conversion, an `int`/`uint`
   array-subscript's/array-literal-key's normalization, and now ADR 0035's truthy conversion for a scalar
   or `Ty::Array` `if`/`while` condition. What remains:
   - **A `mixed`/union operand** — needs a `Ty::Mixed`-shaped IR representation first; none exists yet.
     Adding one is its own small design question (how a `mixed` value's runtime type tag is represented)
     before any helper call — arithmetic's `mixed` fallback, ADR 0035's `null`/`mixed` truthy case, and
     item 3's "mixed-erased array base" gap above are all blocked on this same representation landing.
     **You are authorized to design this yourself and proceed if you pick this up — no need to stop and
     ask** (standing user direction, carried over from the previous session).
   - The `.`-concatenation `Stringable`-object-operand gap is *not* primarily a `HelperCall` gap: it needs
     `.` to synthesize a resolved `toString()` call — see the "runtime-helper calls" session's
     design-choices writeup in `mwl-ir`'s module docs before picking this up, a small but genuine decision.
6. **Virtual dispatch** — skip this one (per standing user direction, deferred until M3 starts) unless it
   turns out to be the only item left, in which case stop and report that instead of attempting it.
7. ~~`var` locals, multi-base integer-literal cooking, integer-literal magnitude range-checking.~~ **Done.**
8. ~~String-literal cooking completeness.~~ **Done.**

**Recommended pick for next session:** `Ty::Mixed`'s IR representation (item 5's first bullet) — you're
pre-authorized to design and proceed on this one, and it's the single biggest unblock left: it's what item
3's "mixed-erased array base" gap, ADR 0035's `null`/`mixed` truthy case, and ordinary arithmetic's `mixed`
fallback are all separately waiting on. Consider scoping the *first* slice narrowly (e.g. just enough
representation to let a `mixed`-typed local/parameter/return value exist and round-trip, before wiring any
new `Helper` variant to dispatch on it) rather than trying to close every blocked gap in one session — the
same "narrow slice first" discipline every prior `mwl-ir` session in this file has used.

If `Ty::Mixed` doesn't fit in one session either, other standing options, roughly in order of how
self-contained they are:
- The `&&`/`||`/`!`/ternary-elvis truthy positions (ADR 0035's other four) — currently *none* of the four
  are lowered by this crate at all (not even for a plain `bool` operand), so landing them is a genuinely
  separate slice from this session's `if`/`while` work, not a trivial follow-on.
- `$a[]`/`$a[] = expr;` append syntax (needs the "next available integer key" counter design).

Once control flow, calls, and property/array access all lower, M2's own *Verify* bullet ("IR snapshot
tests; no program in the corpus produces an `Unknown` type") is worth revisiting for a real corpus-driven
snapshot suite, not just hand-written fixtures — at that point M2 as a whole should be closeable and M3
(baseline Cranelift backend, `Hello World`) can start.

Also still open from before (independent, low priority, unrelated to `mwl-ir`): a `set`-hooked property is
exempted from ADR 0022's constructor check entirely rather than verified against the hook's body; the
identical question now also applies to whether a `lateinit` + hooked property should discharge on the
hook's first commit (ADR 0038's own *Revisiting* names this, deferred to `docs/spec/`).

**Housekeeping note:** `python .claude/brief.py`'s "WHERE THE PLAN STANDS" section has been hitting its
4000-byte budget and truncating for at least two sessions now (the M2 paragraph in
`docs/implementation-plan.md` keeps growing as more `mwl-ir` slices land). This is the user's manual
`DOC_CLEANUP_PROMPT.md` trim pass to run, not something to fix automatically — flagging again since it's
still true after this session's edits.

**Separately, whenever M3 finishes and M4 is underway:** keep [ADR 0040](docs/adr/0040-vscode-deep-tooling-and-resilient-parsing.md)
in mind as M4 approaches its own "usable CLI language" exit criterion — M4B (minimal `mwl-lsp` +
`editors/vscode`, plus `mwl-syntax`'s new resilient-parse mode) starts right after, per the plan.

**Separately, whenever M5 (concurrency and script isolates) is underway:** give each of the three
spawn-construct runtime routines its `spawn`-kind trace hook, and whenever the mark-sweep cycle collector's
run routine is built, give it its `gc`-kind hook too — both per [ADR 0041](docs/adr/0041-timeline-export-and-gc-spawn-trace-events.md),
both instrumentation-only inside those already-rare routines, no change to the safepoint poll itself.

**Separately, whenever M6 (config, limits, capabilities, disk cache) is underway:** build exactly what
[ADR 0042](docs/adr/0042-on-disk-artifact-cache-format.md) specifies for the on-disk artifact cache — do not
re-derive the file format or eviction policy from scratch. Its own *Revisiting* section leaves two things
genuinely open for whoever implements it: the exact default values for `opcache.file_cache_max_size` and the
GC-probability/divisor pair, and how many ancestor directories the ownership/permission check walks above
the cache directory itself.
