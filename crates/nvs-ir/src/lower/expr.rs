//! Expression lowering — [`Lowering::lower_expr`]'s dispatch itself, and the
//! shapes with nowhere more specific to be: member access, the nullsafe chain,
//! `match`, the literals and the array forms.
//!
//! Two of its former areas are their own modules, reached the way `lower_expr`
//! reaches any other: ADR 0007 § 2's conversions and ADR 0035's truthiness are
//! [`super::convert`], and § 4's operator table is [`super::operator`].
//!
//! Part of [`super`]'s one `impl Lowering`, split across this directory so a
//! session editing one area does not carry the rest in context. Every item
//! moved here unchanged; the methods are `pub(super)` so they reach across
//! these modules and no further, which is the reach they had when `lower` was
//! a single file.

use super::*;

/// What `nvs_types::expr_table::ExprInfo::ShapeProperty` resolved for one
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
    /// Its declared type, still in `nvs_types`' interner.
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
            // `(expr)` is fully transparent — `nvs_types::expr::check_expr`'s
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
            // nowdoc: `nvs_syntax::parser::collapse_string_parts` only ever
            // reaches `Interpolated` when at least one interpolation site
            // was used, which a nowdoc's body can never contain.
            ExprKind::Interpolated(parts) => {
                let raw = span_text(self.src, expr.span);
                assert!(
                    raw.starts_with('"') || raw.starts_with("<<<"),
                    "nvs-ir only lowers a double-quoted or heredoc-sourced Interpolated string — \
                     got {raw:?}; see the crate docs' known gaps"
                );
                self.lower_interpolated_parts(parts, expr.span, env, cur)
            }
            ExprKind::Variable(span) => {
                let name = strip_sigil(span_text(self.src, *span));
                let &(v, ty) = env.get(name).unwrap_or_else(|| {
                    panic!(
                        "nvs-ir: undeclared local `${name}` — lower_method trusts its input \
                         already passed nvs_types::check_program"
                    )
                });
                // An `inout $x` parameter binds an address, not a value: reading it
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
            // allocation. `nvs_types` resolved the value — the enum's
            // auto-increment rule for one, `nvs_stdlib::registry`'s own row
            // for the other — into `ExprInfo::EnumCase`/`ExprInfo::CoreConst`;
            // a **user-declared** class's constant records neither and is
            // still unlowered.
            ExprKind::ClassConstAccess { .. } => match self.exprs.lookup(expr.span) {
                Some(ExprInfo::EnumCase { value }) => match value {
                    nvs_types::EnumValue::Int(n) => {
                        self.emit(*cur, Ty::Enum(EnumRepr::Int), InstKind::ConstInt(*n))
                    }
                    nvs_types::EnumValue::Uint(n) => {
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
                    "nvs-ir: a `Class::CONST` at {:?} with no resolved enum case or `Core` \
                     constant recorded in the typed-expression table — a user-declared class \
                     constant's value is unmodeled in `nvs_types` (see its own known gaps), so \
                     there is nothing to lower it to",
                    expr.span
                ),
            },
            // `Foo::class` — the class's own fully qualified name, and a
            // `Ty::Str` constant with no storage behind it, exactly like the
            // two constants one arm above. The *name* is resolved by
            // `nvs_types::expr::members::check_class_name_const` and travels
            // in the same `ExprInfo::CoreConst` a `Core` class constant does,
            // for two reasons that point the same way: a constant is inlined
            // at its use site whichever of the three it is, and this crate
            // cannot name a `nvs_hir::QName` to do the resolution itself.
            //
            // The lookup only misses in a compilation that has already
            // aborted — the checker records a name for every `::class` it
            // accepts and reports `E0702` for the one shape it does not — so
            // the fallback is an empty string rather than a panic, which is
            // what keeps this arm's own reachability a fact about the checker
            // rather than a claim in a message.
            ExprKind::ClassNameConst { .. } => match self.exprs.lookup(expr.span) {
                Some(ExprInfo::CoreConst { value }) => {
                    let value = value.clone();
                    self.emit_const_arg(&value, env, *cur)
                }
                _ => self.emit(*cur, Ty::Str, InstKind::ConstStr(String::new())),
            },
            // PHP 8's `throw` in expression position — `$n ?? throw new
            // LogicError("…")`. The same `Self::lower_throw` the statement
            // form goes through, which seals `*cur` with `Terminator::Throw`;
            // what this adds is the fresh block every caller then writes into
            // and the `never`-typed value it hands back, both unreachable by
            // construction and both required because this dispatch is total
            // in `(ValueId, Ty)`. `Self::lower_exit` one arm below is the same
            // shape for the same reason.
            ExprKind::Throw(inner) => {
                self.lower_throw(inner, env, cur);
                *cur = self.new_block();
                self.emit(*cur, Ty::Int, InstKind::ConstInt(0))
            }
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
            // numeric (`nvs_types`' `E0474` refuses the rest), and
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
            // owes. `nvs_types::expr::presence` marks its subscripts guarded,
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
            // Nothing the checker accepts reaches this arm any more, and the
            // proof is the roster rather than the message below it. `ExprKind`
            // has 45 variants; the arms above cover 34 of them, plus one of
            // `Assign`'s two `inout` shapes. Of the eleven with no arm and
            // the one `Assign` shape:
            //
            // * `Error` is a parse error already reported, and does not
            //   survive to a compilation that lowers.
            // * a bare `NAME` (`ConstFetch`) is `E0319` — ADR 0011 § 3 gives
            //   a constant no home but a class — and `self`/`static`/`parent`
            //   used as a *value* are `E0321`, both from `nvs_hir::members`.
            //   All four still appear as the class *side* of a `::`, which is
            //   not this dispatch's business: `walk_class_side` skips them and
            //   the arms above read the checker's own resolution instead.
            // * `$a = &$b` is `E0701`: ADR 0031 § 2 removed by-reference
            //   capture, so there is no owner for the `&`.
            // * every `yield` shape is `E0448` where it has no lowering — a
            //   key half, a `yield from`, a missing value, and (since this
            //   pass) one used as a *value*, ADR 0053 § 5 giving a generator
            //   no `send()` for it to answer with. The statement form goes
            //   through `Self::lower_yield` one file over, reached from
            //   `nvs_types::expr::check_expr_stmt`'s matching split.
            // * `spawn script` is `E0703` and `require` used for its value is
            //   `E0704`, both because nothing below this crate compiles them
            //   yet — ADR 0006's isolates arrive at M5, and the value form of
            //   ADR 0021 § 3 needs the frame-per-file this crate's known gap
            //   22 is about. `require` as a *statement* lowers to nothing, one
            //   file over.
            // * `$obj::class` is `E0702`; the statically-named spelling lowers
            //   one arm above.
            //
            // `Ternary`, `Match`, `Paren` and `ObjectLiteral`, which used to
            // arrive here, all lower above.
            //
            // That subtraction is the proof; the message below is not.
            other => panic!(
                "nvs-ir: unreachable — `ExprKind::{other:?}` reached the expression lowering \
                 dispatch; see this arm's own comment for the roster it subtracts"
            ),
        }
    }
    /// `echo $a, $b;` — writes each operand's bytes to standard output in
    /// order, with no separator and no escaping: `docs/agent/loop-goal.md`
    /// records that ADR 0024 § 5's auto-escaping sink is the HTTP *response*
    /// write, not this one, and that whether `echo` under a future
    /// `nvs serve` becomes that sink is an M7 decision this does not
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
    /// A run of literal text between `?>` and the next `<?nvs` — spec
    /// `00-overview.md` § 1 — written to the request's output verbatim.
    ///
    /// The span points straight at the source bytes, so there is nothing to
    /// cook: unlike a string literal it carries no quotes and no escape
    /// sequences, and unlike [`Self::lower_echo`]'s operands it is never
    /// converted or escaped on the way out. `nvs_syntax::Lexer::lex_code`
    /// already swallowed the one newline immediately after `?>`, and
    /// `lex_html` pushes no token at all for an empty run, so the text this
    /// receives is exactly what the page owes and never the empty string.
    ///
    /// From there it is [`Self::lower_echo`]'s own tail, for the same reasons
    /// that function's doc comment gives: one [`Helper::EchoStr`] call, the
    /// one conversion-free helper that can genuinely fail, carrying the
    /// failure edge [`Self::landing_block`] hands out. The
    /// [`InstKind::ConstStr`] is a fresh value with exactly one use, so it
    /// goes on [`Self::owned_temporaries`] and is released on both edges.
    ///
    /// Nothing here is file-scope-specific: `?>`/`<?nvs` reopen and reclose
    /// code mode anywhere a statement is expected (`nvs_syntax::ast::StmtKind::InlineHtml`),
    /// and a run inside a loop body lowers into that body like any other
    /// statement.
    pub(super) fn lower_inline_html(&mut self, span: Span, cur: &mut BlockId, env: &mut Env) {
        let text = span_text(self.src, span).to_owned();
        let mark = self.temporaries_mark();
        let (v, _) = self.emit(*cur, Ty::Str, InstKind::ConstStr(text));
        self.own_temporary(v);
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
    /// `print $x` — one operand written exactly as [`Self::lower_echo`]
    /// writes it, answering the `1` PHP answers.
    ///
    /// PHP's own difference between `print` and `echo` is that `print` is an
    /// *expression*: it takes one operand rather than a list, and its value is
    /// always the integer `1`, which is what makes `$ok && print "…"` and
    /// `$n = print "…"` legal there. `nvs_types::expr` already types it
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
    /// is `nvs_runtime::EXITED`: the ordinary status check `nvs-codegen`
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
    /// the status is `0`. `nvs_types::expr` refuses everything else, so the
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

    /// Lowers one `.` operand and, if it isn't already [`Ty::Str`], converts
    /// it through a new [`InstKind::HelperCall`] — `nvs_types::expr::
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
            // ADR 0007 § 2's row for `null`, which PHP answers with the empty
            // string and which `Helper::TaggedToString` already answers that
            // way for the `?string` holding one. A *statically* `null`
            // operand is the same value one type earlier, so it renders the
            // same rather than being refused a phase up — the checker's
            // `require_stringable` names the four types that are refused, and
            // this is not one of them. The operand itself is lowered for its
            // effects and then unused; `Ty::Null` is not refcounted, so there
            // is nothing to release.
            Ty::Null => {
                let (sv, _) = self.emit(*cur, Ty::Str, InstKind::ConstStr(String::new()));
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
                // No resolved `toString`: an erased `object` (ADR 0036 § 4), or
                // a `Core`-owned class, which is where ADR 0088 § 5's sink
                // carrier arrives. Both are decided by the value's *runtime*
                // class rather than its static one, so this is the same
                // dispatched conversion a `Ty::Tagged` operand takes —
                // `nvs_runtime::stringify` calls the class's own `toString`
                // where it has one, renders a carrier where it is one, and
                // throws otherwise.
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
            // [`crate::ty::Ty`]'s roster is fifteen and this arm has no
            // reachable target left. Nine are the rows above: `Ty::Str`, the
            // five scalars each through their own helper, `Ty::Null` as the
            // empty string, `Ty::Tagged` through the runtime tag and
            // `Ty::Object` through ADR 0028 § 1's `toString`.
            //
            // Four are refused a phase up by
            // `nvs_types::expr::operators::require_stringable`, the one check
            // every implicit site goes through, each naming the spelling that
            // says what was meant: `Ty::Bytes` (ADR 0009 § 3 grants
            // `as string` and nothing implicit), `Ty::Array` (PHP prints
            // `"Array"` and a notice; Novis names `Core\Json::encode`),
            // `Ty::Enum` (ADR 0010 § 3's named integer, `$case as int`) and
            // `Ty::Void` (a call with no value at all).
            //
            // The last two are not types a *source expression* ever has.
            // `Ty::ClassDesc` is produced only as a static call's receiver
            // slot, by `InstKind::ClassDescOf`/`ClassDescConst` — never
            // `Self::lower_expr`'s answer, `Foo::class` folding to a `string`
            // constant instead — and `Ty::Ref` only by `InstKind::RefSlot`
            // staging an `inout $x` argument, which goes straight to the callee.
            // That subtraction is the proof; the message below is not.
            other => panic!(
                "nvs-ir: unreachable — a `{other:?}` operand reached the implicit `string` \
                 conversion; see this arm's own comment for the roster it subtracts"
            ),
        }
    }

    /// ADR 0028 § 1's implicit `toString()`, for an operand that lowered to a
    /// [`Ty::Object`]. `.`, an interpolated piece, `echo`/`print` and
    /// `as string` all reach it, because
    /// `nvs_types::expr::operators::require_stringable` is the single check
    /// all four go through — so it is also the single place that records the
    /// resolved target, under the operand's own span.
    ///
    /// `None` when nothing was recorded there, which both callers answer the
    /// same way: `Helper::TaggedToString` over the receiver, which dispatches
    /// `toString` on its *runtime* class. The checker records a target wherever
    /// the operand's static type names a class to resolve against, so a missing
    /// one means it named none — an erased `object` (ADR 0036 § 4) or a union
    /// — or that it named ADR 0088 § 5's sink carrier, the one rendering class
    /// with no `toString` member at all, whose bytes `nvs_runtime::stringify`
    /// hands back as they are.
    ///
    /// The *user-declared* call is ordinary in every respect, exactly as
    /// [`Self::lower_object_comparison`]'s `compareTo` is: ADR 0002's error
    /// edge, since a `toString` body may throw like any other, and the same
    /// ownership convention [`Self::lower_call_args`] applies to a receiver —
    /// an aliasing operand is retained here because the callee releases every
    /// refcounted parameter at scope exit, and a fresh one (`echo new Name()`)
    /// transfers the reference it already has. It dispatches on the
    /// receiver's runtime class, so a `toString` overridden in a subclass wins
    /// over the one the static type names.
    pub(super) fn lower_to_string_call(
        &mut self,
        expr: &Expr,
        receiver: ValueId,
        env: &mut Env,
        cur: BlockId,
    ) -> Option<ValueId> {
        let call = self.exprs.to_string_call(expr.span)?;
        // A `Core`-owned class renders through a native symbol rather than an
        // entry in a compiled method table, so the dispatch below would find
        // nothing to call — the same `InstKind::CoreCall`
        // [`Self::lower_method_call`] emits for `$uri->toString()` written out,
        // with the receiver in argument slot 0 and **borrowed** there, which is
        // why the ownership rule inverts: an aliasing receiver needs no retain,
        // and a fresh one (`echo Core\Uuid::v4()`) is this frame's to release
        // on both edges. Nothing is dispatched on the runtime class because a
        // `Core` class is final by construction: the registry's row is the only
        // `toString` it can have.
        if let Some(symbol) = nvs_types::core_symbol_of(&call.class, &call.method) {
            let mark = self.temporaries_mark();
            if !self.aliasing_read(expr) {
                self.own_temporary(receiver);
            }
            let (s, _) = self.emit_fallible(
                cur,
                Ty::Str,
                InstKind::CoreCall {
                    symbol,
                    args: vec![receiver],
                },
                env,
            );
            self.release_temporaries_since(mark, cur);
            return Some(s);
        }
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
    /// `null`, and the reason `nvs_types` gives such an access no `null` in
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
    /// purpose**: `nvs_types::locals`' narrowing is what proves the tag, the
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
    /// cranelift rejection rather than a panic. `nvs_types` therefore records
    /// the narrowing on the read's own span
    /// (`nvs_types::expr_table::ExprInfo::NarrowedRead`) and it is discharged
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
    /// `nvs-codegen`'s own `Untag`-to-tagged identity has it.
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
    /// to and what `nvs_types` typed the whole access as. `value`/`ty` are
    /// whatever the member access produced in `*cur` — which need not be the
    /// block [`Self::open_nullsafe`] handed back, since an argument may
    /// itself have branched.
    ///
    /// A `void` member has nothing to merge: both arms simply rejoin, and the
    /// value handed back is the unusable one the call produced — the same
    /// reason `nvs_types` unions no `null` into a `void` member's type.
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
                "nvs-ir: a `??` at {:?} has no recorded result type — either it wasn't checked \
                 with the same table, or `nvs_types::expr` stopped recording one",
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
    /// No operand can *throw* on absence: `nvs_types::expr::presence` marks
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
            .expect("nvs-ir: `nvs_syntax`'s `parse_isset` always parses at least one operand");
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
    /// [`is_aliasing_read`] never lists [`nvs_syntax::ast::ExprKind::Ternary`]
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
    /// *union* of its branches (`nvs_types::expr::check_expr`'s `Ternary` arm
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
    ///   `nvs_hir::errors`' tree is closed and has no such entry, so this
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
    /// A subject whose static type names no representation of its own — a
    /// `mixed`, a union — compares against each label through
    /// [`Helper::Identical`] rather than through [`BinOp::Eq`], which is
    /// [`Self::lower_binary`]'s own rule for a written `==` over the same
    /// pair: where one side is a tag there is no machine comparison to emit,
    /// so the tags decide it at run time. The label itself is unchanged by
    /// it, being written at its own representation either way.
    ///
    /// # Panics
    ///
    /// Panics — as engine invariants, not refusals — for a label whose
    /// representation differs from a subject that is not a tag, and for a
    /// `match` with no arms at all, which `nvs_types` refuses as `E0476`
    /// before this crate ever sees it.
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
            "an arm-less `match` reached lowering — this is a bug in nvs-types, whose \
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
                // ADR 0090 § 5's `mixed`-or-union row, which is the same row
                // `Self::lower_binary` takes for a written `==`: where either
                // side's representation is a runtime tag there is no machine
                // comparison to emit, so the tags decide it in
                // `nvs_runtime::value_identical`. A label is written at its
                // own representation whatever the subject's is — a digit run
                // beside a `mixed` is still a `ConstInt` — and needs no
                // widening to get there, `nvs-codegen`'s helper convention
                // storing every argument as a 16-byte tagged `Value` already.
                let (eq_v, _) = if subj_ty == Ty::Tagged || cond_ty == Ty::Tagged {
                    self.emit(
                        test_cur,
                        Ty::Bool,
                        InstKind::HelperCall {
                            helper: Helper::Identical,
                            args: vec![subj_v, cond_v],
                        },
                    )
                } else {
                    // Not a refusal: `nvs_types` has already made every label
                    // comparable with the subject (`E0466`, ADR 0090 § 6), and the arm above
                    // takes the one pairing whose types name no static row. So
                    // an arrival here is a `nvs-ir` site that lowered a label
                    // against an expectation it then did not honour.
                    assert_eq!(
                        cond_ty, subj_ty,
                        "nvs-ir lowers a `match` label only at the subject's own representation, \
                         or through `Helper::Identical` where one of the two is a tag — got \
                         {cond_ty:?} against a {subj_ty:?} subject"
                    );
                    self.emit(
                        test_cur,
                        Ty::Bool,
                        InstKind::BinOp {
                            op: BinOp::Eq,
                            lhs: subj_v,
                            rhs: cond_v,
                        },
                    )
                };
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
    /// [`nvs_types::string_lit::cook_double_quoted_text`] — the same routine
    /// [`cook_str_literal`] delegates to for a plain double-quoted `Str`,
    /// since a `Text` run's escape grammar is identical either way (see that
    /// function's own doc comment) — unless `whole_span` opens with `<<<`
    /// (a heredoc; never a nowdoc, see this function's caller), in which
    /// case each `Text` run first goes through
    /// [`nvs_types::string_lit::dedent_heredoc_run`] against the one
    /// [`nvs_types::string_lit::heredoc_shape`] computed for the whole
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
        whole_span: nvs_diagnostics::Span,
        env: &mut Env,
        cur: &mut BlockId,
    ) -> (ValueId, Ty) {
        assert!(
            !parts.is_empty(),
            "nvs-syntax's collapse_string_parts only ever produces ExprKind::Interpolated for a \
             non-empty parts vec"
        );
        let is_heredoc = span_text(self.src, whole_span).starts_with("<<<");
        let indent = if is_heredoc {
            nvs_types::string_lit::heredoc_shape(self.src, whole_span)
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
                        let mut issues = Vec::new(); // discarded: nvs_types::check_program already reported these
                        let dedented = nvs_types::string_lit::dedent_heredoc_run(
                            self.src,
                            &indent,
                            *span,
                            i == 0,
                            Some(i) == last_text_idx,
                            &mut issues,
                        );
                        nvs_types::string_lit::cook_double_quoted_text_str(&dedented, *span).0
                    } else {
                        nvs_types::string_lit::cook_double_quoted_text(self.src, *span).0
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
    /// instruction as the `int` it already was, and `nvs-codegen` calls
    /// `nvs_array_get_index`/`nvs_array_set_index`, which answer from the
    /// packed form with nothing rendered and nothing allocated and
    /// synthesize a key only where the array is already `Hashed` — exactly
    /// the case that was building one anyway (`nvs_runtime::array`'s module
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
    /// `nvs_types::expr::check_array_key_type` now enforces at both call
    /// sites (an `Index` subscript and an array-literal explicit key alike),
    /// so the `other` arm below is an internal-invariant panic — unreachable
    /// for anything that already passed `nvs_types::check_program` — rather
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
                "nvs-ir: an array key lowered to {other:?} — nvs_types::check_program is trusted \
                 to have already rejected a float/bool/null key (ADR 0007 § 5) at both the \
                 subscript and array-literal explicit-key sites, so this should be unreachable"
            ),
        }
    }

    /// [`Self::lower_array_key`], forced all the way to a [`Ty::Str`] key.
    ///
    /// [`ir::InstKind::ArrayUnset`] is the one key-taking array instruction
    /// with no index-shaped runtime primitive beside it — `nvs-runtime`
    /// added `nvs_array_get_index` and `nvs_array_set_index` and no third —
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
    /// `int` otherwise. `nvs_types::expr::literals::infer_int_literal`
    /// enforces ADR 0007 § 4's magnitude rule
    /// at check time — too large for `int` is only legal where `uint`
    /// is expected, and too large even for `uint`'s full `u64` range
    /// is a diagnostic regardless — so `lower_method`'s usual "trusts
    /// its input already passed `nvs_types::check_program`" contract
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
            // can overflow — and `nvs_types` has already reported that
            // if it did.
            let mantissa = u128::from_str_radix(&digits, radix).unwrap_or_else(|_| {
                panic!("nvs-ir: integer literal `{digits}` doesn't fit a `decimal`")
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
                panic!("nvs-ir: integer literal `{digits}` doesn't fit a `uint`")
            });
            self.emit(*cur, Ty::Uint, InstKind::ConstUint(n))
        } else {
            let n: i64 = i64::from_str_radix(&digits, radix).unwrap_or_else(|_| {
                panic!("nvs-ir: integer literal `{digits}` doesn't fit an `int`")
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
                panic!("nvs-ir: float literal `{digits}` doesn't fit a `decimal`")
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
            .unwrap_or_else(|_| panic!("nvs-ir: float literal `{digits}` failed to parse"));
        self.emit(*cur, Ty::Float, InstKind::ConstFloat(n))
    }

    /// ADR 0070 § 3: the grammar is resolved while compiling, so what
    /// reaches the IR is one folded nanosecond count. The value it
    /// becomes is built by the *same* `Core` member a written
    /// `Duration::nanoseconds($n)` calls — `nvs_stdlib::time`'s
    /// `FROM_NANOS_SYMBOL`, named there rather than spelled here — so a
    /// literal and a computed count cannot come to mean different
    /// things.
    ///
    /// § 3 also wants no allocation at all: a constant-pool entry with
    /// an immortal header, which is exactly what a string literal is
    /// owed by `nvs-runtime`'s own gap 3. Both close together; until
    /// then this is one call on a constant.
    fn lower_duration_literal(
        &mut self,
        span: Span,
        env: &mut Env,
        cur: &mut BlockId,
    ) -> (ValueId, Ty) {
        let text = span_text(self.src, span);
        let nanos = nvs_syntax::duration::parse(text).unwrap_or_else(|err| {
            panic!(
                "nvs-ir: duration literal `{text}` does not parse ({}) — the lexer \
                 only produces this token for text that does",
                err.message()
            )
        });
        let (count, _) = self.emit(*cur, Ty::Int, InstKind::ConstInt(nanos));
        self.emit_fallible(
            *cur,
            Ty::Object,
            InstKind::CoreCall {
                symbol: nvs_types::CORE_DURATION_FROM_NANOS,
                args: vec![count],
            },
            env,
        )
    }

    /// `.` concatenation is not `InstKind::BinOp` — it allocates a
    /// fresh buffer rather than computing a native scalar result, so
    /// it gets its own arm (and its own `InstKind::Concat`) ahead of
    /// the scalar-operator table below. Each operand goes through
    /// `Self::concat_operand` first, which converts a scalar through
    /// a new `InstKind::HelperCall` when it isn't already `Ty::Str`,
    /// and a `Stringable`-object operand through the `toString()`
    /// `nvs_types::expr::operators::require_stringable` resolved for
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
                "nvs-ir: the `fn` literal at {:?} has no resolved closure recorded in \
                 the typed-expression table — did this program pass \
                 nvs_types::check_program with the same table?",
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
        // The declared parameter types are readable here and nowhere below
        // this crate — `callable` carries no parameter list for a call site to
        // compare against (ADR 0031 § 1), so the object is what carries them
        // to the one caller that can act on them. See `FN_PARAM_TAGS`.
        let tags = param_tags_word(fn_expr, self.exprs, self.checked_types);
        let (tags_v, _) = self.emit(*cur, Ty::Int, InstKind::ConstInt(tags));
        self.emit_field_set(*cur, obj, class.clone(), FN_PARAM_TAGS.to_owned(), tags_v);
        let mut captured = Vec::with_capacity(names.len());
        for name in names {
            let &(v, ty) = env.get(&name).unwrap_or_else(|| {
                panic!(
                    "nvs-ir: the closure at {:?} captures `${name}`, which is not bound \
                     in the enclosing frame — nvs_types records a capture only for a \
                     name its own scope resolved",
                    expr.span
                )
            });
            // An `inout $x` parameter binds an address, not a value, and ADR 0031 § 2
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
                "nvs-ir: `new` at {:?} has no resolved class recorded in the \
                 typed-expression table — did this program pass \
                 nvs_types::check_program with the same table?",
                expr.span
            );
        };
        let target_label = class.to_string();
        // The declaring class, not the constructed one: `new Dog(...)`
        // on a `Dog` with no `constructor` of its own invokes
        // `Animal::constructor`. Only `nvs_types` resolved that, so
        // the label is carried rather than re-derived downstream.
        let ctor_label = ctor
            .as_ref()
            .map(|call| format!("{}::{}", call.class, call.method));
        // A `Core`-owned class is built by a native helper rather than by an
        // `InstKind::New`: nothing below this crate holds a descriptor for
        // one, because a `Core` class is in no program's class list.
        // `nvs_stdlib::instance`'s module docs own that decision; here it is
        // the same `InstKind::CoreCall` a static `Core` member lowers to,
        // with the same **borrowed** arguments — which is why this branch
        // lowers them itself rather than sharing the transferred ones below.
        // `nvs_stdlib::registry::CONSTRUCTORS` is what gives it a signature to
        // check them against: `new Core\Heap<T>($by)` carries one.
        if let Some(symbol) = nvs_types::core_constructor_symbol(&target_label) {
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
        // A constructor may declare `inout $x` like any other method, so this site
        // owns its own staging window — see `Lowering::pending_refs`.
        let staged_refs = self.pending_refs_mark();
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
                        "nvs-ir: `new {target_label}(...)` has no resolved constructor \
                         but wasn't called with a plain argument list — {args:?}"
                    );
                };
                assert!(
                    list.is_empty(),
                    "nvs-ir: `new {target_label}(...)` has no resolved constructor but \
                     was called with arguments — nvs_types doesn't yet enforce a \
                     zero-arity check here (see its own known gaps), so this crate \
                     cannot trust it was rejected upstream"
                );
                Vec::new()
            }
        };
        // `new static()` — ADR-free by construction: the class comes
        // from this frame's called class rather than from the label
        // `nvs_types` resolved, which is the enclosing class and so
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
        let built = self.emit_fallible(*cur, Ty::Object, kind, env);
        self.flush_ref_writebacks(staged_refs, env, *cur);
        built
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
                "nvs-ir: an instance method call at {:?} has no resolved target \
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
        // `nvs_types` seeds a signature table rather than special-
        // casing `Core`; see `nvs_stdlib::registry::CoreTy::Instance`.
        if let Some(symbol) = nvs_types::core_symbol_of(&call.class, &call.method) {
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
        // is the receiver's own — see `nvs_runtime::object`'s module
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
            // argument for ownership purposes: Novis's convention is
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
        // Every `inout $x` this call stages is written back below, in the block the
        // call returns into — inside the `?->` guard when there is one, since
        // a receiver that was `null` ran no callee and wrote nothing back. See
        // `Lowering::pending_refs`.
        let staged_refs = self.pending_refs_mark();
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
        // must run `Child::m`. `nvs_types` answers that whole-program
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
        self.flush_ref_writebacks(staged_refs, env, *cur);
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
    /// sees (`nvs_runtime::object`'s late-static-binding decision):
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
                "nvs-ir: a static call at {:?} has no resolved target recorded in the \
                 typed-expression table — did this program pass \
                 nvs_types::check_program with the same table?",
                expr.span
            );
        };
        // A Tier 0 `Core` member is native Rust behind a helper
        // symbol, not a compiled Novis function, so it takes a
        // different instruction and a different argument-ownership
        // rule — see `InstKind::CoreCall`, which owns both. Resolved
        // through the identical `ResolvedCall` up to this point,
        // which is the whole reason `nvs_types` seeds a signature
        // table rather than special-casing `Core` at each call site.
        if let Some(symbol) = nvs_types::core_symbol_of(&call.class, &call.method) {
            let sig = ArgSig::of_helper(call);
            let return_ty = lower_checked_ty(call.return_ty, self.checked_types);
            let checked_types = self.checked_types;
            // A member on `nvs_stdlib::registry::WRITTEN_CLASS_MEMBERS`
            // is handed the class its call site wrote, as argument 0 —
            // that roster owns the ABI. A descriptor is not
            // refcounted, so it is neither retained nor released here.
            let written_class =
                nvs_types::core_takes_written_class(&call.class.to_string(), &call.method).then(
                    || {
                        let label = call.written_class.as_ref().unwrap_or_else(|| {
                            panic!(
                                "nvs-ir: `{}::{}` needs the class written at its call site, \
                         and nvs_types recorded none — did this program pass \
                         nvs_types::check_program with the same table?",
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
            // frame's. See `nvs_runtime::object`'s module docs.
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
                    "nvs-ir: `{target_label}` is not static but is reached from a frame \
                     with no `$this` — nvs_types is expected to have refused that"
                )
            });
            if this_ty.is_refcounted() {
                self.emit_retain(*cur, this_v);
            }
            Some(this_v)
        };
        // As for an instance call: this site's own staging window, flushed
        // below once the call has returned. See `Lowering::pending_refs`.
        let staged_refs = self.pending_refs_mark();
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
        let result = self.emit_fallible(*cur, return_ty, kind, env);
        self.flush_ref_writebacks(staged_refs, env, *cur);
        result
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
                "nvs-ir: a static property access at {:?} has no resolved declaring class \
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
    /// access. That is an internal-consistency check with no reachable
    /// target rather than a hole:
    /// `nvs_types::expr::members::check_property_member` records an entry for
    /// every access it returns from and refuses the rest, and its own doc
    /// comment is that proof's only home. `lower_store`'s `PropertyAccess`
    /// arm asserts the same thing from the write side.
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
        // still reads its own slot, since Novis's hooked properties are
        // always backed (`nvs_types::signatures::PropertyHooks` owns
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
                "nvs-ir: a property access at {:?} has neither a resolved declaring class \
                  nor an ADR 0036 § 4 erased entry recorded in the typed-expression table, \
                  so it was not checked with the same table — \
                  `nvs_types::expr::members::check_property_member` records one for every \
                  access it returns from and refuses the rest, and its own doc comment \
                  carries that proof",
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
    /// `nvs_types::ty::TypeInterner::shape` interns a shape's fields in, and
    /// therefore the order [`InstKind::SlotGet`]'s index counts through: the
    /// read side resolved its slot number from the checker's field list, so
    /// the write side has to lay the slots out the same way. That agreement
    /// is the whole reason a shape needs no layout table.
    ///
    /// Nothing else is synthesized. The class is methodless, conforms to
    /// nothing and declares no constructor — the literal assigns every field
    /// itself, which is ADR 0022 § 2's definite-assignment obligation
    /// discharged by construction — so `nvs-codegen` defines it through
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
    /// set, so `nvs_types::expr::literals::check_object_literal` refuses the
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
             fields are a set, so `nvs_types::expr::literals::check_object_literal` refuses \
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
    /// no declaring class, no hook question and no label; what `nvs_types`
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
        // `&value` never arrives here, at any depth: `nvs_types` refuses it as
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
    /// `ExprInfo::Index` entry at all, and `nvs_types` refuses one as
    /// `E0482` where it is written, so the panic here is an invariant
    /// check rather than a gap.
    ///
    /// **A [`Ty::Tagged`] base is the one that declares nothing and is not
    /// refused**, because it is ADR 0007 § 2's unchecked position: a `mixed`
    /// defers whether there is an array here at all, which is ADR 0036 § 4's
    /// deferral one storage kind along from a member access, so the read goes
    /// to [`Helper::ValueIndexGet`] (or [`Helper::ValueIndexOptionalGet`]
    /// under a `??`) and the tag answers. The choice is made off the base's
    /// *representation* rather than off the recorded entry, exactly as
    /// [`Self::lower_instanceof`] reads its own subject's.
    ///
    /// `base[]` (`index`
    /// is `None`) has no meaning as a read at all — it is PHP's
    /// append syntax, assignment-target-only — and `nvs_types`
    /// refuses it as `E0481` where it is written, so the arm here is
    /// an invariant check that no source file can reach.
    ///
    /// A read the checker marked **coalesce-guarded** — any level of the
    /// subscript chain under a `??`, so a guarded read's own base may be
    /// another one and may therefore be `null` at run time, which
    /// `nvs_array_optional_get` answers with `null` again — takes
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
                "nvs-ir reached `$a[]` as a read expression — append syntax (`index` \
                 is `None`) is assignment-target-only and `nvs_types` refuses every \
                 other position as `E0481`, so this body was not checked"
            );
        };
        let Some(ExprInfo::Index { elem_ty, guarded }) = self.exprs.lookup(expr.span) else {
            panic!(
                "nvs-ir: an array-index read at {:?} has no resolved element type \
                 recorded in the typed-expression table — `nvs_types` refuses a base \
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
        // A tagged base has no declared element type to answer with — the
        // element is whatever the array turns out to hold — so it takes the
        // representation the guarded read already takes, and
        // `Lowering::coerce` absorbs it into a declared type by the rows it
        // absorbs integer `/`'s union with.
        let result_ty = if base_ty == Ty::Tagged {
            Ty::Tagged
        } else {
            match absent {
                AbsentKey::Throws => lower_checked_ty(*elem_ty, self.checked_types),
                AbsentKey::Null => Ty::Tagged,
            }
        };
        // ADR 0036 § 4's deferral, one storage kind along from a member
        // access: a base whose representation is a tag has not yet answered
        // *whether there is an array here*, so the read goes to the helper
        // pair that asks the tag rather than to the instruction, whose base
        // is an `array<T>` by declaration. `nvs_types` records the same
        // `ExprInfo::Index` either way — it is the base's own representation
        // that picks here, exactly as it does for `instanceof`'s subject.
        let kind = if base_ty == Ty::Tagged {
            let helper = if *guarded {
                Helper::ValueIndexOptionalGet
            } else {
                Helper::ValueIndexGet
            };
            InstKind::HelperCall {
                helper,
                args: vec![array_v, key_v],
            }
        } else {
            InstKind::ArrayGet {
                array: array_v,
                key: key_v,
                absent,
            }
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
                "nvs-ir: an `instanceof` at {:?} has no resolved class recorded in the \
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
            "nvs-ir lowers `instanceof` only against a subject that can hold an object — \
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
            "nvs-ir lowers `clone` only for an object — got representation {ty:?}. ADR \
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
}

/// What [`Lowering::open_nullsafe`] is allowed to assume about the tag of a
/// [`Ty::Tagged`] receiver — the one question a member access asks that
/// nothing else in this file does.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum ReceiverProof {
    /// `nvs_types` proved the tag before this ever ran: a narrowed `?T`, the
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
