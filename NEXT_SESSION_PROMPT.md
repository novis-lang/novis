# Next session prompt

Continue MWL. M1 (front end) is done. M2 (HIR/types/IR) is close to done — run `python .claude/brief.py`
first, then read `docs/implementation-plan.md`'s M2 paragraph for exactly what landed and how (this file
only points at what's next; the plan is the one home for status detail, per CLAUDE.md's "state a fact
once").

**Last session landed the first bullet of item 4 below — a bare call/`new` used purely as its own
statement.** This was flagged as a "noticed along the way" gap two sessions ago and was already scoped in
detail; no new IR shape was needed:

- `Lowering::lower_stmt`'s `StmtKind::Expr` arm now dispatches through a new `Lowering::lower_expr_stmt`
  instead of calling `lower_reassignment` directly. It matches on the expression: a plain
  `ExprKind::Assign { op: AssignOp::Assign, by_ref: false, .. }` still routes to `lower_reassignment`
  (unchanged behavior — that function's own `let ... else` on the assignment shape is now `unreachable!()`
  rather than a panic, since `lower_expr_stmt` already guarantees it matched); a bare
  `ExprKind::MethodCall`/`ExprKind::StaticCall`/`ExprKind::New` — i.e. `doSomething();`, `self::helper();`,
  `new Foo();` with no assignment at all, the ordinary way to invoke a `void`-returning method or run a
  constructor purely for a side effect — now lowers through the ordinary `lower_expr` path and immediately
  releases the produced value with `emit_release` when `Ty::is_refcounted` is true. Nothing else in the
  function will ever bind or return that value, so this couldn't reuse `bind_local`'s declare/reassign
  policy or `release_all_locals`'s exit sweep — it's its own one-line "lower it, then release if
  refcounted" right after the call/`new` instruction. A `void`-returning call needs no release (nothing to
  release); a `new` needs none either today, since `Ty::Object` still isn't refcounted at all (a
  pre-existing, separate gap — see item 4's `Ty::Object` bullet below). Anything else reaching this arm
  (e.g. a compound-assign or by-ref assignment) still panics naming the shape, same as before.
- Four new `insta` snapshot tests, all under `crates/mwl-ir/src/lower.rs`'s `tests` module:
  `a_bare_void_call_used_as_a_statement_lowers_with_no_release` (`self::helper();` where `helper` returns
  `void` — call, then straight to `return`, no release inserted), `a_bare_call_used_as_a_statement_releases_a_discarded_string_result`
  (`self::make();` where `make` returns `string` — exactly one `release` right after the call, no retain,
  since a call's own result is a fresh producer per `is_aliasing_read`), `a_bare_new_used_as_a_statement_lowers_with_no_release`
  (`new Foo();` — constructs and immediately discards, no release since `Ty::Object` isn't refcounted yet),
  and `a_bare_instance_call_used_as_a_statement_lowers_too` (`$obj->greet();` through a local receiver — makes
  sure the dispatch isn't accidentally `StaticCall`-only). Read the actual snapshot output before trusting
  the design holds: it does (verified this session, all four checked by hand) — `mwl-ir` is now at 33 tests
  (`mwl-types` unchanged at 162). `cargo build`/`test`/`clippy --all-targets -- -D warnings`/`fmt --check`
  all clean across the whole workspace.

**Known gaps, all named in `mwl-ir`'s own module docs — pick up widening from here, in roughly this
order** (each is its own reasonably-sized slice; don't try all of them in one session):

1. ~~Control flow (`if`/`while`).~~ **Done.**
2. ~~Safepoints.~~ **Done** (reserved shape only). Revisit once M3's codegen exists.
3. ~~`new`/a static call, an instance method call, and a compile-time-known property access (including the
   shape/`object`-erasure panic case).~~ **Done.** One shape remains:
   - **Array access (`$arr[$i]`)** is still unsupported; lowering panics naming the expression. No
     `array<T>` *data* representation exists yet either (see item 4 below), so this may naturally land
     together with that slice rather than alone.
4. **Non-scalar *data* values and refcount operations — `string` locals, the call/return/property-read
   boundary, and a bare call/`new` statement are all landed; four pieces remain:**
   - **A `string`-typed property *write*.** `$obj->prop = expr;` panics for any field type today, not just
     `string` — `Lowering::lower_reassignment` only accepts a plain-local assignment target (its `target`
     check, distinct from the assignment-shape check `lower_expr_stmt` now does before calling it). Landing
     this needs a new `InstKind::FieldSet` (mirroring `FieldGet`) plus a retain of the new value (if it's an
     aliasing read — reuse `is_aliasing_read`) and a release of whatever the field previously held (a
     `FieldGet`-then-release, since nothing tracks a field's prior value beyond re-reading it — unlike a
     local, there is no `Env` entry to consult before the overwrite). This is the most natural next pickup:
     small, independent, and already scoped in detail, same as the bare-call-statement gap was.
   - **String concatenation (`.`).** `ExprKind::Binary`'s arm only accepts the scalar
     arithmetic/equality/ordering operators; `BinaryOp::Concat` on two `string` operands panics there.
     Lowering this needs a runtime-helper call (see item 5) or a dedicated `InstKind`, since concatenation
     allocates a new buffer rather than being a native scalar instruction. The result is a fresh value (one
     natural owner, same as a literal) — `is_aliasing_read` should *not* need to grow a `Binary` arm for it.
   - **`bytes`.** Expected to be a mechanical repeat of `Ty::Str`'s shape (same refcounted-heap-value
     treatment, different content, same `bind_local`/`lower_call_args`/`release_all_locals`/`lower_expr_stmt`
     insertion points, same `is_aliasing_read` reuse) — extend `Ty::is_refcounted`, `lower_decl_type`,
     `lower_checked_ty`, and add an escape-cooking helper once a fixture needs one.
   - **`array<T>`.** Needs its own element-layout decision first (this is the one still-open piece with a
     real design tradeoff — contiguous vs. hashmap-backed storage, COW-on-write semantics per ADR 0004 —
     work out the tradeoff explicitly before implementing, per CLAUDE.md's priority ordering).
   - **`Ty::Object` refcounting.** Still zero retain/release operations for an object reference — the
     `bind_local`/`lower_call_args`/`release_all_locals`/`lower_expr_stmt` insertion points already extended
     for `Ty::Str` (most recently to `lower_expr_stmt`'s own discard-and-release path this session) are
     expected to extend to it directly (just flip `Ty::is_refcounted` to include `Ty::Object` and re-run the
     existing test suite to see what breaks), once there's an actual allocation/field-layout story to attach
     it to.
   - **Qualified string/bytes types (`tainted`, `secret`, and their combination).** `lower_checked_ty` only
     has an arm for the plain `CheckedTy::String`; the four qualified variants
     (`TaintedString`/`SecretString`/`SecretTaintedString`, and their `Bytes` counterparts once `bytes`
     lands) still panic. These likely want to wait for ADR 0024 §4/0033's stdlib-dependent sinks anyway
     (M7/M8), since a qualifier with nothing to launder against isn't very actionable yet.
5. **Runtime-helper calls** — the milestone's fourth named ingredient, needed for a `mixed`/union operand,
   for ADR 0035's truthy conversion on a non-`bool` `if`/`while` condition, and now also for string
   concatenation (see item 4).
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
   whenever convenient — likely wants to happen alongside string concatenation (item 4) since
   interpolation desugars to concatenation-like codegen anyway.

Once control flow, calls, and property/array access all lower, M2's own *Verify* bullet ("IR snapshot
tests; no program in the corpus produces an `Unknown` type") is worth revisiting for a real corpus-driven
snapshot suite, not just hand-written fixtures — at that point M2 as a whole should be closeable and M3
(baseline Cranelift backend, `Hello World`) can start.

Also still open from before (independent, low priority, unrelated to `mwl-ir`): a `set`-hooked property is
exempted from ADR 0022's constructor check entirely rather than verified against the hook's body; the
identical question now also applies to whether a `lateinit` + hooked property should discharge on the
hook's first commit (ADR 0038's own *Revisiting* names this, deferred to `docs/spec/`).

---

Pick the best-scoped next item and land it end to end (design decision if needed → implementation → tests
→ verification → docs → commits). If you pick up `array<T>`'s element-layout decision (flagged above as a
real tradeoff), work out the design question explicitly (1-2 paragraphs weighing the options against
CLAUDE.md's priority ordering: security > correctness > latency > simplicity > memory) before writing
code, and stop to report back if it's genuinely a decision only the user should make rather than a
mechanical extension of an existing pattern. Good luck.
