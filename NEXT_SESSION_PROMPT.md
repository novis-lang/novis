# Next session prompt

Continue MWL. M1 (front end) is done. M2 (HIR/types/IR) is close to done — run `python .claude/brief.py`
first, then read `docs/implementation-plan.md`'s M2 paragraph for exactly what landed and how (this file
only points at what's next; the plan is the one home for status detail, per CLAUDE.md's "state a fact
once").

**Last session was ADR-only — no code changed.** The user asked how the compiled-code cache works for a
long-running HTTP server versus a one-off CLI invocation, specifically whether a one-shot `mwl run` has to
recompile every process start. Answer: no — the plan already committed to a content-addressed on-disk cache
(BLAKE3) alongside the in-process cache [ADR 0017](docs/adr/0017-hot-reload-without-restart.md) covers, and
M6 already named "integrity verification" and "refusal to use a world-writable cache directory" as
requirements — but nothing had ever specified the file format, the read/write mechanics, or the eviction
policy. That gap is now closed: **[ADR 0042](docs/adr/0042-on-disk-artifact-cache-format.md)** decides it.
Headline: one immutable file per compiled unit, addressed by `BLAKE3(source ‖ target triple ‖ CPU features
‖ compiler version hash)` — folding the environment into the *address* so a wrong-environment artifact is a
plain miss, never an open-then-reject. A reader `mmap`s read-only, hashes the mapped bytes, and only then
`mprotect`s to executable (W^X, extended one step earlier). A writer compiles to a temp file, `fsync`s it,
and does one atomic rename — no lock file anywhere. Eviction rides the already-expensive cold-compile path
at a small probability (PHP's own `session.gc_probability`/`gc_divisor` shape), so a warm hit never pays for
it. One point stated explicitly rather than left implied: the payload checksum defends against corruption,
never against a hostile co-resident writer — that threat is closed only by the world-writable/wrong-owner
directory refusal, a permission check, not a hash. `docs/adr/README.md`, `CLAUDE.md`'s "Where to look" table
and ground-rules list, and `docs/implementation-plan.md`'s Code-cache row and M6 paragraph were all updated
to point at it, per the ADR-README's own "touch exactly these" checklist.

**Separately, uncommitted M2 work from before this session was found sitting in the working tree and got
folded into the same commit** (built, tested, clippy- and fmt-clean before committing): gap item 8,
string-literal cooking completeness, is now **done for every double-quoted-sourced case** — a numeric
escape (`\xHH` hex, `\NNN` octal, `\u{...}` Unicode) cooks to the byte/codepoint it names, and a
non-heredoc `ExprKind::Interpolated` lowers to the same `InstKind::Concat` chain a written-out `.`
expression already produces, sharing one escape-cooking routine (`mwl_types::string_lit::cook_double_quoted_text`,
a new `mwl-types` module) between the checker and `mwl-ir` so the two can never silently disagree on what
an escape means. `mwl-ir`'s own known-gaps doc (crate-level, in `lib.rs`) was updated in place to reflect
this — read it there for the details, per CLAUDE.md's "per-file known-gap detail belongs in the crate's own
module docs, not the plan." **Still open, still panicking naming the case:** a heredoc/nowdoc-sourced
`ExprKind::Str`/`ExprKind::Interpolated` — no flexible-heredoc indentation-stripping story exists yet. The
rest of the M2 gap list below is unchanged from before — pick up from here:

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
8. ~~String-literal cooking completeness (numeric escapes, non-heredoc interpolation).~~ **Done**, this
   session. **Remaining, and independent:** a heredoc/nowdoc-sourced `ExprKind::Str`/`ExprKind::Interpolated`
   — needs PHP's flexible-heredoc indentation-stripping rule designed first, not just wired up.

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
