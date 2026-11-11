//! `rule:expressions/an-operand-keeps-its-value-while-later-operands-run`:
//! the one place that decides whether a borrowed operand needs a reference of
//! its own, given the expressions lowered after it and before its consumer
//! reads it.
//!
//! Part of [`super`]'s one `impl Lowering`, under the rule [`super::expr`]'s
//! header states.
//!
//! A borrowed read is one `Lowering::aliasing_read` judges a borrow: the value is in a durable
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

use nvs_syntax::ast::{Arg, CallArgs, Expr, ExprKind, StringPart};
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
    /// The receiver of a `Core` member, read by the call after the last
    /// argument. A user method's receiver is transferred and retained when
    /// it is read, so it needs no hold.
    Receiver,
    /// The receiver of `$o->$name` or `$o->{expr}`, read by the property
    /// read after the name.
    KeyedReceiver,
    /// The receiver of a property store, `$a->b->c = e`, written after the
    /// value runs. A shape and a keyed store's receiver are the same site.
    StoreReceiver,
    /// The receiver of the array an element store writes, `$o->arr[k] = e`
    /// and `$o->arr[] = e`, and of a compound store's target: the array is
    /// read and written after the keys and the value run.
    StoreBase,
    /// A key of an element store, `$m[$o->key] = e`, or the name of a keyed
    /// property store, stored after the later keys and the value run.
    StoreKey,
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
        OperandSite::Receiver,
        OperandSite::KeyedReceiver,
        OperandSite::StoreReceiver,
        OperandSite::StoreBase,
        OperandSite::StoreKey,
    ];

    /// Whether the site is a store's, which `nvs-codegen`'s `borrowed_stores`
    /// test proves; every other site is `borrowed_reads`'s.
    #[must_use]
    pub fn is_store(self) -> bool {
        matches!(
            self,
            OperandSite::StoreReceiver | OperandSite::StoreBase | OperandSite::StoreKey
        )
    }
}

/// Who owns an operand once [`Lowering::borrowed_operand`] has judged it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum OperandOwner {
    /// A fresh value this frame owns, such as a call's result.
    Fresh,
    /// A borrow the durable slot still owns, and nothing later can free it.
    Slot,
    /// A borrow a later operand could free: retained at its read, so this
    /// frame owns it like a fresh value.
    Held,
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
    /// Whether `e` reads storage some durable slot still owns, so a value
    /// taken from it needs a retain before anything else can own it too —
    /// [`super::is_aliasing_read`]'s syntactic judgment, plus the one case that
    /// judgment cannot make from syntax alone.
    ///
    /// `$obj->prop` *looks* like a slot read at every hooked and unhooked
    /// property alike, but a property with an `rule:classes/property-hooks` `get` hook is a
    /// **call**: its result is a fresh, already-owned value, exactly like any
    /// other call's, and retaining it would leak one reference per read. Only
    /// `nvs_types`' resolution can tell the two apart, which is why this is a
    /// method on the lowering rather than a free function over the AST —
    /// every retain decision in this file goes through it.
    ///
    /// The second case a syntactic judgment cannot make is the *base*: a slot
    /// is only durable if whatever holds it is. `$m->make()->name` reads a
    /// field of an object nothing else owns, so
    /// [`Lowering::lower_property_access`] retains the value it reads and
    /// releases the object underneath it — the whole expression is a fresh
    /// producer, exactly like the call inside it, and a consumer that
    /// retained it again would leak one reference per read. `$a["k"]->name`
    /// is still an aliasing read, because the base is.
    ///
    /// `$m->rows()["0"]` is the same shape over an array's element rather than
    /// an object's slot, and [`Lowering::lower_index`] answers it identically,
    /// so the recursion below covers both spellings of "reads a slot of
    /// something nothing else owns".
    ///
    /// An element read whose key can free its borrowed base is the third
    /// case: [`Lowering::lower_index`] holds the base while the key runs, so
    /// it retains the element and releases the base, exactly as it does for a
    /// fresh base ([`Lowering::index_base_held`]).
    ///
    /// Private to this module, so no consumer judges a read without naming
    /// what follows it: [`Self::sole_operand_borrowed`] is the read with
    /// nothing lowered after it, and [`Self::borrowed_operand`] and
    /// [`Self::borrowed_arg`] judge the rest.
    fn aliasing_read(&self, e: &Expr) -> bool {
        // A staged sub-expression of an assignment target is a borrow of a
        // value this frame's own temporaries stack (or a durable slot) already
        // owns — see `Self::staged_targets`. It answers `true` whatever its
        // syntax was, which is what stops the second read of a rewritten
        // `$t = $t ⊕ e` releasing a receiver the first read still needs.
        if self.staged(e.span).is_some() {
            return true;
        }
        // Parentheses group and never change what an expression *is*
        // (`nvs_syntax::ast::Expr::unparenthesized`), so every question below
        // is asked of what they wrap: `($a)` aliases the local exactly as `$a`
        // does, and answering `false` here for one would make
        // `array<string> $b = ($a);` release the array twice. The staged
        // lookup runs on both spans because a rewritten target's staged
        // sub-expression is recorded under the span it was *written* with.
        let e = e.unparenthesized();
        if self.staged(e.span).is_some() {
            return true;
        }
        // `rule:types/anonymous-function-self-name`'s self-name reads the invoke's own receiver, a binding
        // this frame's `Env` holds for the whole body — so a call through it
        // borrows exactly as `$f(...)` borrows the local `$f`, and answering
        // `false` here would have the call release a receiver the rest of the
        // body still needs. Asked before the syntactic judgment because a bare
        // name is not one of its shapes, and only the checker's record tells
        // this one from the `E0319` every other bare name is.
        if matches!(
            self.exprs.lookup(e.span),
            Some(nvs_types::expr_table::ExprInfo::AnonFnSelf)
        ) {
            return true;
        }
        if !super::is_aliasing_read(&e.kind) {
            return false;
        }
        if matches!(
            self.exprs.lookup(e.span),
            Some(nvs_types::expr_table::ExprInfo::HookedProperty { get: Some(_), .. })
        ) {
            return false;
        }
        // A read through a union where one class has a `get` hook is a fresh
        // producer on every arm (`Self::lower_union_member`).
        if let Some(nvs_types::expr_table::ExprInfo::UnionMember { arms }) =
            self.exprs.lookup(e.span)
            && arms.iter().any(|(_, arm)| super::is_get_hook(arm))
        {
            return false;
        }
        match &e.kind {
            ExprKind::PropertyAccess { object, .. } => self.aliasing_read(object),
            ExprKind::Index { base, index } => {
                self.aliasing_read(base)
                    && index
                        .as_deref()
                        .is_none_or(|key| !self.index_base_held(base, key))
            }
            _ => true,
        }
    }

    /// Whether `e`, the one operand its consumer reads, is a borrow of a
    /// durable slot ([`Self::aliasing_read`]).
    ///
    /// For a consumer that lowers nothing between this read and its use, so
    /// nothing can free the slot while the value is in flight.
    pub(crate) fn sole_operand_borrowed(&self, e: &Expr) -> bool {
        self.aliasing_read(e)
    }

    /// Who owns the operand `e`, lowered to `(v, ty)`, once the expressions in
    /// `later` are judged: [`Self::aliasing_read`], then [`Self::hold_borrowed`].
    ///
    /// A [`OperandOwner::Held`] operand has its retain emitted into `cur`, and
    /// the consumer stages it while `later` runs and treats it as fresh after.
    pub(crate) fn borrowed_operand(
        &mut self,
        site: OperandSite,
        e: &Expr,
        (v, ty): (ValueId, Ty),
        later: &[&Expr],
        cur: BlockId,
    ) -> OperandOwner {
        if !self.aliasing_read(e) {
            return OperandOwner::Fresh;
        }
        if !ty.is_refcounted() || self.hold_borrowed(site, e, v, later, cur) {
            return OperandOwner::Slot;
        }
        OperandOwner::Held
    }

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

    /// Holds the receiver of the slot a store writes while `later` runs, and
    /// stages it, so the store reads and writes the object it began with.
    ///
    /// `target` is the store's target: `$o->p`, or an element path whose root
    /// is `$o->arr`. The receiver is `$o`. It is lowered and retained here only
    /// when it is a borrow a later operand can free, and is then this frame's
    /// temporary: the caller's `temporaries_mark` and `staged_mark` bracket
    /// releases and unstages it. Every other receiver is left to be lowered
    /// where the store reads it, which costs nothing.
    pub(crate) fn hold_store_receiver(
        &mut self,
        site: OperandSite,
        target: &Expr,
        later: &[&Expr],
        env: &mut super::Env,
        cur: &mut BlockId,
    ) {
        let mut root = target.unparenthesized();
        while let ExprKind::Index { base, .. } = &root.kind {
            root = base.unparenthesized();
        }
        let ExprKind::PropertyAccess {
            object,
            nullsafe: false,
            ..
        } = &root.kind
        else {
            return;
        };
        if later.is_empty()
            || self.staged(object.span).is_some()
            || !self.aliasing_read(object)
            || !self.later_can_free(object, later)
        {
            return;
        }
        let (v, ty) = self.lower_expr(object, None, env, cur);
        if ty.is_refcounted() && !self.hold_borrowed(site, object, v, later, *cur) {
            self.own_temporary(v);
        }
        self.stage(object.span, v, ty);
    }

    /// Holds the key `e` of an element store, lowered to `(v, ty)` with its
    /// `aliasing` answer, while `later` runs: the later keys and the value.
    ///
    /// A held key becomes this frame's temporary and is released with the
    /// store's other temporaries. Its `aliasing` answer stays `true`, so the
    /// store still retains the reference the array keeps.
    pub(crate) fn hold_store_key(
        &mut self,
        e: &Expr,
        (v, ty, aliasing): (ValueId, Ty, bool),
        later: &[&Expr],
        cur: BlockId,
    ) {
        if aliasing
            && ty.is_refcounted()
            && !self.hold_borrowed(OperandSite::StoreKey, e, v, later, cur)
        {
            self.own_temporary(v);
        }
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

    /// Whether converting `e`, lowered to `ty`, to a `string` can run program
    /// code: a `toString` the static class resolves to, or the one
    /// `nvs_runtime::stringify` finds on the runtime class of an `object` or
    /// a union. A `Core` class renders in native code and runs none.
    ///
    /// [`Self::is_inert`] judges syntax and calls `$o` inert, so a join asks
    /// this once each piece is lowered, before its conversion runs.
    pub(crate) fn conversion_runs_code(&self, e: &Expr, ty: Ty) -> bool {
        match ty {
            Ty::Tagged => true,
            Ty::Object => self
                .exprs
                .to_string_call(e.span)
                .is_none_or(|call| nvs_types::core_symbol_of(&call.class, &call.method).is_none()),
            _ => false,
        }
    }

    /// Holds every borrowed `string` piece a join has already read, before a
    /// later piece's conversion runs a `toString` that could free it
    /// ([`Self::conversion_runs_code`]).
    ///
    /// Each pair is a piece and whether nothing is staged for it. A held
    /// piece is retained, staged on [`Self::owned_temporaries`] and reported
    /// as staged, so the join's sweep and every landing block release it. A
    /// held leading piece records its slot in `leading`, so the join hands
    /// that reference to `Concat` as it does for a fresh leading piece.
    ///
    /// **What it spends:** one non-atomic increment and one decrement per
    /// borrowed piece, only in a join with an `object` or union piece after it.
    pub(crate) fn hold_earlier_pieces(
        &mut self,
        pieces: &mut [(crate::ir::ConcatPart, bool)],
        leading: &mut Option<usize>,
        cur: BlockId,
    ) {
        use crate::ir::PartKind;
        for (i, (part, borrowed)) in pieces.iter_mut().enumerate() {
            if !*borrowed || !matches!(part.kind, PartKind::Str | PartKind::HtmlText) {
                continue;
            }
            self.emit_retain(cur, part.value);
            if i == 0 {
                *leading = Some(self.temporaries_mark());
            }
            self.own_temporary(part.value);
            *borrowed = false;
        }
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
    pub(crate) fn writes_local(&self, x: &Expr, name: &str) -> bool {
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

    /// The written arguments of `args`, in the order they lower: what a
    /// receiver is read after. A method reference writes none.
    pub(crate) fn call_arg_exprs(args: &CallArgs) -> Vec<&Expr> {
        match args {
            CallArgs::List(list) => list.iter().map(|arg| &arg.value).collect(),
            CallArgs::MethodRef => Vec::new(),
        }
    }
}
