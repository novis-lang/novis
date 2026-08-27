//! Expression lowering — the dispatch, conversions, ADR 0035's truthiness, and the short-circuiting operators.
//!
//! Part of [`super`]'s one `impl Lowering`, split across this directory so a
//! session editing one area does not carry the rest in context. Every item
//! moved here unchanged; the methods are `pub(super)` so they reach across
//! these modules and no further, which is the reach they had when `lower` was
//! a single file.

use super::*;

/// What `mwl_types::expr_table::ExprInfo::ShapeProperty` resolved for one
/// `$shape->field` access, read or write, carried as one argument because the
/// three parts are only ever used together — see
/// [`Lowering::lower_shape_property_access`] and
/// [`Lowering::lower_shape_property_assign`].
pub(super) struct ShapeField {
    /// The field's own name, `$`-sigil not included: what ADR 0036 § 4's
    /// fetch is keyed on.
    pub(super) name: String,
    /// Its position in the *receiver's* sorted shape — the runtime's hint,
    /// and not the answer through a widened view.
    pub(super) slot: u32,
    /// Its declared type, still in `mwl_types`' interner.
    pub(super) ty: TypeId,
}

impl<'a> Lowering<'a> {
    /// The one entry point for lowering an expression, in every position.
    ///
    /// `cur` is redirected to whichever block the expression's own evaluation
    /// ends in -- unchanged unless the expression branched. That is why it is
    /// a `&mut`: `&&`, `||`, `!`, a ternary and `??` each lower to a
    /// branch/merge, and a caller that could not learn the merge block would
    /// go on emitting into a block control has already left. There used to be
    /// a second, "top-level only" entry point that owned the mutable block and
    /// a plain one that did not, which is what made `string $s = $a ?? "d";`
    /// compile while `echo "x=" . ($a ?? "d")` panicked; the two are one
    /// function now, so every position composes.
    ///
    /// `expected` is the representation the position wants where it has one.
    /// It steers a literal (ADR 0007 § 4's `int`/`uint` choice) and nothing
    /// else -- reconciling a mismatch is [`Self::coerce`]'s job, at the
    /// boundary that owns the declared type.
    pub(super) fn lower_expr(
        &mut self,
        expr: &Expr,
        expected: Option<Ty>,
        env: &mut Env,
        cur: &mut BlockId,
    ) -> (ValueId, Ty) {
        // An assignment target's address, already lowered once by
        // `Self::lower_read_modify_write` — answered before the kind is looked
        // at, so `Box::make()->count += 1` runs `make()` once however many
        // times the `$t = $t ⊕ e` rewrite writes the receiver down. See
        // `Self::staged_targets`.
        if let Some((v, ty)) = self.staged(expr.span) {
            return (v, ty);
        }
        match &expr.kind {
            // `(expr)` is fully transparent — `mwl_types::expr::check_expr`'s
            // own `ExprKind::Paren` arm just recurses with the same
            // `expected`, and this does the same for lowering. Needed for
            // `!($a && $b)`-shaped input at all: `!` binds tighter than
            // `&&`/`||` in the grammar, so writing "not (a and b)" requires
            // the explicit parens, which the parser keeps as their own node
            // rather than discarding.
            ExprKind::Paren(inner) => self.lower_expr(inner, expected, env, cur),
            // The four shapes that branch. They sit here, in the one
            // expression-lowering entry point, rather than in a second
            // "top-level only" one — that split was this crate's known gap 5,
            // and it is what made `echo "x=" . ($a ?? "d")` panic while
            // `string $s = $a ?? "d";` compiled. `cur` is redirected to
            // whichever block the expression's own control flow ends in, so
            // every caller composes with them for free.
            ExprKind::Binary {
                op: BinaryOp::And,
                lhs,
                rhs,
            } => (self.lower_and(lhs, rhs, env, cur), Ty::Bool),
            ExprKind::Binary {
                op: BinaryOp::Or,
                lhs,
                rhs,
            } => (self.lower_or(lhs, rhs, env, cur), Ty::Bool),
            ExprKind::Binary {
                op: BinaryOp::Coalesce,
                lhs,
                rhs,
            } => self.lower_coalesce(expr, lhs, rhs, env, cur),
            ExprKind::Ternary { cond, then, else_ } => {
                self.lower_ternary(cond, then.as_deref(), else_, env, cur)
            }
            ExprKind::Match { subject, arms } => {
                self.lower_match(subject, arms, expected, env, cur)
            }
            ExprKind::Bool(b) => self.emit(*cur, Ty::Bool, InstKind::ConstBool(*b)),
            // The literal `null`. Its own type, not a tagged one -- see
            // `Ty::Null`; `Self::coerce` widens it wherever the position it
            // lands in declares `?T`.
            ExprKind::Null => self.emit(*cur, Ty::Null, InstKind::ConstNull),
            ExprKind::Int(span) => self.lower_int_literal(*span, expr, expected, cur),
            ExprKind::Float(span) => self.lower_float_literal(*span, expr, expected, cur),
            ExprKind::Duration(span) => self.lower_duration_literal(*span, env, cur),
            // A fresh `Ty::Str` value with exactly one natural owner — see
            // `Self::bind_local`'s doc comment for why a value produced here
            // never needs a retain of its own, only whatever consumes it.
            ExprKind::Str(span) => {
                let s = cook_str_literal(self.src, *span);
                self.emit(*cur, Ty::Str, InstKind::ConstStr(s))
            }
            // A double-quoted- or heredoc-sourced `Interpolated` both lower
            // to the same `InstKind::Concat` chain a written-out `.`
            // expression already does — see `Self::lower_interpolated_parts`'s
            // own doc comment for the one subtlety plain N-ary `.`-folding
            // wouldn't force into the open on its own, and for how a heredoc's
            // own flexible-indentation strip fits into that fold. Never a
            // nowdoc: `mwl_syntax::parser::collapse_string_parts` only ever
            // reaches `Interpolated` when at least one interpolation site
            // was used, which a nowdoc's body can never contain.
            ExprKind::Interpolated(parts) => {
                let raw = span_text(self.src, expr.span);
                assert!(
                    raw.starts_with('"') || raw.starts_with("<<<"),
                    "mwl-ir only lowers a double-quoted or heredoc-sourced Interpolated string — \
                     got {raw:?}; see the crate docs' known gaps"
                );
                self.lower_interpolated_parts(parts, expr.span, env, cur)
            }
            ExprKind::Variable(span) => {
                let name = strip_sigil(span_text(self.src, *span));
                let &(v, ty) = env.get(name).unwrap_or_else(|| {
                    panic!(
                        "mwl-ir: undeclared local `${name}` — lower_method trusts its input \
                         already passed mwl_types::check_program"
                    )
                });
                // A `&$x` parameter binds an address, not a value: reading it
                // is a load out of the caller-staged slot, at the declared
                // (pointee) type `Self::ref_locals` remembers. See `Ty::Ref`.
                let (v, ty) = if ty == Ty::Ref {
                    let pointee = self.pointee_of(name);
                    self.emit(*cur, pointee, InstKind::RefLoad { slot: v })
                } else {
                    (v, ty)
                };
                // A `?T` local the checker narrowed is read at what it proved,
                // not at the tagged slot it lives in — `Self::untag_narrowed`
                // owns why that happens here and nowhere else.
                self.untag_narrowed(expr.span, v, ty, *cur)
            }
            // `!` always produces `Ty::Bool` via ADR 0035's truthy table,
            // regardless of `inner`'s own type — a separate arm from the plain
            // arithmetic/bitwise unary operators below, which just pass their
            // operand's own type straight through.
            ExprKind::Unary {
                op: AstUnaryOp::Not,
                expr: inner,
            } => (self.lower_not(inner, env, cur), Ty::Bool),
            ExprKind::Unary { op, expr: inner } => {
                self.lower_unary(*op, inner, expected, env, cur)
            }
            ExprKind::Binary {
                op: BinaryOp::Concat,
                lhs,
                rhs,
            } => self.lower_concat(lhs, rhs, env, cur),
            // ADR 0013 § 2: ordering two objects is a `Comparable::compareTo`
            // call, never a comparison of the values themselves — there is no
            // property-walk fallback and nothing else an object `<` could
            // mean. Split out ahead of the general arm below, which would
            // otherwise compare two heap pointers as integers.
            ExprKind::Binary {
                op:
                    op @ (BinaryOp::Lt
                    | BinaryOp::LtEq
                    | BinaryOp::Gt
                    | BinaryOp::GtEq
                    | BinaryOp::Cmp),
                lhs,
                rhs,
            }
                // The discriminator is the recorded `compareTo` target, not
                // the operands' representations: deciding those would mean
                // lowering each operand to find out, and an operand is lowered
                // exactly once.
                if matches!(self.exprs.lookup(expr.span), Some(ExprInfo::Call(_))) =>
            {
                self.lower_object_comparison(*op, expr, lhs, rhs, env, cur)
            }
            ExprKind::Binary {
                op: op @ (BinaryOp::Eq | BinaryOp::NotEq),
                lhs,
                rhs,
            } if matches!(lhs.kind, ExprKind::Null) != matches!(rhs.kind, ExprKind::Null) => {
                self.lower_null_identity(*op, lhs, rhs, env, cur)
            }
            ExprKind::Binary { .. } => self.lower_binary(expr, expected, env, cur),
            ExprKind::Fn(fn_expr) => self.lower_closure_literal(fn_expr, expr, env, cur),
            ExprKind::New { target, args, .. } => self.lower_new(target, args, expr, env, cur),
            ExprKind::MethodCall {
                object,
                nullsafe,
                args,
                ..
            } => self.lower_method_call(object, *nullsafe, args, expr, env, cur),
            ExprKind::StaticCall { class, args, .. } => {
                self.lower_static_call(class, args, expr, env, cur)
            }
            ExprKind::PropertyAccess {
                object, nullsafe, ..
            } => self.lower_property_access(object, *nullsafe, expr, env, cur),
            ExprKind::ArrayLiteral(items) => self.lower_array_literal(items, env, cur),
            ExprKind::ObjectLiteral(fields) => self.lower_object_literal(fields, env, cur),
            ExprKind::Index { base, index } => {
                self.lower_index(base, index.as_deref(), expr, env, cur)
            }
            ExprKind::InstanceOf { expr: inner, .. } => {
                self.lower_instanceof(inner, expr, env, cur)
            }
            ExprKind::Clone(inner) => self.lower_clone_expr(inner, env, cur),
            // Two things wear this syntax, and both are inlined constants.
            // ADR 0010 § 3 makes `EnumName::CaseName` "an integer constant,
            // inlined at every use site"; ADR 0011's `Core\Math::PI` is the
            // same rule for a class constant. So each lowers to exactly the
            // constant a literal would, with no storage, no descriptor and no
            // allocation. `mwl_types` resolved the value — the enum's
            // auto-increment rule for one, `mwl_stdlib::registry`'s own row
            // for the other — into `ExprInfo::EnumCase`/`ExprInfo::CoreConst`;
            // a **user-declared** class's constant records neither and is
            // still unlowered.
            ExprKind::ClassConstAccess { .. } => match self.exprs.lookup(expr.span) {
                Some(ExprInfo::EnumCase { value }) => match value {
                    mwl_types::EnumValue::Int(n) => {
                        self.emit(*cur, Ty::Enum(EnumRepr::Int), InstKind::ConstInt(*n))
                    }
                    mwl_types::EnumValue::Uint(n) => {
                        self.emit(*cur, Ty::Enum(EnumRepr::Uint), InstKind::ConstUint(*n))
                    }
                },
                // The same `ConstArg` an omitted parameter default is
                // materialized from, through the same emitter — a constant is
                // a constant whichever side of the call it was written on.
                Some(ExprInfo::CoreConst { value }) => {
                    let value = value.clone();
                    self.emit_const_arg(&value, env, *cur)
                }
                _ => panic!(
                    "mwl-ir: a `Class::CONST` at {:?} with no resolved enum case or `Core` \
                     constant recorded in the typed-expression table — a user-declared class \
                     constant's value is unmodeled in `mwl_types` (see its own known gaps), so \
                     there is nothing to lower it to",
                    expr.span
                ),
            },
            ExprKind::Conversion { expr: inner, ty } => {
                self.lower_conversion(inner, ty, env, cur)
            }
            // ADR 0007 § 4's `± 1` *as a value*. Both spellings run the same
            // read-modify-write the statement form and `$x += 1;` already go
            // through — so the target's address is computed exactly once here
            // too — and differ only in which of its two values they hand back:
            // the prefix form the one just written, the postfix form the one
            // that was there.
            //
            // Neither owes a retain. Every target that reaches lowering is
            // numeric (`mwl_types`' `E0474` refuses the rest), and
            // `Self::emit_const_one` names the four representations that
            // leaves — `int`, `uint`, `float`, `decimal` — none of which is
            // [`Ty::is_refcounted`].
            ExprKind::PreIncDec { op, expr: target } => {
                let (_, _, new, new_ty) = self.lower_incdec(expr, *op, target, env, cur);
                (new, new_ty)
            }
            ExprKind::PostIncDec { op, expr: target } => {
                let (old, old_ty, _, _) = self.lower_incdec(expr, *op, target, env, cur);
                (old, old_ty)
            }
            // ADR 0007 § 2's assignment *as a value* — `int $b = ($a = 2);`,
            // and the chain `$a = $b = 0;` that is the same thing written
            // right-associatively. See `Self::lower_assign_expr` for what it
            // answers and the one retain it owes; `$a = &$b` is not lowered
            // in any position and falls through to the panic below.
            ExprKind::Assign {
                op,
                target,
                value,
                by_ref: false,
            } => self.lower_assign_expr(expr, *op, target, value, env, cur),
            // PHP's one expression-valued output construct — see
            // `Self::lower_print` for why its answer is always `1`.
            ExprKind::Print(operand) => self.lower_print(operand, env, cur),
            // ADR 0028 § 3's `isset($x)` is `$x != null`, and a list of them
            // is the conjunction — see `Self::lower_isset`.
            ExprKind::Isset(operands) => (self.lower_isset(operands, env, cur), Ty::Bool),
            // `empty($x)` is `!$x` — ADR 0035 § 2's truthy table negated — so
            // it *is* `Self::lower_not`, down to the release a fresh operand
            // owes. `mwl_types::expr::presence` marks its subscripts guarded,
            // which is what makes `empty($a["nope"])` answer `true`.
            ExprKind::Empty(operand) => (self.lower_not(operand, env, cur), Ty::Bool),
            // `Class::$prop` — one load out of the request's own static slot.
            // The written class expression (`self`, `static`, `parent` or a
            // name) is never looked at here: the checker already resolved it
            // to the *declaring* class, which is the storage's identity, and
            // recorded the pair — see `InstKind::StaticGet`.
            ExprKind::StaticPropertyAccess { .. } => self.lower_static_property(expr, cur),
            // `exit`/`exit(...)` — one helper call, see `Self::lower_exit`.
            // The construct is typed `never`, so the value handed back here is
            // unreachable by construction; it exists because this dispatch is
            // total in `(ValueId, Ty)`.
            ExprKind::Exit(arg) => self.lower_exit(arg.as_deref(), env, cur),
            // `$fn(...)` — a closure called through the variable holding it,
            // which is one `Helper::CallClosure` and not a lowered `Call`:
            // ADR 0031 § 1 gives `callable` no parameter list, so there is no
            // resolved target to name. See `Self::lower_closure_call`.
            ExprKind::Call { callee, args } => self.lower_closure_call(callee, args, env, cur),
            other => panic!(
                "mwl-ir's control-flow slice only lowers literals, locals, unary/binary \
                 operators, `new`, a static or instance method call, property access, a static \
                 property, an array literal, an array-element read, `instanceof`, `isset`, \
                 `empty`, an enum case, an increment, an assignment, `print`, `exit`, a call \
                 through a `callable` and an \
                 `as` conversion — got {other:?}; \
                 see the crate docs' known gaps"
            ),
        }
    }
    /// `echo $a, $b;` — writes each operand's bytes to standard output in
    /// order, with no separator and no escaping: `docs/agent/loop-goal.md`
    /// records that ADR 0024 § 5's auto-escaping sink is the HTTP *response*
    /// write, not this one, and that whether `echo` under a future
    /// `mwl serve` becomes that sink is an M7 decision this does not
    /// pre-empt.
    ///
    /// Each operand is converted to [`Ty::Str`] by [`Self::concat_operand`]
    /// — the same shared path `.` concatenation already uses, so a scalar
    /// goes through its own [`Helper`] conversion and a `Stringable`-object
    /// operand panics naming the identical gap — and then handed to one
    /// [`Helper::EchoStr`] [`InstKind::HelperCall`] each. That call defines
    /// no value, so it is pushed with `result: None` rather than emitted
    /// through [`Self::emit`].
    ///
    /// An operand `concat_operand` reports as non-aliasing (a literal, a
    /// nested `Concat`'s own result, or a freshly converted `HelperCall`
    /// result) is released right after the write reads it, since nothing
    /// else ever will — the same "release a fresh value once its one and
    /// only use is done" policy the [`ExprKind::Binary`] concatenation arm
    /// already applies. It goes through [`Self::owned_temporaries`] to get
    /// there, so the write's own failure edge drops it too. No safepoint is
    /// emitted: `echo` is neither of the two reserved sites (function entry,
    /// a loop's back edge).
    pub(super) fn lower_echo(&mut self, operands: &[Expr], cur: &mut BlockId, env: &mut Env) {
        for operand in operands {
            let mark = self.temporaries_mark();
            let (v, aliasing) = self.concat_operand(operand, env, cur);
            if !aliasing {
                self.own_temporary(v);
            }
            // The one conversion-free helper that can genuinely fail: a write
            // to the request's output. See `Inst::on_error` for why the
            // scalar-to-string conversions around it carry no landing block.
            let landing = self.landing_block(env);
            self.block_insts[cur.index() as usize].push(Inst {
                result: None,
                ty: None,
                kind: InstKind::HelperCall {
                    helper: Helper::EchoStr,
                    args: vec![v],
                },
                on_error: Some(landing),
            });
            self.release_temporaries_since(mark, *cur);
        }
    }
    /// `print $x` — one operand written exactly as [`Self::lower_echo`]
    /// writes it, answering the `1` PHP answers.
    ///
    /// PHP's own difference between `print` and `echo` is that `print` is an
    /// *expression*: it takes one operand rather than a list, and its value is
    /// always the integer `1`, which is what makes `$ok && print "…"` and
    /// `$n = print "…"` legal there. `mwl_types::expr` already types it
    /// `int`, so the whole of it here is the write plus that constant — no
    /// second output path, and the same [`Helper::EchoStr`] failure edge.
    ///
    /// A statement-position `print "…";` is this same lowering with the `1`
    /// discarded, which costs one dead [`InstKind::ConstInt`] rather than a
    /// second entry point; `Ty::Int` is not [`Ty::is_refcounted`], so there is
    /// nothing to release either way.
    pub(super) fn lower_print(
        &mut self,
        operand: &Expr,
        env: &mut Env,
        cur: &mut BlockId,
    ) -> (ValueId, Ty) {
        self.lower_echo(std::slice::from_ref(operand), cur, env);
        let (one, _) = self.emit(*cur, Ty::Int, InstKind::ConstInt(1));
        (one, Ty::Int)
    }
    /// `exit;` and `exit(...)` — ADR 0002's status vocabulary, plus one.
    ///
    /// The whole construct is a single [`Helper::Exit`] call whose *success*
    /// is `mwl_runtime::EXITED`: the ordinary status check `mwl-codegen`
    /// emits after it takes this site's error edge, so the frame's live
    /// locals are released in its landing block and the status travels on
    /// through every caller's own check. No `catch` sees it and **no
    /// `finally` runs** — [`crate::ir::Terminator::Catch`] admits only
    /// `THROWN`, and every copy of a `finally` body lives behind it. Both are
    /// PHP's own behaviour, checked against `php -r` rather than assumed;
    /// `docs/adr/README.md` § *Decisions taken at project start* owns the
    /// decision.
    ///
    /// The operand carries PHP's two spellings at once: an `int` is the
    /// process status, and a `string` is a message written first, after which
    /// the status is `0`. `mwl_types::expr` refuses everything else, so the
    /// branch here is on the operand's already-checked representation and
    /// needs no third case.
    ///
    /// What follows the call is dead by construction — the helper never
    /// returns `OK` — but is still lowered, because a `never`-typed
    /// expression has to hand a value back to whatever dispatched to it.
    pub(super) fn lower_exit(
        &mut self,
        arg: Option<&Expr>,
        env: &mut Env,
        cur: &mut BlockId,
    ) -> (ValueId, Ty) {
        let mark = self.temporaries_mark();
        let code = match arg {
            None => self.emit(*cur, Ty::Int, InstKind::ConstInt(0)).0,
            Some(operand) => {
                let (v, ty) = self.lower_expr(operand, None, env, cur);
                if matches!(ty, Ty::Str) {
                    // PHP's `exit("…")`: the message is written exactly the
                    // way `echo` writes it, and the status is `0`.
                    if !self.aliasing_read(operand) {
                        self.own_temporary(v);
                    }
                    let landing = self.landing_block(env);
                    self.block_insts[cur.index() as usize].push(Inst {
                        result: None,
                        ty: None,
                        kind: InstKind::HelperCall {
                            helper: Helper::EchoStr,
                            args: vec![v],
                        },
                        on_error: Some(landing),
                    });
                    self.emit(*cur, Ty::Int, InstKind::ConstInt(0)).0
                } else {
                    v
                }
            }
        };
        let landing = self.landing_block(env);
        self.block_insts[cur.index() as usize].push(Inst {
            result: None,
            ty: None,
            kind: InstKind::HelperCall {
                helper: Helper::Exit,
                args: vec![code],
            },
            on_error: Some(landing),
        });
        self.release_temporaries_since(mark, *cur);
        let (zero, _) = self.emit(*cur, Ty::Int, InstKind::ConstInt(0));
        (zero, Ty::Int)
    }
    /// Whether the checker *placed* the numeric literal at `span` at
    /// `decimal` — ADR 0054 § 2's rule, read back from the one recording
    /// `mwl_types::expr::record_decimal_placement` makes.
    ///
    /// [`Lowering::lower_expr`]'s own `expected` answers the same question
    /// wherever the position's representation reaches this crate, which is
    /// most of them. This covers the positions where it does not: an array
    /// literal's elements, whose type [`Ty::Array`] erases, is the one that
    /// matters today, because a `float` stored where the checker typed a
    /// `decimal` is a *silently* wrong value rather than a loud one.
    fn placed_at_decimal(&self, span: mwl_diagnostics::Span) -> bool {
        self.exprs
            .declared_ty(span)
            .is_some_and(|id| matches!(self.checked_types.get(id), CheckedTy::Decimal))
    }
    /// One binary operator with a [`Ty::Decimal`] operand —
    /// [ADR 0054](../../../docs/adr/0054-decimal-scalar-type.md) § 3's whole
    /// table, as [`Helper`] calls rather than machine instructions.
    ///
    /// Three helpers cover all six comparisons, which is why this is a
    /// rewrite rather than a lookup: `!=` is [`Helper::DecimalEq`] under a
    /// [`UnOp::Not`], and `>`/`>=` are [`Helper::DecimalLt`]/
    /// [`Helper::DecimalLtEq`] with their operands swapped. That is not
    /// merely fewer variants — it is what gives an unordered operand (a
    /// `NaN` on the `float` side of § 3's comparison row) PHP's answer to all
    /// six at once, which a single compare-to-zero result could not encode.
    ///
    /// Neither operand is ever refcounted, so nothing is released here.
    fn lower_decimal_binary(
        &mut self,
        op: BinaryOp,
        lhs: ValueId,
        rhs: ValueId,
        env: &mut Env,
        cur: &mut BlockId,
    ) -> (ValueId, Ty) {
        let (helper, ty, args, negate) = match op {
            BinaryOp::Add => (Helper::DecimalAdd, Ty::Decimal, vec![lhs, rhs], false),
            BinaryOp::Sub => (Helper::DecimalSub, Ty::Decimal, vec![lhs, rhs], false),
            BinaryOp::Mul => (Helper::DecimalMul, Ty::Decimal, vec![lhs, rhs], false),
            BinaryOp::Div => (Helper::DecimalDiv, Ty::Decimal, vec![lhs, rhs], false),
            BinaryOp::Mod => (Helper::DecimalMod, Ty::Decimal, vec![lhs, rhs], false),
            BinaryOp::Eq => (Helper::DecimalEq, Ty::Bool, vec![lhs, rhs], false),
            BinaryOp::NotEq => (Helper::DecimalEq, Ty::Bool, vec![lhs, rhs], true),
            BinaryOp::Lt => (Helper::DecimalLt, Ty::Bool, vec![lhs, rhs], false),
            BinaryOp::Gt => (Helper::DecimalLt, Ty::Bool, vec![rhs, lhs], false),
            BinaryOp::LtEq => (Helper::DecimalLtEq, Ty::Bool, vec![lhs, rhs], false),
            BinaryOp::GtEq => (Helper::DecimalLtEq, Ty::Bool, vec![rhs, lhs], false),
            // The one row here whose answer is neither a `bool` nor a
            // `decimal`: `<=>` is the ordering the four above each ask one
            // question of, returned whole. ADR 0054 § 3 grants it on the same
            // grounds it grants them — an exact comparison is computable
            // across every pairing, including the `decimal`/`float` one
            // arithmetic refuses.
            BinaryOp::Cmp => (Helper::DecimalCmp, Ty::Int, vec![lhs, rhs], false),
            other => panic!(
                "mwl-ir lowers ADR 0054 § 3's arithmetic, equality and ordering operators over \
                 `decimal` — got {other:?}; `**` has no row there, and is the \
                 diagnostic `mwl_types::expr::operators::power_result` reports"
            ),
        };
        let inst = InstKind::HelperCall { helper, args };
        // Only the arithmetic rows can fail: every one of them throws
        // `ArithmeticError` on either overflow kind, and `/` on a zero
        // divisor. A comparison is total.
        let (v, _) = if ty == Ty::Decimal {
            self.emit_fallible(*cur, ty, inst, env)
        } else {
            self.emit(*cur, ty, inst)
        };
        if negate {
            return self.emit(
                *cur,
                Ty::Bool,
                InstKind::UnOp {
                    op: UnOp::Not,
                    operand: v,
                },
            );
        }
        (v, ty)
    }

    /// Lowers one `.` operand and, if it isn't already [`Ty::Str`], converts
    /// it through a new [`InstKind::HelperCall`] — `mwl_types::expr::
    /// check_expr`'s own `require_stringable` already accepts a scalar or a
    /// `Stringable`-implementing object on either side of `.` (PHP-style
    /// implicit stringification); this crate can express the scalar half
    /// today — statically, and through [`Helper::TaggedToString`] for a union
    /// operand whose row only its runtime tag names — and an object operand
    /// through the `toString()` that check itself resolved and recorded for
    /// this very span ([`Self::lower_to_string_call`]).
    ///
    /// Returns the resulting `Ty::Str` value together with whether it
    /// [`is_aliasing_read`] of storage a durable slot still owns. A scalar
    /// conversion is never an aliasing read regardless of where the scalar
    /// itself came from — the `Ty::Str` `HelperCall` produces is always a
    /// brand new buffer with exactly one owner, the conversion result
    /// itself, same as a literal or a call's own result.
    pub(super) fn concat_operand(
        &mut self,
        expr: &Expr,
        env: &mut Env,
        cur: &mut BlockId,
    ) -> (ValueId, bool) {
        let (v, ty) = self.lower_expr(expr, None, env, cur);
        match ty {
            Ty::Str => (v, self.aliasing_read(expr)),
            Ty::Bool | Ty::Int | Ty::Uint | Ty::Float | Ty::Decimal => {
                let helper = match ty {
                    Ty::Bool => Helper::BoolToString,
                    Ty::Int => Helper::IntToString,
                    Ty::Uint => Helper::UintToString,
                    Ty::Float => Helper::FloatToString,
                    Ty::Decimal => Helper::DecimalToString,
                    Ty::Str
                    | Ty::Bytes
                    | Ty::Void
                    | Ty::Null
                    | Ty::Object
                    | Ty::Array
                    | Ty::Tagged
                    | Ty::Enum(_)
                    | Ty::ClassDesc
                    | Ty::Ref => {
                        unreachable!("matched above")
                    }
                };
                let (sv, _) = self.emit(
                    *cur,
                    Ty::Str,
                    InstKind::HelperCall {
                        helper,
                        args: vec![v],
                    },
                );
                (sv, false)
            }
            // A union operand — `mixed`, a `?T`, a `Core` member's
            // `int|float`. The row is picked at runtime from the tag the
            // value already carries, and it can fail, so this is the one
            // conversion here that carries an error edge. The operand itself
            // is refcounted: it is released once the helper has read it,
            // unless a durable slot still owns it.
            Ty::Tagged => {
                let (sv, _) = self.emit_fallible(
                    *cur,
                    Ty::Str,
                    InstKind::HelperCall {
                        helper: Helper::TaggedToString,
                        args: vec![v],
                    },
                    env,
                );
                if !self.aliasing_read(expr) {
                    self.emit_release(*cur, v);
                }
                (sv, false)
            }
            // ADR 0028 § 1's implicit stringification. The call's own result
            // is a fresh `string` with one owner, so it is never an aliasing
            // read — the same answer every scalar row above gives.
            Ty::Object => match self.lower_to_string_call(expr, v, env, *cur) {
                Some(s) => (s, false),
                // No resolved `toString`: a value typed at `Stringable` itself,
                // or a `Core`-owned class, which is where ADR 0088 § 5's sink
                // carrier arrives. Both are decided by the value's *runtime*
                // class rather than its static one, so this is the same
                // tag-dispatched conversion a `Ty::Tagged` operand takes —
                // `mwl_runtime::value_to_string` renders a carrier and throws
                // on every other object, which is a diagnosable program rather
                // than the compiler panic that used to be here.
                None => {
                    let (sv, _) = self.emit_fallible(
                        *cur,
                        Ty::Str,
                        InstKind::HelperCall {
                            helper: Helper::TaggedToString,
                            args: vec![v],
                        },
                        env,
                    );
                    if !self.aliasing_read(expr) {
                        self.emit_release(*cur, v);
                    }
                    (sv, false)
                }
            },
            other => panic!(
                "mwl-ir converts a scalar, an object or a `Ty::Tagged` operand to `string` for \
                 `.` — got {other:?}, which `mwl_types` should already have refused"
            ),
        }
    }

    /// ADR 0028 § 1's implicit `toString()`, for an operand that lowered to a
    /// [`Ty::Object`]. `.`, an interpolated piece, `echo`/`print` and
    /// `as string` all reach it, because
    /// `mwl_types::expr::operators::require_stringable` is the single check
    /// all four go through — so it is also the single place that records the
    /// resolved target, under the operand's own span.
    ///
    /// `None` when nothing was recorded there, which the caller turns into a
    /// panic naming itself: the checker records a target for every object
    /// operand it accepts, so a missing one is a shape it accepted without
    /// resolving rather than anything this crate can lower.
    ///
    /// The call is ordinary in every respect, exactly as
    /// [`Self::lower_object_comparison`]'s `compareTo` is: ADR 0002's error
    /// edge, since a `toString` body may throw like any other, and the same
    /// ownership convention [`Self::lower_call_args`] applies to a receiver —
    /// an aliasing operand is retained here because the callee releases every
    /// refcounted parameter at scope exit, and a fresh one (`echo new Name()`)
    /// transfers the reference it already has. It dispatches on the
    /// receiver's runtime class, so a `toString` overridden in a subclass wins
    /// over the one the static type names.
    fn lower_to_string_call(
        &mut self,
        expr: &Expr,
        receiver: ValueId,
        env: &mut Env,
        cur: BlockId,
    ) -> Option<ValueId> {
        let call = self.exprs.to_string_call(expr.span)?;
        let fallback = call
            .has_body
            .then(|| format!("{}::{}", call.class, call.method));
        let method = call.method.clone();
        if self.aliasing_read(expr) {
            self.emit_retain(cur, receiver);
        }
        let (desc, _) = self.emit(
            cur,
            Ty::ClassDesc,
            InstKind::ClassDescOf { object: receiver },
        );
        let (s, _) = self.emit_fallible(
            cur,
            Ty::Str,
            InstKind::CallVirtual {
                lsb: desc,
                method,
                fallback,
                receiver: Some(receiver),
                args: vec![],
            },
            env,
        );
        Some(s)
    }
    /// `$a < $b` and its four siblings over two objects — ADR 0013 § 2's
    /// `Comparable::compareTo` call, then the comparison of *its* `int`
    /// against zero.
    ///
    /// `<=>` is the call's own result with no second step: `compareTo` already
    /// returns exactly what the spaceship operator means.
    ///
    /// The call is ordinary in every respect — ADR 0002's error edge (a
    /// `compareTo` body may throw like any other), and the same ownership
    /// convention [`Self::lower_call_args`] applies, with the receiver as
    /// parameter 0: an aliasing operand is retained here because the callee
    /// releases every refcounted parameter at scope exit, and a fresh one
    /// (`new Point(1) < $p`) simply transfers the reference it already has.
    ///
    /// It always dispatches on the receiver's runtime class:
    /// `Comparable::compareTo` is a bodiless interface method, so the resolved
    /// declaration names no compiled function — the same `has_body: false`
    /// path an interface method call already takes.
    pub(super) fn lower_object_comparison(
        &mut self,
        op: BinaryOp,
        expr: &Expr,
        lhs: &Expr,
        rhs: &Expr,
        env: &mut Env,
        cur: &mut BlockId,
    ) -> (ValueId, Ty) {
        let Some(ExprInfo::Call(call)) = self.exprs.lookup(expr.span) else {
            unreachable!("the match guard already found this entry")
        };
        let fallback = call
            .has_body
            .then(|| format!("{}::{}", call.class, call.method));
        let method = call.method.clone();
        let (lv, lty) = self.lower_expr(lhs, None, env, cur);
        let (rv, rty) = self.lower_expr(rhs, None, env, cur);
        assert!(
            matches!(lty, Ty::Object) && matches!(rty, Ty::Object),
            "mwl-ir: `mwl_types` recorded a `Comparable::compareTo` target for a comparison \
             whose operands lowered to {lty:?}/{rty:?} rather than two objects"
        );
        for (v, operand) in [(lv, lhs), (rv, rhs)] {
            if self.aliasing_read(operand) {
                self.emit_retain(*cur, v);
            }
        }
        let (desc, _) = self.emit(*cur, Ty::ClassDesc, InstKind::ClassDescOf { object: lv });
        let (ordering, _) = self.emit_fallible(
            *cur,
            Ty::Int,
            InstKind::CallVirtual {
                lsb: desc,
                method,
                fallback,
                receiver: Some(lv),
                args: vec![rv],
            },
            env,
        );
        if op == BinaryOp::Cmp {
            return (ordering, Ty::Int);
        }
        let bop = match op {
            BinaryOp::Lt => BinOp::Lt,
            BinaryOp::LtEq => BinOp::LtEq,
            BinaryOp::Gt => BinOp::Gt,
            _ => BinOp::GtEq,
        };
        let (zero, _) = self.emit(*cur, Ty::Int, InstKind::ConstInt(0));
        self.emit(
            *cur,
            Ty::Bool,
            InstKind::BinOp {
                op: bop,
                lhs: ordering,
                rhs: zero,
            },
        )
    }
    /// Lowers one `expr as T` — ADR 0007 § 2's conversion table, plus
    /// ADR 0010 § 5's two enum rows.
    ///
    /// Three shapes of row exist, and this slice implements the first two:
    ///
    /// * **Free.** The two representations are identical, so nothing runs. A
    ///   conversion to the same representation is the operand itself; an enum
    ///   to its own backing `int`/`uint` is an [`InstKind::Reinterpret`],
    ///   which ADR 0010 § 5 spells out as "total, free ... same
    ///   representation, reinterpreted." ADR 0009 § 3's `string as bytes` is
    ///   the third one, free for the same reason: one `MwlStr` allocation
    ///   under two tags, minus the UTF-8 promise.
    /// * **Total.** A scalar to `string` reuses the same [`Helper`]
    ///   conversions `.` concatenation already goes through
    ///   ([`Self::concat_operand`]), and any value to `bool` reuses ADR 0035's
    ///   truthy table ([`Self::truthy_convert`]) — `as bool` is the explicit
    ///   spelling of exactly the test a condition applies implicitly, so
    ///   giving it a second table would be two answers to one question.
    /// * **Widening.** The target admits more than one runtime shape and is
    ///   therefore [`Ty::Tagged`], so the value travels unchanged under a tag
    ///   — one [`InstKind::Tag`], free in the same sense the free rows are.
    /// * **Checked.** `int` ↔ `uint`, `float` → an integer, `string` → a
    ///   number and ADR 0009 § 3's `bytes as string` each go through a
    ///   [`Helper`] that either produces the value or throws, emitted through
    ///   [`Self::emit_fallible`] so it carries ADR 0002's error edge like any
    ///   other call.
    /// * **Into an enum.** ADR 0010 § 5's other direction is row 1 run
    ///   backwards: the operand is converted to the enum's *backing* scalar
    ///   through whichever row above applies, and a free
    ///   [`InstKind::Reinterpret`] puts the tag back on.
    ///
    /// That last row does **not** emit the section's "throws on a value no
    /// case names" itself, and cannot: the case set lives on the enum's
    /// declaration, which [`Ty::Enum`] has already erased to a backing type by
    /// the time this runs. [`Self::lower_conversion`] emits it instead — the
    /// same membership chain ADR 0047 § 5's closed set gets, built from every
    /// case of the declaration ([`Self::closed_literal_set`]) — and it is this
    /// function's only caller, so the two halves cannot come apart.
    ///
    /// `operand` is the un-lowered source expression, used only to decide
    /// whether a refcounted operand this conversion consumed was borrowed
    /// storage or a fresh value nothing else will release — the same
    /// [`is_aliasing_read`] judgment [`Self::concat_operand`]'s caller makes.
    pub(super) fn convert(
        &mut self,
        v: ValueId,
        from: Ty,
        to: Ty,
        operand: &Expr,
        env: &mut Env,
        cur: BlockId,
    ) -> (ValueId, Ty) {
        if from == to {
            // Nothing runs, but the *ownership* still has to come out right:
            // every consumer of a conversion expression reads `is_aliasing_read`
            // off the `as` node, which reports it as a fresh value the consumer
            // owns — so a free row that hands back borrowed storage gives the
            // local, the argument or the returned value a reference nobody
            // took, and the second release of the pair corrupts the heap. One
            // retain makes the free row honour the contract every other row
            // already does. `$s as string` is the shape this was always true
            // of; ADR 0047 § 5's erasure made `$s as "a"|"b"` a second one.
            if to.is_refcounted() && self.aliasing_read(operand) {
                self.emit_retain(cur, v);
            }
            return (v, to);
        }
        match (from, to) {
            // ADR 0010 § 5, row 1 — an enum to its own underlying type.
            (Ty::Enum(EnumRepr::Int), Ty::Int) | (Ty::Enum(EnumRepr::Uint), Ty::Uint) => {
                self.emit(cur, to, InstKind::Reinterpret { operand: v })
            }
            // ADR 0010 § 5, row 2 — the underlying type back into the enum,
            // and free for the same reason row 1 is: `Ty::Enum` is a
            // zero-byte tag over that integer, so the tag costs one
            // reinterpret and no test.
            //
            // An operand that is not already the backing scalar is converted
            // to it by the rows below *first*, by recursion rather than by a
            // row per source: `$f as Rank` is ADR 0007 § 2's checked
            // `float → int` and then this, which is the same two steps the
            // author wrote and keeps every one of those rows' throw messages
            // naming the conversion that actually failed. A `Ty::Tagged`
            // operand is one of those sources — `$any as Mode` is
            // `Helper::TaggedToInt` and then this — so `mixed` reaches an enum
            // through the same two steps every other source does.
            //
            // The value is not tested against the declaration's cases here;
            // see this function's own doc comment for where that happens and
            // why it cannot happen at this point.
            (_, Ty::Enum(repr)) => {
                let backing = match repr {
                    EnumRepr::Int => Ty::Int,
                    EnumRepr::Uint => Ty::Uint,
                };
                let (backed, _) = self.convert(v, from, backing, operand, env, cur);
                self.emit(cur, to, InstKind::Reinterpret { operand: backed })
            }
            // ADR 0009 § 3's total row: `string as bytes` is free, because a
            // `bytes` *is* the `string`'s allocation minus the UTF-8 promise
            // (`Ty::Bytes`, and `mwl_runtime::Value::bytes`). Valid UTF-8 is
            // already a valid byte sequence, so there is nothing to check and
            // nothing to copy — one `Reinterpret`, exactly as ADR 0010 § 5's
            // enum row above, and the tag only differs where a `Ty::Tagged`
            // value is built.
            //
            // The ownership is the `from == to` branch's, for its reason: a
            // consumer reads `is_aliasing_read` off the `as` node and owns
            // what it gets, so borrowed storage handed straight back needs the
            // one retain that makes this row honour the same contract every
            // helper row does.
            (Ty::Str, Ty::Bytes) => {
                if self.aliasing_read(operand) {
                    self.emit_retain(cur, v);
                }
                self.emit(cur, to, InstKind::Reinterpret { operand: v })
            }
            // **Widening.** The target admits more than one runtime shape, so
            // it is `Ty::Tagged` and the value keeps the payload it already
            // has under a tag — one `InstKind::Tag`, the same instruction
            // `Self::coerce` emits where a *declaration* is the wider side.
            // ADR 0047 § 5's heterogeneous set is the shape that needs it
            // (`$s as 1|"a"`: the set is closed, its members share no one
            // representation, so the whole target erases to a tagged value
            // and the membership test below runs on tags), and `as mixed` is
            // the same row written plainly.
            //
            // The ownership is the `from == to` branch's, for its reason:
            // `Tag` transfers its operand's reference to its result, so
            // borrowed storage handed through it owes the one retain that
            // makes this row honour the contract every helper row does.
            (_, Ty::Tagged) => {
                if from.is_refcounted() && self.aliasing_read(operand) {
                    self.emit_retain(cur, v);
                }
                self.emit(cur, to, InstKind::Tag { operand: v })
            }
            (_, Ty::Bool) => {
                let b = self.truthy_convert(v, from, cur);
                if from.is_refcounted() && !self.aliasing_read(operand) {
                    self.emit_release(cur, v);
                }
                (b, Ty::Bool)
            }
            (Ty::Bool | Ty::Int | Ty::Uint | Ty::Float | Ty::Decimal, Ty::Str) => {
                let helper = match from {
                    Ty::Bool => Helper::BoolToString,
                    Ty::Int => Helper::IntToString,
                    Ty::Uint => Helper::UintToString,
                    // ADR 0054 § 4's row: total, and scale-preserving.
                    Ty::Decimal => Helper::DecimalToString,
                    _ => Helper::FloatToString,
                };
                self.emit(
                    cur,
                    Ty::Str,
                    InstKind::HelperCall {
                        helper,
                        args: vec![v],
                    },
                )
            }
            // ADR 0028 § 1's row: `as string` is the explicit spelling of the
            // same implicit conversion `.` and `echo` apply, so it goes
            // through the same resolved `toString()` rather than a second
            // answer of its own (`Self::lower_to_string_call`). The receiver's own
            // ownership is settled there too, so nothing is released here.
            (Ty::Object, Ty::Str) => {
                let Some(s) = self.lower_to_string_call(operand, v, env, cur) else {
                    panic!(
                        "mwl-ir converts an object to `string` through the `toString` \
                         `mwl_types::expr::operators::require_stringable` resolved for it, and \
                         none was recorded at this span — a value typed at `Stringable` itself, \
                         or a `Core`-owned class, are the two shapes still outside it; see the \
                         crate docs' known gaps"
                    )
                };
                (s, Ty::Str)
            }
            // The same four rows again, from a union operand — one fallible
            // helper picking by runtime tag, shared verbatim with `.` and
            // `echo` (`Self::concat_operand`). See `Helper::TaggedToString`.
            (Ty::Tagged, Ty::Str) => {
                let out = self.emit_fallible(
                    cur,
                    Ty::Str,
                    InstKind::HelperCall {
                        helper: Helper::TaggedToString,
                        args: vec![v],
                    },
                    env,
                );
                if !self.aliasing_read(operand) {
                    self.emit_release(cur, v);
                }
                out
            }
            // ADR 0007 § 2's checked rows. Each either produces the value or
            // throws, so each is a fallible helper carrying ADR 0002's error
            // edge — the same call shape a method call already has. The
            // operand is a scalar in every one of these except the `string`
            // rows, whose operand is released once the helper has read it if
            // nothing else owns it (`Self::concat_operand`'s caller's policy).
            (Ty::Int, Ty::Uint)
            | (Ty::Uint, Ty::Int)
            | (Ty::Int | Ty::Uint, Ty::Float)
            | (Ty::Float, Ty::Int | Ty::Uint)
            | (Ty::Str, Ty::Int | Ty::Uint | Ty::Float)
            // ADR 0054 § 4's rows. `→ decimal` is one helper for every source
            // (including `Ty::Tagged`, whose row only its runtime tag names),
            // the same "one tag per target" arrangement `Helper::ToIntOrNull`
            // already follows; the three out of `decimal` are per-target, like
            // every other row here.
            | (Ty::Int | Ty::Uint | Ty::Float | Ty::Str | Ty::Tagged, Ty::Decimal)
            | (Ty::Decimal, Ty::Int | Ty::Uint | Ty::Float)
            // ADR 0007 § 6's `mixed`: the three numeric targets, each one
            // helper for every source because only the operand's runtime tag
            // names a row — the same arrangement `Ty::Tagged`'s `string` and
            // `decimal` targets above already use. Each throws where
            // `Helper::ToIntOrNull` answers `null`, over one shared row set in
            // `mwl_runtime`.
            | (Ty::Tagged, Ty::Int | Ty::Uint | Ty::Float) => {
                let helper = match (from, to) {
                    (_, Ty::Decimal) => Helper::ToDecimal,
                    (Ty::Decimal, Ty::Int) => Helper::DecimalToInt,
                    (Ty::Decimal, Ty::Uint) => Helper::DecimalToUint,
                    (Ty::Decimal, _) => Helper::DecimalToFloat,
                    (Ty::Int, Ty::Uint) => Helper::IntToUint,
                    (Ty::Uint, Ty::Int) => Helper::UintToInt,
                    (Ty::Int, _) => Helper::IntToFloat,
                    (Ty::Uint, _) => Helper::UintToFloat,
                    (Ty::Float, Ty::Int) => Helper::FloatToInt,
                    (Ty::Float, _) => Helper::FloatToUint,
                    (Ty::Tagged, Ty::Int) => Helper::TaggedToInt,
                    (Ty::Tagged, Ty::Uint) => Helper::TaggedToUint,
                    (Ty::Tagged, _) => Helper::TaggedToFloat,
                    (_, Ty::Int) => Helper::StrToInt,
                    (_, Ty::Uint) => Helper::StrToUint,
                    _ => Helper::StrToFloat,
                };
                let out = self.emit_fallible(
                    cur,
                    to,
                    InstKind::HelperCall {
                        helper,
                        args: vec![v],
                    },
                    env,
                );
                if from.is_refcounted() && !self.aliasing_read(operand) {
                    self.emit_release(cur, v);
                }
                out
            }
            // ADR 0009 § 3's checked row, and the half of that pair that runs
            // anything: the buffer is validated as well-formed UTF-8 and
            // becomes the `string` over the same allocation, or it throws.
            // Never a replacement character and never a truncation, so it is
            // fallible like every other checked row and carries ADR 0002's
            // error edge.
            //
            // Its own operand is refcounted, so it follows the string rows'
            // policy exactly: released once the helper has read it unless a
            // durable slot still owns it.
            (Ty::Bytes, Ty::Str) => {
                let out = self.emit_fallible(
                    cur,
                    Ty::Str,
                    InstKind::HelperCall {
                        helper: Helper::BytesToString,
                        args: vec![v],
                    },
                    env,
                );
                if !self.aliasing_read(operand) {
                    self.emit_release(cur, v);
                }
                out
            }
            _ => panic!(
                "mwl-ir lowers ADR 0007 § 2's scalar conversion rows, ADR 0009 § 3's `string` ↔ \
                 `bytes` pair, both of ADR 0010 § 5's enum ones, a `Ty::Tagged` operand into \
                 every target among them, and every operand into a tagged target — got \
                 `{from:?} as {to:?}`. ADR 0007 § 2's \
                 `array<T> as array<U>` is the shape still missing, along with a `Ty::Tagged` \
                 operand converted to `bytes`, whose runtime-tag row has no helper. See the \
                 crate docs' known gaps"
            ),
        }
    }
    /// Lowers one `expr as ?T` —
    /// [ADR 0066](../../../docs/adr/0066-nullable-conversion-operator.md)
    /// § 1's non-throwing form of [`Self::convert`], where `to` is the target
    /// *inside* the `?`.
    ///
    /// One [`InstKind::HelperCall`] per target type, and no error edge: the
    /// helper answers `null` where the throwing row would throw, so it cannot
    /// fail and needs neither [`Self::emit_fallible`] nor a landing block. The
    /// result is [`Ty::Tagged`] — the one representation `?T` has
    /// ([`Ty::Tagged`]'s own doc comment) — and every source is one helper,
    /// because the helper dispatches on the operand's runtime tag rather than
    /// on a statically chosen row. That is what makes § 2's "a `null` operand
    /// yields `null`" and § 3's "from `mixed` every target has a checked path"
    /// need no branch here: a [`Ty::Tagged`] operand is already the `Value`
    /// the helper reads.
    ///
    /// Ownership matches the checked rows exactly: a refcounted operand this
    /// conversion consumed is released once the helper has read it, unless a
    /// durable slot still owns it ([`is_aliasing_read`]). The result never
    /// carries a refcounted payload — every target is a scalar — so nothing is
    /// retained.
    ///
    /// # Panics
    ///
    /// Panics naming ADR 0066 § 3 for the two shapes that ADR makes a compile
    /// error and `mwl_types` does not yet refuse: a conversion that **cannot
    /// fail** (`$i as ?int`, `$i as ?string`), which the target-type match and
    /// the `from == to` guard catch between them, and a target with no
    /// conversion at all. An enum or literal-type target is that ADR's other
    /// available form and is not built either — it needs the case set
    /// [`Self::convert`]'s own gap already names.
    pub(super) fn convert_or_null(
        &mut self,
        v: ValueId,
        from: Ty,
        to: Ty,
        operand: &Expr,
        cur: BlockId,
    ) -> (ValueId, Ty) {
        let helper = match to {
            _ if from == to => panic!(
                "mwl-ir: `{from:?} as ?{to:?}` converts a value to the representation it already \
                 has, which cannot fail — ADR 0066 § 3 makes that a compile error naming `as T`, \
                 and `mwl_types` does not refuse it yet"
            ),
            Ty::Int => Helper::ToIntOrNull,
            Ty::Uint => Helper::ToUintOrNull,
            Ty::Float => Helper::ToFloatOrNull,
            Ty::Decimal => Helper::ToDecimalOrNull,
            other => panic!(
                "mwl-ir lowers ADR 0066's `as ?T` for the checked numeric targets — got \
                 `{from:?} as ?{other:?}`. A total conversion (`as ?string`, `as ?bool`) is that \
                 ADR § 3's \"cannot fail\" row and a compile error; an enum or literal-type \
                 target is its available form still missing here, blocked on the same case set \
                 `Lowering::convert`'s own gap names. See the crate docs' known gaps"
            ),
        };
        let out = self.emit(
            cur,
            Ty::Tagged,
            InstKind::HelperCall {
                helper,
                args: vec![v],
            },
        );
        if from.is_refcounted() && !self.aliasing_read(operand) {
            self.emit_release(cur, v);
        }
        out
    }
    /// Converts an already-lowered `(v, ty)` pair through ADR 0035's truthy
    /// table, with no ownership decision attached — see [`Self::truthy_value`]
    /// for the usual "release a fresh, non-aliasing refcounted operand once
    /// its truthy test is done" wrapper every caller but
    /// [`Self::lower_ternary`]'s elvis arm wants; elvis needs the bare
    /// conversion on its own, since its truthy-path *value* is `v` itself
    /// (PHP only evaluates a `?:` condition once) and releasing it here would
    /// use-after-free that reuse.
    ///
    /// `Ty::Bool` passes straight through; `Ty::Int`/`Ty::Uint`/`Ty::Float`/
    /// `Ty::Decimal`/`Ty::Str`/`Ty::Bytes` each convert through their own
    /// [`Helper`] variant; [`Ty::Array`]
    /// converts through [`Helper::ArrayTruthy`] (falsy iff empty, ADR 0035's
    /// table); and [`Ty::Object`] — a class instance or an enum case — needs
    /// no helper at all, since ADR 0035 § 4 makes either always truthy: this
    /// folds straight to a fresh [`InstKind::ConstBool`] `true` rather than
    /// emitting a call with nothing to inspect at runtime.
    ///
    /// [`Ty::Tagged`] — a `mixed`, a union, or a `?T` no test narrowed — is
    /// the one row this table does **not** settle here: it converts through
    /// [`Helper::ValueTruthy`], which reads the value's tag and applies
    /// whichever of the rows above it names. That is ADR 0035 § 2's own last
    /// line rather than a fallback, and it is why ADR 0007 § 2 can make
    /// `mixed` the one unchecked position without a condition being a hole in
    /// it: the question a condition asks has an answer for every tag.
    ///
    /// # Panics
    ///
    /// Panics naming the case for anything outside this table, which today is
    /// `Ty::Void` alone — ADR 0007 already keeps `void`/`never` out of value
    /// position, so no program reaches it. [`Ty::Null`] *is* in the table and
    /// is reachable only from the literal `null`: a `?T` is one
    /// [`Ty::Tagged`] slot and takes that row instead.
    pub(super) fn truthy_convert(&mut self, v: ValueId, ty: Ty, cur: BlockId) -> ValueId {
        match ty {
            Ty::Bool => v,
            Ty::Int | Ty::Uint | Ty::Float | Ty::Decimal | Ty::Str | Ty::Bytes => {
                let helper = match ty {
                    Ty::Int => Helper::IntTruthy,
                    Ty::Uint => Helper::UintTruthy,
                    Ty::Float => Helper::FloatTruthy,
                    Ty::Decimal => Helper::DecimalTruthy,
                    Ty::Str => Helper::StrTruthy,
                    Ty::Bytes => Helper::BytesTruthy,
                    Ty::Bool
                    | Ty::Void
                    | Ty::Null
                    | Ty::Object
                    | Ty::Array
                    | Ty::Tagged
                    | Ty::Enum(_)
                    | Ty::ClassDesc
                    | Ty::Ref => {
                        unreachable!("matched above")
                    }
                };
                self.emit(
                    cur,
                    Ty::Bool,
                    InstKind::HelperCall {
                        helper,
                        args: vec![v],
                    },
                )
                .0
            }
            Ty::Array => {
                self.emit(
                    cur,
                    Ty::Bool,
                    InstKind::HelperCall {
                        helper: Helper::ArrayTruthy,
                        args: vec![v],
                    },
                )
                .0
            }
            // A class instance or an enum case — ADR 0035 § 4, always
            // truthy, nothing to inspect at runtime. The enum arm is the whole
            // reason `Ty::Enum` is a representation of its own rather than the
            // backing integer it is made of: `Rank::Bronze` is backed by `0`
            // and is still `true` here, where a plain `int` `0` goes through
            // `Helper::IntTruthy` and comes back `false`.
            Ty::Object | Ty::Enum(_) => self.emit(cur, Ty::Bool, InstKind::ConstBool(true)).0,
            // ADR 0035 § 2's first row, reachable only from the *literal*
            // `null` — a `?T` is one `Ty::Tagged` slot and goes through the
            // arm below. `empty(null)`, `!null` and `if (null)` are the three
            // spellings that get here, and nothing about the value needs
            // reading to answer them.
            Ty::Null => self.emit(cur, Ty::Bool, InstKind::ConstBool(false)).0,
            // ADR 0035 § 2's last row, and the one this table answers at run
            // time rather than at compile time: a `mixed`, a union or a `?T`
            // no test narrowed carries its row in its tag, so the dispatch
            // moves into `Helper::ValueTruthy` and the arms above become the
            // cases where a static type already picked one.
            Ty::Tagged => {
                self.emit(
                    cur,
                    Ty::Bool,
                    InstKind::HelperCall {
                        helper: Helper::ValueTruthy,
                        args: vec![v],
                    },
                )
                .0
            }
            other => panic!(
                "mwl-ir's truthy-condition slice only converts a `bool`, a scalar, `null`, \
                 `Ty::Array`, `Ty::Object` or a tagged value — got {other:?}; see the crate \
                 docs' known gaps"
            ),
        }
    }
    /// [`Self::truthy_convert`] plus the ownership half every truthy-tested
    /// position but elvis wants: a refcounted operand (`Ty::Str`/`Ty::Array`)
    /// that isn't [`is_aliasing_read`] — a fresh call/`new`/literal result
    /// whose only use is this truthy test — is released right after it's
    /// read, the same "release a fresh value once its one and only use is
    /// done" precedent [`Self::concat_operand`]'s own caller already sets for
    /// `.` concatenation; an aliasing read (a bare variable, a
    /// compile-time-known property or array-element read) still durably
    /// belongs to whatever slot it came from and needs no release here.
    pub(super) fn truthy_value(
        &mut self,
        v: ValueId,
        ty: Ty,
        is_alias: bool,
        cur: BlockId,
    ) -> ValueId {
        let cond_v = self.truthy_convert(v, ty, cur);
        if ty.is_refcounted() && !is_alias {
            self.emit_release(cur, v);
        }
        cond_v
    }
    /// Lowers `cond` — an `if`/`while` condition, or `&&`/`||`'s own operand
    /// (see [`Self::lower_and`]/[`Self::lower_or`]) — through
    /// [`Self::lower_expr`] (so a nested `&&`/`||`/`!`/ternary composes,
    /// e.g. `if ($a && $b)`) and then [`Self::truthy_value`]'s table.
    /// `*cur` is updated to whichever block `cond`'s own evaluation ends in —
    /// unchanged unless `cond` itself needed to branch.
    ///
    /// # Panics
    ///
    /// See [`Self::truthy_convert`]'s own panic doc — the same restriction
    /// applies here.
    pub(super) fn lower_truthy_cond(
        &mut self,
        cond: &Expr,
        env: &mut Env,
        cur: &mut BlockId,
    ) -> ValueId {
        let (v, ty) = self.lower_expr(cond, None, env, cur);
        self.truthy_value(v, ty, self.aliasing_read(cond), *cur)
    }
    /// `$a ?? $b` — the right operand is evaluated only when the left one is
    /// `null`, which is one [`InstKind::IsNull`] and the same branch/phi shape
    /// [`Self::lower_ternary`] uses.
    ///
    /// Both types come from the checker's own `ExprInfo::Coalesce` entry, and
    /// have to: the left operand's representation is [`Ty::Tagged`] by the
    /// time it is lowered, so nothing here could work out on its own which
    /// representation the non-`null` arm holds. That variant's doc comment
    /// owns why.
    ///
    /// # Ownership
    ///
    /// The non-`null` arm's [`InstKind::Untag`] *transfers* whatever the
    /// tagged value owned, so a left operand this expression built needs no
    /// release on either arm — on the `null` arm it owns nothing by
    /// definition, and on the other the narrowed value has taken it over. A
    /// left operand read out of storage someone else owns
    /// ([`is_aliasing_read`]) is retained on the non-`null` arm, exactly as
    /// [`Self::lower_ternary`] retains an aliasing branch value.
    ///
    /// # Panics
    ///
    /// Panics if the checker recorded no `ExprInfo::Coalesce` for this
    /// expression, which would mean it was checked with a different table.
    /// Opens the `null` guard `?->` needs, having first lowered the receiver
    /// itself — the shared front half of a nullsafe method call and a
    /// nullsafe property read, closed again by [`Self::close_nullsafe`].
    ///
    /// Returns the value the member access should use as its receiver, that
    /// value's representation, and the guard to close — with `cur` left
    /// pointing at the block the member access must be emitted into.
    ///
    /// A receiver whose representation is not [`Ty::Tagged`] cannot hold
    /// `null` at runtime, so no guard is opened at all and `?->` lowers to
    /// exactly what `->` does: the same short-circuit
    /// [`Self::lower_coalesce`] applies to a left operand that cannot be
    /// `null`, and the reason `mwl_types` gives such an access no `null` in
    /// its type either.
    ///
    /// The narrowing [`InstKind::Untag`] cannot fail — the branch above it
    /// already ruled the `null` tag out, and a receiver's only other shape is
    /// an object. Ownership rides along with it (see [`Self::coerce`]), so
    /// the member access's own aliasing retain still applies exactly once, to
    /// the untagged value it now sees.
    ///
    /// That last paragraph is what `proof` selects. A
    /// [`ReceiverProof::Erased`] receiver has no proven tag at all — it is a
    /// `mixed`, ADR 0007 § 2's one unchecked position — so no `Untag` is
    /// emitted for it and the *tagged* value is handed back for
    /// [`InstKind::SlotGet`] to check at run time. Ownership is unchanged
    /// either way: a tagged value is refcounted ([`Ty::is_refcounted`]) and
    /// its release is the same release, tag-dispatched.
    pub(super) fn open_nullsafe(
        &mut self,
        object: &Expr,
        nullsafe: bool,
        proof: ReceiverProof,
        env: &mut Env,
        cur: &mut BlockId,
    ) -> (ValueId, Ty, Option<NullsafeGuard>) {
        let (object_v, object_ty) = self.lower_expr(object, None, env, cur);
        if object_ty != Ty::Tagged {
            return (object_v, object_ty, None);
        }
        if !nullsafe {
            let (v, ty) = match proof {
                ReceiverProof::Proven => self.untag_receiver(object_v, object_ty, *cur),
                ReceiverProof::Erased => (object_v, object_ty),
            };
            return (v, ty, None);
        }
        let is_null = self
            .emit(*cur, Ty::Bool, InstKind::IsNull { operand: object_v })
            .0;
        let null_block = self.new_block();
        let member_block = self.new_block();
        let merge_block = self.new_block();
        let null_edge = self.ids.next_edge(object.span);
        let member_edge = self.ids.next_edge(object.span);
        self.seal(
            *cur,
            Terminator::Branch {
                cond: is_null,
                then_block: null_block,
                then_edge: null_edge,
                else_block: member_block,
                else_edge: member_edge,
            },
        );
        *cur = member_block;
        let (receiver, receiver_ty) = match proof {
            ReceiverProof::Proven => (
                self.emit(
                    member_block,
                    Ty::Object,
                    InstKind::Untag { operand: object_v },
                )
                .0,
                Ty::Object,
            ),
            ReceiverProof::Erased => (object_v, Ty::Tagged),
        };
        (
            receiver,
            receiver_ty,
            Some(NullsafeGuard {
                null_block,
                merge_block,
                pre_env: env.clone(),
            }),
        )
    }
    /// The receiver a plain `->` reaches a member through, when its slot
    /// holds a [`Ty::Tagged`] value.
    ///
    /// A `?T` local is one slot wide whatever a condition later proves about
    /// it, so `if ($m !== null) { $m->text(); }` reads a tagged value and
    /// hands it to a call that wants an object. The [`InstKind::Untag`] here
    /// is what makes that the object again, and it is **unchecked on
    /// purpose**: `mwl_types::locals`' narrowing is what proves the tag, the
    /// same way the `?->` branch above proves it with a test. Reaching here
    /// with an un-narrowed receiver is impossible — the checker either
    /// refuses it (`E0459`) or records no resolved target at all, and the
    /// member arms panic on the missing entry before this is called.
    ///
    /// Ownership is unchanged, exactly as in the `?->` arm: the untagged
    /// value owns the reference the tagged one owned, and the aliasing retain
    /// each member access already emits applies to it once.
    pub(super) fn untag_receiver(&mut self, v: ValueId, ty: Ty, cur: BlockId) -> (ValueId, Ty) {
        if ty != Ty::Tagged {
            return (v, ty);
        }
        (
            self.emit(cur, Ty::Object, InstKind::Untag { operand: v }).0,
            Ty::Object,
        )
    }

    /// Narrows a read of a `?T` local a dominating `!= null` test proved
    /// non-`null`, down to the representation of what it proved.
    ///
    /// A `?T` local is one [`Ty::Tagged`] slot wide whatever the checker later
    /// proves about it, so *every* consumer of such a read — a subscript base,
    /// a `foreach` subject, an array-write root, a call argument, a receiver —
    /// would otherwise have to narrow for itself, and one forgotten site is a
    /// cranelift rejection rather than a panic. `mwl_types` therefore records
    /// the narrowing on the read's own span
    /// (`mwl_types::expr_table::ExprInfo::NarrowedRead`) and it is discharged
    /// **once, here**, where the value is produced — which is what leaves no
    /// site to forget. [`Self::untag_receiver`] is the same move written for
    /// the one consumer that predates this, and is a no-op once this has run.
    ///
    /// The [`InstKind::Untag`] is unchecked for [`Self::untag_receiver`]'s
    /// reason, and transfers ownership unchanged — so a borrowed slot read
    /// stays a borrow, and [`Self::aliasing_read`], which answers on the
    /// *syntax* rather than on the representation, still decides the one
    /// retain a consumer owes.
    ///
    /// A residue that erases to [`Ty::Tagged`] itself (a `?(A|B)` narrowed to
    /// `A|B`) is left alone: there is no representation to change, exactly as
    /// `mwl-codegen`'s own `Untag`-to-tagged identity has it.
    pub(super) fn untag_narrowed(
        &mut self,
        span: Span,
        v: ValueId,
        ty: Ty,
        cur: BlockId,
    ) -> (ValueId, Ty) {
        if ty != Ty::Tagged {
            return (v, ty);
        }
        let Some(ExprInfo::NarrowedRead { to }) = self.exprs.lookup(span) else {
            return (v, ty);
        };
        let Some(to) = super::erase_checked_ty(*to, self.checked_types) else {
            return (v, ty);
        };
        if to == Ty::Tagged {
            return (v, ty);
        }
        (self.emit(cur, to, InstKind::Untag { operand: v }).0, to)
    }

    /// Closes the guard [`Self::open_nullsafe`] opened, merging the member's
    /// own value with the `null` the short-circuiting arm answers.
    ///
    /// The merged value is always [`Ty::Tagged`], which is what `?T` erases
    /// to and what `mwl_types` typed the whole access as. `value`/`ty` are
    /// whatever the member access produced in `*cur` — which need not be the
    /// block [`Self::open_nullsafe`] handed back, since an argument may
    /// itself have branched.
    ///
    /// A `void` member has nothing to merge: both arms simply rejoin, and the
    /// value handed back is the unusable one the call produced — the same
    /// reason `mwl_types` unions no `null` into a `void` member's type.
    pub(super) fn close_nullsafe(
        &mut self,
        guard: Option<NullsafeGuard>,
        value: ValueId,
        ty: Ty,
        env: &mut Env,
        cur: &mut BlockId,
    ) -> (ValueId, Ty) {
        let Some(NullsafeGuard {
            null_block,
            merge_block,
            pre_env,
        }) = guard
        else {
            return (value, ty);
        };
        if ty == Ty::Void {
            let member_end = *cur;
            self.seal(member_end, Terminator::Jump(merge_block));
            self.seal(null_block, Terminator::Jump(merge_block));
            *env = self.merge_envs(
                merge_block,
                &[(member_end, env.clone()), (null_block, pre_env.clone())],
                &pre_env,
            );
            *cur = merge_block;
            return (value, ty);
        }
        let member_end = *cur;
        let member_v = self.coerce(member_end, value, ty, Ty::Tagged, env);
        self.seal(member_end, Terminator::Jump(merge_block));
        let null_v = self.emit(null_block, Ty::Null, InstKind::ConstNull).0;
        let null_v = self.coerce(null_block, null_v, Ty::Null, Ty::Tagged, env);
        self.seal(null_block, Terminator::Jump(merge_block));
        *env = self.merge_envs(
            merge_block,
            &[(member_end, env.clone()), (null_block, pre_env.clone())],
            &pre_env,
        );
        let (merged, _) = self.emit(
            merge_block,
            Ty::Tagged,
            InstKind::Phi {
                incoming: vec![(member_end, member_v), (null_block, null_v)],
            },
        );
        *cur = merge_block;
        (merged, Ty::Tagged)
    }
    pub(super) fn lower_coalesce(
        &mut self,
        whole: &Expr,
        lhs: &Expr,
        rhs: &Expr,
        env: &mut Env,
        cur: &mut BlockId,
    ) -> (ValueId, Ty) {
        let (non_null, result) = match self.exprs.lookup(whole.span) {
            Some(ExprInfo::Coalesce { non_null, result }) => (*non_null, *result),
            _ => panic!(
                "mwl-ir: a `??` at {:?} has no recorded result type — either it wasn't checked \
                 with the same table, or `mwl_types::expr` stopped recording one",
                whole.span
            ),
        };
        let non_null_repr = lower_checked_ty(non_null, self.checked_types);
        let result_repr = lower_checked_ty(result, self.checked_types);

        let (lhs_v, lhs_ty) = self.lower_expr(lhs, None, env, cur);
        let lhs_is_alias = self.aliasing_read(lhs);

        // A left operand whose representation is not tagged cannot be `null`
        // at runtime, so `??` is the left operand and the right one is never
        // evaluated — which is what short-circuiting already means. The
        // mirror case, a statically `null` left operand, is the right one.
        if lhs_ty != Ty::Tagged {
            if lhs_ty == Ty::Null {
                let mut rhs_cur = *cur;
                let (rv, rty) = self.lower_expr(rhs, Some(result_repr), env, &mut rhs_cur);
                if rty.is_refcounted() && self.aliasing_read(rhs) {
                    self.emit_retain(rhs_cur, rv);
                }
                let rv = self.coerce(rhs_cur, rv, rty, result_repr, env);
                *cur = rhs_cur;
                return (rv, result_repr);
            }
            if lhs_ty.is_refcounted() && lhs_is_alias {
                self.emit_retain(*cur, lhs_v);
            }
            let v = self.coerce(*cur, lhs_v, lhs_ty, result_repr, env);
            return (v, result_repr);
        }

        let is_null = self
            .emit(*cur, Ty::Bool, InstKind::IsNull { operand: lhs_v })
            .0;
        let pre_block = *cur;
        let null_block = self.new_block();
        let value_block = self.new_block();
        let merge_block = self.new_block();
        let null_edge = self.ids.next_edge(rhs.span);
        let value_edge = self.ids.next_edge(lhs.span);
        self.seal(
            pre_block,
            Terminator::Branch {
                cond: is_null,
                then_block: null_block,
                then_edge: null_edge,
                else_block: value_block,
                else_edge: value_edge,
            },
        );

        let untagged = self
            .emit(
                value_block,
                non_null_repr,
                InstKind::Untag { operand: lhs_v },
            )
            .0;
        if non_null_repr.is_refcounted() && lhs_is_alias {
            self.emit_retain(value_block, untagged);
        }
        let value_v = self.coerce(value_block, untagged, non_null_repr, result_repr, env);
        self.seal(value_block, Terminator::Jump(merge_block));

        // The default runs only on the `null` edge, so it rebinds into its own
        // copy — `$a ?? $x++` — and the two edges meet at `Self::merge_envs`.
        let pre_env = env.clone();
        let mut null_env = pre_env.clone();
        let mut rhs_cur = null_block;
        let (rv, rty) = self.lower_expr(rhs, Some(result_repr), &mut null_env, &mut rhs_cur);
        if rty.is_refcounted() && self.aliasing_read(rhs) {
            self.emit_retain(rhs_cur, rv);
        }
        let null_v = self.coerce(rhs_cur, rv, rty, result_repr, &mut null_env);
        self.seal(rhs_cur, Terminator::Jump(merge_block));
        *env = self.merge_envs(
            merge_block,
            &[(value_block, pre_env.clone()), (rhs_cur, null_env)],
            &pre_env,
        );

        let (value, _) = self.emit(
            merge_block,
            result_repr,
            InstKind::Phi {
                incoming: vec![(value_block, value_v), (rhs_cur, null_v)],
            },
        );
        *cur = merge_block;
        (value, result_repr)
    }
    /// `isset($a, $b, …)` — ADR 0028 § 3: each operand is `!= null`, and the
    /// list is their conjunction, evaluated left to right and **short-circuit**
    /// exactly as PHP's is. That is observable rather than an optimisation:
    /// `isset($a, $b[$i++])` leaves `$i` alone when `$a` is `null`, so the
    /// tail is lowered on one edge only, in the same branch/merge shape
    /// [`Self::lower_and`] uses, and the recursion is over the tail rather
    /// than a fold so a three-operand `isset` short-circuits at either point.
    ///
    /// No operand can *throw* on absence: `mwl_types::expr::presence` marks
    /// every subscript level under an `isset` in `Env::coalesce_guarded`, the
    /// same set `??` fills, so ADR 0007 § 7 row 11's throw is off for exactly
    /// the reads this construct exists to ask about.
    pub(super) fn lower_isset(
        &mut self,
        operands: &[Expr],
        env: &mut Env,
        cur: &mut BlockId,
    ) -> ValueId {
        let (first, rest) = operands
            .split_first()
            .expect("mwl-ir: `mwl_syntax`'s `parse_isset` always parses at least one operand");
        let first_v = self.lower_isset_operand(first, env, cur);
        if rest.is_empty() {
            return first_v;
        }
        let first_end = *cur;
        let short_v = self.emit(first_end, Ty::Bool, InstKind::ConstBool(false)).0;

        let rest_block = self.new_block();
        let merge_block = self.new_block();
        let rest_edge = self.ids.next_edge(rest[0].span);
        let short_edge = self.ids.next_edge(first.span);
        self.seal(
            first_end,
            Terminator::Branch {
                cond: first_v,
                then_block: rest_block,
                then_edge: rest_edge,
                else_block: merge_block,
                else_edge: short_edge,
            },
        );

        // The tail runs on one edge only, so it rebinds into its own copy and
        // the two edges are reconciled at the merge — [`Self::lower_and`]'s own
        // rule, and what `isset($a, $b[$i++])` needs to write `$i` exactly
        // where PHP writes it.
        let pre_env = env.clone();
        let mut rest_env = pre_env.clone();
        let mut rest_cur = rest_block;
        let rest_v = self.lower_isset(rest, &mut rest_env, &mut rest_cur);
        let rest_end = rest_cur;
        self.seal(rest_end, Terminator::Jump(merge_block));
        *env = self.merge_envs(
            merge_block,
            &[(first_end, pre_env.clone()), (rest_end, rest_env)],
            &pre_env,
        );

        let (result, _) = self.emit(
            merge_block,
            Ty::Bool,
            InstKind::Phi {
                incoming: vec![(first_end, short_v), (rest_end, rest_v)],
            },
        );
        *cur = merge_block;
        result
    }
    /// One `isset` operand's own answer: `!= null`, decided by the operand's
    /// *representation* wherever that already settles it.
    ///
    /// Only a [`Ty::Tagged`] operand can hold `null` at run time, so every
    /// other one is a constant — `true` for a value that exists by its own
    /// declaration (ADR 0022 makes a declared property definitely initialised,
    /// ADR 0007 § 1 a local), `false` for the literal `null`'s own
    /// [`Ty::Null`]. That is the same short-circuit on representation
    /// [`Self::lower_coalesce`] takes, and it is why `isset` costs nothing at
    /// all on a non-nullable operand.
    fn lower_isset_operand(&mut self, operand: &Expr, env: &mut Env, cur: &mut BlockId) -> ValueId {
        let (v, ty) = self.lower_expr(operand, None, env, cur);
        let present = match ty {
            Ty::Null => self.emit(*cur, Ty::Bool, InstKind::ConstBool(false)).0,
            Ty::Tagged => {
                let is_null = self.emit(*cur, Ty::Bool, InstKind::IsNull { operand: v }).0;
                self.emit(
                    *cur,
                    Ty::Bool,
                    InstKind::UnOp {
                        op: UnOp::Not,
                        operand: is_null,
                    },
                )
                .0
            }
            _ => self.emit(*cur, Ty::Bool, InstKind::ConstBool(true)).0,
        };
        // The operand is only read, so a fresh one nothing else owns — an
        // element read off a temporary, a `get` hook's return — is released
        // once the test has read it. Same rule [`Self::lower_instanceof`]
        // applies to its own subject, and the answer being a [`Ty::Bool`] is
        // what makes "right after" safe.
        if !self.aliasing_read(operand) && ty.is_refcounted() {
            self.emit_release(*cur, v);
        }
        present
    }
    /// `!expr` — ADR 0035's truthy table applied to `expr`, then negated;
    /// always produces [`Ty::Bool`] regardless of `expr`'s own type, unlike a
    /// plain arithmetic/bitwise unary operator. `expr` is lowered through
    /// [`Self::lower_expr`], so `!($a && $b)`/`!($a ? $b : $c)` compose the
    /// same way a bare `&&`/`||`/ternary does.
    pub(super) fn lower_not(&mut self, inner: &Expr, env: &mut Env, cur: &mut BlockId) -> ValueId {
        let (v, ty) = self.lower_expr(inner, None, env, cur);
        let b = self.truthy_value(v, ty, self.aliasing_read(inner), *cur);
        self.emit(
            *cur,
            Ty::Bool,
            InstKind::UnOp {
                op: UnOp::Not,
                operand: b,
            },
        )
        .0
    }
    /// `lhs && rhs` — PHP's short-circuit `&&`: `rhs` is only evaluated when
    /// `lhs` is truthy. Lowered exactly like [`Self::lower_if`]'s own
    /// branch/merge shape, except the join point produces the expression's
    /// own [`Ty::Bool`] value via a fresh [`InstKind::Phi`]. The named locals
    /// are merged too, by the same [`Self::merge_envs`] an `if` uses: `rhs`
    /// runs on one edge only, so an increment written inside it re-points a
    /// binding on that edge alone.
    /// `lhs`/`rhs` each go through [`Self::lower_truthy_cond`], so either may
    /// itself be any type ADR 0035's table covers, and either may itself be a
    /// nested `&&`/`||`/`!`/ternary.
    pub(super) fn lower_and(
        &mut self,
        lhs: &Expr,
        rhs: &Expr,
        env: &mut Env,
        cur: &mut BlockId,
    ) -> ValueId {
        let lhs_v = self.lower_truthy_cond(lhs, env, cur);
        let lhs_end = *cur;
        // Emitted in `lhs_end` before it's sealed below — this is the join's
        // incoming value for the short-circuit (falsy-`lhs`) edge.
        let short_v = self.emit(lhs_end, Ty::Bool, InstKind::ConstBool(false)).0;

        let rhs_block = self.new_block();
        let merge_block = self.new_block();
        let rhs_edge = self.ids.next_edge(rhs.span);
        let short_edge = self.ids.next_edge(lhs.span);
        self.seal(
            lhs_end,
            Terminator::Branch {
                cond: lhs_v,
                then_block: rhs_block,
                then_edge: rhs_edge,
                else_block: merge_block,
                else_edge: short_edge,
            },
        );

        // `rhs` runs on one edge only, so it rebinds into its own copy and the
        // two edges are reconciled at the merge — `$a && $x++` writes `$x`
        // exactly where PHP does. See `Self::merge_envs`.
        let pre_env = env.clone();
        let mut rhs_env = pre_env.clone();
        let mut rhs_cur = rhs_block;
        let rhs_v = self.lower_truthy_cond(rhs, &mut rhs_env, &mut rhs_cur);
        let rhs_end = rhs_cur;
        self.seal(rhs_end, Terminator::Jump(merge_block));
        *env = self.merge_envs(
            merge_block,
            &[(lhs_end, pre_env.clone()), (rhs_end, rhs_env)],
            &pre_env,
        );

        let (result, _) = self.emit(
            merge_block,
            Ty::Bool,
            InstKind::Phi {
                incoming: vec![(lhs_end, short_v), (rhs_end, rhs_v)],
            },
        );
        *cur = merge_block;
        result
    }
    /// `lhs || rhs` — [`Self::lower_and`]'s mirror: `rhs` is only evaluated
    /// when `lhs` is falsy, and the short-circuit (truthy-`lhs`) edge carries
    /// `true` instead of `false`.
    pub(super) fn lower_or(
        &mut self,
        lhs: &Expr,
        rhs: &Expr,
        env: &mut Env,
        cur: &mut BlockId,
    ) -> ValueId {
        let lhs_v = self.lower_truthy_cond(lhs, env, cur);
        let lhs_end = *cur;
        let short_v = self.emit(lhs_end, Ty::Bool, InstKind::ConstBool(true)).0;

        let rhs_block = self.new_block();
        let merge_block = self.new_block();
        let short_edge = self.ids.next_edge(lhs.span);
        let rhs_edge = self.ids.next_edge(rhs.span);
        self.seal(
            lhs_end,
            Terminator::Branch {
                cond: lhs_v,
                then_block: merge_block,
                then_edge: short_edge,
                else_block: rhs_block,
                else_edge: rhs_edge,
            },
        );

        // `Self::lower_and`'s own one-edge rebinding, mirrored.
        let pre_env = env.clone();
        let mut rhs_env = pre_env.clone();
        let mut rhs_cur = rhs_block;
        let rhs_v = self.lower_truthy_cond(rhs, &mut rhs_env, &mut rhs_cur);
        let rhs_end = rhs_cur;
        self.seal(rhs_end, Terminator::Jump(merge_block));
        *env = self.merge_envs(
            merge_block,
            &[(lhs_end, pre_env.clone()), (rhs_end, rhs_env)],
            &pre_env,
        );

        let (result, _) = self.emit(
            merge_block,
            Ty::Bool,
            InstKind::Phi {
                incoming: vec![(lhs_end, short_v), (rhs_end, rhs_v)],
            },
        );
        *cur = merge_block;
        result
    }
    /// `cond ? then : else` (`then` is `None` for elvis, `cond ?: else`) —
    /// [`Self::lower_if`]'s branch/merge shape again, this time joining the
    /// expression's own value via a [`InstKind::Phi`] rather than merging
    /// named locals.
    ///
    /// `cond` is converted through [`Self::truthy_convert`] directly, not
    /// [`Self::lower_truthy_cond`]/[`Self::truthy_value`]: elvis's truthy
    /// path reuses `cond`'s own value (PHP evaluates a `?:` condition exactly
    /// once), so releasing it as part of the truthy test — the usual rule
    /// every other truthy-tested position wants — would use-after-free that
    /// reuse. Instead: a refcounted, non-aliasing `cond` is released once
    /// `then` is given (nothing left to reuse it for), or retained once more
    /// when `then` is omitted and `cond` *is* an aliasing read (its value is
    /// about to gain a second, independent owner — the ternary's own
    /// result) — the same [`is_aliasing_read`]-keyed retain
    /// [`Self::bind_local`]/[`Self::lower_call_args`] already apply at their
    /// own ownership-transfer boundaries. A fresh, non-aliasing `cond`
    /// reused by elvis needs neither: it already has exactly one owner,
    /// which simply becomes the ternary's result.
    ///
    /// The same retain question applies to every `then`/`else` branch, not
    /// just elvis's reused `cond`: every consumer of this method's own result
    /// ([`Self::bind_local`], `return`) treats it as an ordinary fresh value —
    /// [`is_aliasing_read`] never lists [`mwl_syntax::ast::ExprKind::Ternary`]
    /// — so this method has to guarantee that itself. A branch whose own
    /// expression [`is_aliasing_read`] (a bare variable, a compile-time-known
    /// property or array-element read) is retained right there, converting a
    /// still-slot-owned reference into the ternary's own independent one,
    /// exactly like [`Self::lower_interpolated_parts`]' own "single-part
    /// alias" case; a branch that's already fresh (a literal, `new`, a call
    /// result, or a nested `&&`/`||`/`!`/ternary — the last already guarantees
    /// its own freshness by this same rule) needs no retain, since ownership
    /// just transfers.
    ///
    /// Two branches that lower to two different representations join at
    /// [`Ty::Tagged`], which is what the checker's own union of their static
    /// types already erases to — see [`Self::join_representations`], which
    /// performs it, for why that is an erasure rather than a promotion.
    ///
    /// # Panics
    ///
    /// See [`Self::truthy_convert`]'s own panic doc for `cond`'s restriction.
    pub(super) fn lower_ternary(
        &mut self,
        cond: &Expr,
        then: Option<&Expr>,
        else_: &Expr,
        env: &mut Env,
        cur: &mut BlockId,
    ) -> (ValueId, Ty) {
        let (cond_v, cond_ty) = self.lower_expr(cond, None, env, cur);
        let cond_is_alias = self.aliasing_read(cond);
        let truthy_v = self.truthy_convert(cond_v, cond_ty, *cur);
        let pre_block = *cur;
        if then.is_some() && cond_ty.is_refcounted() && !cond_is_alias {
            self.emit_release(pre_block, cond_v);
        }

        let then_block = self.new_block();
        let else_block = self.new_block();
        let merge_block = self.new_block();
        let then_edge = self.ids.next_edge(then.map_or(cond.span, |t| t.span));
        let else_edge = self.ids.next_edge(else_.span);
        self.seal(
            pre_block,
            Terminator::Branch {
                cond: truthy_v,
                then_block,
                then_edge,
                else_block,
                else_edge,
            },
        );

        // Each branch rebinds into its own copy — `$c ? $x++ : $y` writes `$x`
        // on one edge only — and the two are reconciled at the merge below by
        // the same `Self::merge_envs` an `if` uses. See `Self::branch_env`.
        let pre_env = env.clone();
        let mut then_env = pre_env.clone();
        let (then_v, then_ty, then_end) = match then {
            Some(then_expr) => {
                let mut then_cur = then_block;
                let (v, ty) = self.lower_expr(then_expr, None, &mut then_env, &mut then_cur);
                if ty.is_refcounted() && self.aliasing_read(then_expr) {
                    self.emit_retain(then_cur, v);
                }
                (v, ty, then_cur)
            }
            None => {
                if cond_ty.is_refcounted() && cond_is_alias {
                    self.emit_retain(then_block, cond_v);
                }
                (cond_v, cond_ty, then_block)
            }
        };

        let mut else_env = pre_env.clone();
        let mut else_cur = else_block;
        let (else_v, else_ty) = self.lower_expr(else_, None, &mut else_env, &mut else_cur);
        if else_ty.is_refcounted() && self.aliasing_read(else_) {
            self.emit_retain(else_cur, else_v);
        }

        // Neither branch is sealed until both have been lowered: the
        // representation they join at is not known until the second one has a
        // type, and the instruction that widens a branch into it belongs in
        // that branch's own block, ahead of its jump.
        let mut branches = [(then_end, then_v, then_ty), (else_cur, else_v, else_ty)];
        let ty = self.join_representations(&mut branches, env);
        self.seal(then_end, Terminator::Jump(merge_block));
        self.seal(else_cur, Terminator::Jump(merge_block));
        *env = self.merge_envs(
            merge_block,
            &[(then_end, then_env), (else_cur, else_env)],
            &pre_env,
        );

        let (result, _) = self.emit(
            merge_block,
            ty,
            InstKind::Phi {
                incoming: branches.iter().map(|&(b, v, _)| (b, v)).collect(),
            },
        );
        *cur = merge_block;
        (result, ty)
    }
    /// The one representation a value-producing join — a ternary's two
    /// branches, a `match`'s arms — hands to its [`InstKind::Phi`], with
    /// every branch widened into it in its own block.
    ///
    /// Branches that already share a representation cost nothing: no
    /// instruction is emitted and the representation is returned unchanged.
    /// A mismatched set joins at [`Ty::Tagged`], and that is not a choice
    /// made here — the checker has already typed the whole expression as the
    /// *union* of its branches (`mwl_types::expr::check_expr`'s `Ternary` arm
    /// interns one), and `erase_checked_ty` erases a union whose members do
    /// not share a representation to exactly [`Ty::Tagged`]. This performs
    /// that erasure rather than inventing a type the rest of the crate would
    /// then disagree with.
    ///
    /// So it deliberately does **not** reach for [`Self::widen_to_float`].
    /// ADR 0007 § 4's promotion rows belong to an *operator*, whose result
    /// type that table fixes outright, and § 2's implicit `int`/`uint` →
    /// `float` widening happens at a **`float` position** — a binding, a
    /// parameter, a `return`. A branch of a ternary is neither, so
    /// `$c ? 1 : 2.5` keeps PHP's answer on its truthy path (an `int`, not
    /// `1.0`), and `float $x = $c ? 1 : 2.5;` widens exactly once, at the
    /// binding, through [`Self::coerce`]'s own `(Ty::Tagged, Ty::Float)` row.
    /// That is the same line integer `/` already draws: widen where the
    /// declared type is, never at the expression that produced the union.
    ///
    /// Ownership is unchanged in either direction. [`InstKind::Tag`]
    /// transfers its operand's reference to its result, so a branch that
    /// retained an aliasing read still owns exactly one afterwards, and a
    /// tagged non-refcounted payload is a no-op for the runtime's own
    /// tag-dispatched release.
    ///
    /// An empty set has no value and so no representation; both callers
    /// refuse that shape before they reach here (a ternary always has two
    /// branches, and an arm-less `match` is refused in [`Self::lower_match`]).
    fn join_representations(
        &mut self,
        branches: &mut [(BlockId, ValueId, Ty)],
        env: &mut Env,
    ) -> Ty {
        let Some(&(_, _, first)) = branches.first() else {
            return Ty::Void;
        };
        if branches.iter().all(|&(_, _, ty)| ty == first) {
            return first;
        }
        for branch in branches.iter_mut() {
            branch.1 = self.coerce(branch.0, branch.1, branch.2, Ty::Tagged, env);
            branch.2 = Ty::Tagged;
        }
        Ty::Tagged
    }
    /// `match (subject) { a, b => x, default => y }` — [`Self::lower_switch`]'s
    /// equality chain producing a **value** instead of running statements.
    ///
    /// What it shares with `switch`: one evaluation of the subject, one
    /// [`BinOp::Eq`] per label in source order, and `default` as the chain's
    /// fall-off wherever it is written. What is its own:
    ///
    /// * **An arm is an expression, and the arms join in a phi**, the way
    ///   [`Self::lower_ternary`]'s two branches do — including its retain
    ///   rule, since every consumer of this method's result treats it as an
    ///   ordinary fresh value: an arm body that [`is_aliasing_read`]s a slot
    ///   is retained right there, one that is already fresh is not.
    /// * **There is no fallthrough**, so each arm body block is entered only
    ///   from its own labels and leaves straight for the merge.
    /// * **No arm matching throws.** PHP raises `UnhandledMatchError`;
    ///   `mwl_hir::errors`' tree is closed and has no such entry, so this
    ///   raises the entry that already means "a bug in the program",
    ///   `LogicError` (`docs/spec/01-core-library.md` § 10). The message names
    ///   the construct rather than the unmatched value: rendering an arbitrary
    ///   subject would need the `Stringable`/`mixed` rendering this crate does
    ///   not have, and `Self::write_throw_location` already puts the file and
    ///   line on the exception.
    ///
    /// The subject's reference is not parked in the [`Env`] the way
    /// [`Self::lower_switch`] parks its own — an expression has no name to
    /// park it under. Instead a *fresh* subject (one no slot owns) is
    /// released at the top of every block the chain can leave for:
    /// each arm body and the throw block. Those are disjoint paths, so the
    /// release runs exactly once.
    ///
    /// Two arms whose bodies lower to different representations join at
    /// [`Ty::Tagged`], [`Self::lower_ternary`]'s rule applied to N branches
    /// rather than two — [`Self::join_representations`] owns it.
    ///
    /// # Panics
    ///
    /// Panics for a label whose representation differs from the subject's,
    /// and — as an engine invariant, not a refusal — for a `match` with no
    /// arms at all, which `mwl_types` refuses as `E0476` before this crate
    /// ever sees it.
    pub(super) fn lower_match(
        &mut self,
        subject: &Expr,
        arms: &[MatchArm],
        expected: Option<Ty>,
        env: &mut Env,
        cur: &mut BlockId,
    ) -> (ValueId, Ty) {
        assert!(
            !arms.is_empty(),
            "an arm-less `match` reached lowering — this is a bug in mwl-types, whose \
             `E0476` refuses one where it is written precisely so that this crate never \
             has to invent a value for a merge phi with no incoming edge"
        );
        let (subj_v, subj_ty) = self.lower_expr(subject, None, env, cur);
        // A fresh subject is this expression's to free; an aliasing one stays
        // the slot's, exactly the split `Self::lower_ternary` applies to its
        // own reused condition.
        let owed = subj_ty.is_refcounted() && !self.aliasing_read(subject);

        let arm_blocks: Vec<BlockId> = arms.iter().map(|_| self.new_block()).collect();
        let merge_block = self.new_block();
        let default_index = arms.iter().position(|a| a.conditions.is_none());

        // The env each arm body is entered with is the label chain's as of the
        // label that jumped there — the same bookkeeping `Self::lower_switch`
        // keeps, and needed for the same reason now that a label or an arm
        // body may rebind a local (`match ($x) { 1 => $c++, default => 0 }`).
        let pre_env = env.clone();
        let mut entry_edges: Vec<Vec<(BlockId, Env)>> = vec![Vec::new(); arms.len()];
        let mut test_cur = *cur;
        for (i, arm) in arms.iter().enumerate() {
            let Some(conditions) = &arm.conditions else {
                continue;
            };
            for cond in conditions {
                let (cond_v, cond_ty) = self.lower_expr(cond, Some(subj_ty), env, &mut test_cur);
                assert_eq!(
                    cond_ty, subj_ty,
                    "mwl-ir lowers a `match` label only at the subject's own representation — \
                     got {cond_ty:?} against a {subj_ty:?} subject; see the crate docs' known \
                     gaps"
                );
                let (eq_v, _) = self.emit(
                    test_cur,
                    Ty::Bool,
                    InstKind::BinOp {
                        op: BinOp::Eq,
                        lhs: subj_v,
                        rhs: cond_v,
                    },
                );
                if cond_ty.is_refcounted() && !self.aliasing_read(cond) {
                    self.emit_release(test_cur, cond_v);
                }
                let next = self.new_block();
                let hit_edge = self.ids.next_edge(arm.span);
                let miss_edge = self.ids.next_edge(cond.span);
                self.seal(
                    test_cur,
                    Terminator::Branch {
                        cond: eq_v,
                        then_block: arm_blocks[i],
                        then_edge: hit_edge,
                        else_block: next,
                        else_edge: miss_edge,
                    },
                );
                entry_edges[i].push((test_cur, env.clone()));
                test_cur = next;
            }
        }
        match default_index {
            Some(i) => {
                self.seal(test_cur, Terminator::Jump(arm_blocks[i]));
                entry_edges[i].push((test_cur, env.clone()));
            }
            None => {
                if owed {
                    self.emit_release(test_cur, subj_v);
                }
                let (message, _) = self.emit(
                    test_cur,
                    Ty::Str,
                    InstKind::ConstStr("no `match` arm matched the subject".to_owned()),
                );
                // Argument 2 is the `{previous}` bag, flattened: this throw
                // chains to nothing, so it is the option's own `null` default,
                // widened into the `Ty::Tagged` slot spec § 10's
                // `Throwable|null` erases to. `lower_call_args` does the same
                // for a written `new`; this site builds the list by hand and
                // so has to say it.
                let (absent, _) = self.emit(test_cur, Ty::Null, InstKind::ConstNull);
                let absent = self.coerce(test_cur, absent, Ty::Null, Ty::Tagged, env);
                let (exception, _) = self.emit_fallible(
                    test_cur,
                    Ty::Object,
                    InstKind::New {
                        class: "LogicError".to_owned(),
                        ctor: Some(THROWABLE_CTOR.to_owned()),
                        args: vec![message, absent],
                    },
                    env,
                );
                self.write_throw_location(test_cur, exception);
                let landing = self.landing_block(env);
                self.seal(
                    test_cur,
                    Terminator::Throw {
                        value: exception,
                        landing,
                    },
                );
            }
        }

        let mut branches: Vec<(BlockId, ValueId, Ty)> = Vec::with_capacity(arms.len());
        let mut arm_envs: Vec<(BlockId, Env)> = Vec::with_capacity(arms.len());
        for (i, arm) in arms.iter().enumerate() {
            let mut arm_cur = arm_blocks[i];
            let mut arm_env = self.merge_envs(arm_cur, &entry_edges[i], &pre_env);
            if owed {
                self.emit_release(arm_cur, subj_v);
            }
            let (v, ty) = self.lower_expr(&arm.body, expected, &mut arm_env, &mut arm_cur);
            if ty.is_refcounted() && self.aliasing_read(&arm.body) {
                self.emit_retain(arm_cur, v);
            }
            branches.push((arm_cur, v, ty));
            arm_envs.push((arm_cur, arm_env));
        }
        // No arm is sealed inside the loop above: an arm that has to widen
        // into the representation the whole `match` joins at needs the
        // widening in its own block, and which representation that is is not
        // known until the last arm has a type. `Self::join_representations`
        // owns the rule, and it is the ternary's rule exactly — a `match` is
        // a ternary with more than two branches.
        let ty = self.join_representations(&mut branches, env);
        for &(block, _, _) in &branches {
            self.seal(block, Terminator::Jump(merge_block));
        }
        *env = self.merge_envs(merge_block, &arm_envs, &pre_env);
        let (result, _) = self.emit(
            merge_block,
            ty,
            InstKind::Phi {
                incoming: branches.iter().map(|&(b, v, _)| (b, v)).collect(),
            },
        );
        *cur = merge_block;
        (result, ty)
    }
    /// Lowers `ExprKind::Interpolated`'s parts into the single [`Ty::Str`]
    /// value they denote — one n-ary [`InstKind::Concat`] over every piece,
    /// reusing [`Self::concat_operand`] per `StringPart::Expr` piece exactly
    /// the way `.`-concatenation's own arm does (a
    /// `Stringable`-object piece hits the identical "needs a resolved
    /// `toString`" panic that method's own doc comment already names as a
    /// shared, not-yet-lowerable case — this is not primarily a new gap, just
    /// the same one reached from a second syntax). A `StringPart::Text` piece
    /// cooks straight to a fresh [`InstKind::ConstStr`] via
    /// [`mwl_types::string_lit::cook_double_quoted_text`] — the same routine
    /// [`cook_str_literal`] delegates to for a plain double-quoted `Str`,
    /// since a `Text` run's escape grammar is identical either way (see that
    /// function's own doc comment) — unless `whole_span` opens with `<<<`
    /// (a heredoc; never a nowdoc, see this function's caller), in which
    /// case each `Text` run first goes through
    /// [`mwl_types::string_lit::dedent_heredoc_run`] against the one
    /// [`mwl_types::string_lit::heredoc_shape`] computed for the whole
    /// literal, exactly the way [`cook_heredoc_str`] dedents a `Str`-collapsed
    /// heredoc's own single run — `body_start` is true only for `parts`'
    /// own first entry, and `is_last_run` only for the last `StringPart::Text`
    /// entry (never an `Expr`: the body always ends in literal text, at
    /// minimum the one newline before the closing marker).
    ///
    /// The one shape a written-out `.` expression wouldn't otherwise force
    /// into the open: an `Interpolated` with exactly one part that is itself an
    /// aliasing read (`"$x"` alone, no literal text around it and nothing
    /// else to concatenate against) never emits an `InstKind::Concat` at
    /// all, so nothing along the way copies `$x`'s value into a fresh
    /// buffer. Returning `$x`'s own `ValueId` unchanged would hand the
    /// caller a second durable owner of a slot's existing storage with no
    /// retain behind it — exactly the free-turns-into-a-dangling-reference
    /// bug [`Self::bind_local`]'s own retain-on-aliasing-source rule exists
    /// to avoid. `is_aliasing_read` deliberately does not list
    /// `ExprKind::Interpolated` at all (mirroring `ExprKind::Binary { op:
    /// Concat, .. }`, which never lists it either, since two or more parts
    /// always produce a real `Concat`'s fresh buffer) — so this function,
    /// not `Self::bind_local`, is the one place that single-part degenerate
    /// case has to convert a borrowed reference into an owned one, by
    /// retaining it directly before handing it back as though it were as
    /// fresh as every other shape this function can return.
    pub(super) fn lower_interpolated_parts(
        &mut self,
        parts: &[StringPart],
        whole_span: mwl_diagnostics::Span,
        env: &mut Env,
        cur: &mut BlockId,
    ) -> (ValueId, Ty) {
        assert!(
            !parts.is_empty(),
            "mwl-syntax's collapse_string_parts only ever produces ExprKind::Interpolated for a \
             non-empty parts vec"
        );
        let is_heredoc = span_text(self.src, whole_span).starts_with("<<<");
        let indent = if is_heredoc {
            mwl_types::string_lit::heredoc_shape(self.src, whole_span)
                .0
                .indent
        } else {
            String::new()
        };
        let last_text_idx = is_heredoc
            .then(|| parts.iter().rposition(|p| matches!(p, StringPart::Text(_))))
            .flatten();
        // Every piece is in flight for as long as the pieces after it are
        // still being lowered — and one of those can be a call that throws. So
        // each lives on [`Self::owned_temporaries`], and only the finished
        // string leaves it ([`Self::forget_temporaries_since`]).
        let mark = self.temporaries_mark();
        let mut pieces: Vec<(ValueId, bool)> = Vec::with_capacity(parts.len());
        for (i, part) in parts.iter().enumerate() {
            let piece = match part {
                StringPart::Text(span) => {
                    let s = if is_heredoc {
                        let mut issues = Vec::new(); // discarded: mwl_types::check_program already reported these
                        let dedented = mwl_types::string_lit::dedent_heredoc_run(
                            self.src,
                            &indent,
                            *span,
                            i == 0,
                            Some(i) == last_text_idx,
                            &mut issues,
                        );
                        mwl_types::string_lit::cook_double_quoted_text_str(&dedented, *span).0
                    } else {
                        mwl_types::string_lit::cook_double_quoted_text(self.src, *span).0
                    };
                    (self.emit(*cur, Ty::Str, InstKind::ConstStr(s)).0, false)
                }
                StringPart::Expr(e) => self.concat_operand(e, env, cur),
            };
            if !piece.1 {
                self.own_temporary(piece.0);
            }
            pieces.push(piece);
        }
        let (v, alias) = match pieces[..] {
            [only] => only,
            _ => {
                let ids = pieces.iter().map(|(v, _)| *v).collect();
                let (result, _) = self.emit(*cur, Ty::Str, InstKind::Concat { pieces: ids });
                // Every piece is consumed here, and whichever of them were
                // fresh are on the stack — so this releases exactly what the
                // `if !alias` guard above staged.
                self.release_temporaries_since(mark, *cur);
                self.own_temporary(result);
                (result, false)
            }
        };
        // From here the value is the caller's, not this expression's.
        self.forget_temporaries_since(mark);
        if alias {
            // The single-part-alias degenerate case this function's own doc
            // comment names — no `Concat` ran, so `v` is still someone
            // else's storage; retain it to become this expression's own
            // single fresh owner.
            self.emit_retain(*cur, v);
        }
        (v, Ty::Str)
    }
    /// Lowers `expr` — an `ExprKind::Index`'s subscript — to a key operand
    /// for [`ir::InstKind::ArrayGet`]/[`ir::InstKind::ArraySet`], in
    /// whichever of the two representations the crate docs' *an array key is
    /// a `string`, and an `int` subscript no longer spells it* allows.
    ///
    /// ADR 0007 § 5 is unchanged by this: every key still *is* a `string`
    /// and `$a[8]` is still `$a["8"]`. What changed is that reaching it no
    /// longer renders the decimal. A [`Ty::Int`] subscript is handed to the
    /// instruction as the `int` it already was, and `mwl-codegen` calls
    /// `mwl_array_get_index`/`mwl_array_set_index`, which answer from the
    /// packed form with nothing rendered and nothing allocated and
    /// synthesize a key only where the array is already `Hashed` — exactly
    /// the case that was building one anyway (`mwl_runtime::array`'s module
    /// doc, *the ABI was the part that expired*).
    ///
    /// A [`Ty::Uint`] subscript still renders, deliberately: that ABI's
    /// index is an `i64`, and a `uint` above `i64::MAX` has no `i64`
    /// spelling naming the same key, so passing one would silently read a
    /// different element. Correctness before latency, AGENTS.md's priority
    /// ordering. The conversion is the exact [`Helper::UintToString`]
    /// [`Self::concat_operand`] already gives `.`'s scalar operand — reused
    /// verbatim rather than a new policy. [`Self::lower_rendered_array_key`]
    /// is this function for the one caller that still needs a `Ty::Str`
    /// whatever the subscript was.
    ///
    /// Also used, identically, for an array literal's explicit `key =>`
    /// element (see [`ir::InstKind::ArrayNew`]'s own doc comment). A
    /// `float`, `bool`, or `null` key is a compile-time rejection
    /// `mwl_types::expr::check_array_key_type` now enforces at both call
    /// sites (an `Index` subscript and an array-literal explicit key alike),
    /// so the `other` arm below is an internal-invariant panic — unreachable
    /// for anything that already passed `mwl_types::check_program` — rather
    /// than a live known gap.
    ///
    /// Returns the key value, **the representation it is in** — `Ty::Str` or
    /// `Ty::Int`, which is what every caller's refcount decision now turns
    /// on, since an `int` owns nothing to retain or release — and whether it
    /// [`is_aliasing_read`]s storage a durable slot still owns, exactly the
    /// same second half [`Self::concat_operand`] returns and for the same
    /// reason: a plain `string` subscript passed through unchanged may still
    /// be a bare local/property/array read, while a freshly converted key is
    /// always a brand new buffer with exactly one owner.
    pub(super) fn lower_array_key(
        &mut self,
        expr: &Expr,
        env: &mut Env,
        cur: &mut BlockId,
    ) -> (ValueId, Ty, bool) {
        let (v, ty) = self.lower_expr(expr, None, env, cur);
        match ty {
            Ty::Str => (v, Ty::Str, self.aliasing_read(expr)),
            Ty::Int => (v, Ty::Int, false),
            Ty::Uint => {
                let (sv, _) = self.emit(
                    *cur,
                    Ty::Str,
                    InstKind::HelperCall {
                        helper: Helper::UintToString,
                        args: vec![v],
                    },
                );
                (sv, Ty::Str, false)
            }
            other => panic!(
                "mwl-ir: an array key lowered to {other:?} — mwl_types::check_program is trusted \
                 to have already rejected a float/bool/null key (ADR 0007 § 5) at both the \
                 subscript and array-literal explicit-key sites, so this should be unreachable"
            ),
        }
    }

    /// [`Self::lower_array_key`], forced all the way to a [`Ty::Str`] key.
    ///
    /// [`ir::InstKind::ArrayUnset`] is the one key-taking array instruction
    /// with no index-shaped runtime primitive beside it — `mwl-runtime`
    /// added `mwl_array_get_index` and `mwl_array_set_index` and no third —
    /// so `unset($a[$i])` renders the decimal here rather than having
    /// codegen discover it cannot. Widening the runtime ABI to close that
    /// is a separate decision, not a side effect of this one; ADR 0042's
    /// artifacts and M9's WIT signatures are about to freeze that surface.
    pub(super) fn lower_rendered_array_key(
        &mut self,
        expr: &Expr,
        env: &mut Env,
        cur: &mut BlockId,
    ) -> (ValueId, bool) {
        let (v, ty, aliasing) = self.lower_array_key(expr, env, cur);
        if ty != Ty::Int {
            return (v, aliasing);
        }
        let (sv, _) = self.emit(
            *cur,
            Ty::Str,
            InstKind::HelperCall {
                helper: Helper::IntToString,
                args: vec![v],
            },
        );
        (sv, false)
    }

    /// ADR 0007 § 4, mirroring the checker's own rule: a bare integer
    /// literal means `uint` exactly where that's the expected type,
    /// `int` otherwise. `mwl_types::expr::literals::infer_int_literal`
    /// enforces ADR 0007 § 4's magnitude rule
    /// at check time — too large for `int` is only legal where `uint`
    /// is expected, and too large even for `uint`'s full `u64` range
    /// is a diagnostic regardless — so `lower_method`'s usual "trusts
    /// its input already passed `mwl_types::check_program`" contract
    /// (see the crate docs) covers this too: the `unwrap_or_else`
    /// panics below are unreachable for anything the checker accepted,
    /// the same defensive-invariant shape as `Env::get`'s own panic on
    /// an undeclared local just above.
    fn lower_int_literal(
        &mut self,
        span: Span,
        expr: &Expr,
        expected: Option<Ty>,
        cur: &mut BlockId,
    ) -> (ValueId, Ty) {
        let (radix, digits) = int_literal_digits(self.src, span);
        if expected == Some(Ty::Decimal) || self.placed_at_decimal(expr.span) {
            // ADR 0054 § 2's placing rule, integer half: an integer
            // literal is scale 0 by construction, so only the mantissa
            // can overflow — and `mwl_types` has already reported that
            // if it did.
            let mantissa = u128::from_str_radix(&digits, radix).unwrap_or_else(|_| {
                panic!("mwl-ir: integer literal `{digits}` doesn't fit a `decimal`")
            });
            return self.emit(
                *cur,
                Ty::Decimal,
                InstKind::ConstDecimal {
                    negative: false,
                    mantissa,
                    scale: 0,
                },
            );
        }
        if expected == Some(Ty::Uint) {
            let n: u64 = u64::from_str_radix(&digits, radix).unwrap_or_else(|_| {
                panic!("mwl-ir: integer literal `{digits}` doesn't fit a `uint`")
            });
            self.emit(*cur, Ty::Uint, InstKind::ConstUint(n))
        } else {
            let n: i64 = i64::from_str_radix(&digits, radix).unwrap_or_else(|_| {
                panic!("mwl-ir: integer literal `{digits}` doesn't fit an `int`")
            });
            self.emit(*cur, Ty::Int, InstKind::ConstInt(n))
        }
    }

    fn lower_float_literal(
        &mut self,
        span: Span,
        expr: &Expr,
        expected: Option<Ty>,
        cur: &mut BlockId,
    ) -> (ValueId, Ty) {
        let digits = clean_digits(self.src, span);
        // ADR 0054 § 2: a fractional literal is untyped until placed,
        // and `decimal` is one of the two types that may place it —
        // read from the *text*, so the full 29 significant digits
        // survive rather than being rounded through an `f64` first.
        if expected == Some(Ty::Decimal) || self.placed_at_decimal(expr.span) {
            let (mantissa, scale) = decimal_literal_parts(&digits).unwrap_or_else(|| {
                panic!("mwl-ir: float literal `{digits}` doesn't fit a `decimal`")
            });
            return self.emit(
                *cur,
                Ty::Decimal,
                InstKind::ConstDecimal {
                    negative: false,
                    mantissa,
                    scale,
                },
            );
        }
        let n: f64 = digits
            .parse()
            .unwrap_or_else(|_| panic!("mwl-ir: float literal `{digits}` failed to parse"));
        self.emit(*cur, Ty::Float, InstKind::ConstFloat(n))
    }

    /// ADR 0070 § 3: the grammar is resolved while compiling, so what
    /// reaches the IR is one folded nanosecond count. The value it
    /// becomes is built by the *same* `Core` member a written
    /// `Duration::nanoseconds($n)` calls — `mwl_stdlib::time`'s
    /// `FROM_NANOS_SYMBOL`, named there rather than spelled here — so a
    /// literal and a computed count cannot come to mean different
    /// things.
    ///
    /// § 3 also wants no allocation at all: a constant-pool entry with
    /// an immortal header, which is exactly what a string literal is
    /// owed by `mwl-runtime`'s own gap 3. Both close together; until
    /// then this is one call on a constant.
    fn lower_duration_literal(
        &mut self,
        span: Span,
        env: &mut Env,
        cur: &mut BlockId,
    ) -> (ValueId, Ty) {
        let text = span_text(self.src, span);
        let nanos = mwl_syntax::duration::parse(text).unwrap_or_else(|err| {
            panic!(
                "mwl-ir: duration literal `{text}` does not parse ({}) — the lexer \
                 only produces this token for text that does",
                err.message()
            )
        });
        let (count, _) = self.emit(*cur, Ty::Int, InstKind::ConstInt(nanos));
        self.emit_fallible(
            *cur,
            Ty::Object,
            InstKind::CoreCall {
                symbol: mwl_types::CORE_DURATION_FROM_NANOS,
                args: vec![count],
            },
            env,
        )
    }

    fn lower_unary(
        &mut self,
        op: AstUnaryOp,
        inner: &Expr,
        expected: Option<Ty>,
        env: &mut Env,
        cur: &mut BlockId,
    ) -> (ValueId, Ty) {
        let (v, ty) = self.lower_expr(inner, expected, env, cur);
        // ADR 0054's scalar has no machine negate: like every other
        // operator over one it is a helper call. It cannot fail --
        // the mantissa is unsigned, so there is no asymmetric minimum
        // to overflow the way `-i64::MIN` does.
        if ty == Ty::Decimal && matches!(op, AstUnaryOp::Neg) {
            return self.emit(
                *cur,
                Ty::Decimal,
                InstKind::HelperCall {
                    helper: Helper::DecimalNeg,
                    args: vec![v],
                },
            );
        }
        let uop = match op {
            AstUnaryOp::Neg => UnOp::Neg,
            // ADR 0007 § 4's `~` row: the operand type, preserved, and total
            // over it — every 64-bit pattern is a value of both `int` and
            // `uint`, so this is the one unary arithmetic row with no edge.
            AstUnaryOp::BitNot => UnOp::BitNot,
            other => panic!(
                "mwl-ir's control-flow slice only lowers unary `-`/`!`/`~` — got {other:?}; \
                 see the crate docs' known gaps"
            ),
        };
        let kind = InstKind::UnOp {
            op: uop,
            operand: v,
        };
        // ADR 0007 § 4's overflow throw reaches the unary row too, and for the
        // same reason the additive ones take it: `-i64::MIN` has no `int` and
        // `-$u` no `uint` for any non-zero `$u`, so `ineg` would answer with a
        // wrapped value rather than with the `ArithmeticError` the ADR names.
        // `mwl-codegen`'s `emit_checked_int_arith` raises it inline, so this
        // needs ADR 0002's error edge exactly as `%` and `/` do. `!` over a
        // `bool` and `-` over a `float`/`decimal` cannot fail and do not take
        // one — see `Inst::on_error`.
        if matches!(uop, UnOp::Neg) && matches!(ty, Ty::Int | Ty::Uint) {
            return self.emit_fallible(*cur, ty, kind, env);
        }
        self.emit(*cur, ty, kind)
    }

    /// `.` concatenation is not `InstKind::BinOp` — it allocates a
    /// fresh buffer rather than computing a native scalar result, so
    /// it gets its own arm (and its own `InstKind::Concat`) ahead of
    /// the scalar-operator table below. Each operand goes through
    /// `Self::concat_operand` first, which converts a scalar through
    /// a new `InstKind::HelperCall` when it isn't already `Ty::Str`,
    /// and a `Stringable`-object operand through the `toString()`
    /// `mwl_types::expr::operators::require_stringable` resolved for
    /// it. `concat_operand` also reports
    /// whether the value it hands back aliases storage a durable slot
    /// still owns; an operand that doesn't (a literal, a nested
    /// `Concat`'s own result, or a freshly converted `HelperCall`
    /// result) is released right after this `Concat` reads it, since
    /// nothing else ever will — the same "release a fresh value once
    /// its one and only use is done" precedent `Self::lower_expr_stmt`
    /// already sets for a bare call/`new` statement.
    ///
    /// **The whole `.` spine is one instruction.** `.` is left-associative, so
    /// `"a" . $i . "b"` parses as `("a" . $i) . "b"`; lowering the nested
    /// `Binary` on its own would emit a `Concat` per operator, each allocating
    /// a buffer for the accumulation so far. [`Self::flatten_concat`] collects
    /// the operands instead, and one `InstKind::Concat` carries all of them —
    /// that instruction's own doc comment owns why. Evaluation order is
    /// unchanged: the flatten is a left-to-right walk of the same tree, and
    /// concatenation's intermediate results are not observable.
    ///
    /// Each operand is staged on [`Self::owned_temporaries`] *before* the next
    /// one is lowered, which is what makes `"x" . $obj` — where the
    /// `toString()` throws — release the `"x"` rather than leak it.
    fn lower_concat(
        &mut self,
        lhs: &Expr,
        rhs: &Expr,
        env: &mut Env,
        cur: &mut BlockId,
    ) -> (ValueId, Ty) {
        let mut operands = Vec::new();
        Self::flatten_concat(lhs, &mut operands);
        Self::flatten_concat(rhs, &mut operands);
        let mark = self.temporaries_mark();
        let mut pieces = Vec::with_capacity(operands.len());
        for operand in operands {
            let (v, aliasing) = self.concat_operand(operand, env, cur);
            if !aliasing {
                self.own_temporary(v);
            }
            pieces.push(v);
        }
        let result = self.emit(*cur, Ty::Str, InstKind::Concat { pieces });
        self.release_temporaries_since(mark, *cur);
        result
    }

    /// Appends `expr`'s concatenation operands to `out`, in evaluation order —
    /// descending through any nested `.` so the whole spine reaches one
    /// `InstKind::Concat`.
    ///
    /// Both sides are descended, not just the left one: `.` only ever
    /// associates left, but `$a . ($b . $c)` is written with parentheses often
    /// enough to be worth the one extra arm, and it is the same flattening —
    /// operand order, and therefore evaluation order, is identical either way.
    fn flatten_concat<'e>(expr: &'e Expr, out: &mut Vec<&'e Expr>) {
        if let ExprKind::Binary {
            op: BinaryOp::Concat,
            lhs,
            rhs,
        } = &expr.kind
        {
            Self::flatten_concat(lhs, out);
            Self::flatten_concat(rhs, out);
        } else {
            out.push(expr);
        }
    }

    /// `$x === null` / `$x !== null` — a *tag* comparison, not a value
    /// one. Split out ahead of the general arm below for two reasons,
    /// and either alone would be enough: `null` has its own
    /// representation, so the general arm would hand `mwl-codegen` a
    /// `BinOp` over two different ones; and a `Ty::Tagged` operand's
    /// strict identity is `mwl_runtime::value_identical`, never a
    /// machine compare of the register pair. This is also the test
    /// `mwl_types::locals`' narrowing reads, so the two agree on
    /// exactly one spelling.
    ///
    /// `==`/`!=` are the whole of it:
    /// [ADR 0090](../../../docs/adr/0090-one-equality-operator-and-disjoint-types-do-not-compile.md)
    /// § 1 leaves one spelling, and its § 3 makes that spelling this tag
    /// test rather than PHP's truthy-table question (`0 == null` was
    /// *true* there, which is why this arm read `===`/`!==` while both
    /// spellings existed).
    fn lower_null_identity(
        &mut self,
        op: BinaryOp,
        lhs: &Expr,
        rhs: &Expr,
        env: &mut Env,
        cur: &mut BlockId,
    ) -> (ValueId, Ty) {
        let operand = if matches!(lhs.kind, ExprKind::Null) {
            rhs
        } else {
            lhs
        };
        let (v, ty) = self.lower_expr(operand, None, env, cur);
        // A representation that is not `Ty::Tagged` cannot hold
        // `null` at all, so the answer is a constant — the same
        // reasoning `Self::open_nullsafe` applies to `?->` on a
        // receiver that cannot be `null`. The operand is still
        // lowered (it may have side effects) and released if nothing
        // else owns it, exactly like the general arm's comparison.
        if ty != Ty::Tagged {
            if ty.is_refcounted() && !self.aliasing_read(operand) {
                self.emit_release(*cur, v);
            }
            let is_null = matches!(ty, Ty::Null);
            return self.emit(
                *cur,
                Ty::Bool,
                InstKind::ConstBool(is_null == (op == BinaryOp::Eq)),
            );
        }
        let (is_null, _) = self.emit(*cur, Ty::Bool, InstKind::IsNull { operand: v });
        if !self.aliasing_read(operand) {
            self.emit_release(*cur, v);
        }
        if op == BinaryOp::Eq {
            return (is_null, Ty::Bool);
        }
        self.emit(
            *cur,
            Ty::Bool,
            InstKind::UnOp {
                op: UnOp::Not,
                operand: is_null,
            },
        )
    }

    fn lower_binary(
        &mut self,
        whole: &Expr,
        expected: Option<Ty>,
        env: &mut Env,
        cur: &mut BlockId,
    ) -> (ValueId, Ty) {
        // The whole expression, rather than its three parts: ADR 0033 § 5's
        // arm below needs the *comparison's* own span to read back what the
        // checker recorded there, and taking the span as a fourth parameter
        // beside the parts it already implies is what pushed this signature
        // past `clippy::too_many_arguments`.
        let ExprKind::Binary { op, lhs, rhs } = &whole.kind else {
            unreachable!("lower_binary is reached only from `lower_expr`'s `Binary` arm");
        };
        let op = *op;
        let (lv, lty) = self.lower_expr(lhs, expected, env, cur);
        let (rv, rty) = self.lower_expr(rhs, Some(lty), env, cur);
        // ADR 0054 § 3's table is a set of runtime helpers rather than
        // a machine instruction, so a `decimal` on *either* side takes
        // its own path -- including the mixed `decimal ⊕ int` row,
        // which the helper promotes from the operand's own tag.
        if lty == Ty::Decimal || rty == Ty::Decimal {
            return self.lower_decimal_binary(op, lv, rv, env, cur);
        }
        // ADR 0090 § 5: a `mixed` or union operand is the one pairing whose
        // § 3 row is a runtime tag, so it dispatches through
        // `mwl_runtime::value_identical` rather than through a `BinOp` over a
        // representation neither side has. Every other row is statically
        // known and stays in the table below, where `mwl-codegen` turns it
        // into that row's own comparison.
        if matches!(op, BinaryOp::Eq | BinaryOp::NotEq) && (lty == Ty::Tagged || rty == Ty::Tagged)
        {
            let (equal, _) = self.emit(
                *cur,
                Ty::Bool,
                InstKind::HelperCall {
                    helper: Helper::Identical,
                    args: vec![lv, rv],
                },
            );
            // Released per operand rather than per pair: the two sides may
            // hold different representations here, which is the whole reason
            // this arm exists.
            for (operand, value, ty) in [(lhs, lv, lty), (rhs, rv, rty)] {
                if ty.is_refcounted() && !self.aliasing_read(operand) {
                    self.emit_release(*cur, value);
                }
            }
            if op == BinaryOp::Eq {
                return (equal, Ty::Bool);
            }
            return self.emit(
                *cur,
                Ty::Bool,
                InstKind::UnOp {
                    op: UnOp::Not,
                    operand: equal,
                },
            );
        }
        // ADR 0090 § 2's enum row: an enum is its own equality domain — a
        // case against its underlying integer is a compile error and two
        // different enums are disjoint, so a pair that reaches here is one
        // enum compared with itself. It is answered one representation down,
        // on the integer its cases *are* (ADR 0010 § 3): `Ty::Enum` is a
        // zero-byte tag over that integer, so the free `Reinterpret` row 1 of
        // ADR 0010 § 5 already uses for `$m as int` turns the comparison into
        // the machine compare `mwl-codegen` has — its `BinOp` table is
        // `Ty::Int`/`Ty::Uint`/`Ty::Bool` and has no `Ty::Enum` row at all.
        //
        // Only `==`/`!=` are relabelled. `<` over two cases has no row in any
        // ADR, and ADR 0090 § 2 keeps the two domains apart on purpose, so
        // ordering an enum stays something `$e as int` says out loud.
        let (lv, lty, rv, rty) = if matches!(op, BinaryOp::Eq | BinaryOp::NotEq)
            && matches!(lty, Ty::Enum(_))
            && matches!(rty, Ty::Enum(_))
        {
            let (lv, lty) = self.reinterpret_enum_to_backing(lv, lty, cur);
            let (rv, rty) = self.reinterpret_enum_to_backing(rv, rty, cur);
            (lv, lty, rv, rty)
        } else {
            (lv, lty, rv, rty)
        };
        // ADR 0090 § 2's numeric row: `int`, `uint` and `float` are one
        // domain, so the checker accepts `$n == $f` where the two operands
        // have two *representations*. That pairing is settled here, exactly
        // as the `decimal` and `Tagged` arms above settle theirs, rather than
        // in `mwl-codegen` — which keeps its "a `BinOp` has one
        // representation" invariant intact and its `ty != rty` refusal a
        // genuine internal error. See `Helper::NumericEq` for why the
        // settlement is a call and not a widening conversion: none of the
        // three widenings is exact, so emitting one would answer § 3's
        // "mathematically equal across the whole domain" with a rounding.
        if matches!(op, BinaryOp::Eq | BinaryOp::NotEq)
            && lty != rty
            && matches!(lty, Ty::Int | Ty::Uint | Ty::Float)
            && matches!(rty, Ty::Int | Ty::Uint | Ty::Float)
        {
            let (equal, _) = self.emit(
                *cur,
                Ty::Bool,
                InstKind::HelperCall {
                    helper: Helper::NumericEq,
                    args: vec![lv, rv],
                },
            );
            // Nothing is released: every representation in this arm is a
            // scalar, so neither operand is `Ty::is_refcounted`.
            if op == BinaryOp::Eq {
                return (equal, Ty::Bool);
            }
            return self.emit(
                *cur,
                Ty::Bool,
                InstKind::UnOp {
                    op: UnOp::Not,
                    operand: equal,
                },
            );
        }
        // ADR 0033 § 5: `==` over a pair at least one side of which is
        // `secret` is a constant-time comparison, so that a program comparing
        // its own session token or signature with the language's one equality
        // operator is not a timing oracle. The qualifier is already gone by
        // here — `erase_checked_ty` spends no representation on it, which is
        // ADR 0033 § 1's promise — so the choice cannot be re-derived from
        // `lty`/`rty`, and is read back from what the checker recorded at this
        // comparison instead. See `Helper::SecretEq`.
        //
        // Guarded on the two representations as well as on the entry: a
        // `secret` operand compared against a `mixed` one has already been
        // taken by the `Tagged` arm above, where the row is a runtime tag
        // rather than a buffer this helper could read. § 2's poisoning makes
        // that pair rare, and closing it would mean teaching
        // `mwl_runtime::value_identical` the property, which is wider than
        // this section asks for.
        if matches!(op, BinaryOp::Eq | BinaryOp::NotEq)
            && matches!(lty, Ty::Str | Ty::Bytes)
            && matches!(rty, Ty::Str | Ty::Bytes)
            && matches!(
                self.exprs.lookup(whole.span),
                Some(ExprInfo::SecretEquality)
            )
        {
            let (equal, _) = self.emit(
                *cur,
                Ty::Bool,
                InstKind::HelperCall {
                    helper: Helper::SecretEq,
                    args: vec![lv, rv],
                },
            );
            // Both operands are refcounted, and both are only *read* — the
            // same rule the `BinOp` table below applies to its own, written
            // per operand because the two may be a `string` and a `bytes`.
            for (operand, value) in [(lhs, lv), (rhs, rv)] {
                if !self.aliasing_read(operand) {
                    self.emit_release(*cur, value);
                }
            }
            if op == BinaryOp::Eq {
                return (equal, Ty::Bool);
            }
            return self.emit(
                *cur,
                Ty::Bool,
                InstKind::UnOp {
                    op: UnOp::Not,
                    operand: equal,
                },
            );
        }
        // ADR 0007 § 4's ordering rows, which are *not* its arithmetic ones:
        // the table's own closing paragraph says a comparison "has an exact
        // answer in the mathematical integers and can be lowered as one", so a
        // mixed numeric pair is settled by a helper here — the same shape and
        // the same reason as `Helper::NumericEq` next door — rather than by
        // the widening below, which past 2^53 would raise `ArithmeticError`
        // where PHP answers an ordering. `>`/`>=` are the same two helpers
        // with their operands swapped, the arrangement `lower_decimal_binary`
        // already uses. See `Helper::NumericLt`.
        // `<=>` over a mixed numeric pair, by the same route and for the same
        // reason as the four ordering operators below — one exact answer over
        // the whole domain, where the widening past this point would raise
        // `ArithmeticError` above 2^53 for a pair that orders perfectly well.
        // Split out rather than folded in with them because its result is an
        // `int` and theirs is a `bool`. See `Helper::NumericCmp`.
        if op == BinaryOp::Cmp
            && lty != rty
            && matches!(lty, Ty::Int | Ty::Uint | Ty::Float)
            && matches!(rty, Ty::Int | Ty::Uint | Ty::Float)
        {
            return self.emit(
                *cur,
                Ty::Int,
                InstKind::HelperCall {
                    helper: Helper::NumericCmp,
                    args: vec![lv, rv],
                },
            );
        }
        if matches!(
            op,
            BinaryOp::Lt | BinaryOp::LtEq | BinaryOp::Gt | BinaryOp::GtEq
        ) && lty != rty
            && matches!(lty, Ty::Int | Ty::Uint | Ty::Float)
            && matches!(rty, Ty::Int | Ty::Uint | Ty::Float)
        {
            let (helper, args) = match op {
                BinaryOp::Lt => (Helper::NumericLt, vec![lv, rv]),
                BinaryOp::Gt => (Helper::NumericLt, vec![rv, lv]),
                BinaryOp::LtEq => (Helper::NumericLtEq, vec![lv, rv]),
                _ => (Helper::NumericLtEq, vec![rv, lv]),
            };
            // Nothing is released: every representation in this arm is a
            // scalar, so neither operand is `Ty::is_refcounted`.
            return self.emit(*cur, Ty::Bool, InstKind::HelperCall { helper, args });
        }
        // ADR 0007 § 4's "either operand a `float`" row, made real: the
        // checker types the pair `float`, but until here both operands still
        // travelled in their own representation and `mwl-codegen`'s "a
        // `BinOp` has one representation" invariant refused them. So the
        // integer side widens *here*, beside the `decimal`, `Tagged` and
        // `NumericEq` arms above, which settle their own mixed pairings the
        // same way rather than asking the backend to.
        //
        // It is the checked widening — the very helper `$n as float` emits —
        // because ADR 0007 § 2 names this as the one implicit conversion in
        // the language and says it "throws above 2^53 rather than rounding".
        // A silent `fcvt_from_sint` would answer an exact-in-the-integers
        // question with a rounded one, which is the same reason `==` next
        // door is a helper and not a widening. Being fallible, it carries
        // ADR 0002's error edge exactly as the written conversion does.
        //
        // Only the arithmetic rows reach this: every comparison over a mixed
        // numeric pair has already returned above, exactly so that none of
        // them pays a conversion that can throw.
        let (lv, lty) = self.widen_to_float(lv, lty, rty, env, cur);
        let (rv, _) = self.widen_to_float(rv, rty, lty, env, cur);
        let (bop, ty) = match op {
            BinaryOp::Add => (BinOp::Add, lty),
            BinaryOp::Sub => (BinOp::Sub, lty),
            BinaryOp::Mul => (BinOp::Mul, lty),
            // The one operator whose result representation is not its
            // operands': ADR 0007 § 4 types `int / int` as `int|float` and
            // `uint / uint` as `uint|float`, PHP-exact, so which of the two a
            // given pair produces is only known at run time and the value is
            // therefore [`Ty::Tagged`]. `mwl-codegen`'s `emit_int_div` owns
            // the branch; `Lowering::coerce` owns the widening that absorbs
            // the union back into a declared `float`, which is ADR 0007 § 4's
            // own worked example `float $avg = $sum / $n;`.
            BinaryOp::Div if matches!(lty, Ty::Int | Ty::Uint) => (BinOp::Div, Ty::Tagged),
            BinaryOp::Div => (BinOp::Div, lty),
            BinaryOp::Mod => (BinOp::Mod, lty),
            // ADR 0007 § 4 puts `**` in the same row as `+`, `-` and `*` — the
            // operand type, and a throw rather than a wrap — so it needs no
            // arm of its own here beyond this one. What is not shared is the
            // *emission*: see `BinOp::Pow`, which is a loop over an integer
            // pair and a call over a float one.
            BinaryOp::Pow => (BinOp::Pow, lty),
            // ADR 0007 § 4's bitwise rows, all five of which preserve the
            // operand type. `>>` is the one that reads its operand's
            // signedness rather than only its width — arithmetic on an `int`,
            // logical on a `uint` — and `mwl-codegen` picks that from the
            // representation this carries.
            BinaryOp::BitAnd => (BinOp::BitAnd, lty),
            BinaryOp::BitOr => (BinOp::BitOr, lty),
            BinaryOp::BitXor => (BinOp::BitXor, lty),
            BinaryOp::Shl => (BinOp::Shl, lty),
            BinaryOp::Shr => (BinOp::Shr, lty),
            BinaryOp::Eq => (BinOp::Eq, Ty::Bool),
            BinaryOp::NotEq => (BinOp::NotEq, Ty::Bool),
            BinaryOp::Lt => (BinOp::Lt, Ty::Bool),
            BinaryOp::LtEq => (BinOp::LtEq, Ty::Bool),
            BinaryOp::Gt => (BinOp::Gt, Ty::Bool),
            BinaryOp::GtEq => (BinOp::GtEq, Ty::Bool),
            // ADR 0013 § 2's `<=>` over a *scalar*: the object form never
            // reaches here (`lower_expr`'s guarded arm takes it), and a mixed
            // numeric or `decimal` pair has already returned above — so what
            // is left is one representation and one `BinOp` over it. The
            // result is an `int` whatever the operands hold, which alongside
            // `Div` makes it the second row whose type is not `lty`.
            BinaryOp::Cmp => (BinOp::Cmp, Ty::Int),
            other => panic!(
                "mwl-ir's control-flow slice only lowers arithmetic/equality/ordering \
                 operators — got {other:?}; see the crate docs' known gaps"
            ),
        };
        // A comparison only *reads* its operands, so a refcounted one
        // that no durable slot owns — a string literal in
        // `$key === "bad"` is the shape this exists for — is released
        // right after the instruction reads it, exactly the rule the
        // `Concat` arm above applies to its own fresh operands.
        //
        // Every *integer* arithmetic operator here can fail, and ADR 0007 § 4
        // is why: `+`, `-`, `*` and `**` throw `ArithmeticError` on overflow
        // rather than wrapping, `%` and `/` throw it on a zero divisor, and
        // `**` throws it on a negative exponent as well.
        // `mwl-codegen` raises all six inline rather than through a helper,
        // so each needs an error edge exactly the way a call does. `/` is
        // recognised by its *result* rather than by its operands, since the
        // integer row is the one that produces a `Ty::Tagged`. Every other
        // operator — the comparisons, and all five on floats — returns no
        // status at all, see `Inst::on_error`.
        let inst = InstKind::BinOp {
            op: bop,
            lhs: lv,
            rhs: rv,
        };
        let fallible = match bop {
            BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Mod | BinOp::Pow => {
                matches!(ty, Ty::Int | Ty::Uint)
            }
            BinOp::Div => ty == Ty::Tagged,
            // A shift throws only on a *negative* count, which is PHP's rule
            // and which a `uint` count cannot produce — so the unsigned row is
            // infallible even though the signed one beside it is not.
            BinOp::Shl | BinOp::Shr => ty == Ty::Int,
            _ => false,
        };
        let result = if fallible {
            self.emit_fallible(*cur, ty, inst, env)
        } else {
            self.emit(*cur, ty, inst)
        };
        if lty.is_refcounted() {
            if !self.aliasing_read(lhs) {
                self.emit_release(*cur, lv);
            }
            if !self.aliasing_read(rhs) {
                self.emit_release(*cur, rv);
            }
        }
        result
    }

    /// One operand of a binary operator, widened into ADR 0007 § 4's
    /// `float` row when — and only when — the *other* operand is already
    /// one. `other` is that operand's representation; everything else is
    /// returned untouched, so a matched pair costs nothing and no
    /// instruction is emitted for it.
    ///
    /// The conversion is [`Helper::IntToFloat`]/[`Helper::UintToFloat`],
    /// the same pair `$n as float` lowers to, because ADR 0007 § 2 makes
    /// this implicit row *the same conversion* as the written one — exact
    /// or throwing above 2^53, never rounding. That is why it goes through
    /// [`Lowering::emit_fallible`] and not through
    /// [`Lowering::coerce`], which only reconciles [`Ty::Tagged`] and
    /// emits nothing that can fail.
    fn widen_to_float(
        &mut self,
        v: ValueId,
        ty: Ty,
        other: Ty,
        env: &mut Env,
        cur: &mut BlockId,
    ) -> (ValueId, Ty) {
        if other != Ty::Float || !matches!(ty, Ty::Int | Ty::Uint) {
            return (v, ty);
        }
        let helper = if ty == Ty::Int {
            Helper::IntToFloat
        } else {
            Helper::UintToFloat
        };
        self.emit_fallible(
            *cur,
            Ty::Float,
            InstKind::HelperCall {
                helper,
                args: vec![v],
            },
            env,
        )
    }

    /// `new Target(...)` — the constructed class and its resolved
    /// constructor (if any) come from `self.exprs`, not from `target`
    /// itself: `target` may be `self`/`static`/`parent`, which this
    /// crate has no enclosing-class context to resolve on its own
    /// (see `lower_decl_type`'s doc comment).
    /// ADR 0031's `fn` literal. Evaluating one allocates its
    /// captured-environment object and stores a snapshot of every
    /// captured binding into it — "by value at the point the closure
    /// literal is evaluated" (§ 2), which is exactly what a field
    /// store at this program point is. The body itself becomes that
    /// class's one method, lowered later; see `lower_closure`, which
    /// owns the whole representation.
    fn lower_closure_literal(
        &mut self,
        fn_expr: &FnExpr,
        expr: &Expr,
        env: &mut Env,
        cur: &mut BlockId,
    ) -> (ValueId, Ty) {
        let Some(ExprInfo::Closure {
            class,
            captures,
            return_ty,
        }) = self.exprs.lookup(expr.span)
        else {
            panic!(
                "mwl-ir: the `fn` literal at {:?} has no resolved closure recorded in \
                 the typed-expression table — did this program pass \
                 mwl_types::check_program with the same table?",
                expr.span
            );
        };
        let class = class.clone();
        let ret = lower_checked_ty(*return_ty, self.checked_types);
        let names: Vec<String> = captures.iter().map(|(n, _)| n.clone()).collect();
        let (obj, _) = self.emit(
            *cur,
            Ty::Object,
            InstKind::New {
                class: class.clone(),
                ctor: None,
                args: Vec::new(),
            },
        );
        let arity = i64::try_from(fn_expr.params.len()).expect("a parameter list fits an i64");
        let (arity_v, _) = self.emit(*cur, Ty::Int, InstKind::ConstInt(arity));
        self.emit_field_set(*cur, obj, class.clone(), FN_ARITY.to_owned(), arity_v);
        let mut captured = Vec::with_capacity(names.len());
        for name in names {
            let &(v, ty) = env.get(&name).unwrap_or_else(|| {
                panic!(
                    "mwl-ir: the closure at {:?} captures `${name}`, which is not bound \
                     in the enclosing frame — mwl_types records a capture only for a \
                     name its own scope resolved",
                    expr.span
                )
            });
            // A `&$x` parameter binds an address, not a value, and ADR 0031 § 2
            // captures by value — so the field takes a snapshot of the cell's
            // value here, which is the same `RefLoad` at the declared (pointee)
            // type that reading `$x` anywhere else lowers to. That is also what
            // makes the closure safe to outlive the call that staged the cell:
            // it holds a copy, and the retain below gives the copy its own
            // reference exactly as it does for any other captured value.
            let (v, ty) = if ty == Ty::Ref {
                let pointee = self.pointee_of(&name);
                self.emit(*cur, pointee, InstKind::RefLoad { slot: v })
            } else {
                (v, ty)
            };
            if ty.is_refcounted() {
                self.emit_retain(*cur, v);
            }
            self.emit_field_set(*cur, obj, class.clone(), name.clone(), v);
            captured.push((name, ty));
        }
        self.closures.push(PendingClosure {
            class,
            fn_expr: fn_expr.clone(),
            captures: captured,
            ret,
        });
        (obj, Ty::Object)
    }

    fn lower_new(
        &mut self,
        target: &NewTarget,
        args: &CallArgs,
        expr: &Expr,
        env: &mut Env,
        cur: &mut BlockId,
    ) -> (ValueId, Ty) {
        let Some(ExprInfo::New { class, ctor, .. }) = self.exprs.lookup(expr.span) else {
            panic!(
                "mwl-ir: `new` at {:?} has no resolved class recorded in the \
                 typed-expression table — did this program pass \
                 mwl_types::check_program with the same table?",
                expr.span
            );
        };
        let target_label = class.to_string();
        // The declaring class, not the constructed one: `new Dog(...)`
        // on a `Dog` with no `constructor` of its own invokes
        // `Animal::constructor`. Only `mwl_types` resolved that, so
        // the label is carried rather than re-derived downstream.
        let ctor_label = ctor
            .as_ref()
            .map(|call| format!("{}::{}", call.class, call.method));
        // A `Core`-owned class is built by a native helper rather than by an
        // `InstKind::New`: nothing below this crate holds a descriptor for
        // one, because a `Core` class is in no program's class list.
        // `mwl_stdlib::instance`'s module docs own that decision; here it is
        // the same `InstKind::CoreCall` a static `Core` member lowers to,
        // with the same **borrowed** arguments — which is why this branch
        // lowers them itself rather than sharing the transferred ones below.
        // `mwl_stdlib::registry::CONSTRUCTORS` is what gives it a signature to
        // check them against: `new Core\Heap<T>($by)` carries one.
        if let Some(symbol) = mwl_types::core_constructor_symbol(&target_label) {
            let mark = self.temporaries_mark();
            let values = match ctor {
                Some(call) => {
                    let sig = ArgSig::of_helper(call);
                    let checked_types = self.checked_types;
                    self.lower_call_args(
                        args,
                        &sig,
                        checked_types,
                        ArgOwnership::Borrowed,
                        env,
                        cur,
                    )
                    .values
                }
                None => Vec::new(),
            };
            let built = self.emit_fallible(
                *cur,
                Ty::Object,
                InstKind::CoreCall {
                    symbol,
                    args: values,
                },
                env,
            );
            self.release_temporaries_since(mark, *cur);
            return built;
        }
        let arg_values = match ctor {
            Some(call) => {
                let sig = ArgSig::of(call);
                let checked_types = self.checked_types;
                self.lower_call_args(
                    args,
                    &sig,
                    checked_types,
                    ArgOwnership::Transferred,
                    env,
                    cur,
                )
                .values
            }
            None => {
                let CallArgs::List(list) = args else {
                    panic!(
                        "mwl-ir: `new {target_label}(...)` has no resolved constructor \
                         but wasn't called with a plain argument list — {args:?}"
                    );
                };
                assert!(
                    list.is_empty(),
                    "mwl-ir: `new {target_label}(...)` has no resolved constructor but \
                     was called with arguments — mwl_types doesn't yet enforce a \
                     zero-arity check here (see its own known gaps), so this crate \
                     cannot trust it was rejected upstream"
                );
                Vec::new()
            }
        };
        // `new static()` — ADR-free by construction: the class comes
        // from this frame's called class rather than from the label
        // `mwl_types` resolved, which is the enclosing class and so
        // would allocate the *base* through two levels of
        // inheritance. `new self()`/`new parent()`/`new Foo()` all
        // name a fixed class and keep the constant form.
        let kind = if matches!(target, NewTarget::StaticTy) {
            let desc = self.lsb();
            InstKind::NewDynamic {
                desc,
                ctor: ctor_label,
                args: arg_values,
            }
        } else {
            InstKind::New {
                class: target_label,
                ctor: ctor_label,
                args: arg_values,
            }
        };
        self.emit_fallible(*cur, Ty::Object, kind, env)
    }

    /// `$obj->method(...)`/`$this->method(...)` — the receiver is
    /// lowered like any other expression (for `$this`, that's just an
    /// `Env` lookup, since `lower_method` already seeded it as the
    /// implicit parameter 0); the resolved target itself still comes
    /// from `self.exprs`, exactly like a static call/`new` below.
    fn lower_method_call(
        &mut self,
        object: &Expr,
        nullsafe: bool,
        args: &CallArgs,
        expr: &Expr,
        env: &mut Env,
        cur: &mut BlockId,
    ) -> (ValueId, Ty) {
        let Some(ExprInfo::Call(call)) = self.exprs.lookup(expr.span) else {
            panic!(
                "mwl-ir: an instance method call at {:?} has no resolved target \
                 recorded in the typed-expression table — either it wasn't checked \
                 with the same table, or its receiver was a `mixed`, a union or a \
                 scalar, which the checker does not yet refuse (an *erased* one is \
                 `E0477`); see the crate docs' known gaps",
                expr.span
            );
        };
        // A member of a `Core`-owned class is native Rust behind a
        // helper symbol, exactly as a static `Core` member is — the
        // same `InstKind::CoreCall`, the same borrowed arguments, with
        // the receiver in argument slot 0. Resolved through the
        // identical `ResolvedCall` up to this point, which is why
        // `mwl_types` seeds a signature table rather than special-
        // casing `Core`; see `mwl_stdlib::registry::CoreTy::Instance`.
        if let Some(symbol) = mwl_types::core_symbol_of(&call.class, &call.method) {
            let sig = ArgSig::of_helper(call);
            let return_ty = lower_checked_ty(call.return_ty, self.checked_types);
            let checked_types = self.checked_types;
            let mark = self.temporaries_mark();
            let (object_v, receiver_ty, guard) =
                self.open_nullsafe(object, nullsafe, ReceiverProof::Proven, env, cur);
            // The receiver is borrowed like every other argument to a
            // `Core` member, so a *freshly built* one — a nested
            // call's own result — has no other owner and this frame
            // owes its release. A receiver read out of a local or a
            // field is that binding's to release, not this call's.
            // Staged *before* the arguments, since it is already in
            // flight while they are evaluated and an argument that
            // throws has to drop it.
            if receiver_ty.is_refcounted() && !self.aliasing_read(object) {
                self.own_temporary(object_v);
            }
            let LoweredArgs { values } =
                self.lower_call_args(args, &sig, checked_types, ArgOwnership::Borrowed, env, cur);
            let mut arg_values = Vec::with_capacity(values.len() + 1);
            arg_values.push(object_v);
            arg_values.extend(values);
            let (v, ty) = self.emit_fallible(
                *cur,
                return_ty,
                InstKind::CoreCall {
                    symbol,
                    args: arg_values,
                },
                env,
            );
            self.release_temporaries_since(mark, *cur);
            return self.close_nullsafe(guard, v, ty, env, cur);
        }
        let target_label = format!("{}::{}", call.class, call.method);
        let sig = ArgSig::of(call);
        let return_ty = lower_checked_ty(call.return_ty, self.checked_types);
        let checked_types = self.checked_types;
        let is_static = call.is_static;
        // `?->` guards everything below on the receiver not being
        // `null`; `->` opens no guard and lowers exactly as before.
        let (object_v, receiver_ty, guard) =
            self.open_nullsafe(object, nullsafe, ReceiverProof::Proven, env, cur);
        // A `static` method reached through an instance
        // (`$obj->staticMethod()`, which PHP allows) takes no
        // receiver: its parameter 0 is the *called* class, which here
        // is the receiver's own — see `mwl_runtime::object`'s module
        // docs. Nothing is retained for it; a descriptor is not
        // refcounted.
        let receiver_v = if is_static {
            let (v, _) = self.emit(
                *cur,
                Ty::ClassDesc,
                InstKind::ClassDescOf { object: object_v },
            );
            v
        } else {
            // The receiver is parameter 0, so it is an ordinary
            // argument for ownership purposes: MWL's convention is
            // that the caller retains an aliasing argument and the
            // callee releases every refcounted parameter at scope exit
            // (see `Self::release_all_locals`). `$this->m()` and
            // `$obj->m()` both read an existing slot, so both need the
            // retain `Self::lower_call_args` already inserts for one.
            if receiver_ty.is_refcounted() && self.aliasing_read(object) {
                self.emit_retain(*cur, object_v);
            }
            object_v
        };
        let arg_values = self
            .lower_call_args(
                args,
                &sig,
                checked_types,
                ArgOwnership::Transferred,
                env,
                cur,
            )
            .values;
        // Two shapes have no static answer, and both take the
        // receiver's own class instead.
        //
        // A resolved declaration with **no body** names no compiled
        // function at all — an `abstract` method, or the interface
        // method an interface *default* body calls back into
        // (`$this->name()` inside `Greets::greet`).
        //
        // A resolved declaration some subtype **overrides** names the
        // wrong one: `$base->m()` on a value that is really a `Child`
        // must run `Child::m`. `mwl_types` answers that whole-program
        // question once (`ResolvedCall::overridden`), so the ordinary
        // case — a method nothing overrides — still binds straight to
        // a label and pays nothing. A `static` method reached through
        // an instance is never virtual: PHP resolves it on the
        // written class, and its slot 0 carries a descriptor rather
        // than a receiver.
        let late_bound = !call.has_body || (call.overridden && !is_static);
        let kind = if late_bound {
            let (lsb, _) = self.emit(
                *cur,
                Ty::ClassDesc,
                InstKind::ClassDescOf { object: object_v },
            );
            InstKind::CallVirtual {
                lsb,
                method: call.method.clone(),
                fallback: call.has_body.then_some(target_label),
                receiver: if is_static { None } else { Some(receiver_v) },
                args: arg_values,
            }
        } else {
            InstKind::Call {
                target: target_label,
                receiver: Some(receiver_v),
                args: arg_values,
            }
        };
        let (v, ty) = self.emit_fallible(*cur, return_ty, kind, env);
        self.close_nullsafe(guard, v, ty, env, cur)
    }

    /// `parent::method(...)`/`self::method(...)`/`Class::method(...)`.
    ///
    /// Written like a static call, but *not* necessarily one: PHP's
    /// `parent::constructor(...)` and `self::helper()` invoke an
    /// instance method on the enclosing `$this` whenever the resolved
    /// target is not declared `static`. So the receiver is decided by
    /// `ResolvedCall::is_static` rather than by the `::` in the source
    /// — passing `null` to a method that reads `$this` would be a
    /// null-pointer write into a field slot, not a diagnostic.
    ///
    /// The `::`'s left-hand side decides the *called* class the callee
    /// sees (`mwl_runtime::object`'s late-static-binding decision):
    /// `Foo::m()` sets it to `Foo`, `self::`/`parent::` forward this
    /// frame's, and `static::m()` additionally resolves the target
    /// itself at run time through `InstKind::CallVirtual`.
    fn lower_static_call(
        &mut self,
        class: &Expr,
        args: &CallArgs,
        expr: &Expr,
        env: &mut Env,
        cur: &mut BlockId,
    ) -> (ValueId, Ty) {
        let Some(ExprInfo::Call(call)) = self.exprs.lookup(expr.span) else {
            panic!(
                "mwl-ir: a static call at {:?} has no resolved target recorded in the \
                 typed-expression table — did this program pass \
                 mwl_types::check_program with the same table?",
                expr.span
            );
        };
        // A Tier 0 `Core` member is native Rust behind a helper
        // symbol, not a compiled MWL function, so it takes a
        // different instruction and a different argument-ownership
        // rule — see `InstKind::CoreCall`, which owns both. Resolved
        // through the identical `ResolvedCall` up to this point,
        // which is the whole reason `mwl_types` seeds a signature
        // table rather than special-casing `Core` at each call site.
        if let Some(symbol) = mwl_types::core_symbol_of(&call.class, &call.method) {
            let sig = ArgSig::of_helper(call);
            let return_ty = lower_checked_ty(call.return_ty, self.checked_types);
            let checked_types = self.checked_types;
            // A member on `mwl_stdlib::registry::WRITTEN_CLASS_MEMBERS`
            // is handed the class its call site wrote, as argument 0 —
            // that roster owns the ABI. A descriptor is not
            // refcounted, so it is neither retained nor released here.
            let written_class =
                mwl_types::core_takes_written_class(&call.class.to_string(), &call.method).then(
                    || {
                        let label = call.written_class.as_ref().unwrap_or_else(|| {
                            panic!(
                                "mwl-ir: `{}::{}` needs the class written at its call site, \
                         and mwl_types recorded none — did this program pass \
                         mwl_types::check_program with the same table?",
                                call.class, call.method
                            )
                        });
                        let (v, _) = self.emit(
                            *cur,
                            Ty::ClassDesc,
                            InstKind::ClassDescConst {
                                class: label.to_string(),
                            },
                        );
                        v
                    },
                );
            let mark = self.temporaries_mark();
            let lowered =
                self.lower_call_args(args, &sig, checked_types, ArgOwnership::Borrowed, env, cur);
            let arg_values = written_class
                .into_iter()
                .chain(lowered.values)
                .collect::<Vec<_>>();
            let result = self.emit_fallible(
                *cur,
                return_ty,
                InstKind::CoreCall {
                    symbol,
                    args: arg_values,
                },
                env,
            );
            // A `Core` member borrows, so a freshly built argument —
            // an `fn` literal, a nested `Core` call's own result — has
            // no other owner and would leak without this.
            self.release_temporaries_since(mark, *cur);
            return result;
        }
        let target_label = format!("{}::{}", call.class, call.method);
        let method = call.method.clone();
        let sig = ArgSig::of(call);
        let is_static = call.is_static;
        let has_body = call.has_body;
        let named_class = call.static_class.as_ref().map(ToString::to_string);
        let return_ty = lower_checked_ty(call.return_ty, self.checked_types);
        let checked_types = self.checked_types;
        // `static::m()` never has a compile-time target; a resolved
        // declaration with no body has none either, for a different
        // reason — see `InstKind::CallVirtual::fallback`.
        let late_bound = matches!(class.kind, ExprKind::StaticExpr) || !has_body;
        let receiver = if is_static {
            // A static callee has no `$this`, so its receiver slot
            // carries the *called* class instead — an explicitly named
            // one sets it, `self::`/`parent::`/`static::` forward this
            // frame's. See `mwl_runtime::object`'s module docs.
            Some(match &named_class {
                Some(label) => {
                    let (v, _) = self.emit(
                        *cur,
                        Ty::ClassDesc,
                        InstKind::ClassDescConst {
                            class: label.clone(),
                        },
                    );
                    v
                }
                None => self.lsb(),
            })
        } else {
            // The enclosing frame's own `$this`, retained the same way
            // an explicit `$obj->m()` receiver is — the callee will
            // release it. A file-scope frame has none, which the
            // checker has already refused for a non-static target.
            let &(this_v, this_ty) = env.get("this").unwrap_or_else(|| {
                panic!(
                    "mwl-ir: `{target_label}` is not static but is reached from a frame \
                     with no `$this` — mwl_types is expected to have refused that"
                )
            });
            if this_ty.is_refcounted() {
                self.emit_retain(*cur, this_v);
            }
            Some(this_v)
        };
        let arg_values = self
            .lower_call_args(
                args,
                &sig,
                checked_types,
                ArgOwnership::Transferred,
                env,
                cur,
            )
            .values;
        let kind = if late_bound {
            // `static::m()` — the target is whichever class this frame
            // was *called* on, which is only known at run time.
            let lsb = self.lsb();
            InstKind::CallVirtual {
                lsb,
                method,
                fallback: has_body.then_some(target_label),
                // A static target's slot 0 already holds `lsb`, so the
                // dispatch value and the receiver are the same value;
                // saying it once keeps `emit_invoke`'s slot rule
                // identical to `InstKind::Call`'s.
                receiver: if is_static { None } else { receiver },
                args: arg_values,
            }
        } else {
            InstKind::Call {
                target: target_label,
                receiver,
                args: arg_values,
            }
        };
        self.emit_fallible(*cur, return_ty, kind, env)
    }

    /// `Class::$prop` — one [`InstKind::StaticGet`] against the slot the
    /// checker's resolved `(declaring class, name)` pair names.
    ///
    /// No receiver is lowered and the written class expression is not looked
    /// at: `self`, `static`, `parent` and a spelled-out name all resolve to
    /// the same declaring class, and late static binding has nothing to say
    /// about storage that is not per-instance. That is why this is a read with
    /// no base to release, unlike [`Self::lower_property_access`].
    ///
    /// # Panics
    ///
    /// Panics when the typed-expression table has no
    /// `ExprInfo::StaticProperty` for this access — the same
    /// internal-consistency check a property access makes, and reachable only
    /// from a body checked against a different table.
    pub(super) fn lower_static_property(
        &mut self,
        expr: &Expr,
        cur: &mut BlockId,
    ) -> (ValueId, Ty) {
        let (class, name, ty) = self.static_property_of(expr);
        self.emit(*cur, ty, InstKind::StaticGet { class, name })
    }

    /// The resolved `(declaring class label, property name, representation)`
    /// behind a `Class::$prop` access — the one place the read side and
    /// `Lowering::lower_store`'s write side agree on what a static names.
    ///
    /// # Panics
    ///
    /// See [`Self::lower_static_property`].
    pub(super) fn static_property_of(&self, expr: &Expr) -> (String, String, Ty) {
        let Some(ExprInfo::StaticProperty { class, name, ty }) = self.exprs.lookup(expr.span)
        else {
            panic!(
                "mwl-ir: a static property access at {:?} has no resolved declaring class \
                 recorded in the typed-expression table — it wasn't checked with the same table",
                expr.span
            );
        };
        (
            class.to_string(),
            name.clone(),
            lower_checked_ty(*ty, self.checked_types),
        )
    }

    /// `$obj->prop` — the receiver's declaring class comes from
    /// `self.exprs`, exactly like a call's resolved target. Every receiver
    /// with **no** declaring class to resolve — a shape (naming one of its
    /// own fields or not), a plain `object`, and a `mixed` — records
    /// `ExprInfo::ShapeProperty` instead and is handed to
    /// [`Self::lower_shape_property_access`], which is ADR 0036 § 4's
    /// name-keyed fetch.
    ///
    /// # Panics
    ///
    /// Panics when the typed-expression table holds neither entry for this
    /// access. That is an internal-consistency check rather than a hole: a
    /// receiver whose type can hold no object at all is `E0495` at the
    /// checker (ADR 0007 § 7 row 13), so a body that reaches here was
    /// checked against a different table.
    fn lower_property_access(
        &mut self,
        object: &Expr,
        nullsafe: bool,
        expr: &Expr,
        env: &mut Env,
        cur: &mut BlockId,
    ) -> (ValueId, Ty) {
        // ADR 0014 § 1: a read of a property that declares a `get`
        // hook is a call to that hook's compiled function, with the
        // receiver in the ordinary parameter-0 slot — see
        // `lower_property_hook`. A property with only a `set` hook
        // still reads its own slot, since MWL's hooked properties are
        // always backed (`mwl_types::signatures::PropertyHooks` owns
        // that decision), so both shapes recover the same three
        // fields and only the `get` label decides between them.
        // An ADR 0036 § 4 shape receiver naming one of its own fields is the
        // one access with no class to resolve: the slot index is already in
        // the table, so this reads it and is done. Everything below — the
        // hook question, the declaring class, the label — is a class
        // receiver's problem and none of it applies.
        if let Some(ExprInfo::ShapeProperty { name, slot, ty }) = self.exprs.lookup(expr.span) {
            let field = ShapeField {
                name: name.clone(),
                slot: *slot,
                ty: *ty,
            };
            return self.lower_shape_property_access(object, &field, nullsafe, env, cur);
        }
        let (class, name, ty, get) = match self.exprs.lookup(expr.span) {
            Some(ExprInfo::Property { class, name, ty }) => (class, name, *ty, None),
            Some(ExprInfo::HookedProperty {
                class,
                name,
                ty,
                get,
                ..
            }) => (class, name, *ty, get.clone()),
            _ => panic!(
                "mwl-ir: a property access at {:?} has neither a resolved declaring class \
                 nor an ADR 0036 § 4 erased entry recorded in the typed-expression table, \
                 so it was not checked with the same table — every erased receiver records \
                 one and every receiver that can hold no object at all is `E0495`",
                expr.span
            ),
        };
        let field_ty = lower_checked_ty(ty, self.checked_types);
        let class_label = class.to_string();
        let field_name = name.clone();
        // See the `MethodCall` arm above: `?->` guards the access on
        // the receiver not being `null`, `->` opens no guard.
        let mark = self.temporaries_mark();
        let (object_v, receiver_ty, guard) =
            self.open_nullsafe(object, nullsafe, ReceiverProof::Proven, env, cur);
        // A base that is itself a fresh producer — `$m->make()->name` — has
        // no other owner, so this frame owes its release. Only the slot read
        // stages it: a `get` hook's receiver is parameter 0 and the callee's
        // own exit sweep releases it, which is what the retain below is
        // deliberately skipped for. Staged before the read so a throw on the
        // way drops it too, exactly like the `Core`-call arm's receiver.
        let base_is_temporary =
            get.is_none() && receiver_ty.is_refcounted() && !self.aliasing_read(object);
        if base_is_temporary {
            self.own_temporary(object_v);
        }
        let (v, ty) = match get {
            Some(label) => {
                // The receiver is parameter 0, so it is an ordinary
                // argument for ownership purposes — the same retain
                // an explicit `$obj->m()` inserts, for the same reason
                // (the callee releases every refcounted parameter at
                // scope exit).
                if receiver_ty.is_refcounted() && self.aliasing_read(object) {
                    self.emit_retain(*cur, object_v);
                }
                self.emit_fallible(
                    *cur,
                    field_ty,
                    InstKind::Call {
                        target: label,
                        receiver: Some(object_v),
                        args: Vec::new(),
                    },
                    env,
                )
            }
            None => self.emit(
                *cur,
                field_ty,
                InstKind::FieldGet {
                    object: object_v,
                    class: class_label,
                    field: field_name,
                },
            ),
        };
        if base_is_temporary {
            // The slot's reference dies with the base, so the value read out
            // of it needs one of its own first — `FieldGet` borrows, and
            // releasing the object underneath a borrowed result is a
            // use-after-free rather than a leak. That makes this whole
            // expression a *fresh producer*, which is why
            // `Lowering::aliasing_read` reports a property read off a
            // temporary as non-aliasing: the consumer must not retain it a
            // second time.
            if ty.is_refcounted() {
                self.emit_retain(*cur, v);
            }
            self.release_temporaries_since(mark, *cur);
        }
        self.close_nullsafe(guard, v, ty, env, cur)
    }

    /// `{x: 1, y: 2}` — [ADR 0036](../../../docs/adr/0036-anonymous-object-shapes.md)
    /// § 2's anonymous object literal, which is an ordinary instance of a
    /// class this function invents: one [`InstKind::New`] with no constructor,
    /// then one [`InstKind::FieldSet`] per field.
    ///
    /// **The class is per *shape*, not per site** — [`shape_class_label`]
    /// renders the sorted field-name list into the label, so every occurrence
    /// of `{x: …, y: …}` anywhere in the unit names the same synthesized
    /// class and the table carries one copy of it ([`super::lower_file`]
    /// dedups). Sorted because that is the order
    /// `mwl_types::ty::TypeInterner::shape` interns a shape's fields in, and
    /// therefore the order [`InstKind::SlotGet`]'s index counts through: the
    /// read side resolved its slot number from the checker's field list, so
    /// the write side has to lay the slots out the same way. That agreement
    /// is the whole reason a shape needs no layout table.
    ///
    /// Nothing else is synthesized. The class is methodless, conforms to
    /// nothing and declares no constructor — the literal assigns every field
    /// itself, which is ADR 0022 § 2's definite-assignment obligation
    /// discharged by construction — so `mwl-codegen` defines it through
    /// `Classes::define` like any other class and no codegen knows a shape
    /// exists.
    ///
    /// The field values are lowered in **source** order, whatever the sorted
    /// slot order is: a field initializer can call, and a call can have
    /// effects. Each is written by name, so the two orders never have to meet.
    /// A value that is an aliasing read is retained before the slot durably
    /// owns it, exactly as [`Self::lower_array_literal`]'s elements are; the
    /// object under construction is not on the owned-temporaries stack, so a
    /// throw from a later field's initializer abandons it — the identical
    /// edge a keyed array literal already has, and closed for both at once or
    /// not at all.
    ///
    /// # Panics
    ///
    /// The assert on a literal that writes one field name twice is an
    /// internal-consistency check rather than a gap: a shape's fields are a
    /// set, so `mwl_types::expr::literals::check_object_literal` refuses the
    /// repeat where it is written as `E0494` and nothing that reaches here
    /// carries one. It stays because the disagreement it would otherwise hide
    /// is silent — the interned shape reads the first of the pair and the
    /// class below carries one slot per name.
    fn lower_object_literal(
        &mut self,
        fields: &[ObjectLiteralField],
        env: &mut Env,
        cur: &mut BlockId,
    ) -> (ValueId, Ty) {
        let names: Vec<String> = fields
            .iter()
            .map(|field| span_text(self.src, field.name).to_owned())
            .collect();
        let mut sorted = names.clone();
        sorted.sort();
        sorted.dedup();
        assert!(
            sorted.len() == names.len(),
            "an object literal writing one field name twice reached lowering: a shape's \
             fields are a set, so `mwl_types::expr::literals::check_object_literal` refuses \
             the repeat where it is written, as `E0494` — the interned shape reads the \
             first of the pair while this class carries one slot per name, and there is no \
             layout the two sides agree on"
        );
        let class = shape_class_label(&sorted);
        let (obj, _) = self.emit(
            *cur,
            Ty::Object,
            InstKind::New {
                class: class.clone(),
                ctor: None,
                args: Vec::new(),
            },
        );
        // What each field's initializer lowered to, keyed by name so the
        // list handed to `record_shape_class` is in the class's own sorted
        // slot order rather than the literal's written one. This is the only
        // record of a shape field's type that survives to run time, and
        // `InstKind::SlotSet` is its one reader.
        let mut reprs: FxHashMap<&str, Ty> = FxHashMap::default();
        for (field, name) in fields.iter().zip(&names) {
            let (v, ty) = self.lower_expr(&field.value, None, env, cur);
            if ty.is_refcounted() && self.aliasing_read(&field.value) {
                self.emit_retain(*cur, v);
            }
            reprs.insert(name.as_str(), ty);
            self.emit_field_set(*cur, obj, class.clone(), name.clone(), v);
        }
        let reprs = sorted
            .iter()
            .map(|name| reprs[name.as_str()])
            .collect::<Vec<_>>();
        self.record_shape_class(class, sorted, reprs);
        (obj, Ty::Object)
    }

    /// `$issue->path` — the shape half of [`Self::lower_property_access`]
    /// (ADR 0036 § 4). A shape value is anonymous and methodless, so there is
    /// no declaring class, no hook question and no label; what `mwl_types`
    /// resolved is the field's *name* plus its position in the receiver's own
    /// sorted field list, and `InstKind::SlotGet` keys on the first and takes
    /// the second as a hint — see that variant's docs for why a widened view
    /// makes the position unusable on its own.
    ///
    /// Emitted through [`Self::emit_fallible`], because § 4 makes a name the
    /// concrete class does not carry a catchable throw. Nothing the checker
    /// records a `ShapeProperty` for can reach that edge — a field the
    /// receiver's shape lists is proven present — so the landing block is the
    /// price of the erased half of § 4 being expressible at all.
    ///
    /// The refcounting is the class receiver's, minus the hook case.
    /// `InstKind::SlotGet` borrows, so a base that is itself a fresh producer
    /// — `$e->issues["0"]->path` — has to hand the value read out of it a
    /// reference of its own before the base is released underneath it.
    fn lower_shape_property_access(
        &mut self,
        object: &Expr,
        field: &ShapeField,
        nullsafe: bool,
        env: &mut Env,
        cur: &mut BlockId,
    ) -> (ValueId, Ty) {
        let field_ty = lower_checked_ty(field.ty, self.checked_types);
        let mark = self.temporaries_mark();
        let (object_v, receiver_ty, guard) =
            self.open_nullsafe(object, nullsafe, ReceiverProof::Erased, env, cur);
        let base_is_temporary = receiver_ty.is_refcounted() && !self.aliasing_read(object);
        if base_is_temporary {
            self.own_temporary(object_v);
        }
        let (v, ty) = self.emit_fallible(
            *cur,
            field_ty,
            InstKind::SlotGet {
                object: object_v,
                field: field.name.clone(),
                slot: field.slot,
            },
            env,
        );
        if base_is_temporary {
            if ty.is_refcounted() {
                self.emit_retain(*cur, v);
            }
            self.release_temporaries_since(mark, *cur);
        }
        self.close_nullsafe(guard, v, ty, env, cur)
    }

    /// `$issue->path = "x";` — [`Self::lower_shape_property_access`]'s write
    /// half (ADR 0036 § 4), and the same three facts about the field: no
    /// declaring class, no hook, the name plus the receiver's own slot index.
    ///
    /// [`InstKind::SlotSet`] owns why this is one fallible call rather than
    /// the `FieldGet`/`Release`/`FieldSet` sequence a named class's property
    /// write lowers to. Two consequences show up here:
    ///
    /// * **Nothing is read back.** The runtime releases what the slot held,
    ///   because the field's concrete type — and so whether it is refcounted
    ///   at all — is not a fact this site has.
    /// * **The value is borrowed, not transferred.** The runtime retains what
    ///   it stores, so a value this expression *built* is staged as an
    ///   ordinary owned temporary and swept on whichever edge is taken. That
    ///   is the mirror image of the receiver's own accounting, and of the
    ///   retain-if-aliasing the named-class write in `crate::lower::stmt`
    ///   does: this instruction can throw after both operands are in hand,
    ///   and a transferred reference on that edge would have no owner left.
    pub(super) fn lower_shape_property_assign(
        &mut self,
        object: &Expr,
        field: &ShapeField,
        value: &Stored<'_>,
        extra_owner: bool,
        env: &mut Env,
        cur: &mut BlockId,
    ) -> (ValueId, Ty) {
        let field_ty = lower_checked_ty(field.ty, self.checked_types);
        let mark = self.temporaries_mark();
        // A narrowed `?{...}` receiver arrives tagged, and so does a `mixed`
        // one — and neither is untagged here. [`InstKind::SlotSet`] takes the
        // receiver as it finds it and checks the tag where it checks the
        // name, which is [`ReceiverProof::Erased`] on the read side and the
        // same rule for the same reason: an unchecked untag over a `mixed`
        // holding an `int` is a pointer the runtime would then store through.
        let (object_v, receiver_ty) = self.lower_expr(object, None, env, cur);
        if receiver_ty.is_refcounted() && !self.aliasing_read(object) {
            self.own_temporary(object_v);
        }
        let (v, vty, aliasing) = self.lower_stored(value, Some(field_ty), env, cur);
        let v = self.coerce(*cur, v, vty, field_ty, env);
        if field_ty.is_refcounted() && !aliasing {
            self.own_temporary(v);
        }
        self.emit_fallible(
            *cur,
            Ty::Void,
            InstKind::SlotSet {
                object: object_v,
                field: field.name.clone(),
                slot: field.slot,
                value: v,
            },
            env,
        );
        // While this frame still holds a reference of its own: the slot owns
        // one too by now, but the release below is what ends *ours*.
        if field_ty.is_refcounted() && extra_owner {
            self.emit_retain(*cur, v);
        }
        self.release_temporaries_since(mark, *cur);
        (v, field_ty)
    }

    /// `[...]`/legacy `array(...)` — see `InstKind::ArrayNew`'s own
    /// doc comment for the full policy this mirrors and its known
    /// gaps. A *purely positional* literal (no element has an
    /// explicit `key =>`, and none is a `...spread`) keeps the
    /// original single-`ArrayNew` shape: each element's key is simply
    /// its index, auto-numbered from `0` exactly like PHP's own
    /// `[$a, $b]` shorthand, computed at lowering time with no runtime
    /// key instruction at all. Anything else instead builds an empty
    /// array first and writes one element at a time into it in source
    /// order — seeing `crate::ir::InstKind::ArrayNew`'s own doc
    /// comment for why that's the only shape general enough to give an
    /// explicit key's (possibly runtime-computed) value a place to
    /// live. Each value that's itself `Ty::is_refcounted` and
    /// `is_aliasing_read` is retained before the array durably owns
    /// it, the same policy `Self::lower_call_args` already applies at
    /// a call-argument boundary; an explicit key gets the identical
    /// treatment via `Self::lower_array_key`'s own aliasing flag. The
    /// array literal's own result needs no retain — a fresh producer,
    /// same as `new`/a call's result.
    ///
    /// **A `...spread` element is one [`InstKind::ArraySpread`]**, which
    /// copies the subject's entries in and owns which of their keys survive
    /// (ADR 0007 § 5). The subject is *borrowed*, so a freshly-built one is
    /// staged as this frame's temporary and released on whichever edge the
    /// copy takes, rather than transferred the way a written-out element is.
    ///
    /// A keyless element of a literal that contains a spread is an
    /// [`InstKind::ArrayAppend`] rather than a lowering-time index: after a
    /// spread there is no index this pass can compute, since how many entries
    /// arrived is the subject's own run-time length. That is PHP's real rule
    /// — "the next free integer key" — and it is why a literal with a spread
    /// does *not* share the one deliberate divergence the keyed shape still
    /// has, where a positional element's numbering ignores an explicit
    /// `int`-looking key elsewhere in the same literal.
    ///
    /// The array under construction is itself staged as an owned temporary
    /// while it is being filled, and re-pointed after every write. Both an
    /// element's own expression and the spread copy can throw, and the
    /// half-built array is named by no local and no other temporary, so
    /// without this the landing block would have nothing to release.
    fn lower_array_literal(
        &mut self,
        items: &[ArrayItem],
        env: &mut Env,
        cur: &mut BlockId,
    ) -> (ValueId, Ty) {
        // `&value` never arrives here, at any depth: `mwl_types` refuses it as
        // `E0483`, because ADR 0031 § 2 and ADR 0023 between them leave an
        // aliasing element no owner, so it is a shape the language does not
        // have rather than one this function has not learned.
        let spread = items.iter().any(|item| item.spread);
        if !spread && items.iter().all(|item| item.key.is_none()) {
            let mut entries = Vec::with_capacity(items.len());
            for (i, item) in items.iter().enumerate() {
                let (v, ty) = self.lower_expr(&item.value, None, env, cur);
                if ty.is_refcounted() && self.aliasing_read(&item.value) {
                    self.emit_retain(*cur, v);
                }
                entries.push((i.to_string(), v));
            }
            self.emit(*cur, Ty::Array, InstKind::ArrayNew { entries })
        } else {
            let array = self.emit(
                *cur,
                Ty::Array,
                InstKind::ArrayNew {
                    entries: Vec::new(),
                },
            );
            let mut next_index = 0usize;
            // Each write yields the array the next one writes into —
            // the same pointer every time here, since a literal under
            // construction is solely owned, but threaded rather than
            // assumed so the one protocol has no exception.
            let mut array_v = array.0;
            let slot = self.temporaries_mark();
            self.own_temporary(array_v);
            for item in items {
                if item.spread {
                    let mark = self.temporaries_mark();
                    let (subject, subject_ty) = self.lower_expr(&item.value, None, env, cur);
                    if subject_ty.is_refcounted() && !self.aliasing_read(&item.value) {
                        self.own_temporary(subject);
                    }
                    array_v = self
                        .emit_fallible(
                            *cur,
                            Ty::Array,
                            InstKind::ArraySpread {
                                array: array_v,
                                subject,
                            },
                            env,
                        )
                        .0;
                    self.retarget_temporary(slot, array_v);
                    self.release_temporaries_since(mark, *cur);
                    continue;
                }
                let key_v = match &item.key {
                    Some(key) => {
                        let (key_v, _key_ty, key_aliasing) = self.lower_array_key(key, env, cur);
                        if key_aliasing {
                            self.emit_retain(*cur, key_v);
                        }
                        Some(key_v)
                    }
                    // The append the doc comment above explains: a spread's
                    // length is not a lowering-time fact.
                    None if spread => None,
                    None => {
                        let key_str = next_index.to_string();
                        next_index += 1;
                        let (kv, _) = self.emit(*cur, Ty::Str, InstKind::ConstStr(key_str));
                        Some(kv)
                    }
                };
                let (v, ty) = self.lower_expr(&item.value, None, env, cur);
                if ty.is_refcounted() && self.aliasing_read(&item.value) {
                    self.emit_retain(*cur, v);
                }
                array_v = match key_v {
                    Some(key_v) => self.emit_array_set(*cur, array_v, key_v, v),
                    None => self.emit_array_append(*cur, array_v, v, env),
                };
                self.retarget_temporary(slot, array_v);
            }
            self.forget_temporary(slot);
            (array_v, Ty::Array)
        }
    }
    /// `$arr[$i]` — the element's declared type comes from
    /// `self.exprs`, exactly like a property access's declaring
    /// class: a base that declares no element type has no
    /// `ExprInfo::Index` entry at all, and `mwl_types` refuses one as
    /// `E0482` where it is written, so the panic here is an invariant
    /// check rather than a gap. `base[]` (`index`
    /// is `None`) has no meaning as a read at all — it is PHP's
    /// append syntax, assignment-target-only — and `mwl_types`
    /// refuses it as `E0481` where it is written, so the arm here is
    /// an invariant check that no source file can reach.
    ///
    /// A read the checker marked **coalesce-guarded** — any level of the
    /// subscript chain under a `??`, so a guarded read's own base may be
    /// another one and may therefore be `null` at run time, which
    /// `mwl_array_optional_get` answers with `null` again — takes
    /// [`AbsentKey::Null`] instead of the throwing answer ADR
    /// 0007 § 7 row 11 gives every other read, and is therefore infallible and
    /// [`Ty::Tagged`]. That representation is not a widening for its own sake:
    /// `??` tests its left operand for `null` and needs a tag to test, and
    /// `Self::lower_coalesce` short-circuits away any operand that has none.
    fn lower_index(
        &mut self,
        base: &Expr,
        index: Option<&Expr>,
        expr: &Expr,
        env: &mut Env,
        cur: &mut BlockId,
    ) -> (ValueId, Ty) {
        let Some(index) = index else {
            panic!(
                "mwl-ir reached `$a[]` as a read expression — append syntax (`index` \
                 is `None`) is assignment-target-only and `mwl_types` refuses every \
                 other position as `E0481`, so this body was not checked"
            );
        };
        let Some(ExprInfo::Index { elem_ty, guarded }) = self.exprs.lookup(expr.span) else {
            panic!(
                "mwl-ir: an array-index read at {:?} has no resolved element type \
                 recorded in the typed-expression table — `mwl_types` refuses a base \
                 that declares none as `E0482`, so this body was not checked with the \
                 same table",
                expr.span
            );
        };
        let absent = if *guarded {
            AbsentKey::Null
        } else {
            AbsentKey::Throws
        };
        let result_ty = match absent {
            AbsentKey::Throws => lower_checked_ty(*elem_ty, self.checked_types),
            AbsentKey::Null => Ty::Tagged,
        };
        // Exactly `Self::lower_property_access`'s rule, one storage kind
        // along: a base that is itself a fresh producer — `$m->rows()["0"]` —
        // has no other owner, so this frame owes its release, and the element
        // read out of it needs a reference of its own first because
        // `ArrayGet` borrows. Staged on the owned-temporaries stack before the
        // read, so a throw on the way out of the key drops it too.
        let mark = self.temporaries_mark();
        let (array_v, base_ty) = self.lower_expr(base, None, env, cur);
        let base_is_temporary = base_ty.is_refcounted() && !self.aliasing_read(base);
        if base_is_temporary {
            self.own_temporary(array_v);
        }
        let (key_v, key_ty, key_aliasing) = self.lower_array_key(index, env, cur);
        // Only a rendered key is a reference this frame owns; an unrendered
        // `int` subscript owns nothing at all. It is staged rather than
        // released inline because the read below can throw: an absent key
        // leaves through the landing block, which releases the stack this
        // mark opened and would otherwise leave the rendered key behind.
        let key_is_temporary = key_ty.is_refcounted() && !key_aliasing;
        if key_is_temporary {
            self.own_temporary(key_v);
        }
        let kind = InstKind::ArrayGet {
            array: array_v,
            key: key_v,
            absent,
        };
        let result = match absent {
            AbsentKey::Throws => self.emit_fallible(*cur, result_ty, kind, env),
            AbsentKey::Null => self.emit(*cur, result_ty, kind),
        };
        if base_is_temporary {
            // That makes the whole expression a *fresh producer*, which is why
            // `Lowering::aliasing_read` reports an index read off a temporary
            // as non-aliasing: the consumer must not retain it a second time.
            // The retain goes first, since the release below drops the base
            // this borrowed element lives inside.
            if result_ty.is_refcounted() {
                self.emit_retain(*cur, result.0);
            }
        }
        if base_is_temporary || key_is_temporary {
            self.release_temporaries_since(mark, *cur);
        }
        result
    }

    /// `$x instanceof Name` — the tested class comes from
    /// `self.exprs`, exactly like a property access's declaring class,
    /// because resolving a bare `Animal` to `Ns\Animal` needs the
    /// namespace/import context this crate cannot see. Every spelling
    /// that records nothing — the dynamic `$x instanceof $name` form, a
    /// `Core` class, an enum, an undeclared name — is refused at the
    /// checker (`E0496`/`E0303`), so the miss below is an
    /// internal-consistency failure rather than a hole.
    ///
    /// **The subject may be a [`Ty::Tagged`], and the runtime checks its
    /// tag.** A `mixed` or an untested `?Box` is the shape `instanceof`
    /// exists for, so it travels as a whole value by address exactly as
    /// ADR 0036 § 4's name-keyed access does, and a tag that is not an
    /// object answers `false` rather than throwing — PHP's own answer,
    /// and the one every subject whose *declared* type can hold no
    /// object gets at compile time instead (`E0497`).
    fn lower_instanceof(
        &mut self,
        inner: &Expr,
        expr: &Expr,
        env: &mut Env,
        cur: &mut BlockId,
    ) -> (ValueId, Ty) {
        let Some(ExprInfo::InstanceOf { class }) = self.exprs.lookup(expr.span) else {
            panic!(
                "mwl-ir: an `instanceof` at {:?} has no resolved class recorded in the \
                 typed-expression table — it wasn't checked with the same table, every \
                 right-hand side naming no declared class being `E0496` or `E0303` at \
                 the checker",
                expr.span
            );
        };
        let class_label = class.to_string();
        let (value, ty) = self.lower_expr(inner, None, env, cur);
        assert!(
            matches!(ty, Ty::Object | Ty::Tagged),
            "mwl-ir lowers `instanceof` only against a subject that can hold an object — \
             got representation {ty:?}, every subject whose declared type cannot being \
             `E0497` at the checker"
        );
        let result = self.emit(
            *cur,
            Ty::Bool,
            InstKind::InstanceOf {
                value,
                class: class_label,
            },
        );
        // The subject is only read, so a fresh one nothing else owns —
        // `(new Dog()) instanceof Animal`, a call's return, a field read off a
        // temporary — is released once the test has read it. Same rule
        // [`Self::lower_clone_expr`] applies to its own operand, and the result
        // being a [`Ty::Bool`] is what makes "right after" safe.
        if !self.aliasing_read(inner) && ty.is_refcounted() {
            self.emit_release(*cur, value);
        }
        result
    }

    /// ADR 0023 § 1: PHP's shallow, same-heap, single-level copy, with
    /// no `__clone` hook to run — so the whole operation is one
    /// instruction, and the result is a fresh object with exactly one
    /// owner, the same as `new`.
    fn lower_clone_expr(
        &mut self,
        inner: &Expr,
        env: &mut Env,
        cur: &mut BlockId,
    ) -> (ValueId, Ty) {
        let (v, ty) = self.lower_expr(inner, None, env, cur);
        assert!(
            matches!(ty, Ty::Object),
            "mwl-ir lowers `clone` only for an object — got representation {ty:?}. ADR \
             0023 § 1 scopes `clone` to an object; an array is already a copy-on-write \
             value, and a scalar has nothing to copy"
        );
        let result = self.emit(*cur, Ty::Object, InstKind::Clone { object: v });
        // The operand is only *read* — see `InstKind::Clone`. A fresh
        // one nothing else owns is released right after, the same
        // "release a fresh value once its one and only use is done"
        // rule `Self::concat_operand`'s caller applies.
        if !self.aliasing_read(inner) {
            self.emit_release(*cur, v);
        }
        result
    }

    /// ADR 0007 § 2's `as` — the one conversion spelling. The target
    /// type is resolved by `lower_decl_type`, which reads the checker's
    /// own answer for the annotation, so an enum target/source is
    /// already the right representation by the time `convert` sees it.
    fn lower_conversion(
        &mut self,
        inner: &Expr,
        ty: &Type,
        env: &mut Env,
        cur: &mut BlockId,
    ) -> (ValueId, Ty) {
        // ADR 0066's `as ?T` is read off the *annotation*, before
        // `lower_decl_type` erases it: `?string` and `?int` are both
        // `Ty::Tagged`, so a conversion between them would look like
        // `from == to` — the one shape `Self::convert` answers by
        // doing nothing at all.
        //
        // Every `as ?T` that reaches here is a row of ADR 0007 § 2's table or
        // one of ADR 0047's types. A **class** target never does: ADR 0066
        // § 3's class row is absolute, so `mwl_types` has already refused it
        // with `E0473`. That row used to carry a two-class exception — the
        // parse roster, `$s as ?Core\Uri` — lowered here to one non-member
        // `CoreCall` on a symbol the expression table had to carry, since
        // every `?T` erases to `Ty::Tagged` and the class written did not
        // survive. § 3 withdrew it, and `Core\Uri::tryParse` is an ordinary
        // member call now, so nothing about a class reaches this function.
        match nullable_target(ty) {
            Some(target) => {
                // No placement here, unlike the arm below: placing a
                // literal at the target would make `3 as ?uint` the
                // `from == to` shape ADR 0066 § 3 calls a compile
                // error, which `mwl_types` does not refuse yet, so it
                // would panic where it now converts.
                let (v, from) = self.lower_expr(inner, None, env, cur);
                // ADR 0066 § 3 row 2 — a literal or enum-case target, the
                // "non-throwing twin" of the checked conversion. The target
                // is `T|null` minus `null`, which is the one place it still
                // exists: see `Self::nullable_target_atoms` for why the `T`
                // node's own span answers nothing.
                let Some(atoms) = self.nullable_target_atoms(ty) else {
                    let to = lower_decl_type(target, self.exprs, self.checked_types);
                    return self.convert_or_null(v, from, to, inner, *cur);
                };
                let to = shared_repr(&atoms, self.checked_types);
                let Some(accepted) = self.closed_set_of_atoms(&atoms, None, from) else {
                    return self.convert_or_null(v, from, to, inner, *cur);
                };
                self.lower_nullable_membership(v, from, to, &accepted, inner, ty.span, env, cur)
            }
            None => {
                let to = lower_decl_type(ty, self.exprs, self.checked_types);
                // ADR 0054 § 2: `expr as T` is itself a *placing*
                // position, so a numeric literal written directly
                // under one takes `T` as its target rather than being
                // typed first and converted afterwards. Mirrors
                // `mwl_types::expr::check_expr`'s own `Conversion`
                // arm, operand shape included — without it
                // `19.99 as decimal` would round-trip through an
                // `f64` and lose everything past ~17 digits.
                //
                // An enum target places at its *backing* scalar rather than
                // at `Ty::Enum` itself, because ADR 0010 § 5's row 2 is
                // written on that integer: without it `5 as Rank` over a
                // `uint`-backed enum would lower its literal to the `Ty::Int`
                // an unplaced one defaults to and then need a checked
                // `int → uint` to undo it.
                let placed =
                    matches!(inner.kind, ExprKind::Int(_) | ExprKind::Float(_)).then(|| match to {
                        Ty::Enum(EnumRepr::Int) => Ty::Int,
                        Ty::Enum(EnumRepr::Uint) => Ty::Uint,
                        other => other,
                    });
                let (v, from) = self.lower_expr(inner, placed, env, cur);
                let Some(accepted) = self.closed_literal_set(ty, inner, from) else {
                    return self.convert(v, from, to, inner, env, *cur);
                };
                // ADR 0047 § 5's membership test, on whichever side of
                // the base conversion still holds the value the author
                // wrote. A `Ty::Tagged` operand into a **literal** set is
                // tested **first**, against its own runtime tag:
                // converting one to the base would run
                // `Helper::TaggedToString`, which turns a `mixed` holding
                // `1` into `"1"` and would let it satisfy a set naming
                // `"1"` — exactly the coercion § 4's "throws unless the
                // value equals one of the named literals" refuses. Every
                // other operand is converted first instead, so the
                // comparison is over one representation and stays a
                // `BinOp::Eq` machine compare rather than
                // `mwl-codegen`'s refusal of a mismatched pair.
                //
                // An **enum** target is deliberately not in that first
                // case, whether the set is § 3's named subset or the whole
                // declaration. ADR 0010 § 5 words the `mixed → EnumName`
                // row as "exactly the shape `as uint` already has for
                // untrusted input", and its own example converts
                // `Core\Request::query('status')` — a string at run time —
                // into a case whose value is an integer. So the base
                // conversion runs first there, which is ADR 0007 § 2's
                // whole-string numeric row and not a coercion of its own,
                // and the chain then compares two integers. The `"1"`
                // hazard above cannot arise: an enum's base is never
                // `string`.
                if from == Ty::Tagged && !matches!(to, Ty::Enum(_)) {
                    self.lower_literal_membership(v, from, &accepted, ty.span, env, cur, None);
                    // A `bool` set is the one target with no conversion left
                    // to run. Every other base is reached by a row that
                    // happens to be an identity once the test above has
                    // passed (`Helper::TaggedToString` over a tag proved to
                    // be a string), but `as bool`'s row is ADR 0035's truthy
                    // table — and running it here would answer `true` for a
                    // `mixed` holding `1` that the test has just refused, or
                    // rather could not, since `Helper::Identical` compared
                    // tags first. The test *is* the check, so what is left is
                    // one unchecked `Untag`: the same shape
                    // `Self::untag_narrowed` emits over a tag `mwl_types`
                    // proved, on a tag this chain proved instead.
                    if to == Ty::Bool {
                        let (out, _) = self.emit(*cur, Ty::Bool, InstKind::Untag { operand: v });
                        // The proof says the payload is a `bool` and so owns
                        // nothing — but a fresh tagged operand is released
                        // anyway, exactly as every row of `Self::convert`
                        // releases one, because the consumer of a `Ty::Bool`
                        // never will and a release over a tag that owns
                        // nothing is a no-op the runtime already handles.
                        if !self.aliasing_read(inner) {
                            self.emit_release(*cur, v);
                        }
                        return (out, Ty::Bool);
                    }
                    return self.convert(v, from, to, inner, env, *cur);
                }
                let (converted, converted_ty) = self.convert(v, from, to, inner, env, *cur);
                self.lower_literal_membership(
                    converted,
                    converted_ty,
                    &accepted,
                    ty.span,
                    env,
                    cur,
                    None,
                );
                (converted, converted_ty)
            }
        }
    }

    /// The closed set of literals an `expr as T` has to test its operand
    /// against at run time —
    /// [ADR 0047](../../../docs/adr/0047-literal-and-enum-case-types.md) § 5's
    /// "the only place either type costs anything at runtime" — or `None`
    /// where this conversion is one of § 4's ordinary rows.
    ///
    /// A **whole enum** is one of these sets too, and is where
    /// [ADR 0010](../../../docs/adr/0010-enums-are-a-value-type.md) § 5's
    /// "throws on a value no case names" is emitted from: the annotation
    /// names no members, but the declaration does, so the set is built from
    /// every case of it ([`whole_enum_set`]) and the chain that follows
    /// is the same one a named subset gets.
    ///
    /// Read off the **checked** type rather than the annotation, because that
    /// is where the values still are: § 2's `Foo::TYPE_A` folded to the string
    /// it names during checking, and nothing in the AST says which string that
    /// was. It is also the last place they exist at all — `lower_decl_type`
    /// erases the whole set to the one base its members share.
    ///
    /// `None` in three cases, and each is a decision:
    ///
    /// * The target is not a closed set. One wider atom — `string`, or the
    ///   `null` an `as ?T` adds — is a member the operand may reach, so there
    ///   is nothing to test against. This mirrors `closed_set_atoms` in
    ///   `mwl_types::expr::operators`, which decides the same question for
    ///   § 6's compile-time half.
    /// * The operand already names one value, which the checker has therefore
    ///   already settled: a singleton operand outside the set is `E0469`/
    ///   `E0470` and never reaches lowering, so `"a" as "a"|"b"` needs no
    ///   comparison and an enum case needs none either.
    /// * There is no recorded checked type for the annotation, which is the
    ///   shape `lower_decl_type` answers from the AST alone.
    /// * The target is a whole enum and `from` is already that enum's own
    ///   representation, so every value the operand can hold is a case by
    ///   construction. It is the *same* enum and not merely one with the same
    ///   backing type, because `mwl_types` refuses a conversion between two
    ///   different enums outright (`reject_enum_to_enum_conversion`).
    fn closed_literal_set(&self, ty: &Type, inner: &Expr, from: Ty) -> Option<AcceptedSet> {
        let target = self.exprs.declared_ty(ty.span)?;
        let atoms: Vec<TypeId> = match self.checked_types.get(target) {
            CheckedTy::Union(members) => members.clone(),
            _ => vec![target],
        };
        self.closed_set_of_atoms(&atoms, Some(inner), from)
    }

    /// [`Self::closed_literal_set`] over an atom list the caller already
    /// expanded, which is what an `as ?T` needs: its annotation's checked type
    /// is the union `T|null`, so the target is what is left once `null` is
    /// dropped ([`Self::nullable_target_atoms`]) and there is no single
    /// [`TypeId`] naming it — this crate holds the interner by shared
    /// reference and cannot intern one.
    ///
    /// `operand` is `None` for exactly that caller, and the omission is the
    /// rule rather than a shortcut: [`Self::operand_names_one_value`] is this
    /// function deferring to a decision **the checker already took**
    /// (`reject_impossible_literal_conversion`), and that check bails on a
    /// target holding one wider atom — which `null` is. So `3 as Mode` is
    /// settled at compile time and `3 as ?Mode` is not, and the second one
    /// still owes the run-time chain the first one is excused.
    fn closed_set_of_atoms(
        &self,
        atoms: &[TypeId],
        operand: Option<&Expr>,
        from: Ty,
    ) -> Option<AcceptedSet> {
        let types = self.checked_types;
        if let [target] = atoms
            && let CheckedTy::Enum(qname, backing) = types.get(*target)
        {
            let repr = match backing {
                mwl_types::EnumBacking::Int => EnumRepr::Int,
                mwl_types::EnumBacking::Uint => EnumRepr::Uint,
            };
            if from == Ty::Enum(repr) {
                return None;
            }
            let info = self.enums.get(qname).unwrap_or_else(|| {
                panic!(
                    "mwl-ir: `{qname}` is an interned enum type with no entry in the run's \
                     enum table — `mwl_types` interns one only for an enum it resolved, so \
                     the two tables disagree"
                )
            });
            return Some(whole_enum_set(info, &qname.to_string()));
        }
        if operand.is_some_and(|inner| self.operand_names_one_value(inner)) {
            return None;
        }
        // One pass, not a `closed` predicate and then a map over the same
        // atoms: two matches over one list is two places to add an atom kind
        // to, and the second one's catch-all was a panic no program could
        // reach — an internal-consistency check between a list and itself.
        // `collect::<Option<_>>` makes "this target is not a closed set" the
        // same answer here as it is above.
        let members: Vec<LiteralAtom> = atoms
            .iter()
            .map(|id| match types.get(*id) {
                CheckedTy::StringLiteral(text) => Some(LiteralAtom::Str(text.clone())),
                CheckedTy::IntLiteral(value) => Some(LiteralAtom::Int(*value)),
                // ADR 0007 § 3's two `bool` singletons — one value each, so a
                // closed set of exactly the kind § 5 tests, and the reason
                // `$m as true` is `as bool` plus a membership test rather than
                // a target the language parses and cannot lower.
                CheckedTy::True => Some(LiteralAtom::Bool(true)),
                CheckedTy::False => Some(LiteralAtom::Bool(false)),
                // § 3's enum-case subset. The checked type names the enum and
                // the case but deliberately not the value (see
                // `mwl_types::ty::Ty::EnumCase`'s own doc comment for why
                // folding it to an int literal would reopen ADR 0010 § 5), so
                // the constant comes from the run's own enum table — the one
                // place it still exists by the time lowering runs.
                CheckedTy::EnumCase(qname, _, case) => {
                    let value = self.enums.case(qname, case).unwrap_or_else(|| {
                        panic!(
                            "mwl-ir: `{qname}::{case}` is an interned enum-case type with no \
                             entry in the run's enum table — `mwl_types` interns one only for a \
                             case it resolved, so the two tables disagree"
                        )
                    });
                    Some(LiteralAtom::EnumCase(value))
                }
                // One wider atom and the target is not a closed set at all —
                // `string`, or the `null` an `as ?T` adds. See this function's
                // own doc comment: that is one of its three `None`s, not a
                // shape it declines to lower.
                _ => None,
            })
            .collect::<Option<_>>()?;
        // § 6: the accepted set is generated from the type, never written per
        // site — the same rendering `reject_impossible_literal_conversion`
        // produces for the compile-time half, so the two messages read alike.
        let rendered = atoms
            .iter()
            .map(|id| format!("`{}`", types.describe(*id)))
            .collect::<Vec<_>>()
            .join(", ");
        Some(AcceptedSet { members, rendered })
    }

    /// The atoms of an `as ?T` annotation's target — the checker's type for
    /// the *whole* `?T` with `null` dropped.
    ///
    /// Read off the whole annotation and not off the `T` inside it, because
    /// only the whole one was recorded: `mwl_types::lower::lower_type` calls
    /// [`ExprTypeTable::record_type`] once, at its own entry point, so a
    /// nested `Type` node has no entry at all and
    /// [`lower_decl_type`] would fall back to answering `?Mode`'s target from
    /// the AST — where a name-shaped atom is a class and an enum is
    /// indistinguishable from one. That fallback is what made `$m as ?Mode`
    /// panic on `Tagged as ?Object`.
    ///
    /// `None` where the checker never visited the annotation, which is the
    /// same shape [`lower_decl_type`] answers from the AST alone.
    fn nullable_target_atoms(&self, ty: &Type) -> Option<Vec<TypeId>> {
        let whole = self.exprs.declared_ty(ty.span)?;
        // `?T` is interned as `T|null` — the checker has no separate nullable
        // type (`erase_checked_ty`'s `CheckedTy::Null` arm says so).
        let CheckedTy::Union(members) = self.checked_types.get(whole) else {
            return None;
        };
        let kept: Vec<TypeId> = members
            .iter()
            .copied()
            .filter(|id| !matches!(self.checked_types.get(*id), CheckedTy::Null))
            .collect();
        (!kept.is_empty()).then_some(kept)
    }

    /// Whether a conversion's operand names exactly one value, so that
    /// `mwl_types::expr::operators::reject_impossible_literal_conversion` has
    /// already decided this conversion's outcome at compile time.
    ///
    /// The same three expression shapes that checker's own
    /// `conversion_operand_singleton` accepts, asked here only as a yes/no:
    /// what the value *is* does not matter, because a singleton the target
    /// rejects is `E0469`/`E0470` and never reaches lowering, so one that
    /// arrives here is in the set by construction.
    fn operand_names_one_value(&self, inner: &Expr) -> bool {
        match &inner.kind {
            ExprKind::Str(_) | ExprKind::Int(_) | ExprKind::Bool(_) => true,
            ExprKind::Paren(nested) => self.operand_names_one_value(nested),
            ExprKind::ClassConstAccess { .. } => {
                matches!(
                    self.exprs.lookup(inner.span),
                    Some(ExprInfo::EnumCase { .. })
                )
            }
            _ => false,
        }
    }

    /// Relabels an enum value as the `int`/`uint` its cases *are*, leaving
    /// every other representation exactly as it arrived.
    ///
    /// [ADR 0010](../../../docs/adr/0010-enums-are-a-value-type.md) § 3 makes
    /// a case a compile-time integer constant, and [`Ty::Enum`] is a zero-byte
    /// tag over it — so this is the free [`InstKind::Reinterpret`] row 1 of
    /// that ADR's *5* already uses for `$m as int`, emitting no machine
    /// instruction at all. Every comparison over an enum goes through it,
    /// because `mwl-codegen`'s `BinOp` table is `Ty::Int`/`Ty::Uint`/`Ty::Bool`
    /// and carries no `Ty::Enum` row: ADR 0047 § 5's membership chain, and
    /// ADR 0090 § 2's `==` between two cases of one enum.
    ///
    /// Nothing is released or retained around it: an enum is a scalar, so the
    /// relabelled value borrows no ownership from the operand.
    fn reinterpret_enum_to_backing(
        &mut self,
        value: ValueId,
        value_ty: Ty,
        cur: &mut BlockId,
    ) -> (ValueId, Ty) {
        match value_ty {
            Ty::Enum(EnumRepr::Int) => {
                self.emit(*cur, Ty::Int, InstKind::Reinterpret { operand: value })
            }
            Ty::Enum(EnumRepr::Uint) => {
                self.emit(*cur, Ty::Uint, InstKind::Reinterpret { operand: value })
            }
            _ => (value, value_ty),
        }
    }

    /// [ADR 0066](../../../docs/adr/0066-nullable-conversion-operator.md) § 3
    /// row 2 — `expr as ?T` where `T` is a literal type, an enum-case subset
    /// or a whole enum: "that conversion is already checked and throwing;
    /// this is its non-throwing twin."
    ///
    /// Built as the twin rather than as a redirect of the throwing one.
    /// [`Self::landing_block`] ends in `Terminator::Catch`/`Propagate` and a
    /// pending `Throwable`, so re-pointing the checked lowering's error edge
    /// at a null-producing block would have to discard that object and account
    /// for its reference — `lower::exception`'s plumbing, for a form that
    /// needs no exception to exist at all. Two substitutions on
    /// [`Self::lower_conversion`]'s non-nullable arm buy the same thing: the
    /// membership chain takes a `miss` block instead of the throw
    /// ([`Self::lower_literal_membership`]), and the base conversion runs
    /// through [`Self::convert_or_null`] wherever its row can fail.
    ///
    /// The two shapes below are the same split that arm already makes, for
    /// the same reason:
    ///
    /// * A [`Ty::Tagged`] operand into a **literal** set is tested first, on
    ///   its own runtime tag, and **needs no conversion at all** — the result
    ///   of `as ?T` is a [`Ty::Tagged`] value, and on a hit the operand
    ///   already *is* one, holding exactly the value the chain just proved it
    ///   holds. Converting first would run `Helper::TaggedToString` and let a
    ///   `mixed` holding `1` satisfy a set naming `"1"`, which is the coercion
    ///   ADR 0047 § 4 refuses.
    /// * Everything else converts to the target's own base first — an
    ///   **enum** target included, ADR 0010 § 5 wording that row as "exactly
    ///   the shape `as uint` already has for untrusted input" — and a row
    ///   that can fail runs as its `?` form, whose `null` matches no member
    ///   and so reaches the same miss edge with no test of its own. That is
    ///   why the fallible branch keeps the tagged answer and compares through
    ///   [`Helper::Identical`]: an `Untag` of a `null` would read a zero
    ///   payload, and an enum with a case backed by `0` would then *hit* on a
    ///   conversion that failed.
    ///
    /// Ownership is one rule for both shapes: `answer` is a [`Ty::Tagged`]
    /// value this frame owns by the time the chain runs — retained where the
    /// operand was borrowed storage, produced fresh by the conversion
    /// otherwise — so the hit path hands the consumer the reference every
    /// other conversion row hands it, and the miss path releases it, which is
    /// a runtime no-op for every tag that owns nothing.
    #[allow(clippy::too_many_arguments)]
    fn lower_nullable_membership(
        &mut self,
        v: ValueId,
        from: Ty,
        to: Ty,
        accepted: &AcceptedSet,
        inner: &Expr,
        span: Span,
        env: &mut Env,
        cur: &mut BlockId,
    ) -> (ValueId, Ty) {
        let miss = self.new_block();
        let join = self.new_block();
        let (probe, probe_ty, answer) = if from == Ty::Tagged && !matches!(to, Ty::Enum(_)) {
            if self.aliasing_read(inner) {
                self.emit_retain(*cur, v);
            }
            (v, Ty::Tagged, v)
        } else {
            let base = match to {
                Ty::Enum(EnumRepr::Int) => Ty::Int,
                Ty::Enum(EnumRepr::Uint) => Ty::Uint,
                other => other,
            };
            if conversion_can_fail(from, base) {
                let (tagged, _) = self.convert_or_null(v, from, base, inner, *cur);
                (tagged, Ty::Tagged, tagged)
            } else {
                let (converted, converted_ty) = self.convert(v, from, to, inner, env, *cur);
                // A heterogeneous set erases to `Ty::Tagged` already, and
                // `Self::convert`'s widening row put the tag on — a second
                // one would tag a `Value`.
                let answer = if converted_ty == Ty::Tagged {
                    converted
                } else {
                    self.emit(*cur, Ty::Tagged, InstKind::Tag { operand: converted })
                        .0
                };
                (converted, converted_ty, answer)
            }
        };
        self.lower_literal_membership(probe, probe_ty, accepted, span, env, cur, Some(miss));
        let hit = *cur;
        self.seal(hit, Terminator::Jump(join));
        self.emit_release(miss, answer);
        let (null_v, _) = self.emit(miss, Ty::Null, InstKind::ConstNull);
        let null_v = self.coerce(miss, null_v, Ty::Null, Ty::Tagged, env);
        self.seal(miss, Terminator::Jump(join));
        let (merged, _) = self.emit(
            join,
            Ty::Tagged,
            InstKind::Phi {
                incoming: vec![(hit, answer), (miss, null_v)],
            },
        );
        *cur = join;
        (merged, Ty::Tagged)
    }

    /// [ADR 0047](../../../docs/adr/0047-literal-and-enum-case-types.md) § 5's
    /// membership test: a chain of equality comparisons, each branching
    /// straight to the one block where the conversion succeeded, with the
    /// throw at the far end where every one of them missed.
    ///
    /// A chain and not one runtime call over an encoded set, because the set
    /// is small, closed and compile-time-known: each arm is a `BinOp::Eq`,
    /// which `mwl-codegen` turns into a machine comparison for an integer and
    /// a direct two-pointer `mwl_str_eq` for a string. One helper call over an
    /// encoded set would instead pay ADR 0002's calling convention *and* parse
    /// that encoding on every conversion. Only a [`Ty::Tagged`] operand pays a
    /// call, and it pays exactly the one ADR 0090 § 5 already charges a
    /// `mixed` `==`: [`Helper::Identical`], which answers `false` for a
    /// mismatched tag rather than converting either side.
    ///
    /// Nothing is merged at the join: the value under test is defined before
    /// the chain and dominates every block in it, so there is no
    /// [`InstKind::Phi`] here and no `Env` to reconcile — every block this
    /// builds is straight-line and assigns nothing.
    ///
    /// `miss` is what happens where every comparison missed, and it is the
    /// whole difference between the two spellings ADR 0066 § 3 row 2 calls
    /// twins. `None` is `expr as T`: the throw above, on ADR 0002's error
    /// edge. `Some(block)` is `expr as ?T`, which jumps there instead and
    /// answers `null` — [`Self::lower_nullable_membership`] owns that block,
    /// because only it knows what the result value and its ownership are.
    #[allow(clippy::too_many_arguments)]
    fn lower_literal_membership(
        &mut self,
        value: ValueId,
        value_ty: Ty,
        accepted: &AcceptedSet,
        span: Span,
        env: &mut Env,
        cur: &mut BlockId,
        miss: Option<BlockId>,
    ) {
        // An enum operand is tested one representation down, on the integer
        // its cases *are*. The value the conversion answers with is
        // untouched: this reinterpret feeds the comparisons alone.
        let (value, value_ty) = self.reinterpret_enum_to_backing(value, value_ty, cur);
        let hit = self.new_block();
        for member in &accepted.members {
            let (kind, ty) = match member {
                LiteralAtom::Str(text) => (InstKind::ConstStr(text.clone()), Ty::Str),
                LiteralAtom::Int(number) => (InstKind::ConstInt(*number), Ty::Int),
                LiteralAtom::Bool(value) => (InstKind::ConstBool(*value), Ty::Bool),
                // At the enum's *backing* scalar, not at `Ty::Enum` — see
                // the reinterpret above for why the comparison happens one
                // representation down.
                LiteralAtom::EnumCase(mwl_types::EnumValue::Int(n)) => {
                    (InstKind::ConstInt(*n), Ty::Int)
                }
                LiteralAtom::EnumCase(mwl_types::EnumValue::Uint(n)) => {
                    (InstKind::ConstUint(*n), Ty::Uint)
                }
            };
            let (wanted, _) = self.emit(*cur, ty, kind);
            let (equal, _) = if value_ty == Ty::Tagged {
                self.emit(
                    *cur,
                    Ty::Bool,
                    InstKind::HelperCall {
                        helper: Helper::Identical,
                        args: vec![value, wanted],
                    },
                )
            } else {
                self.emit(
                    *cur,
                    Ty::Bool,
                    InstKind::BinOp {
                        op: BinOp::Eq,
                        lhs: value,
                        rhs: wanted,
                    },
                )
            };
            // The constant is fresh and this comparison is its one and only
            // use — the same policy `lower_binary` applies to the string
            // literal in `$key == "bad"`.
            if ty.is_refcounted() {
                self.emit_release(*cur, wanted);
            }
            let next = self.new_block();
            let hit_edge = self.ids.next_edge(span);
            let miss_edge = self.ids.next_edge(span);
            self.seal(
                *cur,
                Terminator::Branch {
                    cond: equal,
                    then_block: hit,
                    then_edge: hit_edge,
                    else_block: next,
                    else_edge: miss_edge,
                },
            );
            *cur = next;
        }
        // ADR 0066 § 3 row 2's non-throwing twin: every comparison missed, so
        // the answer is `null` and the caller's own block builds it.
        if let Some(block) = miss {
            self.seal(*cur, Terminator::Jump(block));
            *cur = hit;
            return;
        }
        let (listed, _) = self.emit(*cur, Ty::Str, InstKind::ConstStr(accepted.rendered.clone()));
        let landing = self.landing_block(env);
        self.block_insts[cur.index() as usize].push(Inst {
            result: None,
            ty: None,
            kind: InstKind::HelperCall {
                helper: Helper::LiteralMismatch,
                args: vec![value, listed],
            },
            on_error: Some(landing),
        });
        // No release for `listed`: `Helper::LiteralMismatch` owns it, for the
        // reason that variant states — one emitted here would sit in the
        // block only an `Ok` return reaches, which this call never makes.
        //
        // `Helper::LiteralMismatch` never returns normally, so this jump is
        // unreachable — written anyway because a block still owes a
        // terminator, and `hit` is the block control would have reached.
        self.seal(*cur, Terminator::Jump(hit));
        *cur = hit;
    }
}

/// What [`Lowering::open_nullsafe`] is allowed to assume about the tag of a
/// [`Ty::Tagged`] receiver — the one question a member access asks that
/// nothing else in this file does.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum ReceiverProof {
    /// `mwl_types` proved the tag before this ever ran: a narrowed `?T`, the
    /// non-`null` arm of a `?->`, a receiver whose declared type is a class.
    /// The tagged slot is one [`InstKind::Untag`] away from the object, and
    /// that untag is unchecked on purpose — see [`Lowering::untag_receiver`].
    Proven,
    /// Nothing proved it: the receiver is a `mixed`, ADR 0007 § 2's one
    /// unchecked position, so the value reaching the member may hold any tag
    /// at all. No `Untag` is emitted — an unchecked one over an `int` payload
    /// is a pointer this frame would then dereference — and the tagged value
    /// travels to [`InstKind::SlotGet`], which checks the tag where it
    /// already checks the name (ADR 0036 § 4).
    Erased,
}

/// The two blocks a `?->` guard still owes once its member access is lowered
/// — see [`Lowering::open_nullsafe`], which is the only thing that builds one,
/// and [`Lowering::close_nullsafe`], which is the only thing that consumes it.
///
/// Absent (`None`) whenever the receiver's representation proved it cannot be
/// `null`, which is what makes a nullsafe access on a non-nullable receiver
/// cost exactly nothing.
/// Every case of one enum declaration, as the [`AcceptedSet`] an
/// `expr as EnumName` tests its operand against —
/// [ADR 0010](../../../docs/adr/0010-enums-are-a-value-type.md) § 5's "throws
/// on a value no case names" made concrete, and the one thing that keeps an
/// enum a *closed* set once a plain integer can be converted into it.
///
/// A free function rather than a method because it needs nothing of the
/// lowering state: `name` is the enum's resolved name already rendered, which
/// is how this stays clear of `mwl_hir::QName` — this crate does not depend on
/// `mwl-hir`, and [`Lowering::closed_literal_set`] holds the one reference to
/// one long enough to do the table lookup itself.
///
/// **Sorted by the case's own constant**, which is not cosmetic:
/// `EnumInfo::cases` is an `FxHashMap`, so an unsorted set would render the
/// throw's accepted list in a different order from run to run and no test
/// could pin the message. By the constant rather than by the name so that the
/// ordinary declaration — no `= n` clause anywhere, values auto-incrementing
/// from 0 (ADR 0010 § 2) — reads back in the order it was written; the name
/// breaks a tie, so the order is total either way.
fn whole_enum_set(info: &mwl_types::EnumInfo, name: &str) -> AcceptedSet {
    let mut cases: Vec<(&str, mwl_types::EnumValue)> = info
        .cases
        .iter()
        .map(|(case, value)| (case.as_str(), *value))
        .collect();
    cases.sort_by_key(|(case, value)| {
        let ordinal = match value {
            mwl_types::EnumValue::Int(n) => i128::from(*n),
            mwl_types::EnumValue::Uint(n) => i128::from(*n),
        };
        (ordinal, *case)
    });
    AcceptedSet {
        members: cases
            .iter()
            .map(|(_, value)| LiteralAtom::EnumCase(*value))
            .collect(),
        rendered: cases
            .iter()
            .map(|(case, _)| format!("`{name}::{case}`"))
            .collect::<Vec<_>>()
            .join(", "),
    }
}

/// The closed set of values a checked `as` into an
/// [ADR 0047](../../../docs/adr/0047-literal-and-enum-case-types.md) literal
/// type accepts — see [`Lowering::closed_literal_set`], which is the only
/// thing that builds one, and [`Lowering::lower_literal_membership`], which is
/// the only thing that consumes it.
struct AcceptedSet {
    /// The values themselves, in the order the target type states them — or,
    /// for a whole enum, in the order [`whole_enum_set`] sorts the
    /// declaration's cases into, since a hash map states no order at all.
    members: Vec<LiteralAtom>,
    /// Those same values rendered for the throw's message — built here rather
    /// than at run time because a literal type does not survive erasure, so
    /// this is the last point at which the set can be named at all.
    rendered: String,
}

/// One member of an [`AcceptedSet`], already reduced to the constant that
/// tests for it.
///
/// § 3's enum case keeps its [`mwl_types::EnumValue`] rather than collapsing
/// into [`Self::Int`]: the backing type decides both the constant's
/// instruction and its representation, and an enum's tag is not `Ty::Int`
/// even where its backing is (see [`Ty::Enum`]).
enum LiteralAtom {
    Str(String),
    Int(i64),
    /// `true` or `false` — ADR 0007 § 3's two `bool` singletons, whose
    /// closed set is the smallest one this crate builds.
    Bool(bool),
    EnumCase(mwl_types::EnumValue),
}

pub(super) struct NullsafeGuard {
    /// Where control lands when the receiver was `null` and the member never
    /// ran; ends holding the `null` the whole access answers with.
    null_block: BlockId,
    /// Where both arms rejoin, holding the merged value.
    merge_block: BlockId,
    /// The bindings as of the receiver test — the `null` edge's own
    /// environment, since nothing on that edge runs. Everything the member
    /// arm evaluates (`$o?->m($x++)`) is conditional on the receiver, so the
    /// two are reconciled at [`Lowering::close_nullsafe`] by the same
    /// [`Lowering::merge_envs`] every other join uses.
    pre_env: Env,
}

/// The `T` of an `as ?T` annotation, or `None` for any other target.
///
/// Read off the AST rather than off the lowered [`Ty`] because that erasure is
/// exactly what loses the distinction: `?string` and `?int` are both
/// [`Ty::Tagged`] (see its own doc comment), so a conversion between them is
/// indistinguishable from a conversion to the type the value already has.
/// `(...)` is transparent here, the same way [`lower_decl_type`] treats it.
/// A `null|T` *union* spelling is deliberately not folded in: ADR 0066 § 1
/// defines the operator over `?T`, and a union target has no lowering at all
/// yet — one gap is better than a second spelling that half works.
/// The one representation a target's atoms share, or [`Ty::Tagged`] where
/// they share none — [`erase_checked_ty`]'s own `CheckedTy::Union` fold, over
/// an atom list rather than over an interned union.
///
/// The `?T` half of ADR 0047 § 5's "zero additional runtime representation"
/// needs this separately because `T|null` is the union that *is* interned, and
/// folding that one would answer [`Ty::Tagged`] for every target: `null` and
/// `Ty::Str` are two representations, not one.
fn shared_repr(atoms: &[TypeId], checked_types: &TypeInterner) -> Ty {
    let mut shared: Option<Ty> = None;
    for atom in atoms {
        match (erase_checked_ty(*atom, checked_types), shared) {
            (Some(ty), None) => shared = Some(ty),
            (Some(ty), Some(seen)) if ty == seen => {}
            _ => return Ty::Tagged,
        }
    }
    shared.unwrap_or(Ty::Tagged)
}

/// Whether [`Lowering::convert`] would emit an error edge for this row —
/// which is the same question as "does this row have a `?` form to run
/// instead", and is asked only by [`Lowering::lower_nullable_membership`].
///
/// The `false` arms are that function's own free, total and widening rows,
/// listed in the order its doc comment names them; everything else is one of
/// its checked rows and goes through [`Lowering::convert_or_null`]. A row that
/// can fail and has no `?` helper — `bytes`/an object into a *string* literal
/// set — reaches that function's own panic naming `as ?string`, which is the
/// gap it already names for the plain `$b as ?string` spelling rather than a
/// second one this form opens.
fn conversion_can_fail(from: Ty, to: Ty) -> bool {
    if from == to {
        return false;
    }
    !matches!(
        (from, to),
        (Ty::Enum(EnumRepr::Int), Ty::Int)
            | (Ty::Enum(EnumRepr::Uint), Ty::Uint)
            | (Ty::Str, Ty::Bytes)
            | (
                Ty::Bool | Ty::Int | Ty::Uint | Ty::Float | Ty::Decimal,
                Ty::Str
            )
            | (_, Ty::Bool | Ty::Tagged)
    )
}

fn nullable_target(ty: &Type) -> Option<&Type> {
    match &ty.kind {
        TypeKind::Nullable(inner) => Some(inner),
        TypeKind::Paren(inner) => nullable_target(inner),
        _ => None,
    }
}
