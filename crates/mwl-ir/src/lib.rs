//! MWL's CFG/SSA IR — `docs/implementation-plan.md`'s M2 milestone's last open
//! thread. See `NEXT_SESSION_PROMPT.md` for how this crate grew: the milestone
//! text ("a CFG/SSA IR carrying explicit safepoints, refcount operations and
//! runtime-helper calls, with a stable per-statement/per-edge id" — see
//! [ADR 0018](../../../docs/adr/0018-coverage-tracing-and-profiling-as-safepoint-shaped-probes.md))
//! is milestone-sized on its own, so this first slice deliberately narrows to
//! exactly what the plan's own M2 *Verify* bullet asks for first: "lower a
//! first, narrow slice ... end to end with a snapshot test, before widening."
//!
//! # What this crate lowers so far
//!
//! One method whose body is typed local declarations, a `var $x = expr;`
//! inferred-type declaration (ADR 0037), plain `$x = expr;` reassignment,
//! `return`, nested `{}` blocks, `if`/`while`, `new`, a static method call
//! (`self::method(...)`/`Class::method(...)`), an instance method call
//! (`$obj->method(...)`, including `$this->…`), a compile-time-known
//! property access (`$obj->prop`, including `$this->prop`), and a
//! `string`-typed local/parameter/return value/call-argument/property-field,
//! initialized, reassigned, passed, returned or read from a literal, another
//! local, a compile-time-known property or a resolved call's own result —
//! with refcount retain/release operations around every one of those
//! boundaries — [`lower::lower_method`] is the entry point. No
//! `for`/`switch`/`try`, no `break`/`continue`, no array access, no `bytes`/
//! `array<T>`, no `.` string concatenation (a class/enum value itself also
//! has a representation, [`ty::Ty::Object`], just not a way to refcount one
//! yet). The straight-line subset was deliberately the *first* slice landed
//! (see git history and `docs/implementation-plan.md`'s M2 paragraph) because
//! it was the smallest shape exercising every structural IR piece with no
//! merge point at all; `if`/`while` came next, and are where SSA's actual
//! join/phi question gets answered — see [`lower`]'s own module docs for
//! exactly how. `new`/a static call were the third slice, and the first to
//! need more than the AST alone — see the next section for the dependency
//! that unlocked them. An instance method call is the fourth slice, and the
//! first to need a receiver represented as a real value — see the
//! design-choices section below for the implicit-receiver-parameter shape
//! that unlocked it. A property access is the fifth slice, and reuses that
//! same receiver-as-a-value machinery, only for a field read instead of a
//! call — see [`ir::InstKind::FieldGet`]'s own doc comment for the
//! compile-time-known-field-only shape landed here. `var` locals and
//! multi-base (`0x`/`0o`/`0b`) integer-literal cooking are the sixth slice,
//! closing out two gaps this crate had been carrying since the straight-line
//! slice — neither needed a new IR shape, only reusing `Lowering::lower_expr`'s
//! existing `expected: None` inference path for `var`, and widening a new
//! `int_literal_digits` helper's radix handling for the literal forms.
//! `string` locals and the retain/release IR shape are the seventh slice,
//! and the first non-scalar *data* representation to land at all — see the
//! design-choices section below for the retain/release insertion policy this
//! needed. Widening that same `string` representation across a call
//! argument, a resolved return type and a compile-time-known property field
//! is the eighth slice, and needed no new IR shape at all — only
//! [`lower::lower_checked_ty`] gaining a `String` arm and the existing
//! aliasing-vs-fresh judgment ([`lower::is_aliasing_read`]) extending to a
//! property read and a call argument/return boundary, both described in the
//! design-choices section below. A bare call/`new` used purely as its own
//! statement (`doSomething();`, with no assignment at all — the ordinary way
//! to invoke a `void`-returning method) is the ninth slice: `StmtKind::Expr`
//! now dispatches through [`lower::Lowering::lower_expr_stmt`], which routes
//! a plain reassignment to the existing [`lower::Lowering::lower_reassignment`]
//! and a bare `MethodCall`/`StaticCall`/`New` through the ordinary
//! `lower_expr` path, releasing its result immediately when
//! [`ty::Ty::is_refcounted`] since nothing else will ever bind or return it —
//! no new `InstKind` needed.
//!
//! # Design choices worth knowing before widening this further
//!
//! - **SSA, not a plain CFG.** `docs/implementation-plan.md`'s M2 paragraph
//!   already commits to "a CFG/**SSA** IR" (not left open by this session) —
//!   adopted here rather than reopened, per CLAUDE.md's "mechanical
//!   follow-through of what the plan already committed to" carve-out.
//!   `if`/`while` are each a single, hand-rolled two-predecessor (or
//!   pre-loop/back-edge) merge, not a general dominance-based phi-placement
//!   algorithm — sufficient for any structured `if`/`while` nesting, since
//!   neither ever produces a join point of another shape. `for`/`switch`
//!   will reuse the same two building blocks (`Lowering::merge_envs` for a
//!   fixed set of incoming edges known up front, the seed-then-patch phi
//!   dance in `Lowering::lower_while` for a join whose back edge isn't known
//!   until its body is lowered) rather than needing a new algorithm.
//! - **IR types are representation-level, not the checker's types.** See
//!   [`ty`]'s own module docs for why [`ty::Ty`] is a small, flat lattice
//!   distinct from `mwl_types::ty::Ty` rather than a reuse of it.
//! - **This crate depends on `mwl-types`, but only for its typed-expression
//!   table — never for `mwl-hir`'s class graph/signature tables directly.**
//!   Every *declared* type (a parameter's, a local's, a method's return type)
//!   is still read straight off the `mwl-syntax` AST via `lower::lower_decl_type`
//!   (renamed from the earlier slice's `lower_scalar_type`, since it now
//!   covers one non-scalar case too), exactly as before: ADR 0007 § 1 already
//!   requires it to be spelled out there in full, so no name resolution is
//!   needed to answer "what type is this" — a plain class-name atom now
//!   erases to [`ty::Ty::Object`] the same way a scalar atom erases to its own
//!   `Ty` variant, needing no more resolution than a scalar did. What *did*
//!   need a new dependency is a call's or `new`'s *resolved target* — which
//!   class actually declares the callee, its parameter/return types — since
//!   that is genuinely absent from the AST (a call site only spells the
//!   method name, not which class in an inheritance chain declares it).
//!   The two options weighed for that were (a) this crate depending on
//!   `mwl-types` and duplicating/re-running its class-hierarchy resolution,
//!   or (b) `mwl-types` publishing a persisted result this crate reads back.
//!   (b) was chosen: `mwl_types::expr_table::ExprTypeTable` is a narrow,
//!   purpose-built table — one `ExprInfo::Call`/`ExprInfo::New` entry per
//!   resolved call/`new`, keyed by the expression's own source span (see that
//!   module's own docs for why a span, not an id, is the lookup key across
//!   this crate boundary) — that `mwl_types::check_program` populates once and
//!   [`lower::lower_method`] reads afterward, via two new parameters
//!   (`exprs`/`checked_types`). This keeps the coupling narrow: this crate
//!   still never depends on `mwl-hir`, `mwl_types::signatures`, or
//!   `mwl_types::ClassGraph` — only on the one table and the type interner
//!   needed to translate a recorded `TypeId` into this crate's own `Ty` (see
//!   `lower::lower_checked_ty`). [`lower::lower_method`] still deliberately
//!   **trusts** that its input already passed `mwl_types::check_program` —
//!   with the very same `exprs`/`checked_types` handed to it — and panics
//!   (naming the unsupported shape) rather than diagnosing when handed
//!   something outside this slice's scope, or when a table lookup comes back
//!   empty for an expression that should have one.
//! - **`$this`/a receiver is an implicit first parameter, not a special-cased
//!   field.** Landing an instance method call needed `$this` (and any other
//!   receiver) represented as a real `ValueId` first — [`lower::lower_method`]
//!   used to seed `Env` only from `m.params`. The shape chosen mirrors
//!   `mwl_types::check.rs`'s `check_method`, which already seeds `$this` into
//!   its own `LocalScope` the same way, unconditionally and not gated on a
//!   `static` modifier (a static method's body referencing `$this` is a
//!   distinct, unrelated diagnostic neither crate adds here): every lowered
//!   method's [`ir::Function::params`] now carries the receiver at index 0,
//!   ahead of every explicit parameter, whether or not the body ever reads
//!   `$this`. The alternative — a receiver-only special case that leaves
//!   `Function::params` untouched and threads a separate `Option<ValueId>`
//!   just for `$this` — was rejected: it would need `Env`'s `$this` lookup to
//!   go through a different path than every other local, duplicating the
//!   `ExprKind::Variable` handling `lower_expr` already has, for a value that
//!   behaves exactly like an ordinary parameter in every other respect. This
//!   changes every existing snapshot's function signature line (regenerated
//!   via `cargo insta test --accept -p mwl-ir` when this landed) — an
//!   IR-representation choice, not a change visible to an MWL developer.
//! - **Ids are stable, not global.** See [`ids`]'s own module docs.
//! - **Refcount insertion is naive and syntactic, not a liveness/move
//!   analysis — correctness first, elision left to a later optimizer pass.**
//!   `docs/implementation-plan.md`'s own optimizer feature list already names
//!   "refcount elision" as separate future work, distinct from *emitting* the
//!   operations at all — this session only had to answer the latter. Two
//!   designs were weighed: (a) a full last-use/move analysis that only
//!   retains when a value is genuinely shared and skips it otherwise, or (b)
//!   inserting a retain everywhere a value is copied into a second durable
//!   slot and a release everywhere a slot's value is overwritten or the slot
//!   itself goes out of scope, with no attempt to prove a copy was
//!   unnecessary. (b) was chosen: CLAUDE.md's priority ordering ranks
//!   correctness and simplicity ahead of memory/latency, nothing can execute
//!   this IR yet to make (a)'s payoff measurable, and ADR 0004/0006/the
//!   project's own architecture notes already commit to "a refcount per
//!   value ... moved only when the refcount is 1" as a *codegen-time*
//!   optimization for the isolate-boundary case specifically — generalizing
//!   that to ordinary lowering here would be scope creep beyond what any ADR
//!   asks for, not a mechanical extension of it. Concretely: reading a value
//!   out of storage some other binding still owns — [`lower::is_aliasing_read`]
//!   names exactly two such shapes today, a bare `ExprKind::Variable` and a
//!   compile-time-known `ExprKind::PropertyAccess` — and copying it into
//!   another durable slot needs a retain first; a freshly constructed value
//!   (a string literal, `new`, or a call's own result) needs none, since it
//!   already has exactly one natural owner and the copy just gives that
//!   owner a new name/slot. A slot's *previous* value is released whenever
//!   it's overwritten, and every slot still live at a
//!   `return`/implicit-`void`-fallthrough is released too — except the one
//!   slot whose value is the return expression itself when that expression is
//!   a bare `$name` read, which transfers out instead (see
//!   [`lower::Lowering::release_all_locals`]'s own doc comment for exactly
//!   why excluding it there, rather than retaining it and releasing
//!   everything unconditionally, keeps the count exactly balanced even under
//!   aliasing). This one judgment now covers every "durable slot" a `string`
//!   value can be copied into: a local bind
//!   ([`lower::Lowering::bind_local`]), a resolved call's argument
//!   ([`lower::Lowering::lower_call_args`] — the callee's own parameter is
//!   just another local, released at the callee's own exit, so the caller's
//!   retain and the callee's release are a symmetric pair, exactly mirroring
//!   what a local's own declare/drop already does), and a returned value
//!   (`Lowering::lower_stmt`'s `StmtKind::Return` arm — a property read has
//!   no local slot for `release_all_locals` to exclude the way a bare
//!   variable does, so it retains explicitly there instead). What stays a
//!   known gap: string concatenation (needs a runtime-helper call, or a
//!   dedicated `InstKind` — either way, a new operand-producing shape this
//!   session didn't need), and a `tainted`/`secret`-qualified string
//!   (`lower_checked_ty` only handles the plain, unqualified `string` type —
//!   see the known gaps below).
//!
//! # Known gaps (all deliberate, all deferred to a later widening session)
//!
//! - `for`/`switch`/`match`/`try`, and `break`/`continue` of any kind, are
//!   still unsupported: lowering panics naming the statement.
//!   [`ir::Terminator::Branch`] and [`ids::EdgeId`] are both already
//!   exercised by `if`/`while`, so widening to the rest is expected to reuse
//!   the same shapes rather than add new ones — see [`lower`]'s module docs.
//! - An `if`/`while` condition must already be statically `bool` — ADR
//!   0035's full truthy-table conversion for a non-`bool` condition needs a
//!   runtime-helper call, which doesn't exist in the IR yet (see the
//!   "no runtime-helper calls" gap below). Lowering panics naming this.
//! - No block-scoped shadowing: the environment `crate::lower` threads
//!   through is one flat, function-wide map, exactly like the straight-line
//!   slice's `locals` was. A nested `{}` declaring a local that shadows an
//!   outer one of the same name is not distinguished from a reassignment of
//!   the outer binding — not observable for any program in scope today (no
//!   shape here can declare a same-named local in a narrower scope in a way
//!   that matters), but worth knowing before trusting `Env` further.
//! - **No array access** — `$arr[$i]` is unsupported; lowering panics naming
//!   the expression.
//! - **Property access is compile-time-known-field-only.** A receiver whose
//!   static type resolved to a known declaring class lowers to
//!   [`ir::InstKind::FieldGet`], reading `mwl_types::expr_table::ExprInfo::Property`
//!   the same way a call reads `ExprInfo::Call`. A receiver that erased to a
//!   shape or plain `object` (ADR 0036 § 4) has no such entry at all — the
//!   checker itself defers that case's runtime-checked fallback to M4, with
//!   no IR/codegen yet to throw from, so lowering panics naming it rather
//!   than guessing a representation. A nullsafe access (`?->`) is equally
//!   unsupported today, same as a nullsafe method call.
//! - **`string` now crosses a local, call-argument, resolved-return, and
//!   compile-time-known property-*read* boundary — but not a property
//!   *write*, `bytes`, or `array<T>`.** [`lower::lower_checked_ty`] gained a
//!   `CheckedTy::String => Ty::Str` arm, so a call/`new` with a `string`
//!   argument, a call whose declared return type is `string`, and a
//!   `string`-typed property *read* (`$obj->prop`) all lower now, with the
//!   same retain policy a local already had extended to each — see the
//!   design-choices section above. `$obj->prop = expr;` (a property *write*)
//!   is still entirely unsupported for any field type, not just `string`:
//!   `Lowering::lower_reassignment` only accepts a plain-local assignment
//!   target, and panics naming anything else — a pre-existing gap this
//!   session didn't touch. `Ty::Str` also still only covers the plain,
//!   unqualified `string` type: `lower_checked_ty` has no arm for
//!   `CheckedTy::TaintedString`/`SecretString`/`SecretTaintedString` (ADR
//!   0024/0033), so a `tainted`/`secret`-qualified `string` parameter, return
//!   or field still panics there — those qualifiers need their own laundering/
//!   sink story before they can flow through an IR value at all, deliberately
//!   out of scope here. No `bytes` or `array<T>` representation exists yet
//!   either — `bytes` is expected to be a mechanical repeat of `Ty::Str`'s
//!   shape once it lands (same refcounted-heap-value treatment, different
//!   content), while `array<T>` needs its own element-layout decision first.
//!   [`ty::Ty::Object`] is a reference too, but nothing allocates or frees
//!   the memory behind one yet, and no retain/release is emitted for one —
//!   see that variant's own doc comment for exactly what is and isn't
//!   modeled; extending `Ty::is_refcounted` to include it is expected to
//!   reuse the exact same `bind_local`/`lower_call_args`/`release_all_locals`
//!   insertion points `Ty::Str` already uses, not new ones.
//! - **No virtual dispatch** — [`ir::InstKind::Call`]'s `target` is always the
//!   statically resolved declaring class from
//!   `mwl_types::expr_table::ResolvedCall`, exactly as MWL's checker resolved
//!   it, for a static call, `new`'s constructor, and now an instance method
//!   call alike. Every instance call lowered so far still has its receiver's
//!   *static* type equal to its *runtime* class (a concrete, non-interface
//!   local/`new` result) — whether a real vtable/interface-dispatch lookup is
//!   ever needed at this IR level (as opposed to purely at codegen) remains a
//!   question for whichever session first lowers a call through an
//!   interface-typed or overridden-method receiver, where the two can
//!   actually differ.
//! - **No variadic, named, or spread call argument** —
//!   `Lowering::lower_call_args` (in [`lower`]) panics naming any of the
//!   three; `mwl_types` itself doesn't fully positionally type-check a
//!   named/spread argument against a signature yet either (see its own known
//!   gaps), so there is no resolved per-argument type to lower against even
//!   if this crate wanted to try.
//! - Safepoints are reserved, not functional. [`ir::InstKind::Safepoint`] is
//!   emitted at function entry and at every `while` back edge (see that
//!   variant's own doc comment), but it is inert — no codegen exists yet to
//!   lower it to an actual CPU-limit/cancellation/cycle-collector check, and
//!   no guard test needs it functional before M3's backend does. `for`
//!   loops will need the same back-edge marker once they land.
//! - No runtime-helper calls (the milestone's third named ingredient) — this
//!   slice's arithmetic lowers directly to [`ir::InstKind::BinOp`]/[`ir::InstKind::UnOp`],
//!   with no helper-call fallback shape modeled yet (that only matters once
//!   `mixed`/union operands exist in the IR).
//! - Integer literal magnitude range-checking is still not implemented
//!   (mirroring `mwl_types::expr`'s own documented "not modeled this slice"
//!   gap) — cooking now handles all four bases `mwl-syntax`'s lexer accepts
//!   (decimal, `0x`, `0o`, `0b`), just not whether a literal overflows the
//!   target width.
//! - **String-literal cooking is escape-incomplete, and interpolation isn't
//!   lowered at all.** `lower::cook_str_literal` handles the two escapes a
//!   single-quoted literal actually has (`\\`, `\'`) and the common named
//!   escapes in a double-quoted one (`\n`, `\t`, `\r`, `\\`, `\"`, `\$`,
//!   `\0`); a numeric escape (`\xHH`, `\u{...}`, octal) passes through
//!   literally rather than cooking to the byte/codepoint it names — see that
//!   function's own doc comment. `ExprKind::Interpolated` (a double-quoted
//!   string or heredoc with at least one interpolation site) and a
//!   heredoc/nowdoc-sourced `ExprKind::Str` are both entirely unsupported —
//!   lowering panics naming either. There is also no `.` string-concatenation
//!   operator lowered yet: `ExprKind::Binary`'s arm only accepts the
//!   arithmetic/equality/ordering operators `BinaryOp` already covers for a
//!   scalar operand, so a `string . string` expression panics there rather
//!   than reaching `Ty::Str` at all.

pub mod ids;
pub mod ir;
pub mod lower;
pub mod print;
pub mod ty;

pub use ir::{Function, Program};
pub use ty::Ty;

use mwl_diagnostics::{SourceFile, Span};

pub(crate) fn span_text(src: &SourceFile, span: Span) -> &str {
    src.span_text(span).unwrap_or_default()
}

/// Strips a variable's leading `$` sigil, if present — the same idiom
/// `mwl-types` uses.
pub(crate) fn strip_sigil(s: &str) -> &str {
    s.strip_prefix('$').unwrap_or(s)
}
