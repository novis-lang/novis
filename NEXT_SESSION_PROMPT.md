# Next session prompt

Continue MWL. M1 (front end) is done. M2 (HIR/types/IR) is close to done — run `python .claude/brief.py`
first, then read `docs/implementation-plan.md`'s M2 paragraph for exactly what landed and how (this file
only points at what's next; the plan is the one home for status detail, per CLAUDE.md's "state a fact
once").

**Last session landed runtime-helper calls — the milestone's third named ingredient, and the item flagged
two sessions running as the most likely to carry a real design tradeoff.** It turned out to have a fairly
clear resolution once ADR 0002 and the existing `InstKind::Call`/`New` shapes were read carefully, so the
session picked it up rather than deferring to `bytes`. What landed, narrowly scoped to exactly the one gap
`.` concatenation still had open (a scalar operand):

- `crate::ir::InstKind::HelperCall { helper: Helper, args: Vec<ValueId> }` — a new instruction, and
  `crate::ir::Helper` — a closed, `#[non_exhaustive]` enum (`IntToString`/`UintToString`/`FloatToString`/
  `BoolToString`) naming which conversion. **Three designs were weighed** (see `mwl-ir`'s own module docs'
  design-choices section for the full writeup): (a) this dedicated instruction with an enum tag — chosen;
  (b) reusing `InstKind::Call` with a synthetic target label (e.g. `"Core::intToString"`); (c) a dedicated
  instruction with a string name instead of an enum. Against (b): `Call::target`'s own doc comment already
  scopes it to a target actually resolved from the class hierarchy via `ExprTypeTable`, and `Call::receiver`
  only makes sense for a user-level instance call — folding a helper into `Call` would blur the line the
  "no virtual dispatch" known gap depends on staying sharp. Against (c): the helper set is small, closed,
  and known entirely to this crate and the future `mwl-codegen` helper table, never user-extensible, so a
  string buys nothing an enum doesn't already give for free while losing compile-time exhaustiveness —
  `BinOp`/`UnOp` already establish the same enum-for-a-closed-set precedent. `HelperCall` also does **not**
  model ADR 0002's checked-return convention yet (no status value, no error edge) — deliberately, matching
  `Call`/`New`'s own still-unmodeled call-that-can-fail case: nothing in this crate models a call that can
  fail at all yet (`try`/`throw` are both still unsupported), so giving only `HelperCall` a partial
  checked-return shape now would be inconsistent rather than the "shape now, functional later" treatment
  `Safepoint`/`Release` already get. That convention is expected to land for `Call`/`New`/`HelperCall`
  together, whenever `try`/`throw` lowering needs it — not one at a time.
- `Lowering::concat_operand` (new): lowers one `.` operand and, if it isn't already `Ty::Str`, converts it
  through a `HelperCall` — `Ty::Bool`/`Int`/`Uint`/`Float` all convert; `Ty::Object` (a `Stringable`
  operand) still panics, since desugaring it needs a resolved `toString()` call `.` has no way to
  synthesize from a bare operand (a `.` operand isn't a call expression, so `mwl_types::expr_table::
  ExprTypeTable` records no `ExprInfo::Call` for it the way an actual `$obj->toString()` call site would
  have — closing that needs either the checker to start recording that resolution for a `.` operand too,
  or this crate to re-resolve `toString` on its own, a second `mwl-types` dependency shape this crate has
  so far avoided; worth deciding deliberately once a fixture actually needs it, not guessed at). `lower_expr`'s
  `ExprKind::Binary`/`BinaryOp::Concat` arm now lowers both operands with `expected: None` (matching
  `mwl_types::expr::check_expr`'s own `check_expr(lhs, None, ...)` for a `.` operand, rather than the
  previous session's `Some(Ty::Str)`, which happened to behave identically but was conceptually wrong) via
  `concat_operand`, instead of asserting both came back `Ty::Str` directly.
- **A latent leak in the previous session's `Concat` lowering, found and fixed as part of this slice.** A
  fresh, non-aliasing `Ty::Str` operand read only by `Concat` and never bound into any durable slot (a bare
  string literal, e.g. `"a" . "b"`) had nothing that would ever release it — `Concat`'s own doc comment
  said ownership "stays wherever it already was," but for a bare literal that's nowhere, so nobody ever
  released it. `concat_operand` now returns `(ValueId, bool)`, the bool being `is_aliasing_read` of the
  operand's own source expression; `lower_expr`'s `Concat` arm releases either operand right after `Concat`
  reads it whenever that bool is `false`, the same "release a fresh value once its one and only use is
  done" precedent a bare call/`new` statement (`Lowering::lower_expr_stmt`) already set. This changed the
  existing `concatenating_two_string_literals_needs_no_retain_of_either_operand` snapshot (now shows two
  `release`s after the `concat`) — read it before trusting the fix; it's correct (verified this session).
- Three snapshot-test changes in `crates/mwl-ir/src/lower.rs`'s `tests` module:
  `concatenating_an_int_literal_with_a_string_uses_a_helper_call` (`return 1 . "x";` — `helper.int_to_string`
  feeding `concat`, both operands released after) and `concatenating_a_bool_local_with_a_string_uses_a_helper_call`
  (`return $flag . "!";` — `helper.bool_to_string` over an aliasing `bool` param read, still releases the
  *conversion result*, since a `bool` local was never refcounted to begin with) are new. The old
  `concatenating_a_non_string_operand_is_still_out_of_scope` test (`1 . "x"`, `#[should_panic]`) no longer
  panics — replaced by `concatenating_a_stringable_object_operand_is_still_out_of_scope`, which pins the one
  remaining `.`-lowering gap (a `Stringable`-implementing object operand) instead. `mwl-ir` is now at 42
  tests (`mwl-types` unchanged at 162). `cargo build`/`test`/`clippy --all-targets -- -D warnings`/
  `fmt --check` all clean across the whole workspace.

**Known gaps, all named in `mwl-ir`'s own module docs — pick up widening from here, in roughly this
order** (each is its own reasonably-sized slice; don't try all of them in one session):

1. ~~Control flow (`if`/`while`).~~ **Done.**
2. ~~Safepoints.~~ **Done** (reserved shape only). Revisit once M3's codegen exists.
3. ~~`new`/a static call, an instance method call, and a compile-time-known property access (including the
   shape/`object`-erasure panic case).~~ **Done.** One shape remains:
   - **Array access (`$arr[$i]`)** is still unsupported; lowering panics naming the expression. No
     `array<T>` *data* representation exists yet either (see item 4 below), so this may naturally land
     together with that slice rather than alone.
4. **Non-scalar *data* values and refcount operations — `string` locals, the call/return/property-read-and-
   write boundary, and `.` concatenation (including a scalar operand, via a helper call) are all landed;
   three pieces remain:**
   - **`bytes`.** Expected to be a mechanical repeat of `Ty::Str`'s shape (same refcounted-heap-value
     treatment, different content, same `bind_local`/`lower_call_args`/`release_all_locals`/`lower_expr_stmt`/
     `lower_reassignment` insertion points, same `is_aliasing_read` reuse) — extend `Ty::is_refcounted`,
     `lower_decl_type`, `lower_checked_ty`, and add an escape-cooking helper once a fixture needs one.
   - **`array<T>`.** Needs its own element-layout decision first (this is the one still-open piece with a
     real design tradeoff — contiguous vs. hashmap-backed storage, COW-on-write semantics per ADR 0004 —
     work out the tradeoff explicitly before implementing, per CLAUDE.md's priority ordering).
   - **`Ty::Object` refcounting.** Still zero retain/release operations for an object reference — the
     `bind_local`/`lower_call_args`/`release_all_locals`/`lower_expr_stmt`/`lower_reassignment` insertion
     points already extended for `Ty::Str` are expected to extend to it directly (just flip
     `Ty::is_refcounted` to include `Ty::Object` and re-run the existing test suite to see what breaks),
     once there's an actual allocation/field-layout story to attach it to.
   - **Qualified string/bytes types (`tainted`, `secret`, and their combination).** `lower_checked_ty` only
     has an arm for the plain `CheckedTy::String`; the four qualified variants
     (`TaintedString`/`SecretString`/`SecretTaintedString`, and their `Bytes` counterparts once `bytes`
     lands) still panic. These likely want to wait for ADR 0024 §4/0033's stdlib-dependent sinks anyway
     (M7/M8), since a qualifier with nothing to launder against isn't very actionable yet.
5. **Runtime-helper calls are landed** (`ir::InstKind::HelperCall`/`ir::Helper`), but only for `.`'s
   scalar-to-`string` conversion. Two more named uses remain, both blocked on something other than the
   `HelperCall` shape itself now:
   - **A `mixed`/union operand** — needs a `Ty::Mixed`-shaped IR representation first; none exists yet, so
     there's nothing for a helper to dispatch on. Adding one is its own small design question (how a
     `mixed` value's runtime type tag is represented) before any helper call can use it.
   - **ADR 0035's truthy conversion** for a non-`bool` `if`/`while` condition — PHP's truthy table differs
     by source type (`0`/`0.0`/`""`/`"0"`/an empty array/`null` are falsy, everything else truthy), and
     most of those source types (an array, a nullable value) have no IR representation to convert *from*
     yet either. A scalar-only truthy helper (`int`/`uint`/`float`/`string`/`bool` — reusing `Ty::Str` for
     the `""`/`"0"` case) could land now as a partial slice if a fixture wants it; the array/`null` cases
     wait on item 4's `array<T>` and a nullable-type representation, neither of which exist yet.
   - Both are expected to add new `Helper` variants to the same enum, not a second call-shaped instruction.
   - The one remaining `.`-concatenation gap — a `Stringable`-object operand — is *not* primarily a
     `HelperCall` gap any more: it needs `.` to synthesize a resolved `toString()` call, which needs either
     a checker-side change (recording an `ExprInfo::Call`-shaped resolution for a `.` operand, not just a
     call expression) or this crate re-resolving it independently. See this session's design-choices
     writeup in `mwl-ir`'s module docs before picking this up — it's a small but genuine decision, not a
     mechanical extension.
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
