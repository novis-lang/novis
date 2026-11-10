//! `rule:expressions/an-operand-keeps-its-value-while-later-operands-run`:
//! the one place that decides whether a borrowed operand needs a reference of
//! its own, given the expressions lowered after it and before its consumer
//! reads it.
//!
//! Part of [`super`]'s one `impl Lowering`, under the rule [`super::expr`]'s
//! header states.
//!
//! A borrowed read is a [`Lowering::aliasing_read`]: the value is in a durable
//! slot (a local, a property, an element, a static property) and this frame
//! holds no reference to it. That is free and sound for as long as nothing
//! between the read and its use can replace the slot. [`Lowering::hold_borrowed`]
//! is asked once per operand with the expressions that follow it, and when one
//! of them can free the slot it takes the reference then, so the consumer
//! treats the value as fresh and stages it like a literal.
//!
//! **What it spends:** one non-atomic increment and one decrement per held
//! operand, per evaluation. A read followed only by inert operands
//! ([`Lowering::is_inert`]) pays nothing.

use nvs_syntax::ast::{Arg, Expr, ExprKind, StringPart};
use nvs_types::expr_table::FormatPiece;
use rustc_hash::FxHashSet;

use super::{ArgOwnership, Lowering};
use crate::Ty;
use crate::ids::{BlockId, ValueId};
use crate::{span_text, strip_sigil};

/// Every consumer that lowers operands and reads them only after the later
/// ones have run. Each passes its own variant to [`Lowering::hold_borrowed`],
/// and `nvs-codegen`'s `borrowed_reads` test builds one program per entry of
/// [`OperandSite::ALL`], so a consumer moved to the one judgment is a
/// consumer the matrix proves.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum OperandSite {
    /// An operand of a `.` spine, read by the one `Concat` after the last.
    Concat,
    /// A hole of a double-quoted string or heredoc, read by the join after
    /// the last hole.
    StringHole,
    /// An operand of a binary operator that reads both sides only once the
    /// second is lowered: `==`, `!=`, an ordering or arithmetic row over a
    /// tag, and a carrier's `+`.
    Binary,
    /// A `match` subject, read by every label's comparison.
    MatchSubject,
    /// An argument a call borrows, read by the call after the last argument:
    /// a `Core` member's argument, and each field of its options bag.
    Argument,
    /// An argument of a call through a `callable` value, and the callable
    /// itself, read by the call after the last argument.
    CallableArgument,
    /// The base of an element read, read by the element read after its key.
    IndexBase,
    /// The left operand of `<`, `<=`, `>`, `>=` or `<=>` over two objects,
    /// read by `compareTo` after the right one.
    Ordering,
}

impl OperandSite {
    /// Every site, in the order the matrix test runs them.
    pub const ALL: &'static [OperandSite] = &[
        OperandSite::Concat,
        OperandSite::StringHole,
        OperandSite::Binary,
        OperandSite::MatchSubject,
        OperandSite::Argument,
        OperandSite::CallableArgument,
        OperandSite::IndexBase,
        OperandSite::Ordering,
    ];
}

/// Where a borrowed read's storage lives, which decides what can free it.
enum ReadRoot {
    /// A property, a static property, or a staged target: storage code outside
    /// this frame can replace, so any operand that runs code can free it.
    Shared,
    /// A local, or an element chain rooted at one with no property in it:
    /// only an operand that writes this local can free it.
    Local(String),
    /// A binding no expression can write, such as an anonymous function's
    /// self-name.
    Frame,
}

impl Lowering<'_> {
    /// Whether the operand `e`, whose value `v` this frame borrows, stays a
    /// borrow once the expressions in `later` have run.
    ///
    /// `true` means nothing in `later` can free it, and the consumer reads the
    /// slot's own reference. `false` means one of them can: this emits the
    /// retain into `cur`, and the consumer stages `v` exactly as it stages a
    /// fresh value, so its sweep and every landing block release it.
    ///
    /// Asked only of a value the consumer already judged a borrow; every
    /// refcounted value is a pointer, so the retain is always well formed.
    pub(crate) fn hold_borrowed(
        &mut self,
        site: OperandSite,
        e: &Expr,
        v: ValueId,
        later: &[&Expr],
        cur: BlockId,
    ) -> bool {
        // The site names the consumer for the matrix test and for a reader of
        // the IR; the judgment itself is the same at every one of them.
        let _ = site;
        if later.is_empty() || !self.later_can_free(e, later) {
            return true;
        }
        self.emit_retain(cur, v);
        false
    }

    /// The `aliasing` answer [`Self::account_for_arg`] takes for the argument
    /// `e`, lowered to `(v, ty)` and followed by the arguments in `later`.
    ///
    /// A transferred argument is retained as soon as it is read, so it needs
    /// no hold. A borrowed one is held by [`Self::hold_borrowed`] when a later
    /// argument can free it, and is then accounted as a fresh value.
    pub(crate) fn borrowed_arg(
        &mut self,
        site: OperandSite,
        e: &Expr,
        (v, ty): (ValueId, Ty),
        ownership: ArgOwnership,
        later: &[&Expr],
        cur: BlockId,
    ) -> bool {
        if !self.aliasing_read(e) {
            return false;
        }
        if ownership == ArgOwnership::Transferred || !ty.is_refcounted() {
            return true;
        }
        self.hold_borrowed(site, e, v, later, cur)
    }

    /// Whether the borrowed base of `base[key]` is held while `key` runs.
    ///
    /// [`Self::lower_index`] holds it through [`Self::hold_borrowed`], whose
    /// answer is this one, and [`Self::aliasing_read`] reads it to call such
    /// an element read a fresh producer: the element is retained before the
    /// held base is released. Every index base is an array or a tag, so it is
    /// always refcounted and the two never disagree.
    pub(crate) fn index_base_held(&self, base: &Expr, key: &Expr) -> bool {
        self.later_can_free(base, &[key])
    }

    /// Whether an expression in `later` can free the storage `e` reads.
    fn later_can_free(&self, e: &Expr, later: &[&Expr]) -> bool {
        match self.read_root(e) {
            ReadRoot::Shared => later.iter().any(|x| !self.is_inert(x)),
            ReadRoot::Local(name) => later.iter().any(|x| self.writes_local(x, &name)),
            ReadRoot::Frame => false,
        }
    }

    fn read_root(&self, e: &Expr) -> ReadRoot {
        if self.staged(e.span).is_some() {
            return ReadRoot::Shared;
        }
        let e = e.unparenthesized();
        if self.staged(e.span).is_some() {
            return ReadRoot::Shared;
        }
        match &e.kind {
            ExprKind::Variable(span) => {
                ReadRoot::Local(strip_sigil(span_text(self.src, *span)).to_owned())
            }
            ExprKind::Index { base, .. } => self.read_root(base),
            ExprKind::PropertyAccess { .. } | ExprKind::StaticPropertyAccess { .. } => {
                ReadRoot::Shared
            }
            _ if matches!(
                self.exprs.lookup(e.span),
                Some(nvs_types::expr_table::ExprInfo::AnonFnSelf)
            ) =>
            {
                ReadRoot::Frame
            }
            // Any other borrow is judged as shared storage: the safe answer.
            _ => ReadRoot::Shared,
        }
    }

    /// Whether `x` writes the local `name`: an assignment, a compound
    /// assignment, `++`, `--` or an `inout` argument anywhere inside it.
    /// [`Self::collect_reassigned_in_expr`] is the one scan for that, and the
    /// loop header's phis read the same answer.
    fn writes_local(&self, x: &Expr, name: &str) -> bool {
        let mut seen = FxHashSet::default();
        let mut out = Vec::new();
        self.collect_reassigned_in_expr(x, &mut seen, &mut out);
        out.iter().any(|n| n == name)
    }

    /// Whether lowering `x` can run no program code and write no storage, so
    /// that no borrowed read before it can be freed while it runs.
    ///
    /// A staged expression, a literal, a local, and a plain read of a property, a static property
    /// or an element whose base and key are themselves inert. A property with
    /// a `get` hook is a call, which [`Self::aliasing_read`] already tells
    /// apart. Every other kind is listed with the answer `false`, and the
    /// last arm is the `#[non_exhaustive]` tax: a kind added to `nvs-syntax`
    /// is judged able to run code until somebody lists it here.
    pub(crate) fn is_inert(&self, x: &Expr) -> bool {
        // A staged expression was evaluated before the target that names it,
        // and lowering it again reads the staged value and runs nothing.
        if self.staged(x.span).is_some() {
            return true;
        }
        match &x.kind {
            ExprKind::Null
            | ExprKind::Bool(_)
            | ExprKind::Int(_)
            | ExprKind::Float(_)
            | ExprKind::Duration(_)
            | ExprKind::Str(_)
            | ExprKind::Variable(_) => true,
            ExprKind::Paren(inner) => self.is_inert(inner),
            ExprKind::PropertyAccess { object, .. } => {
                self.aliasing_read(x) && self.is_inert(object)
            }
            ExprKind::StaticPropertyAccess { .. } => self.aliasing_read(x),
            ExprKind::Index {
                base,
                index: Some(key),
            } => self.aliasing_read(x) && self.is_inert(base) && self.is_inert(key),
            ExprKind::Index { index: None, .. }
            | ExprKind::Interpolated(_)
            | ExprKind::HtmlTemplate(_)
            | ExprKind::ConstFetch(_)
            | ExprKind::SelfExpr
            | ExprKind::StaticExpr
            | ExprKind::ParentExpr
            | ExprKind::ArrayLiteral(_)
            | ExprKind::Unary { .. }
            | ExprKind::PreIncDec { .. }
            | ExprKind::PostIncDec { .. }
            | ExprKind::Binary { .. }
            | ExprKind::Assign { .. }
            | ExprKind::Ternary { .. }
            | ExprKind::Conversion { .. }
            | ExprKind::TypeTest { .. }
            | ExprKind::Call { .. }
            | ExprKind::MethodCall { .. }
            | ExprKind::StaticCall { .. }
            | ExprKind::ClassConstAccess { .. }
            | ExprKind::ClassNameConst { .. }
            | ExprKind::New { .. }
            | ExprKind::Clone(_)
            | ExprKind::Fn(_)
            | ExprKind::Match { .. }
            | ExprKind::Catch { .. }
            | ExprKind::Yield { .. }
            | ExprKind::YieldFrom(_)
            | ExprKind::Print(_)
            | ExprKind::Throw(_)
            | ExprKind::Isset(_)
            | ExprKind::IsInit(_)
            | ExprKind::Empty(_)
            | ExprKind::Exit(_)
            | ExprKind::SpawnScript { .. }
            | ExprKind::Await(_)
            | ExprKind::Require { .. }
            | ExprKind::AnonObject(_)
            | ExprKind::Error(_) => false,
            _ => false,
        }
    }

    /// The arguments the pieces in `rest` lower, in the order they lower them:
    /// what a `Core\Str::format` argument is read after.
    pub(crate) fn later_format_arguments<'e>(
        rest: &[FormatPiece],
        args: &'e [Arg],
    ) -> Vec<&'e Expr> {
        rest.iter()
            .filter_map(|piece| match piece {
                FormatPiece::Text(_) => None,
                FormatPiece::Argument(argument)
                | FormatPiece::Integer { argument, .. }
                | FormatPiece::Converted { argument, .. }
                | FormatPiece::Rendered { argument, .. } => args.get(*argument),
            })
            .map(|arg| &arg.value)
            .collect()
    }

    /// The expressions of `parts` from index `from` on: the holes a string's
    /// earlier hole is read after.
    pub(crate) fn later_holes(parts: &[StringPart], from: usize) -> Vec<&Expr> {
        parts
            .get(from..)
            .unwrap_or_default()
            .iter()
            .filter_map(|p| match p {
                StringPart::Expr(e) => Some(e),
                StringPart::Text(_) => None,
            })
            .collect()
    }
}
