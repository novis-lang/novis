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
//! One method whose body is typed local declarations, plain `$x = expr;`
//! reassignment, `return`, nested `{}` blocks, `if`/`while`, `new`, and a
//! static method call (`self::method(...)`/`Class::method(...)`) —
//! [`lower::lower_method`] is the entry point. No `for`/`switch`/`try`, no
//! `break`/`continue`, no instance method call, no property/array access, no
//! non-scalar-*data* types (`string`/`bytes`/`array<T>` — a class/enum value
//! itself now has a representation, [`ty::Ty::Object`], just not a way to
//! read a field off one yet). The straight-line subset was deliberately the
//! *first* slice landed (see git history and `docs/implementation-plan.md`'s
//! M2 paragraph) because it was the smallest shape exercising every
//! structural IR piece with no merge point at all; `if`/`while` came next,
//! and are where SSA's actual join/phi question gets answered — see
//! [`lower`]'s own module docs for exactly how. `new`/a static call are the
//! third slice, and the first to need more than the AST alone — see the next
//! section for the dependency that unlocked them.
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
//! - **Ids are stable, not global.** See [`ids`]'s own module docs.
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
//! - **No instance method call** (`$obj->method(...)`, including `$this->…`)
//!   — only a static call (`self::method(...)`/`Class::method(...)`) and
//!   `new` lower so far. An instance call needs its receiver represented as a
//!   real value, and `$this` in particular is not bound in `Env` at all
//!   today — [`lower::lower_method`] only seeds `Env` from `m.params`, the
//!   same as before this slice, since widening that (an implicit receiver
//!   parameter ahead of every explicit one, changing `Function::params` for
//!   *every* method whether it uses `$this` or not) is a bigger, separate
//!   change than this slice's scope. [`ir::InstKind::Call`] already reserves
//!   a `receiver` field for exactly this, so landing it needs no new
//!   `InstKind` variant — see that field's own doc comment.
//! - **No property or array access** — `$obj->prop`/`$arr[$i]` are
//!   unsupported; lowering panics naming the expression. Property access
//!   additionally needs ADR 0036 § 4's shape/`object`-erasure semantics
//!   worked out at the IR level (a compile-time-known field vs. a
//!   runtime-checked one), not just a representation to read from.
//! - No `string`/`bytes`/`array<T>` representation, and therefore no refcount
//!   operations at all — the milestone text's "refcount operations" have
//!   nowhere to attach until a reference-counted *data* value exists in the
//!   IR. [`ty::Ty::Object`] is a reference too, but nothing allocates or frees
//!   the memory behind one yet — see that variant's own doc comment for
//!   exactly what is and isn't modeled.
//! - **No virtual dispatch** — [`ir::InstKind::Call`]'s `target` is always the
//!   statically resolved declaring class from
//!   `mwl_types::expr_table::ResolvedCall`, exactly as MWL's checker resolved
//!   it; whether a real vtable/interface-dispatch lookup is ever needed at
//!   this IR level (as opposed to purely at codegen) is a question for
//!   whichever session first lowers a call through an interface-typed or
//!   overridden-method receiver.
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
//! - `var` locals (ADR 0037) are unsupported — this slice only lowers a
//!   [`mwl_syntax::ast::StmtKind::LocalDecl`] with an explicit `ty`.
//! - Integer literal cooking supports plain decimal digits with `_`
//!   separators only; hex/octal/binary literal bodies and magnitude
//!   range-checking are not implemented (mirroring `mwl_types::expr`'s own
//!   documented "not modeled this slice" gap for the same case).

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
