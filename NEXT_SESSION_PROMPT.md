# Next session prompt

Continue MWL. M1 (front end) is done. M2 (HIR/types/IR) is close to done — run `python .claude/brief.py`
first, then read `docs/implementation-plan.md`'s M2 paragraph for exactly what landed and how (this file
only points at what's next; the plan is the one home for status detail, per CLAUDE.md's "state a fact
once").

**Last session landed the first bullet of item 4 below — `string` crossing a call-argument/resolved-return/
compile-time-known-property-read boundary.** No new IR shape was needed; this widened `lower_checked_ty`
and generalized the existing local retain/release policy to two more sites:

- `lower::lower_checked_ty` (a *resolved* call's/`new`'s parameter/return type, or a resolved property's
  field type) gained a `CheckedTy::String => Ty::Str` arm. So now: a call/`new` with a `string` argument, a
  call whose declared return type is `string`, and a `string`-typed property *read* (`$obj->prop`, via
  `InstKind::FieldGet`) all lower. `CheckedTy::TaintedString`/`SecretString`/`SecretTaintedString` still
  have no arm and still panic — ADR 0024/0033's qualifiers need their own laundering/sink story before a
  qualified value can flow through an IR value at all, deliberately out of scope. A property *write*
  (`$obj->prop = expr;`) is still entirely unsupported for **any** field type, not just `string` —
  `Lowering::lower_reassignment` only accepts a plain-local assignment target; this session didn't touch
  that.
- A new `lower::is_aliasing_read(kind: &ExprKind) -> bool` helper generalizes the "was this value copied out
  of storage someone else still owns" judgment `Lowering::bind_local` already had for a bare
  `ExprKind::Variable` read, now also covering `ExprKind::PropertyAccess` (a property read borrows the
  object's own field storage, exactly the same as a local borrows its own slot). Three call sites now share
  it:
  - `Lowering::bind_local` — unchanged behavior for a variable, now *also* retains a property read bound to
    a new local (e.g. `var $s = $obj->name;`).
  - `Lowering::lower_call_args` — retains an aliasing argument (a bare variable or a property read) right
    before the call. The callee's own parameter is bound into its `Env` exactly like a local (see
    `lower_method`) and released at its own exit by `release_all_locals` — so this caller-side retain and
    the callee's own eventual release are a symmetric pair, exactly mirroring what a local's own
    declare/drop already does, just spanning a call frame instead of one function. A fresh literal, `new`,
    or another call's own result passed as an argument needs no retain — it already has exactly one owner,
    which just transfers into the callee's slot.
  - `StmtKind::Return`'s own arm — retains an aliasing-but-not-bare-variable return expression (in practice,
    today, only a property read) explicitly, since unlike a bare `$name` return there is no local slot for
    `release_all_locals`'s existing exclusion mechanism (`except`) to skip. A bare-variable return is
    unchanged (still excluded, not retained — see `except`'s own logic); a fresh producer (literal, `new`,
    call result) still needs nothing, per the existing rule.
- Four new `insta` snapshot tests, all under `crates/mwl-ir/src/lower.rs`'s `tests` module:
  `passing_a_string_local_as_a_call_argument_retains_it` (retain right before the call, release at the
  caller's own exit sweep — the call's own return type is `int`, isolating the argument-side behavior from
  the return-side one), `binding_a_string_property_read_to_a_local_retains_it` (`var $s = $obj->name;` —
  retain on bind, release at exit — the retain/release pair sits right next to each other in the printed
  IR, which is correct: the local is never read again after declaring it), `returning_a_string_property_read_retains_it`
  (`return $obj->name;` — one retain, zero release, leaving the caller with exactly one owned reference),
  and `returning_a_string_returning_calls_result_needs_no_retain` (`return self::make();` where `make`
  returns `string` — the fresh-producer baseline: zero retain, zero release). Read the actual snapshot
  output before trusting the design holds: it does (verified this session, all four checked by hand) —
  `mwl-ir` is now at 29 tests (`mwl-types` unchanged at 162). `cargo build`/`test`/
  `clippy --all-targets -- -D warnings`/`fmt --check` all clean across the whole workspace.
- **Noticed along the way, documented as a new known gap, not fixed this session:** a bare call used purely
  as a statement (`doSomething();` with no assignment — the ordinary way to invoke a `void`-returning
  method) still isn't lowered at all. `StmtKind::Expr`'s arm (`Lowering::lower_reassignment`) only accepts
  an `ExprKind::Assign` expression statement and panics on anything else. Every call fixture landed so far
  (this session's included) routes a call through a `var`/typed local binding or a `return` instead — this
  gap simply hadn't been exercised yet. Fixing it needs `lower_stmt`'s `StmtKind::Expr` arm to also accept
  a bare `MethodCall`/`StaticCall`/`New` expression, lowering it purely for its side effect and immediately
  releasing any `Ty::is_refcounted` result right there (nothing else in the function will ever bind or
  return it, so it can't reuse `bind_local`'s policy directly — it needs its own one-line "lower it, then
  release if refcounted"). Small and independent, land whenever convenient.

**Known gaps, all named in `mwl-ir`'s own module docs — pick up widening from here, in roughly this
order** (each is its own reasonably-sized slice; don't try all of them in one session):

1. ~~Control flow (`if`/`while`).~~ **Done.**
2. ~~Safepoints.~~ **Done** (reserved shape only). Revisit once M3's codegen exists.
3. ~~`new`/a static call, an instance method call, and a compile-time-known property access (including the
   shape/`object`-erasure panic case).~~ **Done.** One shape remains:
   - **Array access (`$arr[$i]`)** is still unsupported; lowering panics naming the expression. No
     `array<T>` *data* representation exists yet either (see item 4 below), so this may naturally land
     together with that slice rather than alone.
4. **Non-scalar *data* values and refcount operations — `string` locals and now the call/return/
   property-read boundary are both landed; four pieces remain:**
   - **A bare call used purely as a statement.** See the "noticed along the way" bullet above — small,
     independent, and the most natural next pickup since it's already scoped in detail.
   - **A `string`-typed property *write*.** `$obj->prop = expr;` panics for any field type today, not just
     `string` — `Lowering::lower_reassignment` only accepts a plain-local assignment target. Landing this
     needs a new `InstKind::FieldSet` (mirroring `FieldGet`) plus a retain of the new value (if it's an
     aliasing read — reuse `is_aliasing_read`) and a release of whatever the field previously held (a
     `FieldGet`-then-release, since nothing tracks a field's prior value beyond re-reading it — unlike a
     local, there is no `Env` entry to consult before the overwrite).
   - **String concatenation (`.`).** `ExprKind::Binary`'s arm only accepts the scalar
     arithmetic/equality/ordering operators; `BinaryOp::Concat` on two `string` operands panics there.
     Lowering this needs a runtime-helper call (see item 5) or a dedicated `InstKind`, since concatenation
     allocates a new buffer rather than being a native scalar instruction. The result is a fresh value (one
     natural owner, same as a literal) — `is_aliasing_read` should *not* need to grow a `Binary` arm for it.
   - **`bytes`.** Expected to be a mechanical repeat of `Ty::Str`'s shape (same refcounted-heap-value
     treatment, different content, same `bind_local`/`lower_call_args`/`release_all_locals` insertion
     points, same `is_aliasing_read` reuse) — extend `Ty::is_refcounted`, `lower_decl_type`,
     `lower_checked_ty`, and add an escape-cooking helper once a fixture needs one.
   - **`array<T>`.** Needs its own element-layout decision first (this is the one still-open piece with a
     real design tradeoff — contiguous vs. hashmap-backed storage, COW-on-write semantics per ADR 0004 —
     work out the tradeoff explicitly before implementing, per CLAUDE.md's priority ordering).
   - **`Ty::Object` refcounting.** Still zero retain/release operations for an object reference — the
     `bind_local`/`lower_call_args`/`release_all_locals` insertion points this session extended for
     `Ty::Str` are expected to extend to it directly (just flip `Ty::is_refcounted` to include `Ty::Object`
     and re-run the existing test suite to see what breaks), once there's an actual allocation/field-layout
     story to attach it to.
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
