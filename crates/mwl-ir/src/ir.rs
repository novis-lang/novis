//! The CFG/SSA IR itself: a [`Program`] of [`Function`]s, each a graph of
//! [`BasicBlock`]s of SSA [`Inst`]ructions ending in exactly one
//! [`Terminator`]. See the crate's own module docs for this first slice's
//! scope and known gaps.

use mwl_diagnostics::Span;

use crate::ids::{BlockId, EdgeId, StmtId, ValueId};
use crate::ty::Ty;

/// Every function this compilation unit lowered.
#[derive(Debug, Default)]
pub struct Program {
    /// The lowered functions, in no particular order.
    pub functions: Vec<Function>,
}

/// One lowered method or function.
#[derive(Debug)]
pub struct Function {
    /// The function's name, for diagnostics and the text listing —
    /// `crate::lower::lower_method`'s caller decides how it is qualified
    /// (bare method name, `Class::method`, ...).
    pub name: String,
    /// Each parameter's representation, positional. Index 0 is always the
    /// implicit receiver (`$this`) — every lowered method carries it, whether
    /// or not its body ever reads `$this`, mirroring
    /// `mwl_types::check.rs`'s `check_method` seeding `$this` into its own
    /// `LocalScope` unconditionally (not gated on a `static` modifier — see
    /// that function's own comment for why). Every explicit
    /// `MethodMember` parameter follows, starting at index 1. See
    /// `crate::lower::lower_method`'s own doc comment for where the
    /// receiver's value comes from.
    pub params: Vec<Ty>,
    /// The return representation, [`Ty::Void`] for a `void`-returning
    /// method.
    pub ret: Ty,
    /// The function's basic blocks. A straight-line body still produces
    /// exactly one; `if`/`while` each add the blocks their join point needs
    /// (see `crate::lower`'s module docs).
    pub blocks: Vec<BasicBlock>,
    /// Which block execution starts in.
    pub entry: BlockId,
    /// The source span [`crate::ids::StmtId::index`] names — see
    /// [`crate::ids`]'s own module docs on why this table, not a hash map,
    /// backs the lookup.
    pub stmt_spans: Vec<Span>,
    /// The source span [`crate::ids::EdgeId::index`] names. Non-empty for
    /// any function containing an `if`/`while`.
    pub edge_spans: Vec<Span>,
}

/// One basic block: a straight-line run of [`Inst`]ructions ending in one
/// [`Terminator`].
#[derive(Debug)]
pub struct BasicBlock {
    /// This block's own id.
    pub id: BlockId,
    /// The block's instructions, in execution order.
    pub insts: Vec<Inst>,
    /// How the block ends.
    pub term: Terminator,
}

/// One SSA instruction. Not every instruction defines a value — a
/// [`InstKind::StmtMarker`] never does, the same way a future `void`-returning
/// call would not.
#[derive(Debug)]
pub struct Inst {
    /// The SSA value this instruction defines, if any.
    pub result: Option<ValueId>,
    /// `result`'s representation, present exactly when `result` is.
    pub ty: Option<Ty>,
    /// What the instruction does.
    pub kind: InstKind,
}

/// What one [`Inst`] does.
#[derive(Debug)]
#[non_exhaustive]
pub enum InstKind {
    /// ADR 0018 § 1's statement-boundary probe site: marks where the
    /// previous lowered statement's instructions end and the next one's
    /// begin, in program order. Defines no value.
    StmtMarker(StmtId),
    /// A reserved safepoint poll site — function entry (recursion) or a
    /// loop's back edge, the two sites the project-start "safepoints emitted
    /// from the first backend commit" decision names
    /// ([`docs/adr/README.md`](../../../docs/adr/README.md)'s "Decisions
    /// taken at project start" section) and that ADR 0018 § *Negative*
    /// contrasts its own, denser probe grid against. This slice reserves the
    /// shape only — no codegen exists yet to lower it to an actual CPU-limit/
    /// cancellation/cycle-collector check, and no guard test needs it
    /// functional before M3 builds the backend on top of this IR. Reserved
    /// now rather than later for the same "cheap now, expensive to
    /// retrofit" reason `crate::ids` already gives for `StmtId`/`EdgeId`:
    /// inserting it after the fact would mean re-walking every already-
    /// lowered function. Defines no value.
    Safepoint,
    /// A `bool` constant.
    ConstBool(bool),
    /// An `int` constant.
    ConstInt(i64),
    /// A `uint` constant.
    ConstUint(u64),
    /// A `float` constant.
    ConstFloat(f64),
    /// A `string` literal's cooked bytes — a fresh [`Ty::Str`] value with
    /// exactly one implicit owner (itself), the same "one natural reference"
    /// starting point [`InstKind::New`] gives a freshly constructed object.
    /// See `crate::lower::cook_str_literal`'s own doc comment for exactly
    /// which escape sequences are cooked this slice and which are a known
    /// gap.
    ConstStr(String),
    /// Reads the function's own parameter at this positional index.
    Param(u32),
    /// A binary arithmetic or comparison operator over two already-lowered
    /// operands.
    BinOp {
        /// Which operator.
        op: BinOp,
        /// The left operand.
        lhs: ValueId,
        /// The right operand.
        rhs: ValueId,
    },
    /// A unary operator over one already-lowered operand.
    UnOp {
        /// Which operator.
        op: UnOp,
        /// The operand.
        operand: ValueId,
    },
    /// A phi node: selects the incoming value based on which predecessor
    /// block control arrived from. One entry per predecessor that can reach
    /// this instruction's own block — `if`/`while`'s join points are the
    /// first thing to construct one; see `crate::lower`'s module docs.
    /// A single-entry phi is a legal (if degenerate) case: a `while` body
    /// that never reaches its own back edge (e.g. it always returns) leaves
    /// the loop header's phi with only the pre-loop incoming edge.
    Phi {
        /// `(predecessor block, incoming value)` pairs.
        incoming: Vec<(BlockId, ValueId)>,
    },
    /// A statically resolved call — a static method call, `new`'s
    /// constructor invocation, or an instance method call
    /// (`$obj->method(...)`, including `$this->…`). The target is already
    /// fully resolved by `mwl_types::expr_table::ExprTypeTable` before
    /// lowering ever sees it — there is no virtual dispatch to model here,
    /// only "which function does this invoke." See the crate docs' "no
    /// virtual dispatch" known gap for what happens once a receiver's static
    /// and runtime types can actually differ.
    Call {
        /// The resolved target, rendered `"Class::method"` — a label for
        /// `crate::print`/a future codegen symbol table, not itself
        /// resolvable back to a `QName` (this crate never depends on
        /// `mwl-hir`; see `crate::lower`'s module docs).
        target: String,
        /// The receiver value, for an instance method call (`Some`) — `None`
        /// for a static call or `new`'s constructor invocation, neither of
        /// which has a receiver at all.
        receiver: Option<ValueId>,
        /// Each positional argument, already lowered.
        args: Vec<ValueId>,
    },
    /// `new Target(...)`: allocates an instance of `class` and, if it
    /// declares one, invokes its resolved `constructor` with `args` —
    /// bundled into one instruction rather than a separate allocation plus
    /// `InstKind::Call`, since no codegen exists yet to make that split
    /// observable (M3's backend work, once real object layout exists) and
    /// splitting it now would be a distinction with nothing to attach it to.
    New {
        /// The constructed class, rendered the same way `Call::target` is.
        class: String,
        /// Each constructor argument, already lowered — empty when `class`
        /// declares no explicit `constructor`.
        args: Vec<ValueId>,
    },
    /// Reads a compile-time-known field off an object — `$obj->prop` whose
    /// receiver's static type resolved to a known declaring class (an
    /// `mwl_types::expr_table::ExprInfo::Property` entry exists for it). No
    /// actual byte offset is computed here: `class`/`field` are labels for a
    /// future codegen layout pass, the same "resolved identity, not yet a
    /// machine offset" shape `Call`/`New`'s own `target`/`class` labels
    /// already use. A property access whose receiver erased to a shape or
    /// plain `object` (ADR 0036 § 4) has no such entry to read at all —
    /// `crate::lower` panics naming that case rather than lowering it; see
    /// the crate docs' known gaps for why (the checker itself defers the
    /// runtime-checked fallback to M4, with no IR/codegen yet to throw from).
    FieldGet {
        /// The receiver, already lowered.
        object: ValueId,
        /// The class that actually declares the field, rendered the same way
        /// `Call::target`'s class half is.
        class: String,
        /// The field's own name, `$`-sigil not included.
        field: String,
    },
    /// Writes a compile-time-known field on an object — `$obj->prop = expr;`
    /// whose receiver's static type resolved to a known declaring class,
    /// exactly the same [`ExprInfo::Property`](mwl_types::expr_table::ExprInfo::Property)
    /// resolution [`InstKind::FieldGet`] already relies on for a read (see
    /// that variant's own doc comment for why `class`/`field` stay labels
    /// rather than a machine offset, and for the erased-receiver panic case
    /// this shares). `crate::lower::Lowering::lower_reassignment` emits this
    /// alongside a retain of `value` (if it's an aliasing read) and a release
    /// of whatever the field previously held, the same refcounting policy
    /// [`Retain`](InstKind::Retain)/[`Release`](InstKind::Release)'s own doc
    /// comments describe for a local. Defines no value; operates on `object`
    /// in place.
    FieldSet {
        /// The receiver, already lowered.
        object: ValueId,
        /// The class that actually declares the field, rendered the same way
        /// `FieldGet::class` is.
        class: String,
        /// The field's own name, `$`-sigil not included.
        field: String,
        /// The new value, already lowered.
        value: ValueId,
    },
    /// `.` string concatenation: builds a fresh [`Ty::Str`] value from the
    /// cooked bytes of `lhs` and `rhs`, both already [`Ty::Str`] by the time
    /// this instruction sees them — see
    /// `crate::lower::Lowering::concat_operand`, which converts a scalar
    /// operand through an [`InstKind::HelperCall`] first, and still panics
    /// naming a `Stringable`-object operand (needs a resolved `toString`
    /// call this crate can't synthesize yet). Modeled as a dedicated
    /// instruction rather than a runtime-helper call itself, the same
    /// "native instruction over already-typed operands" treatment
    /// [`InstKind::BinOp`] already gives scalar arithmetic — `.` only ever
    /// needs this one fixed two-operand shape, unlike the open-ended,
    /// enum-tagged set [`HelperCall`](InstKind::HelperCall) exists for. The
    /// result is a fresh value with exactly one natural owner —
    /// concatenation always allocates a new buffer, so
    /// `crate::lower::is_aliasing_read` stays `false` for `ExprKind::Binary`,
    /// same as it already is for [`InstKind::ConstStr`]/[`InstKind::New`]/
    /// [`InstKind::Call`]. Neither operand is retained by this instruction
    /// itself: each is only *read* to build the new buffer, exactly the way
    /// [`InstKind::FieldGet`] reads its `object` receiver without retaining
    /// it, so ownership of `lhs`/`rhs` stays wherever it already was (their
    /// own local slot, field, ...) — and `crate::lower::Lowering::concat_operand`'s
    /// caller releases either operand right after this instruction reads it
    /// when that operand was never such a slot to begin with (a literal, a
    /// nested `Concat`'s own result, or a freshly converted
    /// [`HelperCall`](InstKind::HelperCall) result), since nothing else will
    /// ever release it otherwise.
    Concat {
        /// The left operand, already lowered and already [`Ty::Str`].
        lhs: ValueId,
        /// The right operand, already lowered and already [`Ty::Str`].
        rhs: ValueId,
    },
    /// Increments a [`Ty::is_refcounted`] value's reference count — emitted
    /// exactly where `crate::lower`'s "copy" case needs a second durable
    /// owner to see it stay alive (see that module's docs for the precise
    /// insertion policy and the syntactic "is this an aliasing read"
    /// judgment it's keyed on). Defines no value; operates on `operand`
    /// in place.
    Retain {
        /// The value being retained.
        operand: ValueId,
    },
    /// Decrements a [`Ty::is_refcounted`] value's reference count, freeing
    /// its heap allocation if it reaches zero — emitted where a durable
    /// slot's previous value is overwritten, and for every such slot still
    /// live when its owning function returns (except the one slot whose
    /// value is the return value itself, which transfers out instead — see
    /// `crate::lower::Lowering::release_all_locals`'s own doc comment).
    /// Defines no value; operates on `operand` in place. No codegen exists
    /// yet to lower this to an actual decrement-and-maybe-free sequence —
    /// reserved the same "shape now, functional once a backend exists" way
    /// [`InstKind::Safepoint`] already is.
    Release {
        /// The value being released.
        operand: ValueId,
    },
    /// Invokes one of a small, closed, engine-owned set of runtime
    /// conversions — the milestone's third named ingredient, and this
    /// crate's first. `helper` is a fixed [`Helper`] tag, never a resolved
    /// class/method name: unlike [`InstKind::Call`]'s `target`, nothing here
    /// comes from `mwl_types::expr_table::ExprTypeTable` or a class
    /// hierarchy, so there is no receiver, no virtual dispatch question, and
    /// no reason to share `Call`'s shape (see `crate::lower`'s module docs'
    /// design-choices section for why a dedicated instruction was chosen
    /// over reusing `Call` with a synthetic target label, and an enum tag
    /// over a string name). Like [`InstKind::Call`]/[`InstKind::New`], this
    /// does not yet model ADR 0002's checked-return convention — no status,
    /// no error edge — since nothing in this crate models a call that can
    /// fail at all yet (no `try`/`throw` lowered); that is expected to land
    /// once such calls do, for every call-shaped instruction at once rather
    /// than only for this one.
    HelperCall {
        /// Which conversion.
        helper: Helper,
        /// Each argument, already lowered.
        args: Vec<ValueId>,
    },
    /// `[...]`/legacy `array(...)`: builds a fresh [`crate::ty::Ty::Array`]
    /// value — ADR 0007 § 5's insertion-ordered, string-keyed hash — from a
    /// fixed list of already-lowered `(key, value)` pairs, bundled into one
    /// instruction rather than an allocation plus a sequence of inserts, the
    /// same "no codegen exists yet to make the split observable" reasoning
    /// [`InstKind::New`]'s own doc comment already gives for a constructor
    /// call. Every `key` here is a decimal string computed at lowering time,
    /// not a lowered expression: `crate::lower::Lowering::lower_expr`'s
    /// `ArrayLiteral` arm only lowers a *positional* element (no explicit
    /// `key =>`), so each element's key is simply its index in the list,
    /// auto-numbered from `0` exactly like PHP's own `[$a, $b]` shorthand —
    /// known statically without a runtime index-tracking instruction. An
    /// explicit `key =>` entry, a `...spread` element, and a `&value` element
    /// are all still unsupported — lowering panics naming whichever is used
    /// (see the crate docs' known gaps): `mwl_types::expr::check_array_literal`
    /// itself has no key-normalization/rejection logic yet either (ADR 0007
    /// § 5's int/uint-to-decimal-string normalization, float/bool/null
    /// rejection), so lowering an explicit key here would mean guessing at a
    /// runtime conversion this crate can't yet synthesize.
    ///
    /// The array itself is a fresh value with exactly one natural owner, the
    /// same starting point [`InstKind::New`]/[`InstKind::ConstStr`] already
    /// give — see `crate::lower::is_aliasing_read`, which stays `false` for
    /// `ExprKind::ArrayLiteral`. Each element that's itself
    /// [`crate::ty::Ty::is_refcounted`] and
    /// [`is_aliasing_read`](crate::lower::is_aliasing_read) (a bare local or a
    /// compile-time-known property read) is retained by
    /// `crate::lower::Lowering::lower_expr`'s `ArrayLiteral` arm *before* this
    /// instruction runs — the same caller-side retain
    /// `crate::lower::Lowering::lower_call_args` already inserts for a
    /// refcounted, aliasing call argument, since the array durably owns
    /// whatever it stores exactly the way a callee's parameter slot does. A
    /// fresh element (a literal, `new`, a nested array literal, or a call's
    /// own result) needs no retain: it already has exactly one natural owner,
    /// which simply transfers into the array's storage.
    ArrayNew {
        /// `(key, value)` pairs, in insertion order — ADR 0007 § 5's
        /// "iteration order is insertion order, always."
        entries: Vec<(String, ValueId)>,
    },
}

/// One member of the closed set of engine-owned runtime conversions
/// [`InstKind::HelperCall`] can invoke — a fixed, non-exhaustive enum for
/// the same reason [`BinOp`]/[`UnOp`] already are one: the set is small,
/// closed, and known entirely to this crate and `mwl-codegen`, never
/// user-extensible, so a string name would only trade compile-time
/// exhaustiveness for nothing. Every variant here today converts one scalar
/// to [`crate::ty::Ty::Str`] for `.` concatenation — see
/// `crate::lower::Lowering::concat_operand`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[non_exhaustive]
pub enum Helper {
    /// `int` to its decimal `string` representation.
    IntToString,
    /// `uint` to its decimal `string` representation.
    UintToString,
    /// `float` to its `string` representation.
    FloatToString,
    /// `bool` to `"1"`/`""`, PHP's own bool-to-string rule.
    BoolToString,
}

/// A binary arithmetic or comparison operator, already resolved to a single
/// representation (no `mixed`/union dispatch — see the crate docs' "no
/// runtime-helper calls" known gap).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[non_exhaustive]
pub enum BinOp {
    /// `+`
    Add,
    /// `-`
    Sub,
    /// `*`
    Mul,
    /// `/`
    Div,
    /// `%`
    Mod,
    /// `==` / `===` — this slice does not yet distinguish loose from strict
    /// equality (no non-scalar operand exists yet for the two to differ on).
    Eq,
    /// `!=` / `!==`
    NotEq,
    /// `<`
    Lt,
    /// `<=`
    LtEq,
    /// `>`
    Gt,
    /// `>=`
    GtEq,
}

/// A unary operator.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[non_exhaustive]
pub enum UnOp {
    /// Numeric negation, `-x`.
    Neg,
    /// Boolean negation, `!x`.
    Not,
}

/// How one [`BasicBlock`] ends.
#[derive(Debug)]
#[non_exhaustive]
pub enum Terminator {
    /// `return;` (`None`) or `return expr;` (`Some`).
    Return(Option<ValueId>),
    /// An unconditional jump — a branch's arm rejoining its merge point, or a
    /// loop's back edge to its header.
    Jump(BlockId),
    /// A two-way conditional branch, carrying the [`EdgeId`] ADR 0018's
    /// branch probe needs on *each* outgoing edge.
    Branch {
        /// The condition value.
        cond: ValueId,
        /// The block entered when `cond` is true.
        then_block: BlockId,
        /// The stable id of the true edge.
        then_edge: EdgeId,
        /// The block entered when `cond` is false.
        else_block: BlockId,
        /// The stable id of the false edge.
        else_edge: EdgeId,
    },
}
