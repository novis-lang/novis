//! Expression lowering — the dispatch, conversions, ADR 0035's truthiness, and the short-circuiting operators.
//!
//! Part of [`super`]'s one `impl Lowering`, split across this directory so a
//! session editing one area does not carry the rest in context. Every item
//! moved here unchanged; the methods are `pub(super)` so they reach across
//! these modules and no further, which is the reach they had when `lower` was
//! a single file.

use super::*;

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
        env: &Env,
        cur: &mut BlockId,
    ) -> (ValueId, Ty) {
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
                if ty == Ty::Ref {
                    let pointee = self.pointee_of(name);
                    return self.emit(*cur, pointee, InstKind::RefLoad { slot: v });
                }
                (v, ty)
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
            ExprKind::Binary { op, lhs, rhs } => {
                self.lower_binary(*op, lhs, rhs, expected, env, cur)
            }
            ExprKind::Fn(fn_expr) => self.lower_closure_literal(fn_expr, expr, env, cur),
            ExprKind::New { target, args } => self.lower_new(target, args, expr, env, cur),
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
            other => panic!(
                "mwl-ir's control-flow slice only lowers literals, locals, unary/binary \
                 operators, `new`, a static or instance method call, property access, an array \
                 literal, an array-element read, `instanceof`, an enum case and an `as` \
                 conversion — got {other:?}; see the crate docs' known gaps"
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
    /// already applies. No safepoint is emitted: `echo` is neither of the
    /// two reserved sites (function entry, a loop's back edge).
    pub(super) fn lower_echo(&mut self, operands: &[Expr], cur: &mut BlockId, env: &Env) {
        for operand in operands {
            let (v, aliasing) = self.concat_operand(operand, env, cur);
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
            if !aliasing {
                self.emit_release(*cur, v);
            }
        }
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
        env: &Env,
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
            other => panic!(
                "mwl-ir lowers ADR 0054 § 3's arithmetic, equality and ordering operators over \
                 `decimal` — got {other:?}; `**` and `<=>` have no row there"
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
        env: &Env,
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
                None => panic!(
                    "mwl-ir stringifies an object operand through the `toString` \
                     `mwl_types::expr::operators::require_stringable` resolved for it, and none \
                     was recorded at this span — a value typed at `Stringable` itself, or a \
                     `Core`-owned class, are the two shapes still outside it; see the crate \
                     docs' known gaps"
                ),
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
        env: &Env,
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
        env: &Env,
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
    ///   representation, reinterpreted."
    /// * **Total.** A scalar to `string` reuses the same [`Helper`]
    ///   conversions `.` concatenation already goes through
    ///   ([`Self::concat_operand`]), and any value to `bool` reuses ADR 0035's
    ///   truthy table ([`Self::truthy_convert`]) — `as bool` is the explicit
    ///   spelling of exactly the test a condition applies implicitly, so
    ///   giving it a second table would be two answers to one question.
    /// * **Checked.** `int` ↔ `uint`, `float` → an integer and `string` → a
    ///   number each go through a [`Helper`] that either produces the value or
    ///   throws, emitted through [`Self::emit_fallible`] so it carries
    ///   ADR 0002's error edge like any other call. ADR 0010 § 5's remaining
    ///   row — an integer *into* an enum — is the one still missing: it throws
    ///   on a value no case names, which needs the declaration's case set
    ///   carried to the check. It panics naming itself.
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
        env: &Env,
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
            | (Ty::Decimal, Ty::Int | Ty::Uint | Ty::Float) => {
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
            _ => panic!(
                "mwl-ir lowers ADR 0007 § 2's scalar conversion rows and ADR 0010 § 5's \
                 enum-to-backing one — got `{from:?} as {to:?}`. An integer into an *enum* is \
                 the row still missing: it throws on a value no case names, which needs the \
                 declaration's case set carried to the check, and nothing in this IR expresses \
                 one. See the crate docs' known gaps"
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
    /// `Ty::Str` each convert through their own [`Helper`] variant
    /// (`IntTruthy`/`UintTruthy`/`FloatTruthy`/`StrTruthy`); [`Ty::Array`]
    /// converts through [`Helper::ArrayTruthy`] (falsy iff empty, ADR 0035's
    /// table); and [`Ty::Object`] — a class instance or an enum case — needs
    /// no helper at all, since ADR 0035 § 4 makes either always truthy: this
    /// folds straight to a fresh [`InstKind::ConstBool`] `true` rather than
    /// emitting a call with nothing to inspect at runtime.
    ///
    /// # Panics
    ///
    /// Panics naming the case for anything outside this table: `Ty::Bytes`
    /// (no truthy row is named for it — ADR 0035's table only covers
    /// `string`, not the separate `bytes` type) or `Ty::Void`. The `null`
    /// case (a nullable type) still has no IR representation to convert
    /// *from* at all, so it can't actually reach this method for any program
    /// in scope today. `Ty::Tagged` still panics too: converting one through
    /// ADR 0035's table needs a runtime type-tag representation this crate
    /// still doesn't have.
    pub(super) fn truthy_convert(&mut self, v: ValueId, ty: Ty, cur: BlockId) -> ValueId {
        match ty {
            Ty::Bool => v,
            Ty::Int | Ty::Uint | Ty::Float | Ty::Decimal | Ty::Str => {
                let helper = match ty {
                    Ty::Int => Helper::IntTruthy,
                    Ty::Uint => Helper::UintTruthy,
                    Ty::Float => Helper::FloatTruthy,
                    Ty::Decimal => Helper::DecimalTruthy,
                    Ty::Str => Helper::StrTruthy,
                    Ty::Bool
                    | Ty::Void
                    | Ty::Null
                    | Ty::Object
                    | Ty::Array
                    | Ty::Bytes
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
            other => panic!(
                "mwl-ir's truthy-condition slice only converts a `bool`, a scalar, `Ty::Array` \
                 or `Ty::Object` value — got {other:?}; a `null` value has no IR representation \
                 to convert from at all, and a `mixed`/union value needs a runtime type-tag \
                 representation this crate doesn't have yet, see the crate docs' known gaps"
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
        env: &Env,
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
    pub(super) fn open_nullsafe(
        &mut self,
        object: &Expr,
        nullsafe: bool,
        env: &Env,
        cur: &mut BlockId,
    ) -> (ValueId, Ty, Option<NullsafeGuard>) {
        let (object_v, object_ty) = self.lower_expr(object, None, env, cur);
        if object_ty != Ty::Tagged {
            return (object_v, object_ty, None);
        }
        if !nullsafe {
            let (v, ty) = self.untag_receiver(object_v, object_ty, *cur);
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
        let receiver = self
            .emit(
                member_block,
                Ty::Object,
                InstKind::Untag { operand: object_v },
            )
            .0;
        (
            receiver,
            Ty::Object,
            Some(NullsafeGuard {
                null_block,
                merge_block,
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
        cur: &mut BlockId,
    ) -> (ValueId, Ty) {
        let Some(NullsafeGuard {
            null_block,
            merge_block,
        }) = guard
        else {
            return (value, ty);
        };
        if ty == Ty::Void {
            self.seal(*cur, Terminator::Jump(merge_block));
            self.seal(null_block, Terminator::Jump(merge_block));
            *cur = merge_block;
            return (value, ty);
        }
        let member_end = *cur;
        let member_v = self.coerce(member_end, value, ty, Ty::Tagged);
        self.seal(member_end, Terminator::Jump(merge_block));
        let null_v = self.emit(null_block, Ty::Null, InstKind::ConstNull).0;
        let null_v = self.coerce(null_block, null_v, Ty::Null, Ty::Tagged);
        self.seal(null_block, Terminator::Jump(merge_block));
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
        env: &Env,
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
                let rv = self.coerce(rhs_cur, rv, rty, result_repr);
                *cur = rhs_cur;
                return (rv, result_repr);
            }
            if lhs_ty.is_refcounted() && lhs_is_alias {
                self.emit_retain(*cur, lhs_v);
            }
            let v = self.coerce(*cur, lhs_v, lhs_ty, result_repr);
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
        let value_v = self.coerce(value_block, untagged, non_null_repr, result_repr);
        self.seal(value_block, Terminator::Jump(merge_block));

        let mut rhs_cur = null_block;
        let (rv, rty) = self.lower_expr(rhs, Some(result_repr), env, &mut rhs_cur);
        if rty.is_refcounted() && self.aliasing_read(rhs) {
            self.emit_retain(rhs_cur, rv);
        }
        let null_v = self.coerce(rhs_cur, rv, rty, result_repr);
        self.seal(rhs_cur, Terminator::Jump(merge_block));

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
    /// `!expr` — ADR 0035's truthy table applied to `expr`, then negated;
    /// always produces [`Ty::Bool`] regardless of `expr`'s own type, unlike a
    /// plain arithmetic/bitwise unary operator. `expr` is lowered through
    /// [`Self::lower_expr`], so `!($a && $b)`/`!($a ? $b : $c)` compose the
    /// same way a bare `&&`/`||`/ternary does.
    pub(super) fn lower_not(&mut self, inner: &Expr, env: &Env, cur: &mut BlockId) -> ValueId {
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
    /// own [`Ty::Bool`] value via a fresh [`InstKind::Phi`] instead of
    /// merging named locals (an expression's own temporaries never live in
    /// [`Env`] — that's [`Self::merge_envs`]' business, not this one's).
    /// `lhs`/`rhs` each go through [`Self::lower_truthy_cond`], so either may
    /// itself be any type ADR 0035's table covers, and either may itself be a
    /// nested `&&`/`||`/`!`/ternary.
    pub(super) fn lower_and(
        &mut self,
        lhs: &Expr,
        rhs: &Expr,
        env: &Env,
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

        let mut rhs_cur = rhs_block;
        let rhs_v = self.lower_truthy_cond(rhs, env, &mut rhs_cur);
        let rhs_end = rhs_cur;
        self.seal(rhs_end, Terminator::Jump(merge_block));

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
        env: &Env,
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

        let mut rhs_cur = rhs_block;
        let rhs_v = self.lower_truthy_cond(rhs, env, &mut rhs_cur);
        let rhs_end = rhs_cur;
        self.seal(rhs_end, Terminator::Jump(merge_block));

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
    /// # Panics
    ///
    /// Panics naming the case if `then`'s and `else`'s branches lower to two
    /// different [`Ty`] representations — the checker's own union of their
    /// static types has no IR representation this crate can fold into yet
    /// (see [`Ty::Tagged`]'s own doc comment on why a union isn't folded into
    /// it automatically). Otherwise see [`Self::truthy_convert`]'s own panic
    /// doc for `cond`'s own restriction.
    pub(super) fn lower_ternary(
        &mut self,
        cond: &Expr,
        then: Option<&Expr>,
        else_: &Expr,
        env: &Env,
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

        let (then_v, then_ty, then_end) = match then {
            Some(then_expr) => {
                let mut then_cur = then_block;
                let (v, ty) = self.lower_expr(then_expr, None, env, &mut then_cur);
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
        self.seal(then_end, Terminator::Jump(merge_block));

        let mut else_cur = else_block;
        let (else_v, else_ty) = self.lower_expr(else_, None, env, &mut else_cur);
        if else_ty.is_refcounted() && self.aliasing_read(else_) {
            self.emit_retain(else_cur, else_v);
        }
        self.seal(else_cur, Terminator::Jump(merge_block));

        assert_eq!(
            then_ty, else_ty,
            "mwl-ir's ternary/elvis slice only lowers a ternary whose branches share the same \
             IR-level type — got {then_ty:?} vs {else_ty:?}; a differing-branch-type ternary \
             erases to a union the checker already computed but this crate has no IR \
             representation to fold it into yet, see the crate docs' known gaps"
        );

        let (result, _) = self.emit(
            merge_block,
            then_ty,
            InstKind::Phi {
                incoming: vec![(then_end, then_v), (else_cur, else_v)],
            },
        );
        *cur = merge_block;
        (result, then_ty)
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
    /// [`Self::lower_switch`] parks its own — an expression is lowered against
    /// a `&Env` it cannot insert into. Instead a *fresh* subject (one no slot
    /// owns) is released at the top of every block the chain can leave for:
    /// each arm body and the throw block. Those are disjoint paths, so the
    /// release runs exactly once.
    ///
    /// # Panics
    ///
    /// Panics naming the case for a `match` with no arms at all (there is no
    /// value for the phi to carry), for a label whose representation differs
    /// from the subject's, and — [`Self::lower_ternary`]'s own restriction —
    /// for two arms whose bodies lower to different representations.
    pub(super) fn lower_match(
        &mut self,
        subject: &Expr,
        arms: &[MatchArm],
        expected: Option<Ty>,
        env: &Env,
        cur: &mut BlockId,
    ) -> (ValueId, Ty) {
        assert!(
            !arms.is_empty(),
            "mwl-ir does not lower an arm-less `match`: it is an expression that can only \
             throw, so there is no value for its merge phi to carry; see the crate docs' \
             known gaps"
        );
        let (subj_v, subj_ty) = self.lower_expr(subject, None, env, cur);
        // A fresh subject is this expression's to free; an aliasing one stays
        // the slot's, exactly the split `Self::lower_ternary` applies to its
        // own reused condition.
        let owed = subj_ty.is_refcounted() && !self.aliasing_read(subject);

        let arm_blocks: Vec<BlockId> = arms.iter().map(|_| self.new_block()).collect();
        let merge_block = self.new_block();
        let default_index = arms.iter().position(|a| a.conditions.is_none());

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
                test_cur = next;
            }
        }
        match default_index {
            Some(i) => self.seal(test_cur, Terminator::Jump(arm_blocks[i])),
            None => {
                if owed {
                    self.emit_release(test_cur, subj_v);
                }
                let (message, _) = self.emit(
                    test_cur,
                    Ty::Str,
                    InstKind::ConstStr("no `match` arm matched the subject".to_owned()),
                );
                let (exception, _) = self.emit_fallible(
                    test_cur,
                    Ty::Object,
                    InstKind::New {
                        class: "LogicError".to_owned(),
                        ctor: Some(THROWABLE_CTOR.to_owned()),
                        args: vec![message],
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

        let mut incoming: Vec<(BlockId, ValueId)> = Vec::with_capacity(arms.len());
        let mut result_ty: Option<Ty> = None;
        for (i, arm) in arms.iter().enumerate() {
            let mut arm_cur = arm_blocks[i];
            if owed {
                self.emit_release(arm_cur, subj_v);
            }
            let (v, ty) = self.lower_expr(&arm.body, expected, env, &mut arm_cur);
            match result_ty {
                None => result_ty = Some(ty),
                Some(first) => assert_eq!(
                    first, ty,
                    "mwl-ir's `match` slice only lowers arms that share one IR-level type — the \
                     checker's union of their static types has no IR representation this crate \
                     can fold into yet, see `Ty::Tagged`'s own doc comment"
                ),
            }
            if ty.is_refcounted() && self.aliasing_read(&arm.body) {
                self.emit_retain(arm_cur, v);
            }
            self.seal(arm_cur, Terminator::Jump(merge_block));
            incoming.push((arm_cur, v));
        }
        let ty = result_ty.expect("the arm list is non-empty, checked above");
        let (result, _) = self.emit(merge_block, ty, InstKind::Phi { incoming });
        *cur = merge_block;
        (result, ty)
    }
    /// Lowers `ExprKind::Interpolated`'s parts into the single [`Ty::Str`]
    /// value they denote — a left-to-right fold of [`InstKind::Concat`],
    /// reusing [`Self::concat_operand`] per `StringPart::Expr` piece exactly
    /// the way `.`-concatenation's own two-operand arm does (a
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
    /// The one shape plain N-ary `.`-folding wouldn't otherwise force into
    /// the open: an `Interpolated` with exactly one part that is itself an
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
        env: &Env,
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
        let mut acc: Option<(ValueId, bool)> = None;
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
            acc = Some(match acc {
                None => piece,
                Some((lv, l_alias)) => {
                    let (rv, r_alias) = piece;
                    let (result, _) =
                        self.emit(*cur, Ty::Str, InstKind::Concat { lhs: lv, rhs: rv });
                    if !l_alias {
                        self.emit_release(*cur, lv);
                    }
                    if !r_alias {
                        self.emit_release(*cur, rv);
                    }
                    (result, false)
                }
            });
        }
        let (v, alias) = acc.expect("checked non-empty above");
        if alias {
            // The single-part-alias degenerate case this function's own doc
            // comment names — no `Concat` ran, so `v` is still someone
            // else's storage; retain it to become this expression's own
            // single fresh owner.
            self.emit_retain(*cur, v);
        }
        (v, Ty::Str)
    }
    /// Lowers `expr` — an `ExprKind::Index`'s subscript — and normalizes it
    /// to a [`Ty::Str`] key: ADR 0007 § 5's "every key is a `string`" rule,
    /// with an `int`/`uint` subscript normalized to its decimal-string form
    /// (`$a[8]` is `$a["8"]`) via the exact [`Helper::IntToString`]/
    /// [`Helper::UintToString`] conversion [`Self::concat_operand`] already
    /// uses for `.`'s scalar operand — reused verbatim rather than a new
    /// policy. Also used, identically, for an array literal's explicit
    /// `key =>` element (see [`ir::InstKind::ArrayNew`]'s own doc comment). A
    /// `float`, `bool`, or `null` key is a compile-time rejection
    /// `mwl_types::expr::check_array_key_type` now enforces at both call
    /// sites (an `Index` subscript and an array-literal explicit key alike),
    /// so the `other` arm below is an internal-invariant panic — unreachable
    /// for anything that already passed `mwl_types::check_program` — rather
    /// than a live known gap.
    ///
    /// Returns the resulting `Ty::Str` value together with whether it
    /// [`is_aliasing_read`]s storage a durable slot still owns — exactly the
    /// same second half [`Self::concat_operand`] returns, for the same
    /// reason: a plain `string` subscript passed through unchanged may still
    /// be a bare local/property/array read, while a freshly converted
    /// `int`/`uint` key is always a brand new buffer with exactly one owner.
    pub(super) fn lower_array_key(
        &mut self,
        expr: &Expr,
        env: &Env,
        cur: &mut BlockId,
    ) -> (ValueId, bool) {
        let (v, ty) = self.lower_expr(expr, None, env, cur);
        match ty {
            Ty::Str => (v, self.aliasing_read(expr)),
            Ty::Int | Ty::Uint => {
                let helper = if ty == Ty::Int {
                    Helper::IntToString
                } else {
                    Helper::UintToString
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
            other => panic!(
                "mwl-ir: an array key lowered to {other:?} — mwl_types::check_program is trusted \
                 to have already rejected a float/bool/null key (ADR 0007 § 5) at both the \
                 subscript and array-literal explicit-key sites, so this should be unreachable"
            ),
        }
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
        env: &Env,
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
        env: &Env,
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
            other => panic!(
                "mwl-ir's control-flow slice only lowers unary `-`/`!` — got {other:?}; \
                 see the crate docs' known gaps"
            ),
        };
        self.emit(
            *cur,
            ty,
            InstKind::UnOp {
                op: uop,
                operand: v,
            },
        )
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
    fn lower_concat(
        &mut self,
        lhs: &Expr,
        rhs: &Expr,
        env: &Env,
        cur: &mut BlockId,
    ) -> (ValueId, Ty) {
        let (lv, l_alias) = self.concat_operand(lhs, env, cur);
        let (rv, r_alias) = self.concat_operand(rhs, env, cur);
        let result = self.emit(*cur, Ty::Str, InstKind::Concat { lhs: lv, rhs: rv });
        if !l_alias {
            self.emit_release(*cur, lv);
        }
        if !r_alias {
            self.emit_release(*cur, rv);
        }
        result
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
        env: &Env,
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
        op: BinaryOp,
        lhs: &Expr,
        rhs: &Expr,
        expected: Option<Ty>,
        env: &Env,
        cur: &mut BlockId,
    ) -> (ValueId, Ty) {
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
        let (bop, ty) = match op {
            BinaryOp::Add => (BinOp::Add, lty),
            BinaryOp::Sub => (BinOp::Sub, lty),
            BinaryOp::Mul => (BinOp::Mul, lty),
            BinaryOp::Div => (BinOp::Div, lty),
            BinaryOp::Mod => (BinOp::Mod, lty),
            BinaryOp::Eq => (BinOp::Eq, Ty::Bool),
            BinaryOp::NotEq => (BinOp::NotEq, Ty::Bool),
            BinaryOp::Lt => (BinOp::Lt, Ty::Bool),
            BinaryOp::LtEq => (BinOp::LtEq, Ty::Bool),
            BinaryOp::Gt => (BinOp::Gt, Ty::Bool),
            BinaryOp::GtEq => (BinOp::GtEq, Ty::Bool),
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
        // Integer `%` is the one operator here that can *fail*: ADR
        // 0007 § 4 makes a zero divisor throw `ArithmeticError`, which
        // `mwl-codegen` raises inline rather than through a helper, so
        // it needs an error edge exactly the way a call does. Every
        // other operator, `%` on floats included, returns no status at
        // all — see `Inst::on_error`.
        let inst = InstKind::BinOp {
            op: bop,
            lhs: lv,
            rhs: rv,
        };
        let result = if matches!(bop, BinOp::Mod) && matches!(ty, Ty::Int | Ty::Uint) {
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
        env: &Env,
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
            assert!(
                ty != Ty::Ref,
                "mwl-ir does not lower a closure capturing the `&$x` parameter \
                 `${name}`: the cell it addresses is the caller's, and the closure may \
                 outlive the call that staged it; see the crate docs' known gaps"
            );
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
        env: &Env,
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
        env: &Env,
        cur: &mut BlockId,
    ) -> (ValueId, Ty) {
        let Some(ExprInfo::Call(call)) = self.exprs.lookup(expr.span) else {
            panic!(
                "mwl-ir: an instance method call at {:?} has no resolved target \
                 recorded in the typed-expression table — did this program pass \
                 mwl_types::check_program with the same table?",
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
            let (object_v, receiver_ty, guard) = self.open_nullsafe(object, nullsafe, env, cur);
            let LoweredArgs {
                values,
                mut temporaries,
            } = self.lower_call_args(args, &sig, checked_types, ArgOwnership::Borrowed, env, cur);
            // The receiver is borrowed like every other argument to a
            // `Core` member, so a *freshly built* one — a nested
            // call's own result — has no other owner and this frame
            // owes its release. A receiver read out of a local or a
            // field is that binding's to release, not this call's.
            if receiver_ty.is_refcounted() && !self.aliasing_read(object) {
                temporaries.push(object_v);
            }
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
            self.release_call_temporaries(temporaries, *cur);
            return self.close_nullsafe(guard, v, ty, cur);
        }
        let target_label = format!("{}::{}", call.class, call.method);
        let sig = ArgSig::of(call);
        let return_ty = lower_checked_ty(call.return_ty, self.checked_types);
        let checked_types = self.checked_types;
        let is_static = call.is_static;
        // `?->` guards everything below on the receiver not being
        // `null`; `->` opens no guard and lowers exactly as before.
        let (object_v, receiver_ty, guard) = self.open_nullsafe(object, nullsafe, env, cur);
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
        self.close_nullsafe(guard, v, ty, cur)
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
        env: &Env,
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
            self.release_call_temporaries(lowered.temporaries, *cur);
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

    /// `$obj->prop` — the receiver's declaring class comes from
    /// `self.exprs`, exactly like a call's resolved target; a shape or
    /// plain-`object` receiver (ADR 0036 § 4) has no such entry at
    /// all, so this panics naming that case rather than lowering it —
    /// see the crate docs' known gaps for why (the checker itself
    /// defers the runtime-checked fallback to M4, with no IR/codegen
    /// yet to throw from).
    fn lower_property_access(
        &mut self,
        object: &Expr,
        nullsafe: bool,
        expr: &Expr,
        env: &Env,
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
                "mwl-ir: a property access at {:?} has no resolved declaring class \
                 recorded in the typed-expression table — either it wasn't checked with \
                 the same table, or its receiver erased to a shape/plain `object` (ADR \
                 0036 § 4), which this crate does not yet lower (see the crate docs' \
                 known gaps)",
                expr.span
            ),
        };
        let field_ty = lower_checked_ty(ty, self.checked_types);
        let class_label = class.to_string();
        let field_name = name.clone();
        // See the `MethodCall` arm above: `?->` guards the access on
        // the receiver not being `null`, `->` opens no guard.
        let (object_v, receiver_ty, guard) = self.open_nullsafe(object, nullsafe, env, cur);
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
        self.close_nullsafe(guard, v, ty, cur)
    }

    /// `[...]`/legacy `array(...)` — see `InstKind::ArrayNew`'s own
    /// doc comment for the full policy this mirrors and its known
    /// gaps. `...spread` and `&value` elements are still unsupported
    /// — each panics naming itself rather than guessing at a merge/
    /// reference representation this crate doesn't have yet. A
    /// *purely positional* literal (no element has an explicit
    /// `key =>`) keeps the original single-`ArrayNew` shape: each
    /// element's key is simply its index, auto-numbered from `0`
    /// exactly like PHP's own `[$a, $b]` shorthand, computed at
    /// lowering time with no runtime key instruction at all. A
    /// literal with at least one explicit `key =>` element instead
    /// builds an empty array first and appends one `ArraySet` per
    /// element in source order — seeing `crate::ir::InstKind::ArrayNew`'s
    /// own doc comment for why that's the only shape general enough
    /// to give an explicit key's (possibly runtime-computed) value a
    /// place to live, and the one PHP behavior it deliberately doesn't
    /// reproduce (a positional element's key numbering ignores any
    /// explicit `int`/`uint` key elsewhere in the same literal, rather
    /// than PHP's real "continues from the highest int key used so
    /// far"). Each value that's itself `Ty::is_refcounted` and
    /// `is_aliasing_read` is retained before the array durably owns
    /// it, the same policy `Self::lower_call_args` already applies at
    /// a call-argument boundary; an explicit key gets the identical
    /// treatment via `Self::lower_array_key`'s own aliasing flag. The
    /// array literal's own result needs no retain — a fresh producer,
    /// same as `new`/a call's result.
    fn lower_array_literal(
        &mut self,
        items: &[ArrayItem],
        env: &Env,
        cur: &mut BlockId,
    ) -> (ValueId, Ty) {
        assert!(
            items.iter().all(|item| !item.spread && !item.by_ref),
            "mwl-ir does not yet lower a `...spread` or `&value` array-literal element \
             — see the crate docs' known gaps"
        );
        if items.iter().all(|item| item.key.is_none()) {
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
            for item in items {
                let (key_v, key_aliasing) = match &item.key {
                    Some(key) => self.lower_array_key(key, env, cur),
                    None => {
                        let key_str = next_index.to_string();
                        next_index += 1;
                        let (kv, _) = self.emit(*cur, Ty::Str, InstKind::ConstStr(key_str));
                        (kv, false)
                    }
                };
                if key_aliasing {
                    self.emit_retain(*cur, key_v);
                }
                let (v, ty) = self.lower_expr(&item.value, None, env, cur);
                if ty.is_refcounted() && self.aliasing_read(&item.value) {
                    self.emit_retain(*cur, v);
                }
                array_v = self.emit_array_set(*cur, array_v, key_v, v);
            }
            (array_v, Ty::Array)
        }
    }

    /// `$arr[$i]` — the element's declared type comes from
    /// `self.exprs`, exactly like a property access's declaring
    /// class: a base that erased to `mixed` (ADR 0007 § 5's own
    /// "nothing compile-time-known to read" case for an unresolved
    /// array) has no `ExprInfo::Index` entry at all, so this panics
    /// naming that case rather than lowering it. `base[]` (`index`
    /// is `None`) has no meaning as a read at all — it is PHP's
    /// append syntax, assignment-target-only — so it panics too.
    fn lower_index(
        &mut self,
        base: &Expr,
        index: Option<&Expr>,
        expr: &Expr,
        env: &Env,
        cur: &mut BlockId,
    ) -> (ValueId, Ty) {
        let Some(index) = index else {
            panic!(
                "mwl-ir does not lower `$a[]` as a read expression — append syntax \
                 (`index` is `None`) is assignment-target-only; see the crate docs' \
                 known gaps"
            );
        };
        let Some(ExprInfo::Index { elem_ty }) = self.exprs.lookup(expr.span) else {
            panic!(
                "mwl-ir: an array-index read at {:?} has no resolved element type \
                 recorded in the typed-expression table — either it wasn't checked with \
                 the same table, or its base erased to `mixed` (an unresolved array), \
                 which this crate does not yet lower (see the crate docs' known gaps)",
                expr.span
            );
        };
        let result_ty = lower_checked_ty(*elem_ty, self.checked_types);
        let (array_v, _) = self.lower_expr(base, None, env, cur);
        let (key_v, key_aliasing) = self.lower_array_key(index, env, cur);
        let result = self.emit(
            *cur,
            result_ty,
            InstKind::ArrayGet {
                array: array_v,
                key: key_v,
            },
        );
        if !key_aliasing {
            self.emit_release(*cur, key_v);
        }
        result
    }

    /// `$x instanceof Name` — the tested class comes from
    /// `self.exprs`, exactly like a property access's declaring class,
    /// because resolving a bare `Animal` to `Ns\Animal` needs the
    /// namespace/import context this crate cannot see. The dynamic
    /// form (`$x instanceof $name`) records nothing and is refused.
    fn lower_instanceof(
        &mut self,
        inner: &Expr,
        expr: &Expr,
        env: &Env,
        cur: &mut BlockId,
    ) -> (ValueId, Ty) {
        let Some(ExprInfo::InstanceOf { class }) = self.exprs.lookup(expr.span) else {
            panic!(
                "mwl-ir: an `instanceof` at {:?} has no resolved class recorded in the \
                 typed-expression table — either it wasn't checked with the same table, \
                 or its right-hand side is the dynamic `$x instanceof $name` form, which \
                 this crate does not lower (see the crate docs' known gaps)",
                expr.span
            );
        };
        let class_label = class.to_string();
        let (value, ty) = self.lower_expr(inner, None, env, cur);
        assert!(
            matches!(ty, Ty::Object),
            "mwl-ir lowers `instanceof` only against an object receiver — got \
             representation {ty:?}"
        );
        self.emit(
            *cur,
            Ty::Bool,
            InstKind::InstanceOf {
                value,
                class: class_label,
            },
        )
    }

    /// ADR 0023 § 1: PHP's shallow, same-heap, single-level copy, with
    /// no `__clone` hook to run — so the whole operation is one
    /// instruction, and the result is a fresh object with exactly one
    /// owner, the same as `new`.
    fn lower_clone_expr(&mut self, inner: &Expr, env: &Env, cur: &mut BlockId) -> (ValueId, Ty) {
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
        env: &Env,
        cur: &mut BlockId,
    ) -> (ValueId, Ty) {
        // ADR 0066's `as ?T` is read off the *annotation*, before
        // `lower_decl_type` erases it: `?string` and `?int` are both
        // `Ty::Tagged`, so a conversion between them would look like
        // `from == to` — the one shape `Self::convert` answers by
        // doing nothing at all.
        match nullable_target(ty) {
            Some(target) => {
                // No placement here, unlike the arm below: placing a
                // literal at the target would make `3 as ?uint` the
                // `from == to` shape ADR 0066 § 3 calls a compile
                // error, which `mwl_types` does not refuse yet, so it
                // would panic where it now converts.
                let (v, from) = self.lower_expr(inner, None, env, cur);
                let to = lower_decl_type(target, self.exprs, self.checked_types);
                self.convert_or_null(v, from, to, inner, *cur)
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
                let placed =
                    matches!(inner.kind, ExprKind::Int(_) | ExprKind::Float(_)).then_some(to);
                let (v, from) = self.lower_expr(inner, placed, env, cur);
                let Some(accepted) = self.closed_literal_set(ty, inner) else {
                    return self.convert(v, from, to, inner, env, *cur);
                };
                // ADR 0047 § 5's membership test, on whichever side of
                // the base conversion still holds the value the author
                // wrote. A `Ty::Tagged` operand is tested **first**,
                // against its own runtime tag: converting one to the
                // base would run `Helper::TaggedToString`, which turns
                // a `mixed` holding `1` into `"1"` and would let it
                // satisfy a set naming `"1"` — exactly the coercion
                // § 4's "throws unless the value equals one of the
                // named literals" refuses. Every other operand is
                // converted first instead, so the comparison is over
                // one representation and stays a `BinOp::Eq` machine
                // compare rather than `mwl-codegen`'s refusal of a
                // mismatched pair.
                if from == Ty::Tagged {
                    self.lower_literal_membership(v, from, &accepted, ty.span, env, cur);
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
    fn closed_literal_set(&self, ty: &Type, inner: &Expr) -> Option<AcceptedSet> {
        let types = self.checked_types;
        let target = self.exprs.declared_ty(ty.span)?;
        let atoms: Vec<TypeId> = match types.get(target) {
            CheckedTy::Union(members) => members.clone(),
            _ => vec![target],
        };
        let closed = atoms.iter().all(|id| {
            matches!(
                types.get(*id),
                CheckedTy::StringLiteral(_) | CheckedTy::IntLiteral(_) | CheckedTy::EnumCase(..)
            )
        });
        if !closed || self.operand_names_one_value(inner) {
            return None;
        }
        let members = atoms
            .iter()
            .map(|id| match types.get(*id) {
                CheckedTy::StringLiteral(text) => LiteralAtom::Str(text.clone()),
                CheckedTy::IntLiteral(value) => LiteralAtom::Int(*value),
                other => panic!(
                    "mwl-ir does not lower ADR 0047 § 3's enum-case subset conversion over an \
                     operand only known at run time — got {other:?}. The test needs each case's \
                     backing value, which lives in `mwl_types::enums` and is not handed to this \
                     crate; `ExprInfo::EnumCase` carries one only for a case written as an \
                     *expression*. See the crate docs' known gaps"
                ),
            })
            .collect();
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
            ExprKind::Str(_) | ExprKind::Int(_) => true,
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
    fn lower_literal_membership(
        &mut self,
        value: ValueId,
        value_ty: Ty,
        accepted: &AcceptedSet,
        span: Span,
        env: &Env,
        cur: &mut BlockId,
    ) {
        let hit = self.new_block();
        for member in &accepted.members {
            let (kind, ty) = match member {
                LiteralAtom::Str(text) => (InstKind::ConstStr(text.clone()), Ty::Str),
                LiteralAtom::Int(number) => (InstKind::ConstInt(*number), Ty::Int),
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
            let miss = self.new_block();
            let hit_edge = self.ids.next_edge(span);
            let miss_edge = self.ids.next_edge(span);
            self.seal(
                *cur,
                Terminator::Branch {
                    cond: equal,
                    then_block: hit,
                    then_edge: hit_edge,
                    else_block: miss,
                    else_edge: miss_edge,
                },
            );
            *cur = miss;
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

/// The two blocks a `?->` guard still owes once its member access is lowered
/// — see [`Lowering::open_nullsafe`], which is the only thing that builds one,
/// and [`Lowering::close_nullsafe`], which is the only thing that consumes it.
///
/// Absent (`None`) whenever the receiver's representation proved it cannot be
/// `null`, which is what makes a nullsafe access on a non-nullable receiver
/// cost exactly nothing.
/// The closed set of values a checked `as` into an
/// [ADR 0047](../../../docs/adr/0047-literal-and-enum-case-types.md) literal
/// type accepts — see [`Lowering::closed_literal_set`], which is the only
/// thing that builds one, and [`Lowering::lower_literal_membership`], which is
/// the only thing that consumes it.
struct AcceptedSet {
    /// The values themselves, in the order the target type states them.
    members: Vec<LiteralAtom>,
    /// Those same values rendered for the throw's message — built here rather
    /// than at run time because a literal type does not survive erasure, so
    /// this is the last point at which the set can be named at all.
    rendered: String,
}

/// One member of an [`AcceptedSet`], already reduced to the constant that
/// tests for it.
///
/// § 3's enum case is deliberately absent: erasing one needs its backing
/// value, which this crate is not handed — [`Lowering::closed_literal_set`]
/// panics naming that gap rather than modelling a case it cannot compare.
enum LiteralAtom {
    Str(String),
    Int(i64),
}

pub(super) struct NullsafeGuard {
    /// Where control lands when the receiver was `null` and the member never
    /// ran; ends holding the `null` the whole access answers with.
    null_block: BlockId,
    /// Where both arms rejoin, holding the merged value.
    merge_block: BlockId,
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
fn nullable_target(ty: &Type) -> Option<&Type> {
    match &ty.kind {
        TypeKind::Nullable(inner) => Some(inner),
        TypeKind::Paren(inner) => nullable_target(inner),
        _ => None,
    }
}
