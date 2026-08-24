//! MWL's CFG/SSA IR: the one representation between the checked AST and
//! `mwl-codegen`, carrying explicit safepoints, refcount operations and
//! runtime-helper calls, with a stable per-statement and per-edge id
//! ([ADR 0018](../../../docs/adr/0018-coverage-tracing-and-profiling-as-safepoint-shaped-probes.md)).
//!
//! [`lower`] is the whole front-to-IR pass, split across `lower/` by area;
//! [`ir`] is the data; [`ty`] is this crate's own representation-level type
//! lattice; [`print`] renders a program for the snapshot tests.
//!
//! # What lowers today
//!
//! Whole-program: [`lower::lower_file`] walks a file, [`lower::lower_method`]
//! one method, [`lower::lower_property_hook`] one ADR 0014 accessor, and
//! [`lower::lower_script`] a file's own top-level statements as one synthesized
//! frame of ordinary locals with no receiver (ADR 0008 § 2), returning
//! [`ty::Ty::Mixed`] because that is what ADR 0021 types a `require`'s result.
//!
//! - **Statements** — typed and `var` local declarations (ADR 0037),
//!   reassignment, `return`, nested blocks, `echo`, `unset`, `if`, `while`,
//!   `foreach` over all three of ADR 0053 § 3's subjects, `break`/`continue`
//!   at level 1, `try`/`catch`, `throw`.
//! - **Expressions** — arithmetic and comparison, `.` concatenation and string
//!   interpolation, `new`, static/instance/`Core` calls, property and
//!   array-element read and write, array literals including an explicit
//!   `key =>` and `$a[] =` append, `&&`/`||`/`!` and the ternary/elvis
//!   operator, ADR 0035's truthy conversion, ADR 0031 closure literals,
//!   `instanceof`, and ADR 0007 § 2's free and total conversion rows.
//! - **Types** — `int`/`uint`/`float`/`bool` scalars, `string`, `bytes`,
//!   `array<T>` (element type erased — see [`ty::Ty`]), `object` (a class or
//!   enum, likewise erased), `mixed`, and [`ty::Ty::Ref`] for a `&$x`
//!   parameter. `string`, `bytes` and `array<T>` are refcounted and cross a
//!   local, call-argument, return and property boundary alike.
//! - **Generators** — ADR 0053 § 4's state-machine transform, in
//!   [`lower::lower_generator`]: one declaration becomes a factory, an
//!   `advance()`, a `current()` and a synthesized state class.
//!
//! # Design choices worth knowing before widening this
//!
//! - **SSA, not a plain CFG.** `if`/`while` are each a hand-rolled
//!   two-predecessor merge, not a dominance-based phi-placement algorithm —
//!   sufficient for any structured nesting, since neither produces a join of
//!   another shape. `for`/`switch` will reuse the same two building blocks:
//!   `merge_envs` for a set of incoming edges known up front, and
//!   `lower_while`'s seed-then-patch phi dance for a join whose back edge is
//!   not known until its body is lowered.
//! - **IR types are representation-level, not the checker's types.** See
//!   [`ty`]'s own module docs for why [`ty::Ty`] is a small, flat lattice
//!   rather than a reuse of `mwl_types::ty::Ty`.
//! - **This crate depends on `mwl-types` only for its typed-expression table,
//!   never on `mwl-hir`'s class graph directly.** Every *declared* type is read
//!   straight off the AST (ADR 0007 § 1 requires it spelled out, so no name
//!   resolution is needed). What genuinely is absent from the AST is a call's
//!   *resolved target* — which class declares the callee, and its parameter
//!   and return types — so `mwl_types::expr_table::ExprTypeTable` publishes
//!   that, keyed by source span, and lowering reads it back. The alternative,
//!   re-running class-hierarchy resolution here, would have duplicated
//!   `mwl-types`. Lowering **trusts** that its input already passed
//!   `mwl_types::check_program` with the same tables, and panics naming an
//!   unsupported shape rather than diagnosing.
//! - **A receiver is an implicit first parameter, not a special case.** Every
//!   lowered method carries `$this` at [`ir::Function::params`] index 0,
//!   whether or not the body reads it — mirroring `mwl_types`' own
//!   `check_method`. A separate `Option<ValueId>` would have needed `Env`'s
//!   `$this` lookup to take a different path than every other local, for a
//!   value that behaves exactly like an ordinary parameter.
//! - **Refcount insertion is naive and syntactic, not a liveness analysis.**
//!   Correctness first; elision is a named later optimizer pass. Reading a
//!   value out of storage another binding still owns — [`lower::is_aliasing_read`]
//!   names the three shapes — and copying it into a second durable slot needs
//!   a retain; a freshly constructed value needs none, since it already has
//!   one natural owner. A slot's previous value is released when overwritten,
//!   every live slot is released at frame exit, and a binding **control flow
//!   drops** is released at the point it disappears — a local declared inside
//!   a loop body or inside one `if` branch. That last half was missed until
//!   `examples/report.mwl`'s valgrind leg found it, which is why AGENTS.md
//!   says to run `tools/leak-check.sh` against a fixture exercising any new
//!   refcount edge.
//! - **A closed, engine-owned runtime helper gets [`ir::InstKind::HelperCall`]
//!   with a `#[non_exhaustive]` [`ir::Helper`] tag**, not [`ir::InstKind::Call`]
//!   with a synthetic target. A helper has neither a class-graph origin nor a
//!   receiver, so folding it into `Call` would blur the line virtual dispatch
//!   depends on staying sharp; and the helper set is closed and known to this
//!   crate, so an enum buys exhaustiveness a string name would lose.
//! - **Nothing may assume the CFG is reducible.** A generator's `advance()`
//!   dispatches straight into a block inside a loop body, giving that loop a
//!   second entry. Cranelift accepts one, so M3's backend does not care — this
//!   is a constraint on any later pass added here.
//! - **Ids are stable, not global.** See [`ids`]'s own module docs.
//!
//! # Known gaps
//!
//! Each panics naming itself rather than miscompiling.
//!
//! 1. **`for`, `switch` and `match` do not lower at all.** Every shape they
//!    need exists — [`ir::Terminator::Branch`] with an [`ids::EdgeId`], and
//!    [`ir::Terminator::Switch`], built general rather than
//!    resumption-specific precisely so `switch` can reach for it — so widening
//!    should add no new ones.
//! 2. **A `finally` does not run when a `catch` clause's own body throws.**
//!    [`lower::Lowering::lower_try`] owns that one — every other exit from a
//!    protected region runs its `finally`, including a `return`, a `break` and
//!    a `continue`. A second, narrower gap sits inside what does lower: a
//!    landing block sweeps the frame's locals, not a temporary still in flight
//!    inside the expression that threw — which is what leaks a closure literal
//!    written directly as a call argument.
//! 3. **`mixed` is a representation to erase *into*, not one to dispatch
//!    *on*.** There is no runtime type-tag shape, so arithmetic, `.`,
//!    ADR 0035's truthy table and an array access through a `mixed`-erased
//!    base all stop there. Closing it should add [`ir::Helper`] variants, not
//!    a second call-shaped instruction. `null` is in the same position: no
//!    nullable-type representation exists to convert from.
//! 4. **Only ADR 0007 § 2's free and total conversion rows lower.** Every
//!    *checked* row — `int` ↔ `uint`, `float` to an integer, `string` to a
//!    number, ADR 0010 § 5's integer-into-an-enum — needs a throwing helper
//!    [`ir::Helper`] has no tag for, and the enum row additionally needs its
//!    case set carried to the check. `EnumName` ↔ `string` is not a gap: ADR
//!    0010 § 5 leaves it out of the language.
//! 5. **`&&`/`||`/`!`/ternary lower only where a mutable `cur: &mut BlockId`
//!    is already owned** — a declaration's initializer, `return`'s value, an
//!    assignment's right-hand side, a condition. Nested inside a call
//!    argument, an array element or an operand, they panic, because
//!    [`lower::Lowering::lower_expr`] cannot redirect the current block. A
//!    ternary whose branches lower to two different [`ty::Ty`] representations
//!    panics for the same reason gap 3 does.
//! 6. **Property and array access are compile-time-known-target-only.** A
//!    receiver that erased to a shape or plain `object` (ADR 0036 § 4) has no
//!    `ExprInfo` entry, so lowering panics; the checker defers that runtime
//!    check to M4. Nullsafe `?->` is unsupported on either side. An
//!    array-element write through a hooked property is refused: the
//!    copy-on-write separation would have to be written back through the `set`
//!    hook, and no PHP-compatible rule for that exists yet. A *nested* write —
//!    `$grid[0][1] = v`, whose base is itself an index expression — is refused
//!    for the same reason: the separated inner array has to be written back
//!    into the outer one, and only a local or a known property is a place this
//!    crate can write back to. Reading `$grid[0][1]` is fine. Neither
//!    [`ir::InstKind::ArrayGet`] nor [`ir::InstKind::ArraySet`] models an
//!    absent key at runtime — deferred wholesale, like every other checked
//!    throw. A **static** property is narrower still: it reads, but
//!    [`lower::Lowering`]'s assignment arm has no target for one, so
//!    `C::$p = v` panics.
//! 7. **Virtual dispatch resolves by name, not by slot.** An instance call
//!    lowers to [`ir::InstKind::Call`] — bound to the statically resolved
//!    label — only when nothing in the program overrides that declaration;
//!    `mwl_types` answers that whole-program question once, per call, as
//!    `mwl_types::expr_table::ResolvedCall::overridden`. When something does,
//!    and for the two shapes with no static answer at all (`static::`/`new
//!    static`, and a call resolving to a body-less declaration), the call
//!    goes through [`ir::InstKind::CallVirtual`]/[`ir::InstKind::NewDynamic`]
//!    over [`ty::Ty::ClassDesc`], which looks the name up in the per-class
//!    method table [`ir::Class::methods`] carries. A real vtable would index
//!    that table by slot instead, which is the remaining half — a lookup
//!    cost, not a correctness gap.
//! 8. **No variadic, named or spread call argument**, and no `...spread` or
//!    `&value` array-literal element. `mwl_types` does not fully
//!    positionally type-check a named or spread argument either, so there is
//!    no resolved per-argument type to lower against.
//! 9. **A closure literal lowers; `$f(...)` does not.** The only caller today
//!    is native `Core` code going through `mwl_runtime::mwl_closure_call`.
//!    ADR 0031 § 3's self-name is parsed and ignored, and a `&$x` capture or
//!    parameter panics — the cell it addresses is the caller's, and a closure
//!    may outlive the call that staged it.
//! 10. **A `&$x` argument's copy-back is emitted at the enclosing statement**,
//!     because that is the nearest scope holding an `&mut Env` — so such a
//!     call lowers only as a bare expression statement or an assignment's
//!     right-hand side. [`lower::Lowering::pending_refs`] owns it.
//! 11. **A `tainted`/`secret`-qualified type has no IR arm.**
//!     [`lower::lower_checked_ty`] handles the plain `string`/`bytes` only;
//!     ADR 0024/0033's qualifiers are compile-time-only and need no runtime
//!     representation, but the erasure has to be written.
//! 12. **Neither `.` nor `as string` covers a `Stringable` operand.**
//!     Desugaring would have to synthesize a resolved call to `toString()`,
//!     but neither a bare `.` operand nor an `as` subject is a call
//!     expression, so no `ExprInfo::Call` is recorded for it. Either the
//!     checker records that resolution too, or this crate re-resolves the
//!     method itself — a second `mwl-types` dependency so far avoided.
//! 13. **Inline HTML at file scope is not lowered.** The lowering is the same
//!     [`ir::Helper::EchoStr`] call `echo` emits over the raw span; it is out
//!     only because `mwl_types` treats `InlineHtml` as a no-op too, so landing
//!     it widens two crates at once.
//! 14. **Safepoints are reserved, not functional.** [`ir::InstKind::Safepoint`]
//!     is emitted at function entry and every loop back edge, but nothing
//!     lowers it to a real CPU-limit or cancellation check yet.
//! 15. **`decimal` has no IR representation.** ADR 0054's scalar checks now —
//!     the keyword, the type atom, literal placement and § 3's arithmetic
//!     table are all live in `mwl-syntax`/`mwl-types` — so a program that
//!     declares one reaches [`lower::lower_decl_type`] and panics naming the
//!     shape. Closing it is [`ty::Ty`]'s 16-byte register pair plus the helper
//!     that ADR's *Consequences* calls "the real implementation cost": `+`,
//!     `-` and comparison at equal scale inline to i128 operations, while `*`,
//!     `/` and mixed-scale operands need a wider intermediate. M4 owes it.
//! 16. **A compound assignment does not lower.** `$x += 1`, `-=`, `*=`, `.=`
//!     and the rest reach [`lower::Lowering`]'s expression-statement arm, which
//!     handles a plain `$x = expr;` and a bare call only, and panic naming the
//!     operator. Desugaring `$x op= e` to `$x = $x op e` is correct for every
//!     target this crate can already assign to, so the work is that rewrite
//!     plus deciding where it belongs — the checker sees the shape first.
//! 17. **The environment is one flat, function-wide map**, so a nested block
//!     declaring a local that shadows an outer one is not distinguished from a
//!     reassignment. Not observable for any program in scope today, but worth
//!     knowing before trusting `Env` further.

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
