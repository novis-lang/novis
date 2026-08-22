//! MWL's CFG/SSA IR — `docs/implementation-plan.md`'s M2 milestone's last open
//! thread. See `NEXT_SESSION_PROMPT.md` for how this crate grew: the milestone
//! text ("a CFG/SSA IR carrying explicit safepoints, refcount operations and
//! runtime-helper calls, with a stable per-statement/per-edge id" — see
//! [ADR 0018](../../../docs/adr/0018-coverage-tracing-and-profiling-as-safepoint-shaped-probes.md))
//! is milestone-sized on its own, so this first slice deliberately narrows to
//! exactly what the plan's own M2 *Verify* bullet asks for first: "lower a
//! first, narrow slice ... end to end with a snapshot test, before widening."
//!
//! # What this slice lowers
//!
//! One method whose body is a straight-line sequence of typed local
//! declarations, plain `$x = expr;` reassignment, and a single `return` —
//! [`lower::lower_method`] is the entry point. No control flow, no calls, no
//! non-scalar types. This is intentionally the smallest program shape that
//! still exercises every structural piece of the IR end to end: a
//! [`ir::Function`] of one [`ir::BasicBlock`], SSA [`ir::Inst`]ructions each
//! carrying an explicit representation type ([`ty::Ty`]), and a
//! [`ir::Terminator::Return`].
//!
//! # Design choices worth knowing before widening this
//!
//! - **SSA, not a plain CFG.** `docs/implementation-plan.md`'s M2 paragraph
//!   already commits to "a CFG/**SSA** IR" (not left open by this session) —
//!   adopted here rather than reopened, per CLAUDE.md's "mechanical
//!   follow-through of what the plan already committed to" carve-out. A
//!   straight-line body needs no phi nodes at all (there is exactly one
//!   predecessor for every use), which is exactly why it was chosen as the
//!   first slice: the SSA construction question phi-nodes raise (which join
//!   algorithm, eager vs. pruned) is deferred untouched to the first slice
//!   that actually has a merge point (`if`/`while`), rather than answered
//!   speculatively now.
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
//! # Known gaps (all deliberate, all deferred to the widening session)
//!
//! - No control flow at all: `if`/`while`/`for`/`switch`/`match`/`try` are
//!   unsupported: lowering panics naming the statement. [`ir::Terminator`]
//!   already reserves a `Branch` variant carrying the [`ids::EdgeId`] ADR
//!   0018's branch probe will need on each outgoing edge, and [`ids::IdGen`]
//!   already reserves the id space for it — deliberately, per the milestone
//!   text's "cheap now, expensive to retrofit" — but nothing constructs one
//!   yet.
//! - No calls, no `new`, no property/array access — nothing that isn't a
//!   local, a parameter, a literal, or a scalar unary/binary operator.
//! - No `string`/`bytes`/array/object representation, and therefore no
//!   refcount operations at all — the milestone text's "refcount operations"
//!   have nowhere to attach until a reference-counted value exists in the IR.
//! - No safepoints — the milestone text's other named ingredient. A
//!   safepoint belongs at a loop back-edge and at function entry; a
//!   straight-line, non-recursive body has neither.
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
