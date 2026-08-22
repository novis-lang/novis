# Next session prompt

Continue MWL. M1 (front end) is done. M2 (HIR/types/IR) is close to done — run `python .claude/brief.py`
first, then read `docs/implementation-plan.md`'s M2 paragraph for exactly what landed and how (this file
only points at what's next; the plan is the one home for status detail, per CLAUDE.md's "state a fact
once").

**Last session closed out the remaining piece of known-gap item 7: integer-literal magnitude
range-checking (ADR 0007 § 4).** The gap note going in named two sub-cases, "negative-into-`uint`" and
"too-large-for-either." Both turned out to already be exactly specified by ADR 0007 § 4's own text — "An
integer literal that does not fit `int` is legal only where a `uint` is expected, and is otherwise a
diagnostic saying exactly that" — so this was a mechanical implementation of an already-decided rule, not a
new design call. Investigating "negative-into-`uint`" first showed it needed **no new check at all**:
`mwl_types::expr::infer`'s `ExprKind::Unary` arm already checks its inner expression with `expected: None`
(not the outer target type), so `-5` assigned into a `uint` local already types the inner literal as plain
`int` and reports the ordinary `int`-vs-`uint` `E_TYPE_MISMATCH` `is_assignable` gives for any other
mismatched pair — nothing magnitude-specific was missing there. What *was* genuinely unimplemented:
"too-large-for-either" — a literal whose digits alone overflow `i64`/`u64`. `mwl-ir`'s own lowering already
silently relied on this never happening (a raw `unwrap_or_else(|| panic!(...))` on the `from_str_radix`
call), so an out-of-range literal would have surfaced as an ugly internal panic instead of a compiler
diagnostic.

**What landed, concretely:**

- `mwl_diagnostics::code::E_INT_LITERAL_OUT_OF_RANGE` (`E0429`, `crates/mwl-diagnostics/src/lib.rs`) — a new
  diagnostic code in the E04xx (types) range, documented with the exact ADR 0007 § 4 rule it enforces.
- `mwl_types::expr::infer`'s `ExprKind::Int` arm (`crates/mwl-types/src/expr.rs`) now parses the literal's
  own digits (via a new private `int_literal_digits` helper, cooking all four bases `mwl-syntax`'s lexer
  accepts — decimal/`0x`/`0o`/`0b` — and stripping `_` separators, a deliberate duplicate of
  `mwl_ir::lower::int_literal_digits` since this crate has no dependency on `mwl-ir`, which depends on it
  the other way) as `u64`, then applies ADR 0007 § 4's exact rule: fits `i64` → `int` or `uint` per
  `expected` as before; doesn't fit `i64` but fits `u64` → `uint` only if `uint` is expected, else
  `E_INT_LITERAL_OUT_OF_RANGE`; doesn't even fit `u64` → `E_INT_LITERAL_OUT_OF_RANGE` regardless of
  `expected`. Each error path still returns a best-effort type (matching every other diagnostic in this
  checker, which reports and keeps going rather than aborting the walk).
- `crates/mwl-ir/src/lower.rs`'s `ExprKind::Int` lowering arm and its `int_literal_digits` helper's doc
  comments are updated to say the magnitude check now happens in `mwl_types` before lowering ever runs, so
  the `unwrap_or_else` panics there are unreachable input under this crate's existing "trusts a prior clean
  `check_program` run" contract — the same defensive-invariant shape `Env::get`'s undeclared-local panic
  already has, not a new kind of gap.
- One deliberately-named, un-fixed asymmetry, documented in `mwl-ir`'s crate docs rather than silently
  left implicit: a bare literal `9223372036854775808` (one past `i64::MAX`) immediately negated is reported
  as "too large for `int`," even though `-9223372036854775808` is `i64::MIN`, a perfectly representable
  value — PHP's own lexer special-cases exactly this shape, but ADR 0007 § 4's text doesn't ask for it, and
  recognizing `ExprKind::Unary { op: Neg, expr: Int(_) }` as a signed literal rather than an unsigned one
  negated would be a second, narrower rule this session didn't decide to add on its own. `as int` is *not*
  a workaround either (confirmed, not assumed): `ExprKind::Conversion` also checks its inner expression with
  `expected: None`, so `9223372036854775808 as int` hits the identical diagnostic. Left as a real, narrow
  MWL/PHP divergence for whoever picks up `Core`'s integer-limit constants to note or revisit.

**Six new tests**, all in `crates/mwl-types/src/check.rs`'s `tests` module, next to the existing
`int`/`uint` literal tests — `mwl-types` is now at 170 tests: a literal too large for `int` but not `uint`
is fine into a `uint` target and diagnosed into an `int` target and with no expected type at all; a literal
too large even for `uint` is diagnosed regardless of target; the same check applies through a hex literal
(`0x1_0000_0000_0000_0000`, one bit past 64); and a negative literal into a `uint` target gets the ordinary
`E_TYPE_MISMATCH`, explicitly *not* `E_INT_LITERAL_OUT_OF_RANGE`. `mwl-ir` needed no new tests — its own
62 stay unchanged, since the crate's behavior for in-range input is unchanged and out-of-range input is now
unreachable there by construction.

`cargo build`/`test`/`clippy --all-targets -- -D warnings`/`fmt --check` all clean across the whole
workspace. `crates/mwl-ir/src/lib.rs`'s known-gaps list and `docs/implementation-plan.md`'s M2 paragraph
were both updated in place (not appended) — note that M2 paragraph was *already* over `.claude/brief.py`'s
4000-byte budget before this session; the edit here was one short in-place addition, not a rewrite, so it
grew only slightly more. Not fixed here — `DOC_CLEANUP_PROMPT.md`'s trim pass is the user-run remedy for
that, unrelated to this session's own scope.

**Known gaps, all named in `mwl-ir`'s own module docs — pick up widening from here, in roughly this
order** (each is its own reasonably-sized slice; don't try all of them in one session):

1. ~~Control flow (`if`/`while`).~~ **Done.**
2. ~~Safepoints.~~ **Done** (reserved shape only). Revisit once M3's codegen exists.
3. ~~`new`/a static call, an instance method call, a compile-time-known property access, and array-element
   access through a known `int`/`uint`/`string` key.~~ **Done.** What's left of this shape:
   - **`$a[]`/`$a[] = expr;` (PHP's append syntax).** Needs a "next available integer key" counter this
     crate has no representation for yet — genuinely more than mechanical, since it means tracking (or
     re-deriving) an array's own highest-inserted-integer-key state at lowering time, not just reading one
     back.
   - **A `float`/`bool`/`null` array-subscript key.** `ADR 0007 § 5` rejects these outright as a key
     source type, but `mwl_types::expr::check_expr`'s `Index` arm doesn't enforce that yet — `mwl-ir`
     panics naming the case in `lower_array_key` rather than guessing at a conversion PHP itself doesn't
     define. Fixing this properly means a `mwl_types` checker-side diagnostic first (same shape as item 4's
     array-literal explicit-key gap below), not an `mwl-ir` change.
   - **Array-element access through a `mixed`-erased base.** No `ExprInfo::Index` entry exists for that
     case (mirrors `ExprInfo::Property`'s shape/`object`-erasure gap), so lowering panics naming it — but
     this is currently *unreachable* without first hitting the unrelated, already-documented "no
     `Ty::Mixed` representation" gap (item 5 below), since `mixed` isn't a lowerable declared type or
     resolved-call return type in this crate yet either. No dedicated fixture for it this session for that
     reason; add one once `Ty::Mixed` lands if it's still worth a dedicated proof at that point.
4. **Non-scalar *data* values and refcount operations — `string`/`bytes`/`array<T>` locals, the
   call/return/property-read-and-write boundary, `.` concatenation, and array-element read/write are all
   landed; two pieces remain, both mechanical:**
   - **An explicit `key =>`, a `...spread`, or a `&value` array-literal element.** Unchanged from before:
     `mwl_types::expr::check_array_literal` itself has no key-normalization/rejection logic yet (ADR 0007
     § 5's int/uint-to-decimal-string normalization, float/bool/null rejection), so lowering an explicit
     key would mean guessing at a runtime conversion this crate can't yet synthesize. Landing this probably
     wants a checker-side fix first (`mwl-types`), not just an `mwl-ir` change — and, now that
     `lower_array_key`'s int/uint-to-string conversion exists, is likely to reuse it once the checker side
     is ready.
   - **`Ty::Object` refcounting.** Still zero retain/release operations for an object reference — the
     `bind_local`/`lower_call_args`/`release_all_locals`/`lower_expr_stmt`/`lower_reassignment` insertion
     points already extended for `Ty::Str`/`Ty::Bytes`/`Ty::Array` are expected to extend to it directly
     (just flip `Ty::is_refcounted` to include `Ty::Object` and re-run the existing test suite to see what
     breaks), once there's an actual allocation/field-layout story to attach it to — check whether one
     exists yet before assuming it's ready.
   - **Qualified string/bytes types (`tainted`, `secret`, and their combination).** `lower_checked_ty` only
     has arms for the plain `CheckedTy::String`/`CheckedTy::Bytes`/`CheckedTy::Array`; the six qualified
     string/bytes variants (`TaintedString`/`SecretString`/`SecretTaintedString` and their `Bytes`
     counterparts) still panic. These likely want to wait for ADR 0024 §4/0033's stdlib-dependent sinks
     anyway (M7/M8), since a qualifier with nothing to launder against isn't very actionable yet.
5. **Runtime-helper calls are landed** (`ir::InstKind::HelperCall`/`ir::Helper`), now used for both `.`'s
   scalar-to-`string` conversion and an `int`/`uint` array-subscript's key normalization. Two more named
   uses remain, both blocked on something other than the `HelperCall` shape itself now:
   - **A `mixed`/union operand** — needs a `Ty::Mixed`-shaped IR representation first; none exists yet, so
     there's nothing for a helper to dispatch on. Adding one is its own small design question (how a
     `mixed` value's runtime type tag is represented) before any helper call can use it. (This is also
     what item 3's "mixed-erased array base" gap above is blocked on.) You are authorized to design this
     yourself and proceed if you reach it — no need to stop and ask.
   - **ADR 0035's truthy conversion** for a non-`bool` `if`/`while` condition — PHP's truthy table differs
     by source type (`0`/`0.0`/`""`/`"0"`/an empty array/`null` are falsy, everything else truthy). Both
     `Ty::Array` and array-element access exist now, giving the array-emptiness case a representation to
     convert *from* — but a nullable-type representation still doesn't, so the `null` case still waits. A
     scalar-plus-array-only truthy helper could land now as a partial slice if a fixture wants it.
   - Both are expected to add new `Helper` variants to the same enum, not a second call-shaped instruction.
   - The one remaining `.`-concatenation gap — a `Stringable`-object operand — is *not* primarily a
     `HelperCall` gap any more: it needs `.` to synthesize a resolved `toString()` call, which needs either
     a checker-side change (recording an `ExprInfo::Call`-shaped resolution for a `.` operand, not just a
     call expression) or this crate re-resolving it independently. See the "runtime-helper calls" session's
     design-choices writeup in `mwl-ir`'s module docs before picking this up — it's a small but genuine
     decision, not a mechanical extension.
6. **Virtual dispatch** — every call/access lowered so far (`new`'s constructor, a static call, an
   instance call, a property access, an array-element access) has its receiver's *static* type equal to
   its *runtime* class — none has gone through an interface-typed or overridden-method/property receiver
   yet, which is the first place the two could actually differ. Whether a real vtable/interface-dispatch
   lookup belongs at this IR level (as opposed to purely at codegen, once M3 exists) is an open,
   architectural question — flag it rather than guessing if you reach it before M3 starts.
7. ~~`var` locals (ADR 0037), multi-base integer-literal cooking (hex/octal/binary), and integer-literal
   magnitude range-checking.~~ **Done**, all three.
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

**Separately, whenever M3 finishes and M4 is underway:** keep [ADR 0040](docs/adr/0040-vscode-deep-tooling-and-resilient-parsing.md)
in mind as M4 approaches its own "usable CLI language" exit criterion — M4B (minimal `mwl-lsp` +
`editors/vscode`, plus `mwl-syntax`'s new resilient-parse mode) starts right after, per the plan.

**Separately, whenever M5 (concurrency and script isolates) is underway:** give each of the three
spawn-construct runtime routines its `spawn`-kind trace hook, and whenever the mark-sweep cycle collector's
run routine is built, give it its `gc`-kind hook too — both per [ADR 0041](docs/adr/0041-timeline-export-and-gc-spawn-trace-events.md),
both instrumentation-only inside those already-rare routines, no change to the safepoint poll itself.
