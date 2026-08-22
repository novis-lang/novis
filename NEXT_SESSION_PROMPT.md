# Next session prompt

Continue MWL. M1 (front end) is done. M2 (HIR/types/IR) is close to done — run `python .claude/brief.py`
first, then read `docs/implementation-plan.md`'s M2 paragraph for exactly what landed and how (this file
only points at what's next; the plan is the one home for status detail, per CLAUDE.md's "state a fact
once").

**Last session landed the first bullet of item 4 below — a `string`-typed property *write* — closing the
read/write asymmetry the property-read slice had left open.** This was already scoped in detail two
sessions ago; one new `InstKind` was needed:

- `crate::ir::InstKind::FieldSet { object, class, field, value }` — mirrors `FieldGet`, but writes rather
  than reads and defines no value (like `Retain`/`Release`/`Safepoint`). `crate::print::print_inst` renders
  it as `field.set v{object}, {class}::{field}, v{value}`.
- `Lowering::lower_reassignment` now matches on the assignment *target* instead of only accepting a plain
  local: `ExprKind::Variable` still binds into `Env` exactly as before; a new `ExprKind::PropertyAccess`
  arm handles `$obj->prop = expr;` (including `$this->prop = expr;`, which needs nothing special — the
  receiver is just another `Env` lookup). It looks up `ExprInfo::Property` from `self.exprs` keyed by
  `target.span` (the `PropertyAccess` expression's own span) — confirmed by reading `mwl_types::expr::
  check_assign`'s general (non-plain-local) arm: it routes the target through the ordinary `check_expr` →
  `check_property_access`, which records the *same* `ExprInfo::Property` entry a read would, so no checker
  changes were needed. Refcounting mirrors `bind_local`'s local-slot policy, adapted to a field with no
  `Env` entry to consult before the overwrite: retain the new value first if it's an aliasing read (reusing
  `is_aliasing_read` unchanged), *then* read the field's previous value back with a `FieldGet` and release
  it — retain before release, same order `bind_local` already uses, so a self-assignment
  (`$obj->prop = $obj->prop;`) never observes a transient zero refcount. Anything else reaching
  `lower_reassignment`'s target match (an array element, a static-property target, ...) still panics naming
  the shape.
- A nullsafe property-assignment target (`$obj?->prop = expr;`) asserts out, same as a nullsafe read/method
  call; a receiver that erased to a shape or plain `object` (ADR 0036 § 4) panics the same way the read side
  already did, since `check_property_access` never records an entry for either.
- Four new `insta` snapshot tests in `crates/mwl-ir/src/lower.rs`'s `tests` module:
  `writing_a_fresh_string_literal_to_a_property_releases_its_previous_value` (a literal RHS — `FieldGet` +
  `release` of the old value, `field.set`, no retain, since a literal is a fresh producer),
  `writing_a_string_local_to_a_property_retains_it_before_releasing_the_old_value` (an aliasing local RHS —
  `retain` first, *then* `FieldGet` + `release` of the old value, then `field.set`, confirming the ordering),
  `writing_through_this_lowers_too` (`$this->name = "new";`, the implicit-receiver path), and
  `writing_through_a_plain_object_receiver_is_still_out_of_scope` (`#[should_panic]`, the erased-receiver
  case). Read the actual snapshot output before trusting the design holds: it does (verified this session,
  all four checked by hand) — `mwl-ir` is now at 37 tests (`mwl-types` unchanged at 162). `cargo build`/
  `test`/`clippy --all-targets -- -D warnings`/`fmt --check` all clean across the whole workspace.

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
   write boundary, and a bare call/`new` statement are all landed; four pieces remain:**
   - **String concatenation (`.`).** `ExprKind::Binary`'s arm only accepts the scalar
     arithmetic/equality/ordering operators; `BinaryOp::Concat` on two `string` operands panics there.
     Lowering this needs a runtime-helper call (see item 5) or a dedicated `InstKind`, since concatenation
     allocates a new buffer rather than being a native scalar instruction. The result is a fresh value (one
     natural owner, same as a literal) — `is_aliasing_read` should *not* need to grow a `Binary` arm for it.
     This is the most natural next pickup: small, independent, and already scoped in detail.
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
