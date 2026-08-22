# Next session prompt

Continue MWL. M1 (front end) is done. M2 (HIR/types/IR) is close to done — run `python .claude/brief.py`
first, then read `docs/implementation-plan.md`'s M2 paragraph for exactly what landed and how (this file
only points at what's next; the plan is the one home for status detail, per CLAUDE.md's "state a fact
once").

**Last session closed gap item 8 in full.** Heredoc/nowdoc-sourced string literals now cook and lower —
PHP 7.3's "flexible heredoc" indentation-stripping rule is implemented in
`mwl_types::string_lit::heredoc_shape`/`dedent_heredoc_run` (extracts the closing marker's own
indentation straight from the literal's whole span — no parser/AST change needed, since the marker's line
is always the text after that span's last newline — then strips it from every body line, per-run for an
interpolated heredoc so a line starting right after an interpolation site is still recognized as a fresh
line). A nowdoc applies no escapes at all afterward, matching PHP; a heredoc runs the same escape grammar
a double-quoted literal does, via a new `cook_double_quoted_text_str` (owned-`&str` sibling of
`cook_double_quoted_text`, needed since dedenting breaks the byte-for-byte span correspondence the
precise-span cooker relies on). Two new diagnostics, `E_HEREDOC_MIXED_INDENT` and
`E_HEREDOC_INSUFFICIENT_INDENT`, cover a malformed marker/body line; checker and `mwl-ir` share the same
`string_lit` routines so they can never disagree on what a literal cooks to. Built, tested (new unit tests
in `mwl_types::string_lit`, new checker fixtures in `mwl_types::check`, new `mwl-ir` snapshot tests),
clippy- and fmt-clean, committed. One named, documented limitation left in place deliberately (not a
panic, not a bug): a body line whose only content is an interpolation expression (no leading text) isn't
checked against the marker's required indentation, since its leading-whitespace run is empty either way —
strict PHP would flag this as insufficiently indented in that one shape, this compiler doesn't yet.

**With that, `mwl-ir`'s known-gap list (its own module docs in `lib.rs`) is down to:**

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
     resolved-call return type in this crate yet either.
4. **Non-scalar *data* values and refcount operations — `string`/`bytes`/`array<T>` locals, the
   call/return/property-read-and-write boundary, `.` concatenation, and array-element read/write are all
   landed; two pieces remain, both mechanical:**
   - **An explicit `key =>`, a `...spread`, or a `&value` array-literal element.** `mwl_types::expr::check_array_literal`
     itself has no key-normalization/rejection logic yet (ADR 0007 § 5's int/uint-to-decimal-string
     normalization, float/bool/null rejection), so lowering an explicit key would mean guessing at a
     runtime conversion this crate can't yet synthesize. Landing this probably wants a checker-side fix
     first (`mwl-types`), not just an `mwl-ir` change — and, now that `lower_array_key`'s int/uint-to-string
     conversion exists, is likely to reuse it once the checker side is ready. **This is next session's best
     pick if you want a well-scoped item excluding the deferred item 6.**
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
5. **Runtime-helper calls are landed** (`ir::InstKind::HelperCall`/`ir::Helper`), now used for `.`'s
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
   architectural question — **skip this one** (per standing user direction, deferred until M3 starts)
   unless it turns out to be the only item left, in which case stop and report that instead of attempting
   it.
7. ~~`var` locals (ADR 0037), multi-base integer-literal cooking (hex/octal/binary), and integer-literal
   magnitude range-checking.~~ **Done**, all three.
8. ~~String-literal cooking completeness (numeric escapes, non-heredoc interpolation, heredoc/nowdoc
   flexible-indentation stripping).~~ **Done**, fully, as of last session.

**Recommended pick for next session:** item 4's array-literal explicit `key =>`/`...spread`/`&value` gap
— it needs a small `mwl_types::expr::check_array_literal` checker-side fix (key normalization/rejection
per ADR 0007 § 5) before the `mwl-ir` lowering side, which can then reuse `lower_array_key`'s existing
int/uint-to-string helper. `Ty::Object` refcounting is the other mechanical option, but check first
whether an allocation/field-layout story exists yet to attach retain/release to — if not, that one isn't
ready regardless of how mechanical the refcount insertion points themselves are.

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
