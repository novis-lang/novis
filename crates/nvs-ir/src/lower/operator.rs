//! `rule:types/arithmetic`'s operator table — the scalar rows, `decimal`'s own set, the
//! short-circuiting operators, and the widening that places a mixed pair.
//!
//! Part of [`super`]'s one `impl Lowering`, under the rule [`super::expr`]'s own
//! header states: the methods are `pub(crate)`, so they reach across these modules
//! and no further.
//!
//! `lower_binary` is § 4's table and `emit_binop` in `nvs-codegen` is its other
//! end; a compound assignment reaches both by desugaring to the binary form it
//! means, so there is no second table for `⊕=` anywhere.

use super::*;

impl<'a> Lowering<'a> {
    /// Whether the checker *placed* the numeric literal at `span` at
    /// `decimal` — `rule:types/numeric-literal-placement`'s rule, read back from the one recording
    /// `nvs_types::expr::record_decimal_placement` makes.
    ///
    /// [`Lowering::lower_expr`]'s own `expected` answers the same question
    /// wherever the position's representation reaches this crate, which is
    /// most of them. This covers the positions where it does not: an array
    /// literal's elements, whose type [`Ty::Array`] erases, is the one that
    /// matters today, because a `float` stored where the checker typed a
    /// `decimal` is a *silently* wrong value rather than a loud one.
    pub(crate) fn placed_at_decimal(&self, span: nvs_diagnostics::Span) -> bool {
        self.exprs
            .declared_ty(span)
            .is_some_and(|id| matches!(self.checked_types.get(id), CheckedTy::Decimal))
    }
    /// One binary operator with a [`Ty::Decimal`] operand —
    /// `rule:types/arithmetic`'s whole
    /// table, as [`Helper`] calls rather than machine instructions.
    ///
    /// Fewer helpers than comparisons, which is why this is a rewrite rather
    /// than a lookup: `!=` is [`Helper::DecimalEq`] under a [`UnOp::Not`], and
    /// `>`/`>=` are [`Helper::DecimalLt`]/[`Helper::DecimalLtEq`] with their
    /// operands swapped. That is not merely fewer variants — it is what gives
    /// an unordered operand (a `NaN` on the `float` side of § 3's comparison
    /// row) PHP's answer to every comparison at once, which a single
    /// compare-to-zero result could not encode.
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
            // `decimal`: `<=>` is the ordering the comparisons above each ask
            // one question of, returned whole. `rule:types/arithmetic` grants it on the same
            // grounds it grants them — an exact comparison is computable
            // across every pairing, including the `decimal`/`float` one
            // arithmetic refuses.
            BinaryOp::Cmp => (Helper::DecimalCmp, Ty::Int, vec![lhs, rhs], false),
            // Every `BinaryOp` is accounted for and this arm has no reachable
            // target left. The rows above are exactly what `rule:types/arithmetic`
            // grants a `decimal`: the arithmetic operators, `==`/`!=`, the
            // orderings and `<=>`. What it does not grant is either refused a
            // phase up or never arrives at all.
            //
            // Refused: `**` by
            // `nvs_types::expr::operators::power_result`, which names
            // `Core\Decimal::pow` and the rounding it does; and `&`, `|`, `^`,
            // `<<` and `>>` by `reject_bitwise_operand`, `rule:types/arithmetic`'s
            // bitwise row being over `int` and `uint` alone — a `decimal` is a
            // coefficient and a scale, so there is no bit pattern for them to
            // read.
            //
            // Never arriving: `.`, `&&`, `||` and `??` are taken by
            // `Self::lower_expr` before the general `Binary` arm that is
            // `Self::lower_binary`'s only caller, and `lower_binary` is this
            // function's only caller in turn — so they cannot reach a
            // `decimal` row any more than they reach the scalar one. A
            // compound assignment arrives the same way, `AssignOp::to_binary_op`
            // handing back an ordinary `BinaryOp` that re-enters `lower_expr`.
            //
            // That subtraction is the proof; the message below is not.
            other => panic!(
                "nvs-ir: unreachable — `BinaryOp::{other:?}` reached the `decimal` operator \
                 table; see this arm's own comment for the roster it subtracts"
            ),
        };
        let inst = InstKind::HelperCall { helper, args };
        // Only the arithmetic rows can *throw*: every one of them raises
        // `ArithmeticError` on either overflow kind, and `/` on a zero
        // divisor. A comparison is total — and still takes an error edge,
        // because every helper returns a status and an uncatchable one has to
        // leave the frame swept (`Inst::on_error`).
        let (v, _) = self.emit_fallible(*cur, ty, inst, env);
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

    /// `$a < $b` and its four siblings over two objects — `rule:classes/ordering-lowers-to-compare-to`'s
    /// `Comparable::compareTo` call, then the comparison of *its* `int`
    /// against zero.
    ///
    /// `<=>` is the call's own result with no second step: `compareTo` already
    /// returns exactly what the spaceship operator means.
    ///
    /// The call is ordinary in every respect — `rule:errors/propagation`'s error edge (a
    /// `compareTo` body may throw like any other), and the same ownership
    /// convention [`Self::lower_call_args`] applies, with the receiver as
    /// parameter 0: an aliasing operand is retained here because the callee
    /// releases every refcounted parameter at scope exit, and a fresh one
    /// (`new Point(1) < $p`) simply transfers the reference it already has.
    ///
    /// A compiled class dispatches on the receiver's runtime class:
    /// `Comparable::compareTo` is a bodiless interface method, so the resolved
    /// declaration names no compiled function — the same `has_body: false`
    /// path an interface method call already takes. A `Core`-owned class takes
    /// the other branch instead, and the comment on it owns why.
    pub(crate) fn lower_object_comparison(
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
        let core = nvs_types::core_symbol_of(&call.class, &call.method);
        let mark = self.temporaries_mark();
        let (lv, lty) = self.lower_expr(lhs, None, env, cur);
        let (rv, rty) = self.lower_expr(rhs, None, env, cur);
        assert!(
            matches!(lty, Ty::Object) && matches!(rty, Ty::Object),
            "nvs-ir: `nvs_types` recorded a `Comparable::compareTo` target for a comparison \
             whose operands lowered to {lty:?}/{rty:?} rather than two objects"
        );
        let ordering = if let Some(symbol) = core {
            // A `Core`-owned class satisfies `Comparable` by carrying the
            // member (`nvs_stdlib::registry::implements_comparable`), and that
            // member is native Rust behind a helper symbol with no entry in
            // any compiled method table — so the dispatch below would find
            // nothing to call. The same `InstKind::CoreCall`
            // `Lowering::lower_method_call` emits for `$d->compareTo($e)`
            // written out, with the receiver in argument slot 0.
            //
            // The ownership rule inverts with it: a `Core` member **borrows**
            // every argument, so an operand read out of a binding is retained
            // by nobody here, and a freshly built one (`Core\Time::now() <
            // $deadline`) is this frame's temporary to release — the opposite
            // of the transferring branch below.
            for (v, operand) in [(lv, lhs), (rv, rhs)] {
                if !self.aliasing_read(operand) {
                    self.own_temporary(v);
                }
            }
            let (ordering, _) = self.emit_fallible(
                *cur,
                Ty::Int,
                InstKind::CoreCall {
                    symbol,
                    args: vec![lv, rv],
                },
                env,
            );
            self.release_temporaries_since(mark, *cur);
            ordering
        } else {
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
            ordering
        };
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

    /// `!expr` — `rule:expressions/truthy-positions`'s truthy table applied to `expr`, then negated;
    /// always produces [`Ty::Bool`] regardless of `expr`'s own type, unlike a
    /// plain arithmetic/bitwise unary operator. `expr` is lowered through
    /// [`Self::lower_expr`], so `!($a && $b)`/`!($a ? $b : $c)` compose the
    /// same way a bare `&&`/`||`/ternary does.
    pub(crate) fn lower_not(&mut self, inner: &Expr, env: &mut Env, cur: &mut BlockId) -> ValueId {
        let (v, ty) = self.lower_expr(inner, None, env, cur);
        let is_alias = self.aliasing_read(inner);
        let b = self.truthy_value(v, ty, is_alias, *cur, env);
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
    /// itself be any type `rule:expressions/truthy-positions`'s table covers, and either may itself be a
    /// nested `&&`/`||`/`!`/ternary.
    pub(crate) fn lower_and(
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
    pub(crate) fn lower_or(
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

    pub(crate) fn lower_unary(
        &mut self,
        op: AstUnaryOp,
        inner: &Expr,
        expected: Option<Ty>,
        env: &mut Env,
        cur: &mut BlockId,
    ) -> (ValueId, Ty) {
        let (v, ty) = self.lower_expr(inner, expected, env, cur);
        // `rule:types/decimal`'s scalar has no machine negate: like every other
        // operator over one it is a helper call. It cannot fail --
        // the mantissa is unsigned, so there is no asymmetric minimum
        // to overflow the way `-i64::MIN` does.
        if ty == Ty::Decimal && matches!(op, AstUnaryOp::Neg) {
            return self.emit_fallible(
                *cur,
                Ty::Decimal,
                InstKind::HelperCall {
                    helper: Helper::DecimalNeg,
                    args: vec![v],
                },
                env,
            );
        }
        // `rule:types/arithmetic`'s unary rows for the operand shape the binary arms above
        // answer for equality, ordering and arithmetic: a `mixed`, a union or
        // the `int|float` a division returns names no row where it is written,
        // so the tag names it when it arrives. See `Helper::ValueNeg`, which is
        // this pair's home; unary `+` is not among them because it is the
        // identity over every numeric row and so returns below with no
        // instruction at all, whatever the operand's representation.
        //
        // The operand is staged and released exactly as the binary arms stage
        // theirs, and for the same reason: both helpers carry `rule:errors/propagation`'s error
        // edge — the closed table and the negation overflow are each a way one
        // throws — so an operand released inline would be abandoned on the edge
        // a throw leaves by.
        if ty == Ty::Tagged && matches!(op, AstUnaryOp::Neg | AstUnaryOp::BitNot) {
            let mark = self.temporaries_mark();
            let aliasing = self.aliasing_read(inner);
            self.account_for_arg(v, ty, ArgOwnership::Borrowed, aliasing, *cur);
            let helper = if matches!(op, AstUnaryOp::Neg) {
                Helper::ValueNeg
            } else {
                Helper::ValueBitNot
            };
            let (answer, _) = self.emit_fallible(
                *cur,
                Ty::Tagged,
                InstKind::HelperCall {
                    helper,
                    args: vec![v],
                },
                env,
            );
            self.release_temporaries_since(mark, *cur);
            return (answer, Ty::Tagged);
        }
        let uop = match op {
            AstUnaryOp::Neg => UnOp::Neg,
            // `rule:types/arithmetic`'s `~` row: the operand type, preserved, and total
            // over it — every 64-bit pattern is a value of both `int` and
            // `uint`, so this is the one unary arithmetic row with no edge.
            AstUnaryOp::BitNot => UnOp::BitNot,
            // `rule:types/arithmetic` gives unary `+` no row because there is nothing for
            // one to say: over `int`, `uint`, `float` and `decimal` alike it is
            // the identity, and it is the identity in PHP too. So the operand
            // *is* the result — no instruction, and no overflow edge, for the
            // same reason `~` has none. What makes that safe rather than a
            // silent divergence is one refusal a phase up:
            // `nvs_types::expr::operators::reject_unary_arith_operand` turns
            // away every operand that is not one of those four, because PHP's
            // `+"5"` is a *numeric conversion* and `rule:types/conversion` has no implicit
            // one for it to be.
            AstUnaryOp::Plus => return (v, ty),
            // Every `UnaryOp` is accounted for, and this arm has no reachable
            // target left. `-`, `~` and `+` are the arms above; `!` is split out by
            // `Self::lower_expr` into `Self::lower_not` before this function is
            // called at all (`rule:expressions/truthy-positions`'s truthy table answers `Ty::Bool`
            // whatever the operand's own type is); and `@` never reaches the
            // IR, the parser refusing error suppression outright as `E0236`
            // since `rule:errors/escalation-ladder`'s ladder leaves it nothing to suppress. That
            // subtraction is the proof — the message below is not.
            other => panic!(
                "nvs-ir: unreachable — `UnaryOp::{other:?}` reached the lowering dispatch; \
                 see this arm's own comment for the roster it subtracts"
            ),
        };
        let kind = InstKind::UnOp {
            op: uop,
            operand: v,
        };
        // `rule:types/arithmetic`'s overflow throw reaches the unary row too, and for the
        // same reason the additive ones take it: `-i64::MIN` has no `int` and
        // `-$u` no `uint` for any non-zero `$u`, so `ineg` would answer with a
        // wrapped value rather than with the `ArithmeticError` the ADR names.
        // `nvs-codegen`'s `emit_checked_int_arith` raises it inline, so this
        // needs `rule:errors/propagation`'s error edge exactly as `%` and `/` do. `!` over a
        // `bool` and `-` over a `float`/`decimal` cannot fail and do not take
        // one — see `Inst::on_error`.
        if matches!(uop, UnOp::Neg) && matches!(ty, Ty::Int | Ty::Uint) {
            return self.emit_fallible(*cur, ty, kind, env);
        }
        self.emit(*cur, ty, kind)
    }

    /// `$x === null` / `$x !== null` — a *tag* comparison, not a value
    /// one. Split out ahead of the general arm below for two reasons,
    /// and either alone would be enough: `null` has its own
    /// representation, so the general arm would hand `nvs-codegen` a
    /// `BinOp` over two different ones; and a `Ty::Tagged` operand's
    /// strict identity is `nvs_runtime::value_identical`, never a
    /// machine compare of the register pair. This is also the test
    /// `nvs_types::locals`' narrowing reads, so the two agree on
    /// exactly one spelling.
    ///
    /// `==`/`!=` are the whole of it:
    /// `rule:expressions/one-equality-operator` leaves one spelling, and its § 3 makes that spelling this tag
    /// test rather than PHP's truthy-table question, under which `0 == null`
    /// is *true*.
    pub(crate) fn lower_null_identity(
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
        // `rule:types/class-reference`'s `?class<T>`: not tagged, and still able to hold
        // `null` — see [`Ty::ClassDesc`], which owns why and lists every site
        // this one is among. The test is the word against zero, and a
        // non-nullable `class<T>` operand takes it too rather than needing a
        // rule of its own: a descriptor is never at address zero, so the
        // comparison answers the constant the arm below would have.
        if ty == Ty::ClassDesc {
            let word = self.class_desc_word(v, ty, *cur);
            let (zero, _) = self.emit(*cur, Ty::Int, InstKind::ConstInt(0));
            return self.emit(
                *cur,
                Ty::Bool,
                InstKind::BinOp {
                    op: if op == BinaryOp::Eq {
                        BinOp::Eq
                    } else {
                        BinOp::NotEq
                    },
                    lhs: word,
                    rhs: zero,
                },
            );
        }
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

    /// `rule:core-classes/html-auto-escape`'s `Markup + Markup` and
    /// `rule:tooling/styling-is-a-value-not-a-grammar`'s
    /// `Text + Text` — two carrier fragments composed into one, and the last
    /// way each of the two classes is obtained.
    ///
    /// **`symbol` comes from the checker, not from `lty`/`rty`**, for
    /// [`nvs_types::expr_table::ExprInfo::SecretEquality`]'s reason: a class
    /// does not survive [`Ty`], so both carriers reach here as a pair of
    /// `Ty::Object`s and nothing on this side can tell a `Markup` from a
    /// `Text`. The checker admitted the pair and names the composition it
    /// admitted, at the `+` expression's own span.
    ///
    /// It is an [`InstKind::CoreCall`] for the reason
    /// [`Self::lower_markup_lift`] is: what it produces is a one-slot instance
    /// of a class whose layout `nvs-stdlib` owns, and every [`Helper`] symbol
    /// is one `nvs-runtime` exports.
    ///
    /// **The operands are staged rather than released inline**, exactly as the
    /// `Ty::Tagged` arithmetic arm below stages its own and for the same
    /// reason: the call carries `rule:errors/propagation`'s error edge, so an operand released
    /// after it would be abandoned on the edge a throw leaves by.
    /// The two operands arrive paired rather than as four parameters because
    /// this body only ever uses them paired — the expression answers whether
    /// the read aliases and the value is what the call takes — and because
    /// `symbol` is what pushed the flat spelling past
    /// `clippy::too_many_arguments`.
    fn lower_carrier_concat(
        &mut self,
        symbol: &'static str,
        operands: [(&Expr, ValueId); 2],
        env: &mut Env,
        cur: &mut BlockId,
    ) -> (ValueId, Ty) {
        let mark = self.temporaries_mark();
        for (operand, value) in operands {
            let aliasing = self.aliasing_read(operand);
            self.account_for_arg(value, Ty::Object, ArgOwnership::Borrowed, aliasing, *cur);
        }
        let (answer, _) = self.emit_fallible(
            *cur,
            Ty::Object,
            InstKind::CoreCall {
                symbol,
                args: operands.map(|(_, value)| value).to_vec(),
            },
            env,
        );
        self.release_temporaries_since(mark, *cur);
        (answer, Ty::Object)
    }

    /// `rule:expressions/one-equality-operator`'s comparison over two values
    /// already in hand — **the** equality lowering, which a written `==`/`!=`
    /// ([`Self::lower_binary`]), a `switch` label
    /// ([`Self::lower_switch`]) and a `match` arm ([`Self::lower_match`]) all
    /// reach. `rule:expressions/switch-match-equality` is why there is one of
    /// these rather than three: a label is that same comparison written
    /// without the operator, and a second table beside this one is the copy
    /// that eventually disagrees with it.
    ///
    /// `op` is `==` or `!=`, and every row answers both — `!=` being exactly
    /// `!( … == … )`, which is all § 2 leaves it to be.
    ///
    /// The rows, each settled here rather than handed to `nvs-codegen`, whose
    /// `BinOp` carries one representation:
    ///
    /// * a `decimal` on either side is [`Self::lower_decimal_binary`]'s
    ///   [`Helper::DecimalEq`], `rule:types/arithmetic` giving that type a set
    ///   of helpers rather than a machine instruction;
    /// * a [`Ty::Tagged`] on either side is `rule:expressions/mixed-equality`'s
    ///   runtime row: where one side is a tag there is no machine comparison
    ///   to emit, so [`Helper::Identical`] lets the tags decide it;
    /// * `rule:types/class-reference`'s `?class<T>` against `null` compares the
    ///   descriptor *words* — the descriptor through
    ///   [`InstKind::Reinterpret`], `null` as the zero a missed
    ///   [`InstKind::ClassDescIn`] answers with — neither side being tagged and
    ///   neither having a `BinOp` row of its own. A non-nullable `class<T>`
    ///   takes it too: a descriptor is never at address zero, so `$c == null`
    ///   folds to `false` rather than needing a rule;
    /// * an enum pair is answered one representation down, on the integer
    ///   `rule:enums/no-class-machinery` makes its cases, through the free
    ///   [`InstKind::Reinterpret`] of `rule:types/conversion` row 1 — an enum
    ///   is its own equality domain, so a pair reaching here is one enum
    ///   compared with itself;
    /// * a mixed `int`/`uint`/`float` pair — one numeric domain under
    ///   `rule:expressions/disjoint-comparison-refused`, two representations —
    ///   is [`Helper::NumericEq`], which is exact across the whole domain where
    ///   a widening into `float` would round;
    /// * and a matched pair is the [`BinOp`] `nvs-codegen` turns into that
    ///   row's own comparison.
    ///
    /// **Ownership stays with the caller**, which is the only side that knows
    /// whether an operand aliases a slot: nothing here retains or releases.
    /// A caller holding a *fresh* refcounted operand stages it on
    /// [`Lowering::owned_temporaries`] first, because the `decimal`, tagged and
    /// mixed-numeric rows carry `rule:errors/propagation`'s error edge and a
    /// value abandoned on it is a leak.
    ///
    /// # Panics
    ///
    /// Guarded by `E0466`: a pair no single value inhabits never arrives,
    /// `nvs_types::expr::operators::reject_disjoint_equality` refusing it where
    /// it is written — at a written comparison, at a `switch` label and at a
    /// `match` arm alike.
    pub(crate) fn emit_equality(
        &mut self,
        op: BinaryOp,
        lhs: (ValueId, Ty),
        rhs: (ValueId, Ty),
        env: &mut Env,
        cur: &mut BlockId,
    ) -> ValueId {
        let (lv, lty) = lhs;
        let (rv, rty) = rhs;
        if lty == Ty::Decimal || rty == Ty::Decimal {
            let (answer, _) = self.lower_decimal_binary(op, lv, rv, env, cur);
            return answer;
        }
        if lty == Ty::Tagged || rty == Ty::Tagged {
            let (equal, _) = self.emit_fallible(
                *cur,
                Ty::Bool,
                InstKind::HelperCall {
                    helper: Helper::Identical,
                    args: vec![lv, rv],
                },
                env,
            );
            return self.negate_unless_eq(op, equal, *cur);
        }
        let (lv, lty, rv, rty) = if (lty == Ty::ClassDesc || rty == Ty::ClassDesc)
            && matches!(lty, Ty::ClassDesc | Ty::Null)
            && matches!(rty, Ty::ClassDesc | Ty::Null)
        {
            let lv = self.class_desc_word(lv, lty, *cur);
            let rv = self.class_desc_word(rv, rty, *cur);
            (lv, Ty::Int, rv, Ty::Int)
        } else {
            (lv, lty, rv, rty)
        };
        // Both sides or neither: an enum against its own underlying integer is
        // `rule:expressions/disjoint-comparison-refused`'s compile error, so a
        // one-sided pairing is a shape the guard below owns rather than a row.
        let (lv, lty, rv, rty) = if matches!(lty, Ty::Enum(_)) && matches!(rty, Ty::Enum(_)) {
            let (lv, lty) = self.reinterpret_enum_to_backing(lv, lty, cur);
            let (rv, rty) = self.reinterpret_enum_to_backing(rv, rty, cur);
            (lv, lty, rv, rty)
        } else {
            (lv, lty, rv, rty)
        };
        if lty != rty
            && matches!(lty, Ty::Int | Ty::Uint | Ty::Float)
            && matches!(rty, Ty::Int | Ty::Uint | Ty::Float)
        {
            let (equal, _) = self.emit_fallible(
                *cur,
                Ty::Bool,
                InstKind::HelperCall {
                    helper: Helper::NumericEq,
                    args: vec![lv, rv],
                },
                env,
            );
            return self.negate_unless_eq(op, equal, *cur);
        }
        if lty == rty {
            let (answer, _) = self.emit(
                *cur,
                Ty::Bool,
                InstKind::BinOp {
                    op: if op == BinaryOp::Eq {
                        BinOp::Eq
                    } else {
                        BinOp::NotEq
                    },
                    lhs: lv,
                    rhs: rv,
                },
            );
            return answer;
        }
        guarded_by!(
            code::E_DISJOINT_EQUALITY,
            "nvs-ir reached an equality over a {lty:?} and a {rty:?}, which no single value \
             inhabits. `nvs_types::expr::operators::reject_disjoint_equality` refuses that pair \
             where it is written, at a comparison, a `switch` label and a `match` arm alike"
        )
    }

    /// `!=` as `!( … == … )`, which `rule:expressions/one-equality-operator`
    /// makes exactly what it means. Only the helper rows need it: each answers
    /// equality alone and has no negated twin, while the `BinOp` row carries
    /// its own [`BinOp::NotEq`] and never comes through here.
    fn negate_unless_eq(&mut self, op: BinaryOp, equal: ValueId, cur: BlockId) -> ValueId {
        if op == BinaryOp::Eq {
            return equal;
        }
        let (negated, _) = self.emit(
            cur,
            Ty::Bool,
            InstKind::UnOp {
                op: UnOp::Not,
                operand: equal,
            },
        );
        negated
    }

    pub(crate) fn lower_binary(
        &mut self,
        whole: &Expr,
        expected: Option<Ty>,
        env: &mut Env,
        cur: &mut BlockId,
    ) -> (ValueId, Ty) {
        // The whole expression, rather than its three parts: `rule:security/secret-comparison-is-constant-time`'s
        // arm below needs the *comparison's* own span to read back what the
        // checker recorded there, and taking the span as a fourth parameter
        // beside the parts it already implies is what pushed this signature
        // past `clippy::too_many_arguments`.
        let ExprKind::Binary { op, lhs, rhs } = &whole.kind else {
            unreachable!("lower_binary is reached only from `lower_expr`'s `Binary` arm");
        };
        let op = *op;
        // The right operand is lowered against the left's representation,
        // which is `rule:types/numeric-literal-placement`'s placement rule carried one crate down: a
        // digit run beside a `uint` is a `ConstUint`, not an `int` that
        // happens to fit. When the digit run is the *left* operand the two
        // are lowered in the other order, and nothing is observably reordered
        // by it — a literal is a constant with no effects of its own, while
        // the operand that could have some is still evaluated exactly once.
        // The checker makes the same swap for the same reason
        // (`nvs_types::expr::uint_operand_expectation`), and without this
        // half a literal above `i64::MAX` beside a `uint` passes it and then
        // panics below on a value that never fit an `int`.
        let (lv, lty, rv, rty) =
            if matches!(lhs.kind, ExprKind::Int(_)) && !matches!(rhs.kind, ExprKind::Int(_)) {
                let (rv, rty) = self.lower_expr(rhs, expected, env, cur);
                let (lv, lty) = self.lower_expr(lhs, Some(rty), env, cur);
                (lv, lty, rv, rty)
            } else {
                let (lv, lty) = self.lower_expr(lhs, expected, env, cur);
                let (rv, rty) = self.lower_expr(rhs, Some(lty), env, cur);
                (lv, lty, rv, rty)
            };
        // `rule:types/arithmetic`'s table is a set of runtime helpers rather than
        // a machine instruction, so a `decimal` on *either* side takes
        // its own path -- including the mixed `decimal ⊕ int` row,
        // which the helper promotes from the operand's own tag.
        if lty == Ty::Decimal || rty == Ty::Decimal {
            return self.lower_decimal_binary(op, lv, rv, env, cur);
        }
        // `rule:core-classes/html-auto-escape`'s `Markup + Markup` and `rule:tooling/styling-is-a-value-not-a-grammar`'s `Text + Text`,
        // which are the whole of what an object operand may do under an
        // arithmetic operator: `nvs_types::expr::operators`'
        // `carrier_composition_result` admits a carrier beside its own kind and
        // `reject_unrowed_arithmetic_operand` refuses every other class beside
        // `+`. *Which* carrier is the entry's, not this side's — both erase to
        // `Ty::Object`, so the representations no longer tell them apart.
        if op == BinaryOp::Add
            && lty == Ty::Object
            && rty == Ty::Object
            && let Some(ExprInfo::CarrierComposition { symbol }) = self.exprs.lookup(whole.span)
        {
            return self.lower_carrier_concat(symbol, [(&**lhs, lv), (&**rhs, rv)], env, cur);
        }
        // `rule:types/arithmetic`'s ordering rows for the operand shape
        // `Self::emit_equality` answers for equality: a `mixed` or a union names no row, so
        // the tag names it at run time. `>`/`>=` are the `<` helpers with
        // their operands swapped, the arrangement `lower_decimal_binary` and
        // the `NumericLt` pair above both use, which is what gives a `NaN`
        // operand PHP's `false` for every ordering at once.
        //
        // The one comparison in this function emitted with `rule:errors/propagation`'s error
        // edge, and `Helper::ValueLt`'s own doc comment is that decision's
        // home: § 4's ordering table is closed, so a tag pair it names no row
        // for is `E0715`'s refusal made at the first moment it is answerable.
        // Each operand therefore goes on the owned-temporaries stack rather
        // than being released inline — a throw here has an edge to leave by,
        // and a fresh operand abandoned on it is exactly the leak
        // `Lowering::owned_temporaries` exists to stop.
        if matches!(
            op,
            BinaryOp::Lt | BinaryOp::LtEq | BinaryOp::Gt | BinaryOp::GtEq | BinaryOp::Cmp
        ) && (lty == Ty::Tagged || rty == Ty::Tagged)
        {
            let mark = self.temporaries_mark();
            for (operand, value, ty) in [(lhs, lv, lty), (rhs, rv, rty)] {
                let aliasing = self.aliasing_read(operand);
                self.account_for_arg(value, ty, ArgOwnership::Borrowed, aliasing, *cur);
            }
            let (helper, ty, args) = match op {
                BinaryOp::Lt => (Helper::ValueLt, Ty::Bool, vec![lv, rv]),
                BinaryOp::Gt => (Helper::ValueLt, Ty::Bool, vec![rv, lv]),
                BinaryOp::LtEq => (Helper::ValueLtEq, Ty::Bool, vec![lv, rv]),
                BinaryOp::GtEq => (Helper::ValueLtEq, Ty::Bool, vec![rv, lv]),
                _ => (Helper::ValueCmp, Ty::Int, vec![lv, rv]),
            };
            let (answer, _) =
                self.emit_fallible(*cur, ty, InstKind::HelperCall { helper, args }, env);
            self.release_temporaries_since(mark, *cur);
            return (answer, ty);
        }
        // `rule:types/arithmetic`'s *arithmetic* and bitwise rows for the operand shape
        // `Self::emit_equality` and the ordering arm above answer for their own
        // operators, and the last of the three: a `mixed`, a union or the
        // `int|float` a division returns
        // names no row where it is written, so the tags name it when they
        // arrive. See `Helper::ValueAdd`, which is this family's home.
        //
        // The result is `Ty::Tagged` for every row here, because which
        // row a pair of tags takes is exactly what is not known here — `$m + 1`
        // is an `int`, a `float` or a throw. `Lowering::coerce` absorbs it into
        // whatever the position declares, by the same rows it already absorbs
        // integer `/`'s union with.
        //
        // The operands are staged and released exactly as the ordering arm
        // stages its own, and for the same reason: these helpers carry ADR
        // 0002's error edge — a closed table, an overflow and the `int ⊕ uint`
        // pair are each a way one throws — so an operand released inline would
        // be abandoned on the edge a throw leaves by.
        if matches!(
            op,
            BinaryOp::Add
                | BinaryOp::Sub
                | BinaryOp::Mul
                | BinaryOp::Div
                | BinaryOp::Mod
                | BinaryOp::Pow
                | BinaryOp::BitAnd
                | BinaryOp::BitOr
                | BinaryOp::BitXor
                | BinaryOp::Shl
                | BinaryOp::Shr
        ) && (lty == Ty::Tagged || rty == Ty::Tagged)
        {
            let mark = self.temporaries_mark();
            for (operand, value, ty) in [(lhs, lv, lty), (rhs, rv, rty)] {
                let aliasing = self.aliasing_read(operand);
                self.account_for_arg(value, ty, ArgOwnership::Borrowed, aliasing, *cur);
            }
            let helper = match op {
                BinaryOp::Add => Helper::ValueAdd,
                BinaryOp::Sub => Helper::ValueSub,
                BinaryOp::Mul => Helper::ValueMul,
                BinaryOp::Div => Helper::ValueDiv,
                BinaryOp::Mod => Helper::ValueMod,
                BinaryOp::Pow => Helper::ValuePow,
                BinaryOp::BitAnd => Helper::ValueBitAnd,
                BinaryOp::BitOr => Helper::ValueBitOr,
                BinaryOp::BitXor => Helper::ValueBitXor,
                BinaryOp::Shl => Helper::ValueShl,
                _ => Helper::ValueShr,
            };
            let (answer, _) = self.emit_fallible(
                *cur,
                Ty::Tagged,
                InstKind::HelperCall {
                    helper,
                    args: vec![lv, rv],
                },
                env,
            );
            self.release_temporaries_since(mark, *cur);
            return (answer, Ty::Tagged);
        }
        // `rule:security/secret-comparison-is-constant-time`: `==` over a pair at least one side of which is
        // `secret` is a constant-time comparison, so that a program comparing
        // its own session token or signature with the language's one equality
        // operator is not a timing oracle. The qualifier is already gone by
        // here — `erase_checked_ty` spends no representation on it, which is
        // `rule:security/secret-qualifier`'s promise — so the choice cannot be re-derived from
        // `lty`/`rty`, and is read back from what the checker recorded at this
        // comparison instead. See `Helper::SecretEq`.
        //
        // Guarded on the two representations as well as on the entry: a
        // `secret` operand compared against a `mixed` one has already been
        // taken by the `Tagged` arm above, where the row is a runtime tag
        // rather than a buffer this helper could read. § 2's poisoning makes
        // that pair rare, and closing it would mean teaching
        // `nvs_runtime::value_identical` the property, which is wider than
        // this section asks for.
        if matches!(op, BinaryOp::Eq | BinaryOp::NotEq)
            && matches!(lty, Ty::Str | Ty::Bytes)
            && matches!(rty, Ty::Str | Ty::Bytes)
            && matches!(
                self.exprs.lookup(whole.span),
                Some(ExprInfo::SecretEquality)
            )
        {
            let (equal, _) = self.emit_fallible(
                *cur,
                Ty::Bool,
                InstKind::HelperCall {
                    helper: Helper::SecretEq,
                    args: vec![lv, rv],
                },
                env,
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
        // `rule:expressions/switch-match-equality`: one equality lowering, and
        // this is where a written `==`/`!=` reaches it — the same call a
        // `switch` label and a `match` arm make. Every row left is
        // `Self::emit_equality`'s: the tagged one, `?class<T>` against `null`,
        // an enum pair on its backing integer, a mixed numeric pair, and the
        // matched pair `nvs-codegen` compares directly.
        //
        // Ownership stays on this side, which is the only one that knows
        // whether an operand aliases a slot. A comparison only *reads* its
        // operands, so a refcounted one no durable slot owns — the string
        // literal in `$key == "bad"` is the shape this exists for — is
        // released right after the comparison reads it, per operand rather
        // than per pair: the two sides may hold two representations, which is
        // what the rows above the `BinOp` are for.
        if matches!(op, BinaryOp::Eq | BinaryOp::NotEq) {
            let answer = self.emit_equality(op, (lv, lty), (rv, rty), env, cur);
            for (operand, value, ty) in [(lhs, lv, lty), (rhs, rv, rty)] {
                if ty.is_refcounted() && !self.aliasing_read(operand) {
                    self.emit_release(*cur, value);
                }
            }
            return (answer, Ty::Bool);
        }
        // `rule:types/arithmetic`'s ordering rows, which are *not* its arithmetic ones:
        // the table's own closing paragraph says a comparison "has an exact
        // answer in the mathematical integers and can be lowered as one", so a
        // mixed numeric pair is settled by a helper here — the same shape and
        // the same reason as `Helper::NumericEq` next door — rather than by
        // the widening below, which past 2^53 would raise `ArithmeticError`
        // where PHP answers an ordering. `>`/`>=` are the same helpers
        // with their operands swapped, the arrangement `lower_decimal_binary`
        // already uses. See `Helper::NumericLt`.
        // `<=>` over a mixed numeric pair, by the same route and for the same
        // reason as the ordering operators below — one exact answer over
        // the whole domain, where the widening past this point would raise
        // `ArithmeticError` above 2^53 for a pair that orders perfectly well.
        // Split out rather than folded in with them because its result is an
        // `int` and theirs is a `bool`. See `Helper::NumericCmp`.
        if op == BinaryOp::Cmp
            && lty != rty
            && matches!(lty, Ty::Int | Ty::Uint | Ty::Float)
            && matches!(rty, Ty::Int | Ty::Uint | Ty::Float)
        {
            return self.emit_fallible(
                *cur,
                Ty::Int,
                InstKind::HelperCall {
                    helper: Helper::NumericCmp,
                    args: vec![lv, rv],
                },
                env,
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
            return self.emit_fallible(*cur, Ty::Bool, InstKind::HelperCall { helper, args }, env);
        }
        // `rule:types/arithmetic`'s "either operand a `float`" row, made real: the
        // checker types the pair `float`, but until here both operands still
        // travelled in their own representation and `nvs-codegen`'s "a
        // `BinOp` has one representation" invariant refused them. So the
        // integer side widens *here*, beside the `decimal`, `Tagged` and
        // `NumericEq` arms above, which settle their own mixed pairings the
        // same way rather than asking the backend to.
        //
        // It is the checked widening — the very helper `$n as float` emits —
        // because `rule:types/conversion` names this as the one implicit conversion in
        // the language and says it "throws above 2^53 rather than rounding".
        // A silent `fcvt_from_sint` would answer an exact-in-the-integers
        // question with a rounded one, which is the same reason `==` next
        // door is a helper and not a widening. Being fallible, it carries
        // `rule:errors/propagation`'s error edge exactly as the written conversion does.
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
            // operands': `rule:types/arithmetic` types `int / int` as `int|float` and
            // `uint / uint` as `uint|float`, PHP-exact, so which of the two a
            // given pair produces is only known at run time and the value is
            // therefore [`Ty::Tagged`]. `nvs-codegen`'s `emit_int_div` owns
            // the branch; `Lowering::coerce` owns the widening that absorbs
            // the union back into a declared `float`, which is `rule:types/arithmetic`'s
            // own worked example `float $avg = $sum / $n;`.
            BinaryOp::Div if matches!(lty, Ty::Int | Ty::Uint) => (BinOp::Div, Ty::Tagged),
            BinaryOp::Div => (BinOp::Div, lty),
            BinaryOp::Mod => (BinOp::Mod, lty),
            // `rule:types/arithmetic` puts `**` in the same row as `+`, `-` and `*` — the
            // operand type, and a throw rather than a wrap — so it needs no
            // arm of its own here beyond this one. What is not shared is the
            // *emission*: see `BinOp::Pow`, which is a loop over an integer
            // pair and a call over a float one.
            BinaryOp::Pow => (BinOp::Pow, lty),
            // `rule:types/arithmetic`'s bitwise rows, each of which preserves the
            // operand type. `>>` is the one that reads its operand's
            // signedness rather than only its width — arithmetic on an `int`,
            // logical on a `uint` — and `nvs-codegen` picks that from the
            // representation this carries.
            BinaryOp::BitAnd => (BinOp::BitAnd, lty),
            BinaryOp::BitOr => (BinOp::BitOr, lty),
            BinaryOp::BitXor => (BinOp::BitXor, lty),
            BinaryOp::Shl => (BinOp::Shl, lty),
            BinaryOp::Shr => (BinOp::Shr, lty),
            BinaryOp::Lt => (BinOp::Lt, Ty::Bool),
            BinaryOp::LtEq => (BinOp::LtEq, Ty::Bool),
            BinaryOp::Gt => (BinOp::Gt, Ty::Bool),
            BinaryOp::GtEq => (BinOp::GtEq, Ty::Bool),
            // `rule:classes/ordering-lowers-to-compare-to`'s `<=>` over a *scalar*: the object form never
            // reaches here (`lower_expr`'s guarded arm takes it), and a mixed
            // numeric or `decimal` pair has already returned above — so what
            // is left is one representation and one `BinOp` over it. The
            // result is an `int` whatever the operands hold, which alongside
            // `Div` makes it one of the rows whose type is not `lty`.
            BinaryOp::Cmp => (BinOp::Cmp, Ty::Int),
            // Every `BinaryOp` is accounted for and this arm has no reachable
            // target left. Most of them are the rows above (`Div` twice,
            // guarded by its operands' representation); `==` and `!=` are
            // `Self::emit_equality`'s, which the arm above returns through for
            // every pair it admits. The rest never arrive
            // here at all, because `Self::lower_expr` takes each of them
            // *before* the general `Binary` arm that is this function's only
            // caller: `.` goes to `Self::lower_concat`, which flattens the
            // whole spine into one `InstKind::Concat` rather than allocating a
            // buffer per operator; `&&` and `||` short-circuit, so they are
            // branches rather than an instruction with two evaluated operands;
            // and `??` is `Self::lower_coalesce`'s null test over a value
            // that must not be evaluated twice. A compound assignment reaches
            // the same ones the same way — `AssignOp::to_binary_op` hands back
            // an ordinary `BinaryOp` and the desugar re-enters `lower_expr`.
            // That subtraction is the proof; the message below is not.
            other => panic!(
                "nvs-ir: unreachable — `BinaryOp::{other:?}` reached the scalar-operator \
                 table; see this arm's own comment for the roster it subtracts"
            ),
        };
        // A comparison only *reads* its operands, so a refcounted one
        // that no durable slot owns — the string literal in
        // `$key < "bad"` is the shape this exists for — is released
        // right after the instruction reads it, exactly the rule the
        // equality arm above applies to its own pair.
        //
        // Every *integer* arithmetic operator here can fail, and `rule:types/arithmetic`
        // is why: `+`, `-`, `*` and `**` throw `ArithmeticError` on overflow
        // rather than wrapping, `%` and `/` throw it on a zero divisor, and
        // `**` throws it on a negative exponent as well.
        // `nvs-codegen` raises them inline rather than through a helper,
        // so each needs an error edge exactly the way a call does.
        //
        // `/` is the one that is not an integer row: § 4 refuses the zero
        // divisor *before* the operand types are consulted, so the float row
        // throws where the integer one does and carries the same edge. It is
        // recognised by its *result* either way — `Ty::Tagged` is the integer
        // row, whose quotient is `int|float`, and `Ty::Float` the other. Every
        // other operator — the comparisons, and the remaining float rows —
        // returns no status at all, see `Inst::on_error`.
        let inst = InstKind::BinOp {
            op: bop,
            lhs: lv,
            rhs: rv,
        };
        let fallible = match bop {
            BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Mod | BinOp::Pow => {
                matches!(ty, Ty::Int | Ty::Uint)
            }
            BinOp::Div => matches!(ty, Ty::Tagged | Ty::Float),
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

    /// One operand of a binary operator, widened into `rule:types/arithmetic`'s
    /// `float` row when — and only when — the *other* operand is already
    /// one. `other` is that operand's representation; everything else is
    /// returned untouched, so a matched pair costs nothing and no
    /// instruction is emitted for it.
    ///
    /// The conversion is [`Helper::IntToFloat`]/[`Helper::UintToFloat`],
    /// the same pair `$n as float` lowers to, because `rule:types/conversion` makes
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
}
