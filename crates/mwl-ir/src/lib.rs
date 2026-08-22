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
//! reassignment, `return`, nested `{}` blocks, and now `if`/`while` —
//! [`lower::lower_method`] is the entry point. No `for`/`switch`/`try`, no
//! `break`/`continue`, no calls, no non-scalar types. The straight-line
//! subset was deliberately the *first* slice landed (see git history and
//! `docs/implementation-plan.md`'s M2 paragraph) because it was the smallest
//! shape exercising every structural IR piece with no merge point at all;
//! `if`/`while` are the second slice, landed once that shape was proven out,
//! and are where SSA's actual join/phi question gets answered — see
//! [`lower`]'s own module docs for exactly how.
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
//! - **This crate does not depend on `mwl-types` or `mwl-hir` yet.** Every
//!   declared type this slice's lowering needs (a parameter's, a local's, a
//!   method's return type) is read directly off the `mwl-syntax` AST, because
//!   ADR 0007 § 1 already requires it to be spelled out there in full for
//!   every shape this slice covers — no name resolution or inference is
//!   needed to answer "what type is this". [`lower::lower_method`]
//!   deliberately **trusts** that its input already passed
//!   `mwl_types::check_program` with no errors; it is not a second checker,
//!   and panics (naming the unsupported shape) rather than diagnosing when
//!   handed something outside this slice's scope. Widening past scalars —
//!   property access, calls, `new`, anything needing a resolved class or a
//!   checked expression type mwl-types computes but does not persist — will
//!   need to either add those dependencies or have `mwl-types` grow a
//!   published, persisted typed-expression table lowering can read; which of
//!   those is cheaper is an open question for that session, not this one.
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
//! - No calls, no `new`, no property/array access — nothing that isn't a
//!   local, a parameter, a literal, or a scalar unary/binary operator.
//! - No `string`/`bytes`/array/object representation, and therefore no
//!   refcount operations at all — the milestone text's "refcount operations"
//!   have nowhere to attach until a reference-counted value exists in the IR.
//! - No safepoints — the milestone text's other named ingredient. A
//!   safepoint belongs at a loop back-edge and at function entry; `while`
//!   now lowers a real back edge ([`ir::Terminator::Jump`] from the loop
//!   body to its header), but nothing marks it as a safepoint poll site yet
//!   — reserved for whenever M3's codegen needs the marker to exist (no
//!   guard test needs it before then, per the plan's M2 paragraph).
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
