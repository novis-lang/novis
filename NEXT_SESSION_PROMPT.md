# Next session prompt

Continue MWL. M1 (front end) is done. M2 (HIR/types/IR) is close to done — run `python .claude/brief.py`
first, then read `docs/implementation-plan.md`'s M2 paragraph for exactly what landed and how (this file
only points at what's next; the plan is the one home for status detail, per CLAUDE.md's "state a fact
once").

**Last session landed a positional `array<T>` literal — the design tradeoff the plan had flagged as the
one remaining piece needing a real decision, and it resolved to "no decision needed" once the actual
question was pinned down.** A prior session's research (reproduced in that session's own prompt, and now
folded into `crates/mwl-ir/src/ty.rs`'s module doc) established that ADR 0007 fixes *observable* array
semantics (insertion-ordered, copy-on-write, every key a `string`) but never mandates hashmap-vs-contiguous
backing storage — that's a `mwl-codegen`/runtime question, not an IR-representation one. So the actual
question this session answered was narrower: does `mwl-ir`'s own `Ty` need to carry an array's *element*
type at all? No — exactly like `Ty::Object` erases a class/enum's identity because no lowering decision
branches on *which* class it is, no lowering decision branches on an array's element type either (the
checker's own `mwl_types::ty::Ty::Array(TypeId)` already enforces element-type correctness at check time).
So `Ty::Array` landed as a bare, opaque unit variant — same shape as `Ty::Object`, zero new fields, zero new
dependencies.

**What landed, concretely:**

- `Ty::Array` (`crates/mwl-ir/src/ty.rs`) — a third refcounted, heap-allocated representation alongside
  `Ty::Str`/`Ty::Bytes`. `Ty::is_refcounted` now matches `Ty::Str | Ty::Bytes | Ty::Array`.
- `ir::InstKind::ArrayNew { entries: Vec<(String, ValueId)> }` (`crates/mwl-ir/src/ir.rs`) — builds a fresh
  array from a fixed list of already-lowered `(key, value)` pairs. Each `key` is a **decimal string computed
  at lowering time**, not a lowered expression: this slice only lowers a *positional* literal (no explicit
  `key =>`), so an element's key is simply its index, auto-numbered from `0` exactly like PHP's own
  `[$a, $b]` shorthand — no runtime index-tracking instruction needed.
- `lower::lower_decl_type` gained a `TypeAtom::Array(_) => Ty::Array` arm (the type argument is discarded,
  same erasure `TypeAtom::Name(_) => Ty::Object` already does).
- `lower::lower_checked_ty` gained a `CheckedTy::Array(_) => Ty::Array` arm — this is what made the
  call-argument/return/property-read-and-write boundary work **with no new insertion point at all**, the
  same way `bytes` needed none: every retain/release site (`bind_local`, `lower_call_args`,
  `release_all_locals`, `lower_reassignment`'s property-target arm) already keys off
  `Ty::is_refcounted`/`is_aliasing_read` rather than naming `Ty::Str` by name.
- `lower::Lowering::lower_expr` gained an `ExprKind::ArrayLiteral` arm: lowers each positional element
  (panicking naming the gap for an explicit `key =>`, a `...spread`, or a `&value` element), retaining any
  element that's itself `Ty::is_refcounted` and `is_aliasing_read` — the exact same caller-side retain
  `lower_call_args` already gives a refcounted, aliasing call argument, reused verbatim rather than a new
  policy. The array literal's own result needs no retain (a fresh producer, same as `new`/a call's result).
- `print::ty_name` gained a `Ty::Array => "array"` arm, and `print_inst` gained an `InstKind::ArrayNew` arm
  (renders as `array.new ["0": v1, "1": v2, ...]`).
- `concat_operand`'s inner helper-dispatch `unreachable!` arm (matched exhaustively within this crate,
  `#[non_exhaustive]` only restricts other crates) had to add `Ty::Array` alongside `Ty::Str`/`Ty::Bytes`/
  `Ty::Void`/`Ty::Object`.

**Nine new snapshot tests** in `crates/mwl-ir/src/lower.rs`'s `tests` module, all passing and reviewed by
hand against the retain/release bookkeeping they're meant to prove correct:

- `an_empty_array_literal_lowers_with_no_entries` — `[]` returned directly, no retain (fresh producer,
  `is_aliasing_read` is `false` for `ArrayLiteral`), no release (transfers out).
- `a_literal_with_fresh_scalar_elements_needs_no_retain` — `[1, 2, 3]`, none of the elements refcounted, so
  only the array's own slot gets released at the exit sweep.
- `a_literal_with_an_aliasing_element_retains_it` — `[$s]` where `$s` is a `string` local: retains `$s`'s
  value before `array.new`, then releases both the array and `$s`'s own slot at exit (name-sorted order) —
  one retain, two releases, correctly balanced (the array's own release is expected to cascade to its
  stored elements once a runtime exists; that's the array's own drop responsibility, not something this
  lowering needs to spell out per element, same as `InstKind::Release`'s doc comment already frames generically).
- `an_explicit_keyed_array_element_is_still_out_of_scope` / `a_spread_array_element_is_still_out_of_scope`
  — both `should_panic(expected = "known gap")`.
- `passing_an_array_local_as_a_call_argument_retains_it`, `binding_an_array_property_read_to_a_local_retains_it`,
  `writing_an_array_local_to_a_property_retains_it_before_releasing_the_old_value` — mirror the `string`/
  `bytes` call-argument and property read/write tests exactly, proving the "no new insertion point" claim
  above rather than just asserting it. The property tests use the same "no literal default, so the
  constructor takes an `array` parameter and assigns it" pattern `bytes`'s property tests already
  established, since a property initializer has no `array` literal-default precedent in this crate either.

`mwl-ir` is now at 55 tests (`mwl-types` unchanged at 162). `cargo build`/`test`/`clippy --all-targets -- -D
warnings`/`fmt --check` all clean across the whole workspace. `crates/mwl-ir/src/lib.rs`'s module docs (the
"what this crate lowers so far" list, the known-gaps section) and `ty.rs`/`ir.rs`'s own doc comments were
updated in place (not appended) to describe the slice; `docs/implementation-plan.md`'s M2 paragraph was
updated the same way — note that section was *already* over `.claude/brief.py`'s 4000-byte budget before
this session (it truncates and says so); it grew slightly more this session since the edit was
additive-in-place rather than a trim. Not fixed here — `DOC_CLEANUP_PROMPT.md`'s trim pass is the
user-run remedy for that, unrelated to this session's own scope.

**Known gaps, all named in `mwl-ir`'s own module docs — pick up widening from here, in roughly this
order** (each is its own reasonably-sized slice; don't try all of them in one session):

1. ~~Control flow (`if`/`while`).~~ **Done.**
2. ~~Safepoints.~~ **Done** (reserved shape only). Revisit once M3's codegen exists.
3. ~~`new`/a static call, an instance method call, and a compile-time-known property access (including the
   shape/`object`-erasure panic case).~~ **Done.** One shape remains:
   - **Array access (`$arr[$i]`, read or write)** is still unsupported; lowering panics naming the
     expression. Now that `Ty::Array`/`InstKind::ArrayNew` exist, this is the natural next pickup — it
     needs its own new `InstKind` (an indexed read/write) and a real design question: what happens on a
     missing key (PHP throws a warning-and-null on read, autovivifies on write) and whether that's
     modeled at this IR level at all yet, or deferred like other throwing operations are (no `try`/`throw`
     lowering exists yet — see item 5's `try`/`throw` note).
4. **Non-scalar *data* values and refcount operations — `string`/`bytes`/`array<T>` locals, the
   call/return/property-read-and-write boundary, and `.` concatenation (including a scalar operand, via a
   helper call) are all landed; two pieces remain, both mechanical:**
   - **An explicit `key =>`, a `...spread`, or a `&value` array-literal element.** `InstKind::ArrayNew`'s
     own doc comment explains why this session scoped them out: `mwl_types::expr::check_array_literal`
     itself has no key-normalization/rejection logic yet (ADR 0007 § 5's int/uint-to-decimal-string
     normalization, float/bool/null rejection), so lowering an explicit key would mean guessing at a
     runtime conversion this crate can't yet synthesize. Landing this probably wants a checker-side fix
     first (`mwl-types`), not just an `mwl-ir` change.
   - **`Ty::Object` refcounting.** Still zero retain/release operations for an object reference — the
     `bind_local`/`lower_call_args`/`release_all_locals`/`lower_expr_stmt`/`lower_reassignment` insertion
     points already extended for `Ty::Str`/`Ty::Bytes`/`Ty::Array` are expected to extend to it directly
     (just flip `Ty::is_refcounted` to include `Ty::Object` and re-run the existing test suite to see what
     breaks), once there's an actual allocation/field-layout story to attach it to — check whether one
     exists yet before assuming it's ready, per this loop's own standing instructions.
   - **Qualified string/bytes types (`tainted`, `secret`, and their combination).** `lower_checked_ty` only
     has arms for the plain `CheckedTy::String`/`CheckedTy::Bytes`/`CheckedTy::Array`; the six qualified
     string/bytes variants (`TaintedString`/`SecretString`/`SecretTaintedString` and their `Bytes`
     counterparts) still panic. These likely want to wait for ADR 0024 §4/0033's stdlib-dependent sinks
     anyway (M7/M8), since a qualifier with nothing to launder against isn't very actionable yet.
5. **Runtime-helper calls are landed** (`ir::InstKind::HelperCall`/`ir::Helper`), but only for `.`'s
   scalar-to-`string` conversion. Two more named uses remain, both blocked on something other than the
   `HelperCall` shape itself now:
   - **A `mixed`/union operand** — needs a `Ty::Mixed`-shaped IR representation first; none exists yet, so
     there's nothing for a helper to dispatch on. Adding one is its own small design question (how a
     `mixed` value's runtime type tag is represented) before any helper call can use it.
   - **ADR 0035's truthy conversion** for a non-`bool` `if`/`while` condition — PHP's truthy table differs
     by source type (`0`/`0.0`/`""`/`"0"`/an empty array/`null` are falsy, everything else truthy). Now
     that `Ty::Array` exists, the array-emptiness case has a representation to convert *from* — but a
     nullable-type representation still doesn't, so the `null` case still waits. A scalar-plus-array-only
     truthy helper could land now as a partial slice if a fixture wants it.
   - Both are expected to add new `Helper` variants to the same enum, not a second call-shaped instruction.
   - The one remaining `.`-concatenation gap — a `Stringable`-object operand — is *not* primarily a
     `HelperCall` gap any more: it needs `.` to synthesize a resolved `toString()` call, which needs either
     a checker-side change (recording an `ExprInfo::Call`-shaped resolution for a `.` operand, not just a
     call expression) or this crate re-resolving it independently. See the "runtime-helper calls" session's
     design-choices writeup in `mwl-ir`'s module docs before picking this up — it's a small but genuine
     decision, not a mechanical extension.
6. **Virtual dispatch** — every call/access lowered so far (`new`'s constructor, a static call, an
   instance call, a property access) has its receiver's *static* type equal to its *runtime* class — none
   has gone through an interface-typed or overridden-method/property receiver yet, which is the first
   place the two could actually differ. Whether a real vtable/interface-dispatch lookup belongs at this IR
   level (as opposed to purely at codegen, once M3 exists) is an open question for whichever session first
   hits that shape.
7. ~~`var` locals (ADR 0037) and multi-base integer-literal cooking (hex/octal/binary).~~ **Done.** Full
   integer-literal *magnitude* range-checking (negative-into-`uint`, too-large-for-either) is still not
   modeled, mirroring `mwl_types::expr::infer`'s own documented gap for the same case — small and
   independent, land whenever convenient.
8. **String-literal cooking completeness** — a numeric escape (`\xHH`, `\u{...}`, octal) inside a
   double-quoted literal passes through uncooked rather than resolving to the byte/codepoint it names;
   `ExprKind::Interpolated` (a double-quoted string/heredoc with an interpolation site) and a
   heredoc/nowdoc-sourced `ExprKind::Str` are both entirely unsupported. Small and independent, land
   whenever convenient — interpolation is expected to desugar to the same `InstKind::Concat` chain a
   written-out `.` expression already lowers to, so this pairs naturally with that, not with a separate
   mechanism.

Once control flow, calls, and property/array access all lower, M2's own *Verify* bullet ("IR snapshot
tests; no program in the corpus produces an `Unknown` type") is worth revisiting for a real corpus-driven
snapshot suite, not just hand-written fixtures — at that point M2 as a whole should be closeable and M3
(baseline Cranelift backend, `Hello World`) can start.

Also still open from before (independent, low priority, unrelated to `mwl-ir`): a `set`-hooked property is
exempted from ADR 0022's constructor check entirely rather than verified against the hook's body; the
identical question now also applies to whether a `lateinit` + hooked property should discharge on the
hook's first commit (ADR 0038's own *Revisiting* names this, deferred to `docs/spec/`).
