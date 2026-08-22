# Next session prompt

Continue MWL. M1 (front end) is done. M2 (HIR/types/IR) is close to done — run `python .claude/brief.py`
first, then read `docs/implementation-plan.md`'s M2 paragraph for exactly what landed and how (this file
only points at what's next; the plan is the one home for status detail, per CLAUDE.md's "state a fact
once").

**Last session closed gap item 4 (the array-literal explicit `key =>` gap) in full, plus item 3's
float/bool/null-subscript bullet as a byproduct of the same fix.** Two-part change:

- `mwl_types`: a new `check_array_key_type` helper (in `crate::expr`) rejects a `float`/`bool`/`null` array
  key with a new `E_ARRAY_KEY_INVALID_TYPE` diagnostic (`crates/mwl-diagnostics/src/lib.rs`'s `E0434`) —
  called from both `check_array_literal`'s explicit-key arm and `check_expr`'s `Index` arm, so a subscript
  and an array-literal key are enforced identically. An `int`/`uint`/`string` key (or anything statically
  unknown, like `mixed`) is left alone, the same "erase to `mixed` rather than guess" split
  `division_result`/`bitwise_result` already draw.
- `mwl-ir`: a *purely positional* array literal (no explicit key anywhere) is unchanged — still one
  `InstKind::ArrayNew` with compile-time decimal-string keys, zero new instructions, zero snapshot diffs. A
  literal with **at least one** explicit `key =>` element now lowers to an *empty* `ArrayNew` followed by
  one `InstKind::ArraySet` per element in source order, reusing `Lowering::lower_array_key` verbatim for
  every key (explicit or positional) — the same helper an `$arr[$i]` subscript already used, exactly as
  the prior session's recommendation anticipated. `lower_array_key`'s float/bool/null panic arm is now an
  unreachable internal-invariant check rather than a live gap, on both call sites.

One deliberate, documented simplification: a positional element mixed after an explicit `int`/`uint` key
still numbers from "how many positional elements came before it," not PHP's real "continues from the
highest int key used so far" rule — replicating that needs the same "next available integer key" runtime
counter `$a[]` append syntax is still waiting on (see below), so it was explicitly kept out of scope rather
than half-implemented. `...spread` and `&value` array-literal elements are both still unsupported either
way — panicking naming whichever is used, since spread needs array-merge semantics and `&value` needs
reference-value semantics neither of which this crate has any representation for yet (bigger, separate
design questions, not mechanical extensions of this session's work).

Built, tested (`mwl-types` gained 8 new checker fixtures in `check.rs`; `mwl-ir` gained 4 new snapshot tests
and updated 2 existing ones whose old "still out of scope" framing no longer applied — one is now a
checker-rejection test instead of an IR-panic test), clippy- and fmt-clean, committed. `mwl-ir` is at 72
tests, `mwl-types` at 217.

**With that, `mwl-ir`'s known-gap list (its own module docs in `lib.rs`) is down to:**

1. ~~Control flow (`if`/`while`).~~ **Done.**
2. ~~Safepoints.~~ **Done** (reserved shape only). Revisit once M3's codegen exists.
3. ~~`new`/a static call, an instance method call, a compile-time-known property access, and array-element
   access through a known `int`/`uint`/`string` key.~~ **Done.** What's left of this shape:
   - **`$a[]`/`$a[] = expr;` (PHP's append syntax).** Needs a "next available integer key" counter this
     crate has no representation for yet — genuinely more than mechanical, since it means tracking (or
     re-deriving) an array's own highest-inserted-integer-key state at lowering time, not just reading one
     back. (This is also what blocks fully replicating PHP's auto-increment rule for the array-literal
     mixed-key case named above.)
   - ~~A `float`/`bool`/`null` array-subscript key.~~ **Done**, last session — rejected at check time now,
     same fix as item 4 below.
   - **Array-element access through a `mixed`-erased base.** No `ExprInfo::Index` entry exists for that
     case (mirrors `ExprInfo::Property`'s shape/`object`-erasure gap), so lowering panics naming it — but
     this is currently *unreachable* without first hitting the unrelated, already-documented "no
     `Ty::Mixed` representation" gap (item 5 below), since `mixed` isn't a lowerable declared type or
     resolved-call return type in this crate yet either.
4. **Non-scalar *data* values and refcount operations — `string`/`bytes`/`array<T>` locals, the
   call/return/property-read-and-write boundary, `.` concatenation, array-element read/write, and now an
   array literal's explicit `key =>` element are all landed; two pieces remain:**
   - ~~An explicit `key =>` array-literal element.~~ **Done**, last session — see above.
   - **A `...spread` or `&value` array-literal element.** Still unsupported, still panics naming whichever
     is used. Each needs its own design, not a mechanical extension: spread needs array-merge semantics
     (including how it interacts with the same integer-key-renumbering question item 3's first bullet
     already names), and `&value` needs a reference-value representation this crate has none of anywhere
     yet — picking either of these up means designing that representation first, not just wiring up
     existing helpers.
   - **`Ty::Object` refcounting.** Still zero retain/release operations for an object reference. Checked
     this session: there is **still no allocation/field-layout story** for `Ty::Object` to attach retain/
     release to — `InstKind::New`/`FieldGet`/`FieldSet` all operate on a bare, opaque class-name label with
     no memory-layout representation behind it (see `ty::Ty::Object`'s own doc comment). This is **not yet
     ready** — flipping `Ty::is_refcounted` to include it would insert retain/release calls with nothing
     underneath them to be correct about. Leave this for whenever an actual object layout/allocation design
     lands (expected around M3's codegen, not before).
   - **Qualified string/bytes types (`tainted`, `secret`, and their combination).** `lower_checked_ty` only
     has arms for the plain `CheckedTy::String`/`CheckedTy::Bytes`/`CheckedTy::Array`; the six qualified
     string/bytes variants still panic. These likely want to wait for ADR 0024 §4/0033's stdlib-dependent
     sinks anyway (M7/M8), since a qualifier with nothing to launder against isn't very actionable yet.
5. **Runtime-helper calls are landed** (`ir::InstKind::HelperCall`/`ir::Helper`), used for `.`'s scalar-to-
   `string` conversion and an `int`/`uint` array-subscript's/array-literal-key's normalization. Two more
   named uses remain:
   - **A `mixed`/union operand** — needs a `Ty::Mixed`-shaped IR representation first; none exists yet, so
     there's nothing for a helper to dispatch on. Adding one is its own small design question (how a
     `mixed` value's runtime type tag is represented) before any helper call can use it. (This is also
     what item 3's "mixed-erased array base" gap above is blocked on.) **You are authorized to design this
     yourself and proceed if you reach it — no need to stop and ask** (standing user direction).
   - **ADR 0035's truthy conversion** for a non-`bool` `if`/`while` condition — PHP's truthy table differs
     by source type (`0`/`0.0`/`""`/`"0"`/an empty array/`null` are falsy, everything else truthy). Both
     `Ty::Array` and array-element access exist now, giving the array-emptiness case a representation to
     convert *from* — but a nullable-type representation still doesn't, so the `null` case still waits. **A
     scalar-plus-array-only truthy helper could land now as a well-scoped partial slice** — this is this
     session's recommended pick if you want a `HelperCall`-shaped item (see below).
   - Both are expected to add new `Helper` variants to the same enum, not a second call-shaped instruction.
   - The one remaining `.`-concatenation gap — a `Stringable`-object operand — is *not* primarily a
     `HelperCall` gap any more: it needs `.` to synthesize a resolved `toString()` call, which needs either
     a checker-side change (recording an `ExprInfo::Call`-shaped resolution for a `.` operand, not just a
     call expression) or this crate re-resolving it independently. See the "runtime-helper calls" session's
     design-choices writeup in `mwl-ir`'s module docs before picking this up — it's a small but genuine
     decision, not a mechanical extension.
6. **Virtual dispatch** — every call/access lowered so far has its receiver's *static* type equal to its
   *runtime* class — none has gone through an interface-typed or overridden-method/property receiver yet.
   Whether a real vtable/interface-dispatch lookup belongs at this IR level (as opposed to purely at
   codegen, once M3 exists) is an open, architectural question — **skip this one** (per standing user
   direction, deferred until M3 starts) unless it turns out to be the only item left, in which case stop
   and report that instead of attempting it.
7. ~~`var` locals, multi-base integer-literal cooking, integer-literal magnitude range-checking.~~ **Done.**
8. ~~String-literal cooking completeness.~~ **Done.**

**Recommended pick for next session:** ADR 0035's truthy-conversion partial slice (item 5's second bullet)
— a `bool`-producing `Helper` variant for a scalar or `Ty::Array` operand in an `if`/`while` condition,
leaving the `null` case (no nullable-type IR representation yet) out of scope. It's well-scoped, reuses the
existing `HelperCall` shape with no new instruction kind, and unlike `Ty::Object` refcounting has no
missing-prerequisite blocker to check first. If that turns out not to fit in one session, `Ty::Mixed`'s
design (item 5's first bullet) is the other standing option — you're pre-authorized to design and proceed,
per the note above.

Once control flow, calls, and property/array access all lower, M2's own *Verify* bullet ("IR snapshot
tests; no program in the corpus produces an `Unknown` type") is worth revisiting for a real corpus-driven
snapshot suite, not just hand-written fixtures — at that point M2 as a whole should be closeable and M3
(baseline Cranelift backend, `Hello World`) can start.

Also still open from before (independent, low priority, unrelated to `mwl-ir`): a `set`-hooked property is
exempted from ADR 0022's constructor check entirely rather than verified against the hook's body; the
identical question now also applies to whether a `lateinit` + hooked property should discharge on the
hook's first commit (ADR 0038's own *Revisiting* names this, deferred to `docs/spec/`).

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
