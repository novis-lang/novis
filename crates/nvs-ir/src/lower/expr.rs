//! Expression lowering — [`Lowering::lower_expr`]'s dispatch itself, and the
//! shapes with nowhere more specific to be: member access, the nullsafe chain,
//! `match`, the literals and the array forms.
//!
//! Two neighbouring areas are modules of their own, reached the way `lower_expr`
//! reaches any other: `rule:types/conversion`'s conversions and `rule:expressions/truthy-positions`'s truthiness are
//! [`super::convert`], and § 4's operator table is [`super::operator`].
//!
//! Part of [`super`]'s one `impl Lowering`, split across this directory so a
//! session editing one area does not carry the rest in context. The methods
//! are `pub(crate)` so they reach across these modules and no further.

use super::convert::{LiteralAtom, literal_constant};
use super::*;

/// What `nvs_types::expr_table::ExprInfo::ShapeProperty` resolved for one
/// `$shape->field` access, read or write, carried as one argument because its
/// parts are only ever used together — see
/// [`Lowering::lower_shape_property_access`] and
/// [`Lowering::lower_shape_property_assign`].
pub(crate) struct ShapeField {
    /// The field's own name, `$`-sigil not included: what `rule:types/erased-member-access`'s
    /// fetch is keyed on.
    pub(crate) name: String,
    /// Its position in the *receiver's* sorted shape — the runtime's hint,
    /// and not the answer through a widened view.
    pub(crate) slot: u32,
    /// Its declared type, still in `nvs_types`' interner.
    pub(crate) ty: TypeId,
}

impl<'a> Lowering<'a> {
    /// The one entry point for lowering an expression, in every position.
    ///
    /// `cur` is redirected to whichever block the expression's own evaluation
    /// ends in -- unchanged unless the expression branched. That is why it is
    /// a `&mut`: `&&`, `||`, `!`, a ternary and `??` each lower to a
    /// branch/merge, and a caller that could not learn the merge block would
    /// go on emitting into a block control has already left. One entry point
    /// rather than a "top-level only" one beside it is what makes every
    /// position compose: `echo "x=" . ($a ?? "d")` lowers by exactly the route
    /// `string $s = $a ?? "d";` does.
    ///
    /// `expected` is the representation the position wants where it has one.
    /// It steers a literal (`rule:types/arithmetic`'s `int`/`uint` choice) and nothing
    /// else -- reconciling a mismatch is [`Self::coerce`]'s job, at the
    /// boundary that owns the declared type.
    pub(crate) fn lower_expr(
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
            // The shapes that branch. They sit here, in the one
            // expression-lowering entry point, rather than in a second
            // "top-level only" one — a split that leaves `echo "x=" . ($a ??
            // "d")` with no route while `string $s = $a ?? "d";` has one.
            // `cur` is redirected to whichever block the expression's own
            // control flow ends in, so every caller composes with them for
            // free.
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
            // `rule:expressions/catch-lowers-to-block-form`: the expression `catch` lowers to the block form —
            // one protected region over the guarded expression, a landing pad
            // dispatching the arms, and a phi joining the values every side
            // produces. `Self::lower_catch` owns it.
            ExprKind::Catch { guarded, arms } => {
                self.lower_catch(guarded, arms, expected, env, cur)
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
            // The same parts under a different rule: `rule:core-classes/html-literal`
            // trusts every segment and escapes every hole, and what it denotes
            // is a carrier rather than a `string`. It reaches here whatever its
            // body holds, a hole-free one included — `nvs_syntax`'s parser
            // deliberately skips `collapse_string_parts` for this node, since
            // the node and not the part count is what says
            // `Core\Html\Markup`.
            ExprKind::Markup(parts) => self.lower_markup_literal(parts, env, cur),
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
            // `!` always produces `Ty::Bool` via `rule:expressions/truthy-positions`'s truthy table,
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
            // `rule:classes/ordering-lowers-to-compare-to`: ordering two objects is a `Comparable::compareTo`
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
            // `rule:attributes/retrieval-folds-while-checking`'s retrieval is resolved in `nvs check` and recorded
            // here as an ordinary compile-time constant, so what is lowered is
            // the answer rather than the call — `nvs_stdlib::attributes`
            // registers two symbols whose body aborts precisely so a call that
            // slipped past this is loud rather than plausible.
            ExprKind::StaticCall { class, args, .. } => {
                if let Some(ExprInfo::CoreConst { value }) = self.exprs.lookup(expr.span) {
                    let value = value.clone();
                    return self.emit_const_arg(&value, env, *cur);
                }
                // `rule:programs/implementing`'s enumeration, answered in `nvs check` and
                // recorded as the list of classes rather than as a constant,
                // because what it expands to allocates. `nvs_stdlib::program`
                // registers a body that aborts precisely so a call that
                // slipped past this is loud rather than plausible.
                if let Some(ExprInfo::ProgramInstances { classes, ctors }) =
                    self.exprs.lookup(expr.span)
                {
                    let classes: Vec<String> = classes.iter().map(ToString::to_string).collect();
                    let ctors = ctors.clone();
                    return self.lower_program_instances(&classes, &ctors, env, cur);
                }
                // `rule:routing/link-name-and-params-are-checked`'s link, resolved in `nvs check` against § 5's
                // finished table and recorded as the named route's path,
                // already split by § 2's grammar. The call still happens — a
                // link percent-encodes runtime values and is not a constant —
                // so what changes is which implementation answers and what
                // argument 0 is. See `nvs_stdlib::router::link`.
                if let Some(ExprInfo::RouteLink {
                    pieces,
                    absolute,
                    signed,
                }) = self.exprs.lookup(expr.span)
                {
                    let prepared = nvs_types::UrlPiece::prepared(pieces);
                    let absolute = *absolute;
                    let signed = signed.clone();
                    return self.lower_route_link(&prepared, absolute, signed, args, env, cur);
                }
                self.lower_static_call(class, args, expr, env, cur)
            }
            ExprKind::PropertyAccess {
                object,
                nullsafe,
                property,
            } => self.lower_property_access(object, property, *nullsafe, expr, env, cur),
            ExprKind::ArrayLiteral(items) => self.lower_array_literal(items, env, cur),
            ExprKind::ObjectLiteral(fields) => self.lower_object_literal(fields, env, cur),
            ExprKind::Index { base, index } => {
                self.lower_index(base, index.as_deref(), expr, env, cur)
            }
            // `rule:types/type-test`'s `$x is T` — the question `instanceof`
            // one arm below answers only for a class, against a written *type*
            // rather than a class value. See `Self::lower_type_test`.
            ExprKind::TypeTest { expr: inner, .. } => {
                self.lower_type_test(inner, expr, env, cur)
            }
            ExprKind::InstanceOf { expr: inner, class } => {
                self.lower_instanceof(inner, class, expr, env, cur)
            }
            ExprKind::Clone(inner) => self.lower_clone_expr(inner, env, cur),
            // Everything that wears this syntax is an inlined constant.
            // `rule:enums/no-class-machinery` makes `EnumName::CaseName` "an integer constant,
            // inlined at every use site"; `rule:classes/no-free-functions-or-constants`'s `Core\Math::PI` is the
            // same rule for a class constant. So each lowers to exactly the
            // constant a literal would, with no storage, no descriptor and no
            // allocation. `nvs_types` resolved the value — the enum's
            // auto-increment rule for one, `nvs_stdlib::registry`'s own row
            // for the other, `nvs_types::signatures::ConstSig` for a
            // user-declared class's — into
            // `ExprInfo::EnumCase`/`ExprInfo::CoreConst`.
            ExprKind::ClassConstAccess { .. } => match self.exprs.lookup(expr.span) {
                Some(ExprInfo::EnumCase { value, .. }) => match value {
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
                // Unreachable: a read of a constant whose declaration folds to
                // no value is `E0792` in `nvs_types::expr::members`, refused
                // at the span the author can act on, and a unit that failed to
                // check never reaches lowering. Kept as a panic
                // rather than deleted because the arm is what makes that
                // refusal load-bearing — if it is ever removed, this is where
                // the missing value surfaces.
                _ => panic!(
                    "nvs-ir: a `Class::CONST` at {:?} with no value recorded in the \
                     typed-expression table — `nvs_types` refuses that read with \
                     `E0792` before lowering, so this is a checker that did not run \
                     or a value it recorded under a different span",
                    expr.span
                ),
            },
            // `Foo::class` — the class's own fully qualified name, and a
            // `Ty::Str` constant with no storage behind it, exactly like the
            // constants one arm above. The *name* is resolved by
            // `nvs_types::expr::members::check_class_name_const` and travels
            // in the same `ExprInfo::CoreConst` a `Core` class constant does,
            // for two reasons that point the same way: a constant is inlined
            // at its use site whichever spelling it wears, and this crate
            // cannot name a `nvs_hir::QName` to do the resolution itself.
            //
            // `static::class` and `$obj::class` are the two sides that have no
            // name until the call runs; they take the `ClassNameOf` arm below
            // and read one off a descriptor the frame already holds.
            ExprKind::ClassNameConst { class } => match self.exprs.lookup(expr.span) {
                Some(ExprInfo::CoreConst { value }) => {
                    let value = value.clone();
                    self.emit_const_arg(&value, env, *cur)
                }
                // `static::class` and `$obj::class` — the two sides that name
                // a class only the run time knows. The checker recorded
                // `ClassNameOf` for exactly these (see its own doc comment for
                // why one entry covers both), and which descriptor to read is
                // read back off the class side's own shape here rather than
                // carried in the entry: `nvs-ir` already has both instructions
                // and neither needs anything `nvs_types` resolved.
                Some(ExprInfo::ClassNameOf) => {
                    let desc = if matches!(class.kind, ExprKind::StaticExpr) {
                        // Late static binding's own class — parameter 0 in a
                        // `static` method, `$this`'s descriptor in an instance
                        // one. `Self::lsb` owns that split and caches the
                        // value, so a method reading `static::` twice loads
                        // once.
                        self.lsb()
                    } else {
                        // One load at `nvs_runtime::OBJ_CLASS_OFFSET`, which
                        // answers the class the receiver *is* rather than the
                        // one its variable was declared as — the reason this
                        // is not folded even where the declared type resolved.
                        // The receiver needs no lifecycle of its own: it is
                        // read through and not kept, exactly as
                        // [`Self::lower_instanceof`]'s subject is. Its
                        // representation is a [`Ty::Object`] by construction —
                        // the checker records `ClassNameOf` for no other — so
                        // this asserts nothing, the way every other
                        // [`InstKind::ClassDescOf`] emitter does not.
                        let (object, _) = self.lower_expr(class, None, env, cur);
                        let (desc, _) =
                            self.emit(*cur, Ty::ClassDesc, InstKind::ClassDescOf { object });
                        desc
                    };
                    self.emit_fallible(
                        *cur,
                        Ty::Str,
                        InstKind::HelperCall {
                            helper: Helper::ClassDescName,
                            args: vec![desc],
                        },
                        env,
                    )
                }
                // Only reachable in a compilation that has already aborted:
                // the checker records one of the two entries above for every
                // `::class` it accepts and reports `E0702` for every side it
                // does not. The empty string keeps that a fact about the
                // checker rather than a claim in a panic message.
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
            // `rule:types/arithmetic`'s `± 1` *as a value*. Both spellings run the same
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
            // `rule:types/conversion`'s assignment *as a value* — `int $b = ($a = 2);`,
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
            // `rule:classes/unset-is-refused-on-a-property`'s `isset($x)` is `$x != null`, and a list of them
            // is the conjunction — see `Self::lower_isset`.
            ExprKind::Isset(operands) => (self.lower_isset(operands, env, cur), Ty::Bool),
            // `empty($x)` is `!$x` — `rule:expressions/truthy-table`'s truthy table negated — so
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
            // there is no resolved target to name, whether or not the callee's
            // type named its parameters. Which of the two closure-call helpers
            // it is — and so whether the runtime still checks a tag per
            // argument — is the checker's record on this expression's own span,
            // read in `Self::lower_closure_call`.
            ExprKind::Call { callee, args } => {
                self.lower_closure_call(expr, callee, args, env, cur)
            }
            // `rule:types/closure-self-name`'s self-name — `fact` inside
            // `fn fact(int $n): int => … fact($n - 1)`. The closure it names is
            // the frame's own receiver, which `closure::lower_closure` bound
            // under `FN_SELF` at entry, so this is a lookup and never a load:
            // § 3's name is not a slot and the environment class holds no field
            // for it. Every *other* bare name is `E0319` — see the roster
            // below — which is why the guard is the checker's own record and
            // not the syntax.
            ExprKind::ConstFetch(_)
                if matches!(self.exprs.lookup(expr.span), Some(ExprInfo::ClosureSelf)) =>
            {
                *env.get(closure::FN_SELF).unwrap_or_else(|| {
                    panic!(
                        "nvs-ir: `ExprInfo::ClosureSelf` outside a closure body — \
                         nvs_types::expr::calls::check_fn_literal binds `rule:types/closure-self-name`'s \
                         self-name for one body, whose invoke binds `FN_SELF` at entry"
                    )
                })
            }
            // `rule:statements/a-require-expression-is-mixed`'s **value** form — `$c = require 'config.nvs';`.
            // The same call to the target's own script frame the statement
            // form emits (`Self::lower_expr_stmt`, which owns the reasoning
            // about the frame and about running every time the site is
            // reached); the only difference is which end of it is kept. That
            // frame's return type is `Ty::Tagged` by construction, so the
            // value arrives here needing no conversion, and it is a fresh
            // producer — nothing else holds a reference to what the callee
            // returned — so the consumer owns it and `Self::aliasing_read`
            // answers `false` for this shape, as it does for a call.
            //
            // A path with no recorded target is one that is not a literal:
            // the statement form runs nothing there, so the value form has
            // nothing that ran to answer with, and it takes the same tagged
            // `1` § 3 gives a file that did not `return`.
            ExprKind::Require { path } => {
                if let Some(target) = self.exprs.require_target(path.span) {
                    self.emit_fallible(
                        *cur,
                        Ty::Tagged,
                        InstKind::Call {
                            target: crate::lower::file_script_label(target),
                            receiver: None,
                            args: Vec::new(),
                        },
                        env,
                    )
                } else {
                    let (one, _) = self.emit(*cur, Ty::Int, InstKind::ConstInt(1));
                    (self.coerce(*cur, one, Ty::Int, Ty::Tagged, env), Ty::Tagged)
                }
            }
            ExprKind::SpawnScript { path, options } => {
                self.lower_spawn_script(path, options, env, cur)
            }
            ExprKind::Await(operand) => self.lower_await(operand, env, cur),
            // Nothing the checker accepts reaches this arm, and the proof is
            // the roster rather than the message below it. The arms above
            // cover every `ExprKind` variant that has a lowering, plus one of
            // `Assign`'s `inout` shapes. Of the variants with no arm, and that
            // other `Assign` shape:
            //
            // * `Error` is a parse error already reported, and does not
            //   survive to a compilation that lowers.
            // * a bare `NAME` (`ConstFetch`) is `E0319` — `rule:statements/storage-that-outlives-a-call` gives
            //   a constant no home but a class — and `self`/`static`/`parent`
            //   used as a *value* are `E0321`, both from `nvs_hir::members`.
            //   Each still appears as the class *side* of a `::`, which is
            //   not this dispatch's business: `walk_class_side` skips them and
            //   the arms above read the checker's own resolution instead.
            // * `$a = &$b` is `E0701`: `rule:types/implicit-capture` leaves by-reference
            //   capture out of the language, so there is no owner for the `&`.
            // * every `yield` shape is `E0448` where it has no lowering — a
            //   key half, a `yield from`, a missing value, and one used as a
            //   *value*, `rule:iteration/one-way-only` giving a generator
            //   no `send()` for it to answer with. The statement form goes
            //   through `Self::lower_yield` one file over, reached from
            //   `nvs_types::expr::check_expr_stmt`'s matching split.
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
    /// records that `rule:core-classes/html-auto-escape`'s auto-escaping sink is the HTTP *response*
    /// write, not this one, and that whether `echo` under a future
    /// `nvs serve` becomes that sink is an M7 decision this does not
    /// pre-empt.
    ///
    /// A scalar or [`Ty::Str`] operand is converted by
    /// [`Self::convert_operand`] — the same shared path `.` concatenation
    /// uses, so it goes through its own [`Helper`] conversion — and handed to
    /// one [`Helper::EchoStr`] [`InstKind::HelperCall`]. A [`Ty::Object`] or
    /// [`Ty::Tagged`] operand is handed **unconverted** to
    /// [`Helper::EchoValue`] instead, because `rule:tooling/terminal-output-is-a-sink`'s one raw path is
    /// the *type* `Core\Cli\Text` and a conversion in front of the sink would
    /// have thrown that away; that helper's own doc comment owns the rule and
    /// [`Self::convert_operand`]'s rows still describe what it renders. Either
    /// way the write itself is [`Self::emit_write`]'s.
    ///
    /// A ``html`…` `` operand takes neither path and becomes no value at all:
    /// `rule:core-classes/html-literal`'s sink position is a write per piece,
    /// which is [`Self::echo_markup_parts`].
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
    pub(crate) fn lower_echo(&mut self, operands: &[Expr], cur: &mut BlockId, env: &mut Env) {
        for operand in operands {
            // A markup literal written straight at a sink is the one operand
            // that never becomes a value at all — see
            // [`Self::echo_markup_parts`].
            if let ExprKind::Markup(parts) = &operand.kind {
                self.echo_markup_parts(parts, env, cur);
                continue;
            }
            let mark = self.temporaries_mark();
            // The sink substitutes everything except its own carrier, and a
            // carrier is a *class* — `rule:tooling/terminal-output-is-a-sink`'s one raw path is the type
            // `Core\Cli\Text`, so it can only be recognised while the operand
            // still has a type. `Ty::Object` and `Ty::Tagged` are the two it
            // can arrive under, and both go to `Helper::EchoValue` with no
            // conversion in front: that helper renders through the same
            // `stringify` `Self::convert_operand` would have called, after
            // asking the class. Everything else is converted here and takes
            // `Helper::EchoStr`, which no carrier can reach.
            let (v, ty) = self.lower_expr(operand, None, env, cur);
            let (v, aliasing, helper) = match ty {
                Ty::Object | Ty::Tagged => (v, self.aliasing_read(operand), Helper::EchoValue),
                scalar => {
                    let (sv, aliasing) = self.convert_operand(operand, v, scalar, env, cur);
                    (sv, aliasing, Helper::EchoStr)
                }
            };
            if !aliasing {
                self.own_temporary(v);
            }
            self.emit_write(helper, v, env, *cur);
            self.release_temporaries_since(mark, *cur);
        }
    }
    /// One operand's write to the request's output — the tail every spelling
    /// of `echo` ends in.
    ///
    /// The one conversion-free helper that can genuinely *throw*, so it
    /// carries the failure edge [`Self::landing_block`] hands out; the
    /// scalar-to-string conversions in front of it carry one too, for the
    /// reason every status-returning instruction does (see [`Inst::on_error`]).
    /// The call defines no value, which is why it is pushed rather than
    /// emitted through [`Self::emit`].
    fn emit_write(&mut self, helper: Helper, v: ValueId, env: &mut Env, cur: BlockId) {
        let landing = self.landing_block(env);
        self.block_insts[cur.index() as usize].push(Inst {
            result: None,
            ty: None,
            kind: InstKind::HelperCall {
                helper,
                args: vec![v],
            },
            on_error: Some(landing),
        });
    }
    /// `echo html`…`;` — `rule:core-classes/html-literal`'s **sink** position,
    /// which is a run of writes with no carrier built at all.
    ///
    /// Each piece is written as it is produced: a segment's bytes, then a
    /// hole's escaped ones, then the next segment's. Nothing joins them and
    /// nothing wraps the join, because a carrier born and consumed at one sink
    /// is unobservable — the same erasure `rule:security/tainted-qualifier`
    /// performs for the qualifier, and what makes the most-written line in a
    /// web program cost no allocation at all.
    ///
    /// **The write span is the piece, not the statement**, which is the one
    /// thing a reader can tell apart from the value-position lowering: a
    /// bidirectional isolate opened in a segment and closed after a hole is two
    /// unterminated controls to `rule:security/bidi-predicate`, where the joined
    /// bytes would have been one balanced pair. Splitting a write can only ever
    /// neutralize *more* — a control unterminated in the whole text is
    /// unterminated in its own piece too — so this is the safe direction of the
    /// difference, and it is the granularity `echo $a, $b;` already has.
    fn echo_markup_parts(&mut self, parts: &[StringPart], env: &mut Env, cur: &mut BlockId) {
        for part in parts {
            let mark = self.temporaries_mark();
            let piece = match part {
                StringPart::Text(span) => {
                    let s = nvs_types::string_lit::cook_markup_text(self.src, *span).0;
                    self.emit(*cur, Ty::Str, InstKind::ConstStr(s)).0
                }
                StringPart::Expr(e) => self.lower_markup_hole(e, env, cur),
            };
            // Every piece is fresh and has exactly one use, the write below.
            self.own_temporary(piece);
            self.emit_write(Helper::EchoStr, piece, env, *cur);
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
    pub(crate) fn lower_inline_html(&mut self, span: Span, cur: &mut BlockId, env: &mut Env) {
        let text = span_text(self.src, span).to_owned();
        let mark = self.temporaries_mark();
        let (v, _) = self.emit(*cur, Ty::Str, InstKind::ConstStr(text));
        self.own_temporary(v);
        self.emit_write(Helper::EchoStr, v, env, *cur);
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
    pub(crate) fn lower_print(
        &mut self,
        operand: &Expr,
        env: &mut Env,
        cur: &mut BlockId,
    ) -> (ValueId, Ty) {
        self.lower_echo(std::slice::from_ref(operand), cur, env);
        let (one, _) = self.emit(*cur, Ty::Int, InstKind::ConstInt(1));
        (one, Ty::Int)
    }
    /// `exit;` and `exit(...)` — `rule:errors/propagation`'s status vocabulary, plus one.
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
    pub(crate) fn lower_exit(
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
    pub(crate) fn concat_operand(
        &mut self,
        expr: &Expr,
        env: &mut Env,
        cur: &mut BlockId,
    ) -> (ValueId, bool) {
        let (v, ty) = self.lower_expr(expr, None, env, cur);
        self.convert_operand(expr, v, ty, env, cur)
    }
    /// [`Self::concat_operand`]'s conversion half, over an operand the caller
    /// has already lowered.
    ///
    /// Split out for [`Self::lower_echo`], which has to see the operand's
    /// [`Ty`] *before* deciding whether to convert it at all: `rule:tooling/terminal-output-is-a-sink`'s
    /// raw path is a class, so the sink recognises it from the static type and
    /// then does its own rendering. `.` concatenation reaches the rows below
    /// through `concat_operand` instead.
    pub(crate) fn convert_operand(
        &mut self,
        expr: &Expr,
        v: ValueId,
        ty: Ty,
        env: &mut Env,
        cur: &mut BlockId,
    ) -> (ValueId, bool) {
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
                let (sv, _) = self.emit_fallible(
                    *cur,
                    Ty::Str,
                    InstKind::HelperCall {
                        helper,
                        args: vec![v],
                    },
                    env,
                );
                (sv, false)
            }
            // `rule:types/conversion`'s row for `null`, which PHP answers with the empty
            // string and which `Helper::TaggedToString` already answers that
            // way for the `?string` holding one. A *statically* `null`
            // operand is the same value one type earlier, so it renders the
            // same rather than being refused a phase up — the checker's
            // `require_stringable` names the types that are refused, and
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
            // `rule:classes/stringable`'s implicit stringification. The call's own result
            // is a fresh `string` with one owner, so it is never an aliasing
            // read — the same answer every scalar row above gives.
            Ty::Object => match self.lower_to_string_call(expr, v, env, *cur) {
                Some(s) => (s, false),
                // No resolved `toString`: an erased `object` (`rule:types/erased-member-access`), or
                // a `Core`-owned class, which is where `rule:security/capture-answers-the-carrier`'s sink
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
            // [`crate::ty::Ty`]'s whole roster is accounted for and this arm
            // has no reachable target left. The rows above take `Ty::Str`,
            // each scalar through its own helper, `Ty::Null` as the empty
            // string, `Ty::Tagged` through the runtime tag and `Ty::Object`
            // through `rule:classes/stringable`'s `toString`.
            //
            // The rest a source expression can have are refused a phase up by
            // `nvs_types::expr::operators::require_stringable`, the one check
            // every implicit site goes through, each naming the spelling that
            // says what was meant: `Ty::Bytes` (`rule:types/conversion` grants
            // `as string` and nothing implicit), `Ty::Array` (PHP prints
            // `"Array"` and a notice; Novis names `Core\Json::encode`),
            // `Ty::Enum` (`rule:enums/no-class-machinery`'s named integer, `$case as int`) and
            // `Ty::Void` (a call with no value at all).
            //
            // What is left is no type a *source expression* ever has.
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

    /// `rule:classes/stringable`'s implicit `toString()`, for an operand that lowered to a
    /// [`Ty::Object`]. `.`, an interpolated piece, `echo`/`print` and
    /// `as string` all reach it, because
    /// `nvs_types::expr::operators::require_stringable` is the single check
    /// every one of them goes through — so it is also the single place that
    /// records the resolved target, under the operand's own span.
    ///
    /// `None` when nothing was recorded there, which both callers answer the
    /// same way: `Helper::TaggedToString` over the receiver, which dispatches
    /// `toString` on its *runtime* class. The checker records a target wherever
    /// the operand's static type names a class to resolve against, so a missing
    /// one means it named none — an erased `object` (`rule:types/erased-member-access`) or a union
    /// — or that it named `rule:security/capture-answers-the-carrier`'s sink carrier, the one rendering class
    /// with no `toString` member at all, whose bytes `nvs_runtime::stringify`
    /// hands back as they are.
    ///
    /// The *user-declared* call is ordinary in every respect, exactly as
    /// [`Self::lower_object_comparison`]'s `compareTo` is: `rule:errors/propagation`'s error
    /// edge, since a `toString` body may throw like any other, and the same
    /// ownership convention [`Self::lower_call_args`] applies to a receiver —
    /// an aliasing operand is retained here because the callee releases every
    /// refcounted parameter at scope exit, and a fresh one (`echo new Name()`)
    /// transfers the reference it already has. It dispatches on the
    /// receiver's runtime class, so a `toString` overridden in a subclass wins
    /// over the one the static type names.
    pub(crate) fn lower_to_string_call(
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
    /// `mixed`, `rule:types/conversion`'s one unchecked position — so no `Untag` is
    /// emitted for it and the *tagged* value is handed back for
    /// [`InstKind::SlotGet`] to check at run time. Ownership is unchanged
    /// either way: a tagged value is refcounted ([`Ty::is_refcounted`]) and
    /// its release is the same release, tag-dispatched.
    pub(crate) fn open_nullsafe(
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
    pub(crate) fn untag_receiver(&mut self, v: ValueId, ty: Ty, cur: BlockId) -> (ValueId, Ty) {
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
    /// site to forget. [`Self::untag_receiver`] is the same move written for a
    /// member access's own receiver, and is a no-op once this has run.
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
    pub(crate) fn untag_narrowed(
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
    pub(crate) fn close_nullsafe(
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
    pub(crate) fn lower_coalesce(
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
        // `rule:types/class-reference`'s `?class<T>` is the exception to the paragraph below:
        // it is not tagged and it can still be `null`, so it takes the same
        // branch/merge shape the tagged path takes, with the null test written
        // on the word ([`Ty::ClassDesc`] owns why) and no untagging, retain or
        // release — a descriptor is immortal and outside the refcount
        // discipline. A non-nullable `class<T>` left operand takes this path
        // too and is simply never zero.
        if lhs_ty == Ty::ClassDesc {
            let word = self.class_desc_word(lhs_v, lhs_ty, *cur);
            let (zero, _) = self.emit(*cur, Ty::Int, InstKind::ConstInt(0));
            let (is_null, _) = self.emit(
                *cur,
                Ty::Bool,
                InstKind::BinOp {
                    op: BinOp::Eq,
                    lhs: word,
                    rhs: zero,
                },
            );
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
            let value_v = self.coerce(value_block, lhs_v, lhs_ty, result_repr, env);
            self.seal(value_block, Terminator::Jump(merge_block));

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
            return (value, result_repr);
        }
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
    /// `isset($a, $b, …)` — `rule:classes/unset-is-refused-on-a-property`: each operand is `!= null`, and the
    /// list is their conjunction, evaluated left to right and **short-circuit**
    /// exactly as PHP's is. That is observable rather than an optimisation:
    /// `isset($a, $b[$i++])` leaves `$i` alone when `$a` is `null`, so the
    /// tail is lowered on one edge only, in the same branch/merge shape
    /// [`Self::lower_and`] uses, and the recursion is over the tail rather
    /// than a fold so a three-operand `isset` short-circuits at either point.
    ///
    /// No operand can *throw* on absence: `nvs_types::expr::presence` marks
    /// every subscript level under an `isset` in `Env::coalesce_guarded`, the
    /// same set `??` fills, so `rule:php-migration/every-divergence-is-deliberate-and-listed` row 11's throw is off for exactly
    /// the reads this construct exists to ask about.
    pub(crate) fn lower_isset(
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
    /// declaration (`rule:classes/definite-property-initialization` makes a declared property definitely initialised,
    /// `rule:types/declaration` a local), `false` for the literal `null`'s own
    /// [`Ty::Null`]. That is the same short-circuit on representation
    /// [`Self::lower_coalesce`] takes, and it is why `isset` costs nothing at
    /// all on a non-nullable operand.
    fn lower_isset_operand(&mut self, operand: &Expr, env: &mut Env, cur: &mut BlockId) -> ValueId {
        let (v, ty) = self.lower_expr(operand, None, env, cur);
        let present = match ty {
            Ty::Null => self.emit(*cur, Ty::Bool, InstKind::ConstBool(false)).0,
            // `rule:types/class-reference`'s `?class<T>` — the one non-tagged representation
            // the paragraph above is not the whole rule for. See
            // [`Ty::ClassDesc`]; the test is the word against zero, and a
            // non-nullable `class<T>` takes it too and always answers `true`.
            Ty::ClassDesc => {
                let word = self.class_desc_word(v, ty, *cur);
                let (zero, _) = self.emit(*cur, Ty::Int, InstKind::ConstInt(0));
                self.emit(
                    *cur,
                    Ty::Bool,
                    InstKind::BinOp {
                        op: BinOp::NotEq,
                        lhs: word,
                        rhs: zero,
                    },
                )
                .0
            }
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
    pub(crate) fn lower_ternary(
        &mut self,
        cond: &Expr,
        then: Option<&Expr>,
        else_: &Expr,
        env: &mut Env,
        cur: &mut BlockId,
    ) -> (ValueId, Ty) {
        let (cond_v, cond_ty) = self.lower_expr(cond, None, env, cur);
        let cond_is_alias = self.aliasing_read(cond);
        let truthy_v = self.truthy_convert(cond_v, cond_ty, *cur, env);
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
    /// `rule:types/arithmetic`'s promotion rows belong to an *operator*, whose result
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
    /// An empty set has no value and so no representation; every caller
    /// refuses that shape before it reaches here — a ternary always has two
    /// branches, an arm-less `match` is refused in [`Self::lower_match`], and
    /// a `catch` expression's guard is a branch of its own whatever its arms
    /// do ([`Self::lower_catch`]).
    pub(crate) fn join_representations(
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
    ///   not have, and the carrier `Self::throw_source` hands the raise already
    ///   puts the file and line on the exception.
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
    /// Every comparison in the chain is [`Self::emit_equality`], the one
    /// equality lowering a written `==` over the same pair takes
    /// (`rule:expressions/switch-match-equality`). So a subject whose static
    /// type names no representation of its own — a `mixed`, a union — compares
    /// through [`Helper::Identical`], where the tags decide it at run time,
    /// and a label at a representation the subject does not share takes that
    /// pair's own row. The label is unchanged by either, being written at its
    /// own representation whatever the subject's is.
    ///
    /// An **enum** subject goes the other way and is compared one
    /// representation *down*, on the integer `rule:enums/no-class-machinery` makes its cases:
    /// [`Self::reinterpret_enum_to_backing`] relabels the subject once above
    /// the chain and each label as it is lowered, exactly as
    /// [`Self::lower_binary`] relabels a written `==` between two cases and
    /// `Self::lower_literal_membership` `rule:types/literal-types`'s chain — `nvs-codegen`'s
    /// `BinOp` table is `Ty::Int`/`Ty::Uint`/`Ty::Bool` and carries no
    /// `Ty::Enum` row at all. The relabelling is free (no machine instruction)
    /// and feeds the comparisons alone: the subject's own value is what the
    /// arm-entry release reads, and a label still lowers at the subject's
    /// *declared* representation so that `Mode::Read` resolves as the case it
    /// names.
    ///
    /// # Panics
    ///
    /// Panics — as an engine invariant, not a refusal — for a `match` with no
    /// arms at all, which `nvs_types` refuses as `E0476` before this crate
    /// ever sees it. A pair no single value inhabits is
    /// [`Self::emit_equality`]'s `E0466` guard instead, the checker having
    /// refused it at the arm that wrote it.
    pub(crate) fn lower_match(
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
        // It is in flight for the whole label chain, and every comparison in
        // that chain can throw — a label is an arbitrary expression, and a
        // tagged pair goes through the fallible `Helper::Identical`. So it
        // rides `Self::owned_temporaries` until the chain is behind us, which
        // is what makes a throw out of a label drop it. The releases below are
        // *not* replaced by that: the subject has one exit per arm plus the
        // no-arm throw, and the stack releases at one point, so the staging
        // ends with a `forget` and each exit keeps releasing by hand.
        let subject_mark = self.temporaries_mark();
        if owed {
            self.own_temporary(subj_v);
        }
        // An enum subject is compared one representation down, on the integer
        // its cases *are* — the free `Reinterpret` of `rule:types/conversion` row 1,
        // which `Self::lower_binary` already makes for a written `==` between
        // two cases and `Self::lower_literal_membership` for `rule:types/literal-types`'s
        // chain, `nvs-codegen`'s `BinOp` table carrying no `Ty::Enum` row.
        // It is made once, above the chain, and feeds the comparisons alone:
        // `subj_v` stays the value the release below reads, and each label
        // still lowers at the subject's *declared* representation so that
        // `Mode::Read` resolves as the case it names.
        let label_expect = subj_ty;
        let (cmp_subj_v, cmp_subj_ty) = self.reinterpret_enum_to_backing(subj_v, subj_ty, cur);

        let arm_blocks: Vec<BlockId> = arms.iter().map(|_| self.new_block()).collect();
        let merge_block = self.new_block();
        let default_index = arms.iter().position(|a| a.conditions.is_none());

        // The env each arm body is entered with is the label chain's as of the
        // label that jumped there — the same bookkeeping `Self::lower_switch`
        // keeps, and needed for the same reason: a label or an arm body may
        // rebind a local (`match ($x) { 1 => $c++, default => 0 }`).
        let pre_env = env.clone();
        let mut entry_edges: Vec<Vec<(BlockId, Env)>> = vec![Vec::new(); arms.len()];
        let mut test_cur = *cur;
        for (i, arm) in arms.iter().enumerate() {
            let Some(conditions) = &arm.conditions else {
                continue;
            };
            for cond in conditions {
                // The label is in flight for its own comparison, which is the
                // one thing that can throw between building it and releasing
                // it — the subject's case exactly, one value along.
                let label_mark = self.temporaries_mark();
                let (cond_v, cond_ty) =
                    self.lower_expr(cond, Some(label_expect), env, &mut test_cur);
                if cond_ty.is_refcounted() && !self.aliasing_read(cond) {
                    self.own_temporary(cond_v);
                }
                // The label takes the subject's own move, for the subject's
                // own reason. `cond_v` is what the release below reads, so
                // only the comparison sees the relabelled pair.
                let (cmp_cond_v, cmp_cond_ty) =
                    self.reinterpret_enum_to_backing(cond_v, cond_ty, &mut test_cur);
                // An arm is the one comparison the language has, written
                // without the operator
                // (`rule:expressions/switch-match-equality`), so it is the
                // lowering a written `==` over the pair takes:
                // `rule:expressions/mixed-equality`'s runtime row where either
                // side's representation is a tag, and the pair's own row where
                // the arm is written at a representation the subject does not
                // share. An arm is written at its own representation whatever
                // the subject's is — a digit run beside a `mixed` is still a
                // `ConstInt` — and needs no widening to get there,
                // `nvs-codegen`'s helper convention storing every argument as a
                // 16-byte tagged `Value` already.
                let eq_v = self.emit_equality(
                    BinaryOp::Eq,
                    (cmp_subj_v, cmp_subj_ty),
                    (cmp_cond_v, cmp_cond_ty),
                    env,
                    &mut test_cur,
                );
                self.forget_temporaries_since(label_mark);
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
        // Past every comparison, so the subject is back to being accounted for
        // by hand: each arm entry and the no-arm throw below release it, and a
        // stack entry surviving into either would release it twice.
        self.forget_temporaries_since(subject_mark);
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
                let source = self.throw_source(test_cur);
                let landing = self.landing_block(env);
                self.seal(
                    test_cur,
                    Terminator::Throw {
                        value: exception,
                        source,
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
    pub(crate) fn lower_interpolated_parts(
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
    /// Lowers `ExprKind::Markup`'s parts into the one `Core\Html\Markup` they
    /// denote — `rule:core-classes/html-literal`'s **value** position, which is
    /// the literal assigned, returned or put in an array rather than written
    /// straight to a sink.
    ///
    /// **A literal with no hole is a constant.** Its bytes are known while it
    /// is being compiled, so it lowers to one [`InstKind::ConstMarkup`] — an
    /// immortal instance in the unit's own data section — and allocates nothing
    /// per execution, which is what that rule's *What it costs to run* promises
    /// of this shape where it promises one object of the next.
    ///
    /// **One carrier, however many pieces**, for a literal that has holes. Each
    /// piece becomes bytes first — a
    /// segment cooked by `nvs_types::string_lit::cook_markup_text`, a hole by
    /// [`Self::lower_markup_hole`] — and the join is the same n-ary
    /// [`InstKind::Concat`] an interpolated string's parts fold to. Only the
    /// join is lifted, through the identical `nvs_types::CORE_HTML_MARKUP` call
    /// [`Self::lower_markup_lift`] emits for `as Core\Html\Markup`. Escaping
    /// each hole into a carrier of its own and composing those with
    /// `Markup + Markup` would answer the same bytes for an object allocation
    /// per hole, where that rule's *What it costs to run* promises one object
    /// for the whole literal.
    ///
    /// **Ownership is [`Self::lower_interpolated_parts`]'.** Every piece is in
    /// flight for as long as the pieces after it are still being lowered, and
    /// one of those can throw, so each lives on [`Self::owned_temporaries`]
    /// until the `Concat` has read it. The lift borrows its own argument and
    /// takes the reference its slot keeps, so the joined bytes are released
    /// here too and the carrier leaves with exactly one owner — the same pair
    /// [`Self::lower_markup_lift`] leaves.
    pub(crate) fn lower_markup_literal(
        &mut self,
        parts: &[StringPart],
        env: &mut Env,
        cur: &mut BlockId,
    ) -> (ValueId, Ty) {
        if parts.iter().all(|part| matches!(part, StringPart::Text(_))) {
            let mut text = String::new();
            for part in parts {
                if let StringPart::Text(span) = part {
                    // The issues are discarded for the reason the pieces below
                    // discard theirs: `nvs_types::check_program` has already
                    // reported them against this same span.
                    text.push_str(&nvs_types::string_lit::cook_markup_text(self.src, *span).0);
                }
            }
            return self.emit(*cur, Ty::Object, InstKind::ConstMarkup(text));
        }
        let mark = self.temporaries_mark();
        let mut pieces: Vec<ValueId> = Vec::with_capacity(parts.len());
        for part in parts {
            let piece = match part {
                StringPart::Text(span) => {
                    // The issues are discarded: `nvs_types::check_program` has
                    // already reported them against this same span.
                    let s = nvs_types::string_lit::cook_markup_text(self.src, *span).0;
                    self.emit(*cur, Ty::Str, InstKind::ConstStr(s)).0
                }
                StringPart::Expr(e) => self.lower_markup_hole(e, env, cur),
            };
            self.own_temporary(piece);
            pieces.push(piece);
        }
        let bytes = match pieces[..] {
            // ``html`` ``, whose body has nothing in it at all — the one shape
            // that reaches here with no parts, a body of plain text arriving as
            // a single `StringPart::Text`.
            [] => {
                let (empty, _) = self.emit(*cur, Ty::Str, InstKind::ConstStr(String::new()));
                self.own_temporary(empty);
                empty
            }
            [only] => only,
            _ => {
                let (joined, _) = self.emit(
                    *cur,
                    Ty::Str,
                    InstKind::Concat {
                        pieces: pieces.clone(),
                    },
                );
                // Every piece is consumed here and each of them is fresh, so
                // this releases exactly what the loop above staged.
                self.release_temporaries_since(mark, *cur);
                self.own_temporary(joined);
                joined
            }
        };
        let carrier = self.emit_fallible(
            *cur,
            Ty::Object,
            InstKind::CoreCall {
                symbol: nvs_types::CORE_HTML_MARKUP,
                args: vec![bytes],
            },
            env,
        );
        self.release_temporaries_since(mark, *cur);
        carrier
    }
    /// Lowers one of a markup literal's holes to the bytes it contributes.
    ///
    /// **A hole already holding a `Core\Html\Markup` is spliced raw and every
    /// other hole is escaped** (`rule:core-classes/html-literal`), which is the
    /// one question here. It cannot be asked of the [`Ty`]s: a carrier and a
    /// `rule:classes/stringable` object both erase to [`Ty::Object`]. So
    /// `nvs_types::expr::literals::infer_markup_literal` records the hole's
    /// checked type at the hole's own span and [`Self::hole_is_carrier`] reads
    /// it back.
    ///
    /// Neither answer is a [`Helper`] row, for [`Self::lower_markup_lift`]'s
    /// reason: what the escape produces is the sink's own transform and what
    /// the raw splice reads is a `Core` class's slot, and both belong to the
    /// crate that owns that layout. A non-carrier hole reaches the escape as
    /// the `string` [`Self::convert_operand`] makes of it, which is the same
    /// row `.`-concatenation gives a scalar, a union or a `toString`.
    ///
    /// The call borrows its argument, so a fresh operand is staged and released
    /// once the call has read it and a borrowed one is left where it is. The
    /// answer is this expression's own fresh `string`, which the literal's join
    /// then owns.
    fn lower_markup_hole(&mut self, e: &Expr, env: &mut Env, cur: &mut BlockId) -> ValueId {
        let mark = self.temporaries_mark();
        let (v, ty) = self.lower_expr(e, None, env, cur);
        let (operand, operand_ty, symbol, aliasing) = if self.hole_is_carrier(e.span) {
            (
                v,
                Ty::Object,
                nvs_types::CORE_HTML_MARKUP_TEXT,
                self.aliasing_read(e),
            )
        } else {
            let (text, aliasing) = self.convert_operand(e, v, ty, env, cur);
            (text, Ty::Str, nvs_types::CORE_HTML_ESCAPE_TEXT, aliasing)
        };
        self.account_for_arg(operand, operand_ty, ArgOwnership::Borrowed, aliasing, *cur);
        let (bytes, _) = self.emit_fallible(
            *cur,
            Ty::Str,
            InstKind::CoreCall {
                symbol,
                args: vec![operand],
            },
            env,
        );
        self.release_temporaries_since(mark, *cur);
        bytes
    }
    /// Whether the markup hole at `span` already holds a `Core\Html\Markup`.
    ///
    /// `Self::markup_target` answers the same question about an `as`
    /// conversion's written target and reads the same table; what differs is
    /// that this key is an expression's span rather than a type node's, which
    /// is how the checker's answer about a *value* survives into a phase where
    /// classes no longer do.
    fn hole_is_carrier(&self, span: nvs_diagnostics::Span) -> bool {
        let Some(id) = self.exprs.declared_ty(span) else {
            return false;
        };
        matches!(self.checked_types.get(id), CheckedTy::Class(qname, _)
            if qname.to_string() == nvs_types::CORE_HTML_MARKUP_CLASS)
    }
    /// Lowers `expr` — an `ExprKind::Index`'s subscript — to a key operand
    /// for [`ir::InstKind::ArrayGet`]/[`ir::InstKind::ArraySet`], in
    /// whichever of the two representations the crate docs' *an array key is
    /// a `string`, and an `int` subscript no longer spells it* allows.
    ///
    /// `rule:types/arrays` holds throughout: every key *is* a `string` and
    /// `$a[8]` is `$a["8"]`. What an `int` subscript buys is reaching that
    /// key without rendering the decimal. A [`Ty::Int`] subscript is handed
    /// to the instruction as the `int` it is, and `nvs-codegen` calls
    /// `nvs_array_get_index`/`nvs_array_set_index`, which answer from the
    /// packed form with nothing rendered and nothing allocated and
    /// synthesize a key only where the array is already `Hashed` — exactly
    /// the case that builds one anyway (`nvs_runtime::array`'s module
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
    /// `nvs_types::expr::check_array_key_type` enforces at both call
    /// sites (an `Index` subscript and an array-literal explicit key alike),
    /// so the `other` arm below is an internal-invariant panic — unreachable
    /// for anything that already passed `nvs_types::check_program` — rather
    /// than a live known gap.
    ///
    /// Returns the key value, **the representation it is in** — `Ty::Str` or
    /// `Ty::Int`, which is what every caller's refcount decision turns
    /// on, since an `int` owns nothing to retain or release — and whether it
    /// [`is_aliasing_read`]s storage a durable slot still owns, exactly the
    /// same second half [`Self::concat_operand`] returns and for the same
    /// reason: a plain `string` subscript passed through unchanged may still
    /// be a bare local/property/array read, while a freshly converted key is
    /// always a brand new buffer with exactly one owner.
    pub(crate) fn lower_array_key(
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
                let (sv, _) = self.emit_fallible(
                    *cur,
                    Ty::Str,
                    InstKind::HelperCall {
                        helper: Helper::UintToString,
                        args: vec![v],
                    },
                    env,
                );
                (sv, Ty::Str, false)
            }
            other => panic!(
                "nvs-ir: an array key lowered to {other:?} — nvs_types::check_program is trusted \
                 to have already rejected a float/bool/null key (`rule:types/arrays`) at both the \
                 subscript and array-literal explicit-key sites, so this should be unreachable"
            ),
        }
    }

    /// [`Self::lower_array_key`], forced all the way to a [`Ty::Str`] key.
    ///
    /// [`ir::InstKind::ArrayUnset`] is the one key-taking array instruction
    /// with no index-shaped runtime primitive beside it — `nvs-runtime`
    /// carries `nvs_array_get_index` and `nvs_array_set_index` and no third —
    /// so `unset($a[$i])` renders the decimal here rather than having
    /// codegen discover it cannot. Widening the runtime ABI to close that
    /// is a separate decision, not a side effect of this one; `rule:packaging/an-artifact-is-one-immutable-content-addressed-file`'s
    /// artifacts and M9's WIT signatures are about to freeze that surface.
    pub(crate) fn lower_rendered_array_key(
        &mut self,
        expr: &Expr,
        env: &mut Env,
        cur: &mut BlockId,
    ) -> (ValueId, bool) {
        let (v, ty, aliasing) = self.lower_array_key(expr, env, cur);
        if ty != Ty::Int {
            return (v, aliasing);
        }
        let (sv, _) = self.emit_fallible(
            *cur,
            Ty::Str,
            InstKind::HelperCall {
                helper: Helper::IntToString,
                args: vec![v],
            },
            env,
        );
        (sv, false)
    }

    /// `rule:types/arithmetic`, mirroring the checker's own rule: a bare integer
    /// literal means `uint` exactly where that's the expected type,
    /// `int` otherwise. `nvs_types::expr::literals::infer_int_literal`
    /// enforces `rule:types/arithmetic`'s magnitude rule
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
            // `rule:types/numeric-literal-placement`'s placing rule, integer half: an integer
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
        // `rule:types/numeric-literal-placement`: a fractional literal is untyped until placed,
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

    /// `rule:types/duration-literal`: the grammar is resolved while compiling, so what
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
                 produces this token for text that does, and for a mis-cased \
                 unit no compile path continues past the error it reports",
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
    /// `rule:types/closure-literal`'s `fn` literal. Evaluating one allocates its
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
        let (obj, _) = self.emit_fallible(
            *cur,
            Ty::Object,
            InstKind::New {
                class: class.clone(),
                ctor: None,
                args: Vec::new(),
            },
            env,
        );
        let arity = i64::try_from(fn_expr.params.len()).expect("a parameter list fits an i64");
        let (arity_v, _) = self.emit(*cur, Ty::Int, InstKind::ConstInt(arity));
        self.emit_field_set(*cur, obj, class.clone(), FN_ARITY.to_owned(), arity_v);
        // The declared parameter types are readable here and nowhere below
        // this crate — `callable` carries no parameter list for a call site to
        // compare against (`rule:types/closure-literal`), so the object is what carries them
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
            // An `inout $x` parameter binds an address, not a value, and `rule:types/implicit-capture`
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

    /// `rule:types/callable-is-a-closure`'s `Class::method(...)` / `$obj->method(...)`, which *names* the
    /// resolved member rather than calling it and whose value is a closure
    /// over it.
    ///
    /// The object this builds is byte-for-byte the one a `fn` literal builds
    /// — [`FN_ARITY`], [`FN_PARAM_TAGS`], and an `invoke` in the method table
    /// — because `rule:types/closure-literal` makes `callable` the only closure type, so a
    /// native `Core` member handed one of these cannot tell it apart from a
    /// written closure and has nothing new to learn. The body behind that
    /// `invoke` is the forwarding thunk `lower_callable` builds, which owns
    /// the representation and the one divergence it carries.
    ///
    /// `receiver` is the written receiver for the `$obj->method(...)`
    /// spelling and `None` for the `Class::method(...)` one — but *which*
    /// spelling was written does not decide whether a receiver is stored:
    /// `ResolvedCall::is_static` does, exactly as it does for a call, so
    /// `self::helper(...)` over a non-`static` member captures this frame's
    /// own `$this`.
    ///
    /// The arity written into the object is the target's **whole** parameter
    /// list. A member with a trailing default is therefore reachable through
    /// its own name and not through a `callable` that omits the argument:
    /// `callable` carries no parameter list for a call site to read (`rule:types/callable-absorbs-closure`), so the defaults a caller would materialize are ones no caller
    /// can see. That is a refusal at run time by `nvs_runtime::call_closure`,
    /// where every other arity mismatch through a `callable` is reported.
    ///
    /// # Panics
    ///
    /// Panics naming the shape for a non-`static` target reached from a frame
    /// with no `$this` — which the checker refuses where it is written, as it
    /// does for the call spelling.
    fn lower_callable_ref(
        &mut self,
        call: &ResolvedCall,
        receiver: Option<&Expr>,
        expr: &Expr,
        env: &mut Env,
        cur: &mut BlockId,
    ) -> (ValueId, Ty) {
        // The enclosing frame's label plus this site's index. `$` cannot start
        // an Novis identifier, so no declaration can collide with it — the
        // same guarantee `shape_class_label` relies on.
        let class = format!("{}$fcc{}", self.fn_label, self.callables.len());
        let params: Vec<Ty> = call
            .param_tys
            .iter()
            .map(|&id| lower_checked_ty(id, self.checked_types))
            .collect();
        let (obj, _) = self.emit_fallible(
            *cur,
            Ty::Object,
            InstKind::New {
                class: class.clone(),
                ctor: None,
                args: Vec::new(),
            },
            env,
        );
        let arity = i64::try_from(params.len()).expect("a parameter list fits an i64");
        let (arity_v, _) = self.emit(*cur, Ty::Int, InstKind::ConstInt(arity));
        self.emit_field_set(*cur, obj, class.clone(), FN_ARITY.to_owned(), arity_v);
        // The declared parameter types are readable here and nowhere below
        // this crate, exactly as they are at a `fn` literal — the target's,
        // this time, rather than the literal's own. See `FN_PARAM_TAGS`.
        let tags = pack_param_tags(params.iter().copied());
        let (tags_v, _) = self.emit(
            *cur,
            Ty::Int,
            InstKind::ConstInt(i64::from_ne_bytes(tags.to_ne_bytes())),
        );
        self.emit_field_set(*cur, obj, class.clone(), FN_PARAM_TAGS.to_owned(), tags_v);
        // The target's parameter *names*, which only a resolved call knows and
        // only this crate is still holding — ADR 0006 § *Decision* binds an
        // isolate's `args:` by them, and `Core\Socket::upgrade(Chat::run(...))`
        // is such an entry with no constant beside it to carry them. See
        // `FN_PARAM_NAMES` for why the sibling `fn` literal gets no such field.
        let (names_v, _) = self.emit(
            *cur,
            Ty::Str,
            InstKind::ConstStr(call.param_names.join(",")),
        );
        self.emit_field_set(*cur, obj, class.clone(), FN_PARAM_NAMES.to_owned(), names_v);
        let takes_receiver = !call.is_static;
        if takes_receiver {
            // `rule:types/implicit-capture`'s "by value at the point the closure literal is
            // evaluated", which for a first-class callable is the receiver —
            // and the answer PHP's own `(...)` gives.
            let (recv, ty, aliasing) = match receiver {
                Some(object) => {
                    let (v, ty) = self.lower_expr(object, None, env, cur);
                    (v, ty, self.aliasing_read(object))
                }
                None => {
                    let &(v, ty) = env.get("this").unwrap_or_else(|| {
                        panic!(
                            "nvs-ir: `{}::{}` is named as a callable at {:?} and is not \
                             static, from a frame with no `$this` — nvs_types is expected \
                             to have refused that, as it does for the call spelling",
                            call.class, call.method, expr.span
                        )
                    });
                    (v, ty, true)
                }
            };
            // The object owns one reference for as long as it lives. A
            // receiver read out of a local or a field is that binding's, so
            // this is a second one; a freshly built one has no other owner
            // and this store is what takes it over.
            if ty.is_refcounted() && aliasing {
                self.emit_retain(*cur, recv);
            }
            self.emit_field_set(*cur, obj, class.clone(), FCC_RECV.to_owned(), recv);
        }
        self.callables.push(PendingCallable {
            class,
            call: call.clone(),
            takes_receiver,
            params,
            span: expr.span,
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
        // `rule:types/class-reference-sites`'s `new $cls(...)` is the other entry this span can
        // carry, and it is a different lowering rather than a different label:
        // the class to allocate is a value in hand, not a name.
        if let Some(ExprInfo::NewDynamic { ctor, .. }) = self.exprs.lookup(expr.span) {
            return self.lower_new_through_a_class_reference(target, args, ctor.as_ref(), env, cur);
        }
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
        // Each argument is transferred to the constructor, which owns it only
        // once the `New` below exists — so this site brackets them with
        // `Lowering::forget_transferred_since` for the window in which an
        // argument that throws is what abandons them.
        let mark = self.temporaries_mark();
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
        // From the instruction below onward the callee owns every transferred
        // argument — its own exit sweep releases them on its throwing edge as
        // much as on its normal one — so they leave the stack *before* the
        // call's own fault edge is built, and after every fallible
        // instruction that evaluated them.
        self.forget_transferred_since(mark);
        let built = self.emit_fallible(*cur, Ty::Object, kind, env);
        self.flush_ref_writebacks(staged_refs, env, *cur);
        built
    }

    /// `rule:types/class-reference-sites`'s `new $cls(...)` — [`Self::lower_new`]'s dynamic half.
    ///
    /// The same [`InstKind::NewDynamic`] `new static()` lowers to, reached with
    /// a different descriptor: there it is [`Self::lsb`], the class this frame
    /// was called on, and here it is whatever the `class<T>` operand holds. The
    /// constructor label carried is `T`'s, and it is a **fallback** in both
    /// cases — the instruction asks the allocated class first, so a subclass's
    /// own constructor wins. That is what makes checking the arguments against
    /// `T`'s signature the right check rather than an optimistic one, together
    /// with `rule:classes/constructor-compatibility`, which refuses at this site any implementor of `T`
    /// whose constructor would not accept what `T`'s accepts.
    ///
    /// The operand is evaluated before the arguments, which is the order it is
    /// written in. It needs no lifecycle of its own: a descriptor is immortal
    /// and process-wide, so [`Ty::ClassDesc`] is not refcounted.
    fn lower_new_through_a_class_reference(
        &mut self,
        target: &NewTarget,
        args: &CallArgs,
        ctor: Option<&ResolvedCall>,
        env: &mut Env,
        cur: &mut BlockId,
    ) -> (ValueId, Ty) {
        let NewTarget::Expr(operand) = target else {
            panic!(
                "nvs-ir: an `ExprInfo::NewDynamic` entry on a `new` whose target is a \
                 written class name — only `nvs_types::expr::calls`'s `NewTarget::Expr` \
                 arm records one, so this is two crates disagreeing about the table"
            );
        };
        let (desc, _) = self.lower_expr(operand, Some(Ty::ClassDesc), env, cur);
        let ctor_label = ctor.map(|call| format!("{}::{}", call.class, call.method));
        // [`Self::lower_new`]'s bracket, for its reason: the callee owns every
        // transferred argument only from the instruction onward, and until then
        // an argument that throws is what abandons them.
        let mark = self.temporaries_mark();
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
                        "nvs-ir: `new $cls(...)` has no resolved constructor but wasn't \
                         called with a plain argument list — {args:?}"
                    );
                };
                assert!(
                    list.is_empty(),
                    "nvs-ir: `new $cls(...)` has no resolved constructor but was called \
                     with arguments — `Self::lower_new`'s own note owns why this crate \
                     cannot trust that was rejected upstream"
                );
                Vec::new()
            }
        };
        self.forget_transferred_since(mark);
        let built = self.emit_fallible(
            *cur,
            Ty::Object,
            InstKind::NewDynamic {
                desc,
                ctor: ctor_label,
                args: arg_values,
            },
            env,
        );
        self.flush_ref_writebacks(staged_refs, env, *cur);
        built
    }

    /// `Core\Program::implementing<T>()` — `rule:programs/implementing`'s expansion, emitted as the array literal it is specified to be.
    ///
    /// One `InstKind::New` per implementor with no arguments, gathered into
    /// one `InstKind::ArrayNew` — instruction for instruction what
    /// `[new Alpha(), new Beta()]` written by hand lowers to through
    /// [`Self::lower_array_literal`]'s keyless fast path, which is the whole
    /// point of § 3 specifying an *array literal of `new` expressions* rather
    /// than a runtime answer. Nothing is retained: each entry is a fresh
    /// allocation transferred straight into the array, exactly as an
    /// element that is not an aliasing read already is.
    ///
    /// `nvs_types::program` resolved both halves — the classes and each one's
    /// declaring constructor — for [`Self::lower_new`]'s reason: this crate
    /// cannot re-walk the hierarchy, and a class with no `constructor` of its
    /// own may still inherit one.
    fn lower_program_instances(
        &mut self,
        classes: &[String],
        ctors: &[Option<String>],
        env: &mut Env,
        cur: &mut BlockId,
    ) -> (ValueId, Ty) {
        let mut entries = Vec::with_capacity(classes.len());
        for (index, class) in classes.iter().enumerate() {
            let built = self.emit_fallible(
                *cur,
                Ty::Object,
                InstKind::New {
                    class: class.clone(),
                    ctor: ctors.get(index).cloned().flatten(),
                    args: Vec::new(),
                },
                env,
            );
            entries.push((index.to_string(), built.0));
        }
        self.emit(*cur, Ty::Array, InstKind::ArrayNew { entries })
    }

    /// `Core\Router::url(...)`/`::urlAbsolute(...)` over a route name
    /// `nvs_types::links` resolved — `rule:routing/link-name-and-params-are-checked`
    /// 's link.
    ///
    /// Two arguments in and two out, and only the first is different: the
    /// written name is replaced by the *prepared path* the checker resolved it
    /// to, and `$params` is lowered exactly as `lower_static_call`'s `Core`
    /// branch would have lowered it. So this is not a fold — the call is still
    /// made, because percent-encoding a run-time value is run-time work — it is
    /// `rule:expressions/intrinsic-literals`'s *preparation*, with the lookup and the path's grammar
    /// paid once at compile time.
    ///
    /// `$params` is borrowed like every other `Core` argument
    /// (`InstKind::CoreCall`), and the prepared string is a temporary this
    /// frame owns, which is why the release below covers the whole run. Both
    /// go through [`Self::account_for_arg`] to get there: this arm builds its
    /// argument vector by hand rather than through
    /// [`Self::lower_call_args`](crate::lower::Lowering::lower_call_args), and
    /// a value lowered but never *staged* is one `release_temporaries_since`
    /// cannot see — without both lines below, `Core\Router::url("…", ["id" =>
    /// 7])` leaks its literal array once per call.
    /// `spawn script <path> with(…)` — ADR 0006 § *Decision*'s isolate spawn,
    /// as one [`InstKind::CoreCall`] answering a `Core\Script\Handle`.
    ///
    /// A `CoreCall` and not an `InstKind` of its own, which is the decision
    /// this arm records: a spawn is a call into native code that can throw,
    /// takes three values and answers one, and that is exactly what
    /// `nvs-codegen` already emits for a `Core` member — a variant of its own
    /// would buy a second shape in the backend for no behaviour. The symbol
    /// carries no registry row (`nvs_stdlib::script`'s module doc owns why),
    /// so nothing but this arm can reach it, which is what keeps `spawn
    /// script` syntax rather than a member with a keyword in front of it.
    ///
    /// Three arguments in a fixed order — the path, the `args:` value and the
    /// `output:` spelling — with the two options materialized to `null` and
    /// `"capture"` where the program omitted them, the same way
    /// [`Self::lower_options_arg`] materializes an `rule:core-api/shape-rules` R2 bag's defaults.
    /// `limits:`, `grants:` and `on:` never arrive: `nvs_types`' own
    /// `check_spawn_script` refuses each under `E0777` rather than let one be
    /// accepted and dropped.
    ///
    /// The `args:` value is **transferred**, alone among the three, because it
    /// is the one the isolate keeps: it crosses the boundary into the child's
    /// ownership root and this frame may not release it afterwards. The path
    /// and the output spelling are borrowed like every other `Core` argument.
    ///
    /// **Two symbols, one argument list.** ADR 0006 § *Decision*'s method entry
    /// takes `nvs_types::CORE_SCRIPT_SPAWN_METHOD` and differs in argument 0
    /// alone: a constant `Class::method` label instead of a lowered path
    /// expression, since what the child runs is a method of *this* unit and
    /// there is nothing for a resolver to compile. `nvs_stdlib::script`'s
    /// `SPAWN_METHOD_SYMBOL` owns why the fork is a second symbol rather than a
    /// fourth argument, and [`Self::spawn_method_entry`] is the fork itself.
    /// The method symbol does take a **fourth** argument the path form has no
    /// use for — the entry's parameter names, for ADR 0006 § *Decision*'s
    /// `args:` binding — and that one is a value the child needs rather than a
    /// branch this module already took.
    fn lower_spawn_script(
        &mut self,
        path: &Expr,
        options: &[SpawnOption],
        env: &mut Env,
        cur: &mut BlockId,
    ) -> (ValueId, Ty) {
        let mark = self.temporaries_mark();
        // ADR 0006 § *Decision*'s two entry forms, told apart by the operand's
        // own shape exactly as `nvs_types`' `check_entry` tells them apart —
        // and lowered to two symbols, because what differs is what the first
        // argument *means*: a path the child's resolver compiles, or a label
        // naming a method of the unit this frame is already running.
        let method = self.spawn_method_entry(path);
        let (path_v, path_ty) = match &method {
            // A constant, not the operand: a first-class-callable reference
            // lowered as an expression would build a `callable` value, which is
            // the one thing `rule:security/isolate-shares-nothing` refuses to let cross a boundary. The label
            // is `nvs_runtime::call_static`'s own spelling.
            Some((label, _)) => self.emit(*cur, Ty::Str, InstKind::ConstStr(label.clone())),
            None => self.lower_expr(path, Some(Ty::Str), env, cur),
        };
        let aliasing = method.is_none() && self.aliasing_read(path);
        self.account_for_arg(path_v, path_ty, ArgOwnership::Borrowed, aliasing, *cur);

        let written = |key: SpawnOptionKey| options.iter().find(|opt| opt.key == key);
        let args_v = match written(SpawnOptionKey::Args) {
            Some(opt) => {
                let (v, ty) = self.lower_expr(&opt.value, Some(Ty::Tagged), env, cur);
                let aliasing = self.aliasing_read(&opt.value);
                let v = self.coerce(*cur, v, ty, Ty::Tagged, env);
                // The one transferred argument. A value the frame still owns
                // is retained first, so the reference the child consumes is a
                // second one rather than this frame's only one.
                self.account_for_arg(v, Ty::Tagged, ArgOwnership::Transferred, aliasing, *cur);
                v
            }
            // A child that was passed nothing reads `null` from
            // `Core\Script::args()`, which is what a missing option means and
            // not an empty array: the two are distinguishable and a program
            // that passed `[]` said something different.
            None => {
                let (v, _) = self.emit(*cur, Ty::Null, InstKind::ConstNull);
                self.coerce(*cur, v, Ty::Null, Ty::Tagged, env)
            }
        };
        let output_v = match written(SpawnOptionKey::Output) {
            Some(opt) => {
                let (v, ty) = self.lower_expr(&opt.value, Some(Ty::Str), env, cur);
                let aliasing = self.aliasing_read(&opt.value);
                self.account_for_arg(v, ty, ArgOwnership::Borrowed, aliasing, *cur);
                v
            }
            // ADR 0006 § *Output is captured by default*, spelled here rather
            // than defaulted in the helper so that the default is visible in
            // the IR a reader dumps.
            None => {
                let (v, _) = self.emit(*cur, Ty::Str, InstKind::ConstStr("capture".to_owned()));
                self.account_for_arg(v, Ty::Str, ArgOwnership::Borrowed, false, *cur);
                v
            }
        };

        // ADR 0006 § *Decision* binds `args:`'s entries to the entry's own
        // parameters **by name**, which is a question only the declaration
        // answers — so the names travel with the label, as a fourth argument
        // the method symbol alone takes. Comma-separated in declaration order,
        // empty for an entry that declares none: a `ConstStr` costs the child
        // no parse of the unit it is about to call into, and
        // `nvs_runtime::MethodRow` cannot answer this at all (it has arity and
        // parameter tags, never names).
        let mut call_args = vec![path_v, args_v, output_v];
        if let Some((_, names)) = &method {
            let (names_v, _) = self.emit(*cur, Ty::Str, InstKind::ConstStr(names.join(",")));
            self.account_for_arg(names_v, Ty::Str, ArgOwnership::Borrowed, false, *cur);
            call_args.push(names_v);
        }

        let result = self.emit_fallible(
            *cur,
            Ty::Object,
            InstKind::CoreCall {
                symbol: if method.is_some() {
                    nvs_types::CORE_SCRIPT_SPAWN_METHOD
                } else {
                    nvs_types::CORE_SCRIPT_SPAWN
                },
                args: call_args,
            },
            env,
        );
        self.release_temporaries_since(mark, *cur);
        result
    }

    /// `Class::method` and the entry's parameter names, for a `spawn script`
    /// operand written as a first-class callable reference — and `None` for
    /// every other operand.
    ///
    /// The label is the *declaring* class and the method's own name — the
    /// spelling `nvs_runtime::call_static` looks a descriptor up by, and the one
    /// [`Self::lower_static_call`] builds for a direct call to the same target,
    /// so a method entry and an ordinary call reach one address by one route.
    ///
    /// The resolved call is read back out of the typed-expression table rather
    /// than re-derived from the syntax: a bare `Reports` in the operand resolves
    /// against the active namespace and imports, which is context only
    /// `nvs_types` and `nvs-hir` have (`nvs_types::expr_table`'s module docs).
    /// The **names** ride out of the same entry for the same reason: they are
    /// the resolved declaration's, and this frame holds a call site rather
    /// than a signature table.
    fn spawn_method_entry(&self, path: &Expr) -> Option<(String, Vec<String>)> {
        if !matches!(
            &path.kind,
            ExprKind::StaticCall {
                args: CallArgs::FirstClassCallable,
                ..
            }
        ) {
            return None;
        }
        let Some(ExprInfo::CallableRef(call)) = self.exprs.lookup(path.span) else {
            panic!(
                "nvs-ir: a `spawn script` operand at {:?} is written as a static-method \
                 reference and has no resolved target recorded in the typed-expression table \
                 — did this program pass nvs_types::check_program with the same table? \
                 `nvs_types::expr::isolate`'s `check_entry` is what records one",
                path.span
            );
        };
        Some((
            format!("{}::{}", call.class, call.method),
            call.param_names.clone(),
        ))
    }

    /// `await <handle>` — the other half, as the second of the two symbols.
    ///
    /// The handle is **borrowed**, exactly as a `Core` member's receiver is:
    /// what the helper consumes is the request's table entry for the started
    /// isolate, not the object the program is holding, so the frame that wrote
    /// `$job` still owns `$job` afterwards. Awaiting one twice is therefore a
    /// throw from the helper rather than anything this arm can see.
    ///
    /// The answer is an `rule:types/object-top` shape value, which is an ordinary object with
    /// slots, so `$result->ok` on the next line lowers to the `SlotGet` a
    /// written shape literal's read already lowers to — `nvs-ir` learns
    /// nothing here about where the shape came from.
    fn lower_await(&mut self, operand: &Expr, env: &mut Env, cur: &mut BlockId) -> (ValueId, Ty) {
        let mark = self.temporaries_mark();
        let (handle, ty) = self.lower_expr(operand, Some(Ty::Object), env, cur);
        let aliasing = self.aliasing_read(operand);
        self.account_for_arg(handle, ty, ArgOwnership::Borrowed, aliasing, *cur);
        let result = self.emit_fallible(
            *cur,
            Ty::Object,
            InstKind::CoreCall {
                symbol: nvs_types::CORE_SCRIPT_AWAIT,
                args: vec![handle],
            },
            env,
        );
        self.release_temporaries_since(mark, *cur);
        result
    }

    /// One resolved link call, as the prepared path plus whatever else its
    /// member takes.
    ///
    /// `signed` is `urlSigned`'s route name and `None` for the other two. It is
    /// a second constant argument rather than a piece of `prepared` because the
    /// two say opposite things: the prepared path is what a remount changes,
    /// and the name is what it does not, which is the whole of
    /// `rule:core-classes/router-signed-url`. A signing call also carries the
    /// `{keys, until}` shape, flattened here into one argument per declared
    /// field — `rule:core-api/shape-flattens-at-the-abi`'s ABI, reached by hand
    /// because this arm builds its argument vector by hand.
    fn lower_route_link(
        &mut self,
        prepared: &str,
        absolute: bool,
        signed: Option<String>,
        args: &CallArgs,
        env: &mut Env,
        cur: &mut BlockId,
    ) -> (ValueId, Ty) {
        let mark = self.temporaries_mark();
        let (template, _) = self.emit(*cur, Ty::Str, InstKind::ConstStr(prepared.to_owned()));
        // Not an aliasing read of anything: the prepared path is this frame's
        // own constant. Its header is immortal, so the release this stages is
        // a no-op at run time — it is here because the *rule* is that every
        // argument is accounted for, and an exception is what the next hand-
        // rolled argument list would copy.
        self.account_for_arg(template, Ty::Str, ArgOwnership::Borrowed, false, *cur);
        // The arity is the member's own and was checked in `nvs_types`, and a
        // site is only recorded when both arguments are written positionally —
        // `nvs_types::links`' gap 2.
        let CallArgs::List(list) = args else {
            panic!("nvs-ir: a resolved route link has a written argument list")
        };
        let mut lowered = vec![template];
        if let Some(name) = &signed {
            let (name, _) = self.emit(*cur, Ty::Str, InstKind::ConstStr(name.clone()));
            self.account_for_arg(name, Ty::Str, ArgOwnership::Borrowed, false, *cur);
            lowered.push(name);
        }
        let (params, params_ty) = self.lower_expr(&list[1].value, None, env, cur);
        let aliasing = self.aliasing_read(&list[1].value);
        self.account_for_arg(params, params_ty, ArgOwnership::Borrowed, aliasing, *cur);
        lowered.push(params);
        if signed.is_some() {
            self.lower_signing_settings(&list[2].value, env, cur, &mut lowered);
        }
        let symbol = match (signed.is_some(), absolute) {
            (true, _) => nvs_types::CORE_ROUTE_LINK_SIGNED,
            (false, true) => nvs_types::CORE_ROUTE_LINK_ABSOLUTE,
            (false, false) => nvs_types::CORE_ROUTE_LINK,
        };
        let result = self.emit_fallible(
            *cur,
            Ty::Str,
            InstKind::CoreCall {
                symbol,
                args: lowered,
            },
            env,
        );
        self.release_temporaries_since(mark, *cur);
        result
    }

    /// `urlSigned`'s `{keys, until}` argument, as the two ABI arguments
    /// `rule:core-api/shape-flattens-at-the-abi` makes it.
    ///
    /// The names and their order are `nvs_stdlib::signature::SIGNING`'s, which
    /// is the one place the shape is declared; both fields are required
    /// (`rule:core-api/a-lifetime-is-written` is why `until` has no default),
    /// so there is no omission to materialize and nothing here reads a default.
    /// Each is lowered with no expectation and borrowed, which is what
    /// `Self::lower_fixed_arg` does for any `Core` parameter with no single IR
    /// representation: a helper's slot is a whole `Value` that `nvs-codegen`
    /// writes from the argument's own representation.
    ///
    /// # Panics
    ///
    /// Panics for an argument that is not an object literal, or one missing
    /// either field: `nvs_types::check_program` reports `E_OPTIONS_NOT_A_LITERAL`
    /// for the first and a shape mismatch for the second, exactly as
    /// [`Self::lower_options_arg`] trusts it to.
    fn lower_signing_settings(
        &mut self,
        written: &Expr,
        env: &mut Env,
        cur: &mut BlockId,
        out: &mut Vec<ValueId>,
    ) {
        let ExprKind::ObjectLiteral(fields) = &written.kind else {
            panic!(
                "nvs-ir: `Core\\Router::urlSigned`'s settings lowered from something that is not \
                 an object literal — nvs_types::check_program is trusted to have reported \
                 E_OPTIONS_NOT_A_LITERAL"
            )
        };
        for name in ["keys", "until"] {
            let field = fields
                .iter()
                .find(|field| span_text(self.src, field.name) == name)
                .unwrap_or_else(|| {
                    panic!(
                        "nvs-ir: `Core\\Router::urlSigned`'s settings omitted `{name}`, which \
                         nvs_stdlib::signature::SIGNING declares with no default"
                    )
                });
            let (v, ty) = self.lower_expr(&field.value, None, env, cur);
            let aliasing = self.aliasing_read(&field.value);
            self.account_for_arg(v, ty, ArgOwnership::Borrowed, aliasing, *cur);
            out.push(v);
        }
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
        // `rule:types/erased-member-access`'s deferral: a `mixed` receiver resolved to no
        // signature at all and was recorded as the name alone, so the call is
        // dispatched on the value rather than on a class this frame could
        // name. Taken before the resolved arm because it is the *absence* of a
        // resolution that selects it.
        if let Some(ExprInfo::ErasedCall { name }) = self.exprs.lookup(expr.span) {
            let name = name.clone();
            return self.lower_erased_method_call(object, nullsafe, &name, args, env, cur);
        }
        // `rule:types/callable-is-a-closure`'s `$obj->method(...)`, which names the member rather
        // than calling it. Taken before the resolved arm for the erased one's
        // reason: it is the *variant* that selects it, and both carry the
        // same `ResolvedCall`.
        if let Some(ExprInfo::CallableRef(call)) = self.exprs.lookup(expr.span) {
            let call = call.clone();
            return self.lower_callable_ref(&call, Some(object), expr, env, cur);
        }
        let Some(ExprInfo::Call(call)) = self.exprs.lookup(expr.span) else {
            panic!(
                "nvs-ir: an instance method call at {:?} has no resolved target \
                 recorded in the typed-expression table — did this program pass \
                 nvs_types::check_program with the same table? Every receiver naming \
                 no class is either refused where it is written (`E0477`) or, for the \
                 `mixed` `rule:types/conversion` makes the one unchecked position, recorded as \
                 `ExprInfo::ErasedCall` and lowered above; a receiver that does name \
                 a class and calls a member it has not got is `E0405` there too, \
                 `Core` included, since the registry is the whole roster of `Core` \
                 (`nvs_types::core_lib`). `rule:types/callable-is-a-closure`'s \
                 `$obj->method(...)` records `ExprInfo::CallableRef` instead, because \
                 it names the member rather than calling it, and is answered by \
                 `Lowering::lower_callable_ref` above",
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
            // A member on `nvs_stdlib::registry::WRITTEN_CLASS_MEMBERS` is
            // handed what its call site wrote as a type argument — and that
            // block of three goes *ahead of the receiver*, which is the order
            // that roster's docs state and the one
            // `Lowering::lower_callable_ref` already emits.
            // `Lowering::written_type_constants` owns the block, including why
            // it is emitted before the receiver is opened.
            let written_class =
                nvs_types::core_takes_written_class(&call.class.to_string(), &call.method)
                    .then(|| self.written_type_constants(*cur, call));
            // A member on `nvs_stdlib::registry::RECORD_PRODUCERS` is handed
            // where it was called, as argument 0 — `Lowering::producer_source`
            // owns the position and why it is emitted here.
            let source = nvs_types::core_takes_source(&call.class.to_string(), &call.method)
                .then(|| self.producer_source(*cur));
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
            let mut arg_values = Vec::with_capacity(values.len() + 4);
            arg_values.extend(source);
            arg_values.extend(written_class.into_iter().flatten());
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
        // The receiver and every argument are transferred, and the callee owns
        // them only from the `Call` below — see
        // `Lowering::forget_transferred_since`. Taken before the receiver, for
        // the reason `Lowering::temporaries_mark` gives.
        let mark = self.temporaries_mark();
        // `?->` guards everything below on the receiver not being
        // `null`; `->` opens no guard at all.
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
            let aliasing = self.aliasing_read(object);
            self.account_for_arg(
                object_v,
                receiver_ty,
                ArgOwnership::Transferred,
                aliasing,
                *cur,
            );
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
        // From the instruction below onward the callee owns every transferred
        // argument — its own exit sweep releases them on its throwing edge as
        // much as on its normal one — so they leave the stack *before* the
        // call's own fault edge is built, and after every fallible
        // instruction that evaluated them.
        self.forget_transferred_since(mark);
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
        // `rule:types/callable-is-a-closure`'s `Class::method(...)` — as for an instance call, the
        // variant is what selects this and the resolved facts are the same.
        if let Some(ExprInfo::CallableRef(call)) = self.exprs.lookup(expr.span) {
            let call = call.clone();
            return self.lower_callable_ref(&call, None, expr, env, cur);
        }
        // `rule:types/class-reference-sites`'s `$cls::f(...)`, which is the same resolved call
        // reached through a value rather than a name — see
        // [`Self::lower_static_call_on_a_class_reference`].
        if let Some(ExprInfo::ClassRefCall(call)) = self.exprs.lookup(expr.span) {
            return self.lower_static_call_on_a_class_reference(class, args, call, env, cur);
        }
        let Some(ExprInfo::Call(call)) = self.exprs.lookup(expr.span) else {
            panic!(
                "nvs-ir: a static call at {:?} has no resolved target recorded in the \
                 typed-expression table — did this program pass \
                 nvs_types::check_program with the same table? `rule:types/callable-is-a-closure`'s \
                 `Class::method(...)` records `ExprInfo::CallableRef` rather than \
                 `Call` — it names the member rather than calling it, and is answered \
                 by `Lowering::lower_callable_ref` above",
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
            // `Core\Script::finish()` carries a row so the checker can resolve
            // and type it, and is the one row no site ever calls: its symbol is
            // the label the raise is recognised by, and `Self::lower_finish`
            // seals the block with the throw that ends the script instead.
            // `nvs_stdlib::script`'s own symbol doc is the home of why the
            // helper behind that address exists at all.
            if symbol == nvs_types::CORE_SCRIPT_FINISH {
                return self.lower_finish(env, cur);
            }
            let sig = ArgSig::of_helper(call);
            let return_ty = lower_checked_ty(call.return_ty, self.checked_types);
            let checked_types = self.checked_types;
            // A member on `nvs_stdlib::registry::WRITTEN_CLASS_MEMBERS` is
            // handed what its call site wrote as a type argument, as arguments
            // 0 to 2 — that roster owns the ABI and
            // `Lowering::written_type_constants` emits it.
            let written_class =
                nvs_types::core_takes_written_class(&call.class.to_string(), &call.method)
                    .then(|| self.written_type_constants(*cur, call));
            // A member on `nvs_stdlib::registry::RECORD_PRODUCERS` is handed
            // where it was called, as argument 0 — `Lowering::producer_source`
            // owns the position.
            let source = nvs_types::core_takes_source(&call.class.to_string(), &call.method)
                .then(|| self.producer_source(*cur));
            let mark = self.temporaries_mark();
            let lowered =
                self.lower_call_args(args, &sig, checked_types, ArgOwnership::Borrowed, env, cur);
            let arg_values = source
                .into_iter()
                .chain(written_class.into_iter().flatten())
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
        // As for an instance call: the receiver and the arguments are the
        // callee's from the `Call` below and this frame's until then. See
        // `Lowering::forget_transferred_since`.
        let mark = self.temporaries_mark();
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
            // `$this` is a read of this frame's own slot, so it is the
            // aliasing column of `Lowering::account_for_arg` exactly.
            self.account_for_arg(this_v, this_ty, ArgOwnership::Transferred, true, *cur);
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
        // From the instruction below onward the callee owns every transferred
        // argument — its own exit sweep releases them on its throwing edge as
        // much as on its normal one — so they leave the stack *before* the
        // call's own fault edge is built, and after every fallible
        // instruction that evaluated them.
        self.forget_transferred_since(mark);
        let result = self.emit_fallible(*cur, return_ty, kind, env);
        self.flush_ref_writebacks(staged_refs, env, *cur);
        result
    }

    /// `rule:types/class-reference-sites`'s `$cls::f(...)` — [`Self::lower_static_call`]'s class-reference
    /// half.
    ///
    /// One [`InstKind::CallVirtual`], and **always** virtual: the class side is
    /// a descriptor, and a class reference exists to hold an implementor of its
    /// bound, so the whole question at this site is which class's `f` runs. The
    /// resolved label goes in as `fallback`, which is what a bound whose own
    /// declaration has a body still needs when the descriptor's class declares
    /// no `f` of its own.
    ///
    /// `receiver` is `None`, so the callee's slot 0 carries the dispatch
    /// descriptor — the arrangement `static::f()` already uses, and the reason
    /// late static binding inside `f` sees the implementor rather than the
    /// bound. That is sound because the target is always `static`:
    /// `nvs_types::expr_table::ExprInfo::ClassRefCall` owns why an instance one
    /// cannot be reached here at all.
    fn lower_static_call_on_a_class_reference(
        &mut self,
        class: &Expr,
        args: &CallArgs,
        call: &ResolvedCall,
        env: &mut Env,
        cur: &mut BlockId,
    ) -> (ValueId, Ty) {
        let target_label = format!("{}::{}", call.class, call.method);
        let method = call.method.clone();
        let sig = ArgSig::of(call);
        let has_body = call.has_body;
        let return_ty = lower_checked_ty(call.return_ty, self.checked_types);
        let checked_types = self.checked_types;
        // [`Self::lower_static_call`]'s window, for its reason.
        let mark = self.temporaries_mark();
        // The class side is written first and evaluated first. It needs no
        // lifecycle: [`Ty::ClassDesc`] is not refcounted.
        let (desc, _) = self.lower_expr(class, Some(Ty::ClassDesc), env, cur);
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
        self.forget_transferred_since(mark);
        let result = self.emit_fallible(
            *cur,
            return_ty,
            InstKind::CallVirtual {
                lsb: desc,
                method,
                fallback: has_body.then_some(target_label),
                receiver: None,
                args: arg_values,
            },
            env,
        );
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
    pub(crate) fn lower_static_property(
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
    pub(crate) fn static_property_of(&self, expr: &Expr) -> (String, String, Ty) {
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
    /// [`Self::lower_shape_property_access`], which is `rule:types/erased-member-access`'s
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
    /// `rule:classes/property-observer-pipeline`'s second step: `$receiver->onPropertyGet($name, $value)`,
    /// or its `onPropertySet` twin, emitted after the first step has settled
    /// what the read produced or the write committed.
    ///
    /// **It dispatches on the receiver's runtime class**, exactly as
    /// [`Self::lower_object_comparison`]'s `Comparable::compareTo` does and for
    /// the same reason: a subclass may override the observer, and the interface
    /// declaration itself has no body to name. `fallback` is the statically
    /// resolved label `nvs_types` recorded, `None` when that resolution is
    /// bodiless.
    ///
    /// It carries `rule:errors/propagation`'s error edge because § 3 says so outright — an
    /// observer that throws still fails the access it was reporting, even
    /// though the value had already been resolved.
    ///
    /// Ownership is the ordinary argument convention with nothing special in
    /// it: [`InstKind::CallVirtual`] transfers the receiver and every argument,
    /// so both the receiver and the observed value are retained here first —
    /// this frame keeps its own claim on each, the value's because the caller
    /// of a read still receives it and a write's slot still holds it. The name
    /// is a fresh [`InstKind::ConstStr`] with exactly one use and transfers as
    /// it stands. The value is widened to `mixed` *after* its retain, since
    /// [`Self::coerce`]'s `Tag` transfers rather than duplicates.
    #[expect(
        clippy::too_many_arguments,
        reason = "the receiver and its representation, the observed property's \
                  name, the settled value and its representation, and which \
                  half of § 3's pipeline this is — every one of them differs \
                  between the read site and the write site"
    )]
    pub(crate) fn emit_observer_call(
        &mut self,
        cur: BlockId,
        object_v: ValueId,
        receiver_ty: Ty,
        name: &str,
        value: ValueId,
        value_ty: Ty,
        method: &str,
        fallback: Option<String>,
        env: &mut Env,
    ) {
        if receiver_ty.is_refcounted() {
            self.emit_retain(cur, object_v);
        }
        if value_ty.is_refcounted() {
            self.emit_retain(cur, value);
        }
        let (name_v, _) = self.emit(cur, Ty::Str, InstKind::ConstStr(name.to_owned()));
        let tagged = self.coerce(cur, value, value_ty, Ty::Tagged, env);
        let (desc, _) = self.emit(
            cur,
            Ty::ClassDesc,
            InstKind::ClassDescOf { object: object_v },
        );
        self.emit_fallible(
            cur,
            Ty::Void,
            InstKind::CallVirtual {
                lsb: desc,
                method: method.to_owned(),
                fallback,
                receiver: Some(object_v),
                args: vec![name_v, tagged],
            },
            env,
        );
    }

    fn lower_property_access(
        &mut self,
        object: &Expr,
        property: &MemberName,
        nullsafe: bool,
        expr: &Expr,
        env: &mut Env,
        cur: &mut BlockId,
    ) -> (ValueId, Ty) {
        // `rule:classes/property-hooks`: a read of a property that declares a `get`
        // hook is a call to that hook's compiled function, with the
        // receiver in the ordinary parameter-0 slot — see
        // `lower_property_hook`. A property with only a `set` hook
        // still reads its own slot, since Novis's hooked properties are
        // always backed (`nvs_types::signatures::PropertyHooks` owns
        // that decision), so both shapes recover the same fields
        // and only the `get` label decides between them.
        // An `rule:types/erased-member-access` shape receiver naming one of its own fields is the
        // one access with no class to resolve: the slot index is already in
        // the table, so this reads it and is done. Everything below — the
        // hook question, the declaring class, the label — is a class
        // receiver's problem and none of it applies.
        // `ExprInfo::ShapeProperty::guarded` is an `AbsentKey` at this end.
        // The checker marks a read it found under a `??`, an `isset` or an
        // `empty` and typed `?T`; that read takes the `null`-answering entry
        // point, and every other one throws on a name the concrete class does
        // not carry.
        if let Some(ExprInfo::ShapeProperty {
            name,
            slot,
            ty,
            guarded,
        }) = self.exprs.lookup(expr.span)
        {
            let absent = if *guarded {
                AbsentKey::Null
            } else {
                AbsentKey::Throws
            };
            let field = ShapeField {
                name: name.clone(),
                slot: *slot,
                ty: *ty,
            };
            return self.lower_shape_property_access(object, &field, absent, nullsafe, env, cur);
        }
        // `rule:types/property-key-access`'s `$obj->$key`, the one access whose member name is not
        // in this table at all: it arrives as a value when the statement runs,
        // so none of the three facts below — the declaring class, the hook, the
        // label — is a question this site can ask. § 5 lowers it to `rule:types/erased-member-access`'s erased access with the name taken from the key, which is
        // `InstKind::KeyGet`.
        if let Some(ExprInfo::KeyedProperty { ty, .. }) = self.exprs.lookup(expr.span) {
            let ty = *ty;
            return self.lower_keyed_property_access(object, property, ty, nullsafe, env, cur);
        }
        let (class, name, ty, get, observer) = match self.exprs.lookup(expr.span) {
            Some(ExprInfo::Property {
                class,
                name,
                ty,
                observer,
            }) => (class, name, *ty, None, observer.clone()),
            Some(ExprInfo::HookedProperty {
                class,
                name,
                ty,
                get,
                observer,
                ..
            }) => (class, name, *ty, get.clone(), observer.clone()),
            // Every shape a `PropertyAccess` takes is handled above,
            // `rule:types/property-key-access`'s keyed one included, so this arm is the
            // consistency claim it reads as and not a lowering still owed.
            _ => panic!(
                "nvs-ir: a property access at {:?} has neither a resolved declaring class \
                  nor an `rule:types/erased-member-access` erased entry nor an `rule:types/property-key-access` keyed entry recorded \
                  in the typed-expression table, so it was not checked with the same table — \
                  `nvs_types::expr::members::check_property_member` records one for every \
                  access it returns from and refuses the rest, and its own doc comment \
                  carries that proof",
                expr.span
            ),
        };
        let field_ty = lower_checked_ty(ty, self.checked_types);
        let class_label = class.to_string();
        let field_name = name.clone();
        let observed_name = name.clone();
        // `rule:classes/an-unwritten-property-read-throws`'s never-written state, asked of the *declaring* class,
        // which is what the table recorded. A `get` hook answers with its own
        // body rather than with the slot, so there is nothing to guard there.
        let never_written = get.is_none() && self.exprs.is_lateinit_property(&class_label, name);
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
            None => {
                let read = self.emit(
                    *cur,
                    field_ty,
                    InstKind::FieldGet {
                        object: object_v,
                        class: class_label.clone(),
                        field: field_name,
                    },
                );
                if never_written {
                    self.emit_never_written_guard(
                        read.0,
                        &class_label,
                        &observed_name,
                        expr.span,
                        env,
                        cur,
                    );
                }
                read
            }
        };
        // `rule:classes/property-observer-pipeline`'s second step, on the read side: the value is settled
        // first — by the `get` hook above or by the slot — and *that* value is
        // what the observer is told about, never anything it answers, since
        // `onPropertyGet` returns `void`. Emitted before the temporaries are
        // released so a throw out of the observer body drops the base too,
        // and before the fresh-producer retain below so the observer's own
        // reference is accounted separately from the caller's.
        if let Some(calls) = observer {
            self.emit_observer_call(
                *cur,
                object_v,
                receiver_ty,
                &observed_name,
                v,
                ty,
                "onPropertyGet",
                calls.get,
                env,
            );
        }
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

    /// `rule:classes/an-unwritten-property-read-throws`'s never-written storage state, on the compiled read: `value` is
    /// the object `class`'s slot for `$field` just handed back, and this
    /// leaves `cur` on the block where it is a real instance.
    ///
    /// **The test is the payload, not the tag**, and that is the whole reason
    /// this is an inline branch rather than a call. The state is only
    /// reachable on a `lateinit` property (`rule:classes/lateinit`), whose declared type
    /// `rule:classes/lateinit-restrictions` restricts to a non-nullable class or interface — one
    /// pointer, null in this state and in no other, since `rule:classes/definite-property-initialization`
    /// discharges every other non-nullable property at its constructor and a
    /// `?T` is not a `lateinit` at all. So [`InstKind::IsNull`] over the
    /// [`Ty::Object`] already loaded answers the same question
    /// `nvs_runtime::Tag::Unset` answers for a reader holding the whole slot,
    /// at one compare and no second load. `nvs_runtime::Value::unset` is that
    /// argument's other end.
    ///
    /// **It borrows.** `InstKind::FieldGet` took no reference and this takes
    /// none either: the throw edge abandons nothing the slot still owns, and
    /// the caller's own accounting for the read is unchanged by the guard
    /// standing between the two.
    ///
    /// The class is `LogicError` — spec § 10's entry for "a bug in the
    /// program", the same one [`Self::lower_match`]'s unmatched subject
    /// raises — and not `rule:errors/escalation-ladder`'s fatal ladder, § 3 being explicit that this
    /// is a catchable, recoverable condition. The wording is
    /// `nvs_runtime::nvs_object_slot_get`'s, so the erased read and this one
    /// report one failure one way.
    fn emit_never_written_guard(
        &mut self,
        value: ValueId,
        class: &str,
        field: &str,
        span: Span,
        env: &mut Env,
        cur: &mut BlockId,
    ) {
        let (is_unset, _) = self.emit(*cur, Ty::Bool, InstKind::IsNull { operand: value });
        let unset = self.new_block();
        let written = self.new_block();
        let unset_edge = self.ids.next_edge(span);
        let written_edge = self.ids.next_edge(span);
        self.seal(
            *cur,
            Terminator::Branch {
                cond: is_unset,
                then_block: unset,
                then_edge: unset_edge,
                else_block: written,
                else_edge: written_edge,
            },
        );
        let (message, _) = self.emit(
            unset,
            Ty::Str,
            InstKind::ConstStr(format!(
                "`{class}`'s property `${field}` is read before it is written"
            )),
        );
        // Argument 2 is the `{previous}` bag flattened to its own `null`
        // default, widened into the `Ty::Tagged` slot spec § 10's
        // `Throwable|null` erases to — the list `Self::lower_match`'s own
        // throw builds by hand, for the same reason.
        let (absent, _) = self.emit(unset, Ty::Null, InstKind::ConstNull);
        let absent = self.coerce(unset, absent, Ty::Null, Ty::Tagged, env);
        let (exception, _) = self.emit_fallible(
            unset,
            Ty::Object,
            InstKind::New {
                class: "LogicError".to_owned(),
                ctor: Some(THROWABLE_CTOR.to_owned()),
                args: vec![message, absent],
            },
            env,
        );
        let source = self.throw_source(unset);
        let landing = self.landing_block(env);
        self.seal(
            unset,
            Terminator::Throw {
                value: exception,
                source,
                landing,
            },
        );
        *cur = written;
    }

    /// `{x: 1, y: 2}` — `rule:types/object-literal`'s anonymous object literal, which is an ordinary instance of a
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
    /// itself, which is `rule:classes/definite-property-initialization`'s definite-assignment obligation
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
        let (obj, _) = self.emit_fallible(
            *cur,
            Ty::Object,
            InstKind::New {
                class: class.clone(),
                ctor: None,
                args: Vec::new(),
            },
            env,
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
    /// (`rule:types/erased-member-access`). A shape value is anonymous and methodless, so there is
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
        absent: AbsentKey,
        nullsafe: bool,
        env: &mut Env,
        cur: &mut BlockId,
    ) -> (ValueId, Ty) {
        // A guarded read has no declared type to answer with, the field being
        // the one the shape does not prove present, so it takes the
        // representation the guarded subscript takes — and that is the `?T`
        // the checker has already given the expression.
        let field_ty = match absent {
            AbsentKey::Throws => lower_checked_ty(field.ty, self.checked_types),
            AbsentKey::Null => Ty::Tagged,
        };
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
                absent,
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

    /// `$obj->$key` — `rule:types/property-key-access`'s keyed read, which is
    /// [`Self::lower_shape_property_access`] with the name lowered rather than
    /// carried. [`InstKind::KeyGet`] owns why § 5 chose the erased access over
    /// a closed-set chain.
    ///
    /// Two temporaries can be staged here where the erased read stages one: the
    /// receiver, on the same fresh-producer rule, and the **key**, because
    /// `$obj->{$prefix . $field}` builds a string this frame then owns and
    /// [`InstKind::KeyGet`] only borrows. Both are released on whichever edge
    /// the read takes, and the result is retained first for the receiver's
    /// reason — the slot that owns it may be inside the value about to go.
    fn lower_keyed_property_access(
        &mut self,
        object: &Expr,
        property: &MemberName,
        ty: TypeId,
        nullsafe: bool,
        env: &mut Env,
        cur: &mut BlockId,
    ) -> (ValueId, Ty) {
        let field_ty = lower_checked_ty(ty, self.checked_types);
        let mark = self.temporaries_mark();
        let (object_v, receiver_ty, guard) =
            self.open_nullsafe(object, nullsafe, ReceiverProof::Erased, env, cur);
        let mut staged = receiver_ty.is_refcounted() && !self.aliasing_read(object);
        if staged {
            self.own_temporary(object_v);
        }
        let key_v = self.lower_key_name(property, env, cur, &mut staged);
        let (v, ty) = self.emit_fallible(
            *cur,
            field_ty,
            InstKind::KeyGet {
                object: object_v,
                key: key_v,
            },
            env,
        );
        if staged {
            if ty.is_refcounted() {
                self.emit_retain(*cur, v);
            }
            self.release_temporaries_since(mark, *cur);
        }
        self.close_nullsafe(guard, v, ty, env, cur)
    }

    /// The member name of an `rule:types/property-key-access` keyed access, lowered as the ordinary
    /// expression it is: a `property<T>` erases to [`Ty::Str`], so the operand's
    /// own value *is* the name and there is nothing to convert.
    ///
    /// Sets `staged` when the key is a fresh producer this frame now owns, so
    /// the caller knows a `release_temporaries_since` is owed — it is left as an
    /// out-parameter rather than returned because the receiver contributes to
    /// the same one answer.
    fn lower_key_name(
        &mut self,
        property: &MemberName,
        env: &mut Env,
        cur: &mut BlockId,
        staged: &mut bool,
    ) -> ValueId {
        let name = match property {
            MemberName::Variable(e) | MemberName::Expr(e) => e.as_ref(),
            // `nvs_types::expr::members::check_property_member` sends only the
            // two computed forms to `check_keyed_property`, and it is the only
            // thing that records the entry this is reached through. The arm is
            // a catch-all rather than `MemberName::Ident` alone because that
            // enum is `#[non_exhaustive]`.
            other => panic!(
                "nvs-ir: an `rule:types/property-key-access` keyed property entry over the member name {other:?}, \
                 which `nvs_types::expr::members::check_keyed_property` never records one for — \
                 it is reached from the two computed forms and nothing else"
            ),
        };
        let (key_v, key_ty) = self.lower_expr(name, Some(Ty::Str), env, cur);
        if key_ty.is_refcounted() && !self.aliasing_read(name) {
            self.own_temporary(key_v);
            *staged = true;
        }
        key_v
    }

    /// `$obj->$key = v;` — `rule:types/property-key-access`'s checked erased store, which is
    /// [`Self::lower_shape_property_assign`] with the name lowered rather than
    /// carried, and every ownership rule that function states for the reasons
    /// [`InstKind::KeySet`] restates.
    #[expect(
        clippy::too_many_arguments,
        reason = "`Self::lower_shape_property_assign`'s list, plus the member \
                  name the key arrives as — which is the whole difference \
                  between the two"
    )]
    pub(crate) fn lower_keyed_property_assign(
        &mut self,
        object: &Expr,
        property: &MemberName,
        ty: TypeId,
        value: &Stored<'_>,
        extra_owner: bool,
        env: &mut Env,
        cur: &mut BlockId,
    ) -> (ValueId, Ty) {
        let field_ty = lower_checked_ty(ty, self.checked_types);
        let mark = self.temporaries_mark();
        // The receiver is taken as it is found and never untagged, exactly as
        // `Self::lower_shape_property_assign` takes its own: `InstKind::KeySet`
        // checks the tag where it checks the name.
        let (object_v, receiver_ty) = self.lower_expr(object, None, env, cur);
        if receiver_ty.is_refcounted() && !self.aliasing_read(object) {
            self.own_temporary(object_v);
        }
        let mut staged = false;
        let key_v = self.lower_key_name(property, env, cur, &mut staged);
        let (v, vty, aliasing) = self.lower_stored(value, Some(field_ty), env, cur);
        let v = self.coerce(*cur, v, vty, field_ty, env);
        if field_ty.is_refcounted() && !aliasing {
            self.own_temporary(v);
        }
        self.emit_fallible(
            *cur,
            Ty::Void,
            InstKind::KeySet {
                object: object_v,
                key: key_v,
                value: v,
            },
            env,
        );
        if field_ty.is_refcounted() && extra_owner {
            self.emit_retain(*cur, v);
        }
        self.release_temporaries_since(mark, *cur);
        (v, field_ty)
    }

    /// `$issue->path = "x";` — [`Self::lower_shape_property_access`]'s write
    /// half (`rule:types/erased-member-access`), and the same three facts about the field: no
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
    pub(crate) fn lower_shape_property_assign(
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
    /// explicit `key =>`, and none is a `...spread`) takes the
    /// single-`ArrayNew` shape: each element's key is simply
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
    /// (`rule:types/arrays`). The subject is *borrowed*, so a freshly-built one is
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
        // `E0483`, because `rule:types/implicit-capture` and `rule:classes/two-copy-depths` between them leave an
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
    /// refused**, because it is `rule:types/conversion`'s unchecked position: a `mixed`
    /// defers whether there is an array here at all, which is `rule:types/erased-member-access`'s
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
        // `rule:types/erased-member-access`'s deferral, one storage kind along from a member
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
        // Both shapes take an error edge, and for different reasons: the
        // throwing one because an absent key is `rule:errors/propagation`'s own failure, the
        // `null`-answering one because the primitive still returns a status
        // and an uncatchable one has to leave the frame swept — see
        // `Inst::on_error`.
        let result = self.emit_fallible(*cur, result_ty, kind, env);
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

    /// `rule:types/type-test`'s `$x is T` — `bool` for every subject, and
    /// never a throw.
    ///
    /// **The checker's record says which of two shapes this is**, and reading
    /// it is not optional: `nvs_types::expr_table::ExprInfo::TypeTest` carries
    /// the interned right-hand side and means the answer is a run-time `bool`,
    /// while `ExprInfo::SettledTypeTest` carries the constant the checker
    /// folded to. The fold cannot be re-derived here, which is why it travels
    /// as a record — two settled tests can share both representations and fold
    /// opposite ways (`?int $x; $x is int|null` against `$x is string|float`),
    /// so the erasures this crate holds do not distinguish them. A folded test
    /// still **runs its subject**: `f() is int` calls `f`.
    ///
    /// **What the test costs is [`TestShape`]'s question, not this one's**: a
    /// tag comparison, or the descriptor walk `instanceof` and `as C` already
    /// emit. Against a tag row, a subject that is not a [`Ty::Tagged`] carries
    /// exactly one tag, so its answer is a constant — and always `false`,
    /// since a subject whose representation *is* the tested one folded at the
    /// checker. The arm is written as the comparison rather than as that
    /// constant because the comparison is the reason, and `int $n; $n is
    /// float` reaches it: the two types are not disjoint, so neither fold
    /// fires, and an `int` still does not carry a `float`'s tag.
    ///
    /// The subject is only read, so a fresh one nothing else owns is released
    /// once the test has read it — [`Self::lower_instanceof`]'s own rule, and
    /// the result being a [`Ty::Bool`] is what makes "right after" safe.
    fn lower_type_test(
        &mut self,
        inner: &Expr,
        expr: &Expr,
        env: &mut Env,
        cur: &mut BlockId,
    ) -> (ValueId, Ty) {
        let plan = match self.exprs.lookup(expr.span) {
            Some(&ExprInfo::TypeTest { tested }) => TypeTestPlan::AtRunTime(tested),
            Some(&ExprInfo::SettledTypeTest { answer }) => TypeTestPlan::Settled(answer),
            _ => panic!(
                "nvs-ir: an `is` at {:?} with neither a tested type nor a settled answer \
                 recorded in the typed-expression table — `nvs_types::expr::type_test` records \
                 one of the two for every test it accepts, so this is a checker that did not \
                 run or a record under a different span",
                expr.span
            ),
        };
        let (value, subject) = self.lower_expr(inner, None, env, cur);
        let result = match plan {
            TypeTestPlan::Settled(answer) => self.emit(*cur, Ty::Bool, InstKind::ConstBool(answer)),
            TypeTestPlan::AtRunTime(tested) => {
                let Some(shape) = test_shape(tested, self.checked_types, self.enums) else {
                    panic!(
                        "nvs-ir only lowers `is` against a scalar, `null`, `object`, a bare \
                         `array`, a class, a literal, an enum case, `iterable`, `callable`, or \
                         a union or intersection of those — got {:?}; a shape, an element type \
                         no tag decides and a written callable signature each still need a row \
                         of their own, and `test_shape`'s own known gap is which of those is a \
                         decision rather than a slice",
                        self.checked_types.get(tested)
                    );
                };
                self.emit_test_shape(shape, value, subject, expr.span, env, cur)
            }
        };
        if !self.aliasing_read(inner) && subject.is_refcounted() {
            self.emit_release(*cur, value);
        }
        result
    }

    /// One [`TestShape`]'s comparison, over a subject already lowered and read
    /// by nobody else — [`Self::lower_type_test`]'s arms, reachable a second
    /// time because a union's and an intersection's rows are their members'.
    ///
    /// The subject is only read here and released by the caller once, however
    /// many members read it, so nothing in this walk retains it. A row that
    /// retains something of its own releases it in the block it emitted it
    /// into, which is what lets a chain branch away between two members.
    fn emit_test_shape(
        &mut self,
        shape: TestShape,
        value: ValueId,
        subject: Ty,
        span: Span,
        env: &mut Env,
        cur: &mut BlockId,
    ) -> (ValueId, Ty) {
        match shape {
            TestShape::Tag(repr) if subject == Ty::Tagged => self.emit(
                *cur,
                Ty::Bool,
                InstKind::TagIs {
                    operand: value,
                    repr,
                },
            ),
            TestShape::Tag(repr) => self.emit(*cur, Ty::Bool, InstKind::ConstBool(subject == repr)),
            // A literal type is two comparisons rather than one: the tag says
            // the payload word may be read at this representation, and the
            // payload says whether it holds the one value the type is. They
            // are `&&`-shaped and not folded together because the second must
            // not run where the first missed — a `string` literal's compare is
            // `nvs_str_eq` through two pointers, and the payload of a value
            // tagged anything else is not one.
            TestShape::Literal { repr, atom } if subject == Ty::Tagged => {
                let tagged = *cur;
                let (carries, _) = self.emit(
                    tagged,
                    Ty::Bool,
                    InstKind::TagIs {
                        operand: value,
                        repr,
                    },
                );
                let (missed, _) = self.emit(tagged, Ty::Bool, InstKind::ConstBool(false));
                let payload = self.new_block();
                let merge = self.new_block();
                let payload_edge = self.ids.next_edge(span);
                let missed_edge = self.ids.next_edge(span);
                self.seal(
                    tagged,
                    Terminator::Branch {
                        cond: carries,
                        then_block: payload,
                        then_edge: payload_edge,
                        else_block: merge,
                        else_edge: missed_edge,
                    },
                );
                // The unchecked narrowing [`InstKind::Untag`] is for, with the
                // branch above as the proof — and the proof holds on this edge
                // alone, which is the whole reason for the block. Nothing is
                // merged but the answer: both edges assign nothing, so there
                // is no `Env` to reconcile, exactly as
                // [`Self::lower_literal_membership`]'s chain has none.
                let (narrowed, _) = self.emit(
                    payload,
                    payload_repr(repr),
                    InstKind::Untag { operand: value },
                );
                let (equal, _) = self.literal_payload_eq(payload, narrowed, &atom);
                self.seal(payload, Terminator::Jump(merge));
                let answer = self.emit(
                    merge,
                    Ty::Bool,
                    InstKind::Phi {
                        incoming: vec![(tagged, missed), (payload, equal)],
                    },
                );
                *cur = merge;
                answer
            }
            // A subject carrying exactly one tag has answered the first
            // comparison already, so the payload one stands alone: `int $n; $n
            // is 5` is one machine compare, and no fold at the checker could
            // have settled it — the two types are not disjoint and neither
            // contains the other.
            TestShape::Literal { repr, atom } if subject == repr => {
                // The reinterpret is an enum case's, for the reason
                // [`Self::reinterpret_enum_to_backing`] gives; every other atom
                // already arrives at the representation it compares at, where
                // that call is the identity.
                let (narrowed, _) = self.reinterpret_enum_to_backing(value, subject, cur);
                self.literal_payload_eq(*cur, narrowed, &atom)
            }
            // Every other representation carries a tag this literal's is not,
            // which is the class and element rows' constant reached for the
            // same reason.
            TestShape::Literal { .. } => self.emit(*cur, Ty::Bool, InstKind::ConstBool(false)),
            // The walk `instanceof` and `as C` already emit, on the two
            // representations that can reach a descriptor at all. Every other
            // subject holds no object, so the answer is a constant — a `mixed`
            // is the [`Ty::Tagged`] arm and a scalar's disjointness folded at
            // the checker, which leaves this branch reachable only if a fold is
            // ever weakened.
            TestShape::Class(name) if matches!(subject, Ty::Object | Ty::Tagged) => self.emit(
                *cur,
                Ty::Bool,
                InstKind::InstanceOf {
                    value,
                    class: TestedClass::Named(name),
                },
            ),
            TestShape::Class(_) => self.emit(*cur, Ty::Bool, InstKind::ConstBool(false)),
            // `as ?array<T>` is the walk that answers rather than throws, so
            // this is that lowering with the value thrown away and its absence
            // read as the answer — one `Helper::ToArrayOfOrNull` and no second
            // walk anywhere in the tree. The result carries a reference of its
            // own (`nvs_runtime::helpers`' `to_array_of` retains), so it is
            // released as soon as the tag has been read; releasing the `null`
            // it answers with on the false edge is the no-op every other `?T`
            // consumer relies on.
            TestShape::ArrayOf(tags) if matches!(subject, Ty::Array | Ty::Tagged) => {
                let (word, _) = self.emit(*cur, Ty::Uint, InstKind::ConstUint(tags));
                let (walked, _) = self.emit_fallible(
                    *cur,
                    Ty::Tagged,
                    InstKind::HelperCall {
                        helper: Helper::ToArrayOfOrNull,
                        args: vec![value, word],
                    },
                    env,
                );
                let (absent, _) = self.emit(*cur, Ty::Bool, InstKind::IsNull { operand: walked });
                self.emit_release(*cur, walked);
                self.emit(
                    *cur,
                    Ty::Bool,
                    InstKind::UnOp {
                        op: UnOp::Not,
                        operand: absent,
                    },
                )
            }
            // Nothing else holds an array, so the walk would answer one
            // constant — see the class row above, which reaches its own for the
            // same reason.
            TestShape::ArrayOf(_) => self.emit(*cur, Ty::Bool, InstKind::ConstBool(false)),
            TestShape::Any(members) => self.emit_test_chain(
                members,
                true,
                TestSubject {
                    value,
                    repr: subject,
                    span,
                },
                env,
                cur,
            ),
            TestShape::All(members) => self.emit_test_chain(
                members,
                false,
                TestSubject {
                    value,
                    repr: subject,
                    span,
                },
                env,
                cur,
            ),
        }
    }

    /// `rule:types/type-test`'s union and intersection rows: each member's own
    /// test in turn, stopping at the first one that decides the answer.
    ///
    /// `decided` is what stopping early answers with — `true` for a union,
    /// whose first `true` is the whole answer, and `false` for an
    /// intersection, whose first `false` is. That single parameter is the only
    /// difference between the two, which is why there is one chain and not
    /// two: they are the same walk with the branch's edges swapped, and a
    /// second copy is the one that eventually disagrees.
    ///
    /// The shape is [`Self::lower_or`]'s and [`Self::lower_and`]'s, with the
    /// operands already in hand instead of lowered per edge, and no `Env` is
    /// merged: a member test binds nothing, so the early edges carry only the
    /// constant they decided on — [`Self::lower_literal_membership`]'s chain
    /// has no environment to reconcile for the same reason.
    ///
    /// Each member reads the subject and owns none of it, so a chain holds
    /// nothing across an edge and the caller's single release still covers it.
    fn emit_test_chain(
        &mut self,
        members: Vec<TestShape>,
        decided: bool,
        subject: TestSubject,
        env: &mut Env,
        cur: &mut BlockId,
    ) -> (ValueId, Ty) {
        // `nvs_types::ty::TypeInterner::make_union` collapses a one-member
        // union to that member and never interns an empty one, so there is
        // always a last member for the chain to end on.
        debug_assert!(
            !members.is_empty(),
            "a union or intersection interns with at least two members"
        );
        let merge = self.new_block();
        let last = members.len().saturating_sub(1);
        let mut incoming = Vec::with_capacity(members.len());
        for (index, member) in members.into_iter().enumerate() {
            let (answer, _) =
                self.emit_test_shape(member, subject.value, subject.repr, subject.span, env, cur);
            // The last member is not a branch: whatever it answered is the
            // chain's answer, every earlier one having declined to stop.
            if index == last {
                incoming.push((*cur, answer));
                self.seal(*cur, Terminator::Jump(merge));
                break;
            }
            let (early, _) = self.emit(*cur, Ty::Bool, InstKind::ConstBool(decided));
            incoming.push((*cur, early));
            let next = self.new_block();
            let early_edge = self.ids.next_edge(subject.span);
            let next_edge = self.ids.next_edge(subject.span);
            let (then_block, then_edge, else_block, else_edge) = if decided {
                (merge, early_edge, next, next_edge)
            } else {
                (next, next_edge, merge, early_edge)
            };
            self.seal(
                *cur,
                Terminator::Branch {
                    cond: answer,
                    then_block,
                    then_edge,
                    else_block,
                    else_edge,
                },
            );
            *cur = next;
        }
        *cur = merge;
        self.emit(merge, Ty::Bool, InstKind::Phi { incoming })
    }

    /// The payload half of `rule:types/type-test`'s literal row: a value
    /// already at the representation its tag names, against the one constant
    /// the literal type is.
    ///
    /// [`Self::lower_literal_membership`]'s arm without the chain — one atom
    /// rather than a set, and an answer rather than a branch to a throw —
    /// over the same [`super::convert::literal_constant`] table, so `$x as
    /// 'yay'` and `$x is 'yay'` compare the identical way.
    fn literal_payload_eq(
        &mut self,
        block: BlockId,
        value: ValueId,
        atom: &LiteralAtom,
    ) -> (ValueId, Ty) {
        let (kind, ty) = literal_constant(atom);
        let (wanted, _) = self.emit(block, ty, kind);
        let equal = self.emit(
            block,
            Ty::Bool,
            InstKind::BinOp {
                op: BinOp::Eq,
                lhs: value,
                rhs: wanted,
            },
        );
        // The constant is fresh and this comparison is its one and only use —
        // the policy [`Self::lower_literal_membership`] applies to its own.
        if ty.is_refcounted() {
            self.emit_release(block, wanted);
        }
        equal
    }

    /// `$x instanceof Name` — the tested class comes from
    /// `self.exprs`, exactly like a property access's declaring class,
    /// because resolving a bare `Animal` to `Ns\Animal` needs the
    /// namespace/import context this crate cannot see. A `Core` class, an
    /// enum and an undeclared name all record nothing and are refused at
    /// the checker (`E0496`/`E0303`), so the miss below is an
    /// internal-consistency failure rather than a hole.
    ///
    /// **A right-hand side that is not a written name is `rule:types/class-reference-sites`'s
    /// `$x instanceof $cls`**, and it records nothing either — for the
    /// opposite reason. There is no name to resolve: the operand is a
    /// `class<T>`, so the descriptor to test against is the value it
    /// evaluates to, and which form this site takes is decided by the shape
    /// of the expression rather than by an entry. The checker refuses every
    /// *other* dynamic spelling where it is written (`E0496`), which is what
    /// makes reading the syntax sufficient here.
    ///
    /// **The subject may be a [`Ty::Tagged`], and the runtime checks its
    /// tag.** A `mixed` or an untested `?Box` is the shape `instanceof`
    /// exists for, so it travels as a whole value by address exactly as
    /// `rule:types/erased-member-access`'s name-keyed access does, and a tag that is not an
    /// object answers `false` rather than throwing — PHP's own answer,
    /// and the one every subject whose *declared* type can hold no
    /// object gets at compile time instead (`E0497`).
    fn lower_instanceof(
        &mut self,
        inner: &Expr,
        class: &Expr,
        expr: &Expr,
        env: &mut Env,
        cur: &mut BlockId,
    ) -> (ValueId, Ty) {
        let (value, ty) = self.lower_expr(inner, None, env, cur);
        if !matches!(ty, Ty::Object | Ty::Tagged) {
            guarded_by!(
                code::E_INSTANCEOF_SUBJECT_NOT_OBJECT,
                "nvs-ir reached `instanceof` over representation {ty:?}. A subject whose declared \
                 type can hold no object has its answer before the program runs, so the checker \
                 refuses it where it is written rather than lowering a test that is constantly \
                 `false`; what does reach here is an object or the `Ty::Tagged` this method's doc \
                 comment describes"
            );
        }
        // The subject is written first and evaluated first; the class side is
        // second, and for the dynamic form it is an expression of its own.
        let tested = match &class.kind {
            ExprKind::ConstFetch(_) => {
                let Some(ExprInfo::InstanceOf { class }) = self.exprs.lookup(expr.span) else {
                    panic!(
                        "nvs-ir: an `instanceof` at {:?} has no resolved class recorded in the \
                         typed-expression table — it wasn't checked with the same table, every \
                         right-hand side naming no declared class being `E0496` or `E0303` at \
                         the checker",
                        expr.span
                    );
                };
                TestedClass::Named(class.to_string())
            }
            // The class side needs no lifecycle: [`Ty::ClassDesc`] is not
            // refcounted, the same reason
            // [`Self::lower_static_call_on_a_class_reference`] lowers its own
            // and moves on.
            _ => {
                let (desc, _) = self.lower_expr(class, Some(Ty::ClassDesc), env, cur);
                TestedClass::Descriptor(desc)
            }
        };
        let result = self.emit(
            *cur,
            Ty::Bool,
            InstKind::InstanceOf {
                value,
                class: tested,
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

    /// `rule:classes/clone-is-shallow`: PHP's shallow, same-heap, single-level copy, with
    /// no `__clone` hook to run — so the whole operation is one
    /// instruction, and the result is a fresh object with exactly one
    /// owner, the same as `new`.
    ///
    /// # A tagged operand
    ///
    /// A [`Ty::Tagged`] operand arrives here on purpose. `nvs_types`'
    /// `expr::members::reject_non_object_clone` refuses a type that can hold no
    /// object and passes `mixed`, `object` and every union — `?Foo` among them —
    /// because a value that is one at run time names the class to instantiate.
    /// [`Self::guard_cloneable`] asks the one question that leaves, and the copy
    /// itself is then the same single instruction either way.
    ///
    /// # Panics
    ///
    /// Panics naming the operand's representation if it is neither
    /// [`Ty::Object`] nor [`Ty::Tagged`]. Those two are what
    /// `reject_non_object_clone`'s `can_hold_an_object` predicate lets past, so
    /// a third is a bug in this crate and not a shape a program can write.
    fn lower_clone_expr(
        &mut self,
        inner: &Expr,
        env: &mut Env,
        cur: &mut BlockId,
    ) -> (ValueId, Ty) {
        let (v, ty) = self.lower_expr(inner, None, env, cur);
        assert!(
            matches!(ty, Ty::Object | Ty::Tagged),
            "`clone`'s operand is a `Ty::Object` or a `Ty::Tagged`, the pair \
             `nvs_types::expr::members::reject_non_object_clone` admits — representation \
             {ty:?} means this is a bug in nvs-ir"
        );
        let borrowed = self.aliasing_read(inner);
        let object = self.guard_cloneable(v, ty, borrowed, inner.span, env, cur);
        let result = self.emit(*cur, Ty::Object, InstKind::Clone { object });
        // The operand is only *read* — see `InstKind::Clone`. A fresh
        // one nothing else owns is released right after, the same
        // "release a fresh value once its one and only use is done"
        // rule `Self::concat_operand`'s caller applies, and a tagged one is
        // released through the object [`InstKind::Untag`] carried the
        // reference onto.
        if !borrowed {
            self.emit_release(*cur, object);
        }
        result
    }

    /// The tag question a [`Ty::Tagged`] `clone` operand's type did not settle,
    /// asked in front of the copy: whether there is an object to copy at all.
    /// `cur` is left on the block where there is, and the value handed back is
    /// that object. A [`Ty::Object`] operand is handed straight back, its class
    /// being the checker's own answer.
    ///
    /// **The wording is PHP's, word for word**, and it is rendered in
    /// `nvs_runtime`'s `nvs_clone_not_an_object` rather than here because PHP
    /// names the type it was given, which is a tag the operand carries and not
    /// a type this crate holds.
    /// `rule:php-migration/every-divergence-is-deliberate-and-listed` lists no
    /// divergence here, so a program that catches this reads what it reads in
    /// PHP.
    ///
    /// That helper never returns, so it takes the operand's reference with it
    /// and a borrowed operand is retained in front of the call: an instruction
    /// emitted after a call that never returns sits in a block only an `Ok`
    /// would reach, which is the same inversion [`Helper::LiteralMismatch`]
    /// carries for its own rendered argument.
    ///
    /// **What it spends** (`rule:programs/memory-priority`): the one tag compare
    /// [`Self::split_on_object_tag`] states, and no allocation on the path where
    /// the tag holds.
    fn guard_cloneable(
        &mut self,
        v: ValueId,
        ty: Ty,
        borrowed: bool,
        span: Span,
        env: &mut Env,
        cur: &mut BlockId,
    ) -> ValueId {
        if ty != Ty::Tagged {
            return v;
        }
        let (object, not_an_object) = self.split_on_object_tag(v, span, cur);
        if borrowed {
            self.emit_retain(not_an_object, v);
        }
        let landing = self.landing_block(env);
        self.block_insts[not_an_object.index() as usize].push(Inst {
            result: None,
            ty: None,
            kind: InstKind::HelperCall {
                helper: Helper::CloneOperandNotAnObject,
                args: vec![v],
            },
            on_error: Some(landing),
        });
        // `Helper::CloneOperandNotAnObject` never returns normally, so this jump
        // is unreachable — written anyway because a block still owes a
        // terminator, and the block where the tag held is where control would
        // have gone.
        self.seal(not_an_object, Terminator::Jump(*cur));
        object
    }
}

/// What [`Lowering::open_nullsafe`] is allowed to assume about the tag of a
/// [`Ty::Tagged`] receiver — the one question a member access asks that
/// nothing else in this file does.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum ReceiverProof {
    /// `nvs_types` proved the tag before this ever ran: a narrowed `?T`, the
    /// non-`null` arm of a `?->`, a receiver whose declared type is a class.
    /// The tagged slot is one [`InstKind::Untag`] away from the object, and
    /// that untag is unchecked on purpose — see [`Lowering::untag_receiver`].
    Proven,
    /// Nothing proved it: the receiver is a `mixed`, `rule:types/conversion`'s one
    /// unchecked position, so the value reaching the member may hold any tag
    /// at all. No `Untag` is emitted — an unchecked one over an `int` payload
    /// is a pointer this frame would then dereference — and the tagged value
    /// travels to [`InstKind::SlotGet`], which checks the tag where it
    /// already checks the name (`rule:types/erased-member-access`).
    Erased,
}

/// The one value every member of an [`TestShape::Any`]/[`TestShape::All`] chain
/// reads, carried as one argument because a chain hands all three parts back
/// down to [`Lowering::emit_test_shape`] unchanged.
#[derive(Clone, Copy)]
struct TestSubject {
    /// The lowered subject itself, owned by [`Lowering::lower_type_test`] and
    /// by no member that reads it.
    value: ValueId,
    /// The representation it arrived at, which is what decides a tag row and
    /// which of a literal row's two halves runs.
    repr: Ty,
    /// The `is` expression's own span, for the edges a chain seals.
    span: Span,
}

/// Which of `rule:types/type-test`'s two shapes one `$x is T` is, read off the
/// checker's own record before the subject is lowered — see
/// [`Lowering::lower_type_test`], whose `&mut self` is why the record cannot
/// simply be matched in place.
enum TypeTestPlan {
    /// The answer is a run-time `bool`, tested against this interned type.
    AtRunTime(TypeId),
    /// The checker settled the answer. The subject still runs.
    Settled(bool),
}

/// What one `$x is T` costs at run time, which is the whole of what
/// [`Lowering::lower_type_test`] decides — see `rule:types/type-test`'s own
/// table, which states these costs because a reader has to see them before
/// writing a test inside a loop.
enum TestShape {
    /// One tag comparison, against the tag this representation carries.
    Tag(Ty),
    /// The descriptor walk `instanceof` and `as C` already emit, against this
    /// class or interface label. Never a second walk of its own.
    Class(String),
    /// The O(n) element walk `as array<T>` already pays for, against
    /// [`super::array_element_tags`]' word — one tag nibble per level of `T`.
    /// Never a second walk of its own either: the spelling that *answers*
    /// instead of throwing is `as ?array<T>`'s, over the same helper.
    ArrayOf(u64),
    /// A union: each member's own row in turn, stopping at the first that
    /// answers `true`. Never a cost of its own — `$x is int|string` is the two
    /// tag comparisons its members are, and a member expensive on its own is
    /// expensive here for exactly the reason it is alone.
    Any(Vec<TestShape>),
    /// An intersection: [`Self::Any`]'s mirror, stopping at the first member
    /// that answers `false`. `$x is Countable&Traversable` is therefore two
    /// descriptor walks at worst and one whenever the first declines.
    All(Vec<TestShape>),
    /// One tag comparison, and a payload compare behind it. A literal type
    /// names a single value, so the tag only says the payload word is
    /// readable at this representation and the compare says whether it is
    /// that value — `rule:types/literal-types`' two halves, over the atom
    /// `as` already reduces the same type to.
    ///
    /// `rule:types/enum-case-type`'s case is this row too, one representation
    /// down: `rule:enums/representation` makes a case its backing integer, so
    /// the tag is that integer's and the compare is against the constant the
    /// run's enum table holds. That rule's own stated consequence rides along
    /// — a value that reached `mixed` is not distinguishable there from its
    /// backing integer, so a `mixed` holding `1` answers `is Rank::Silver`
    /// exactly as one holding `Rank::Silver` does, and the reserved enum tag
    /// is what would separate them.
    Literal {
        /// The representation whose tag the payload compare needs proved
        /// first, and — through [`payload_repr`] — the one the payload is
        /// then read at.
        repr: Ty,
        /// The value that payload has to hold.
        atom: LiteralAtom,
    },
}

/// Which shape `$x is T` takes, or `None` for a row that has neither yet.
///
/// The tag rows are the ones that cost one comparison: a scalar, `null`, plain
/// `object` and a bare `array`. The class row is the descriptor walk, the
/// element row is the array walk, and the literal row — an enum case included
/// — is one tag comparison with a payload compare behind it. A union, an
/// intersection and `iterable` are their members' rows chained, so none of the
/// three is a cost of its own, and `callable` is the class row against the one
/// label every closure's environment class conforms to
/// ([`CLOSURE_MARKER`](super::CLOSURE_MARKER)).
///
/// `mixed` is not here and cannot arrive: it holds every value, so the checker
/// folded that test to `true`.
///
/// # Known gaps
///
/// Three rows answer `None`, and each reaches the caller's panic rather than a
/// diagnostic.
///
/// A **shape** has no row here, and what it waits on is a walk rather than a
/// decision about `rule:types/type-test`'s table. Neither walk this compiler
/// already has is the one: `as` performs none at all, `convert.rs`'s
/// `lower_checked_downcast` having no shape arm, and `Core\Arr::shapeAs`'s
/// reads an `NvsArray` and builds an object out of it, so it answers a
/// different question from a different source — its
/// `nvs_stdlib::json::Reading::Wire` half is the strict per-field read `is`
/// wants, over a document's keys rather than a receiver's fields.
/// `rule:types/shape-type` makes a shape compile-time-only and structural with
/// width subtyping, so the run-time question is whether *this object* carries
/// the named fields at the named types: an O(n) walk over
/// [`InstKind::SlotGet`]'s name-keyed fetch, one [`TestShape`] per field,
/// chained by [`TestShape::All`] behind the object tag so that a subject
/// holding no object declines before any field is read.
///
/// That walk needs one thing the IR does not carry — a **presence probe that
/// does not throw**. `AbsentKey::Null` answers an absent field with `null`,
/// which a `{a: ?int}` field cannot tell from an `a` holding one, and both it
/// and `AbsentKey::Throws` throw on a slot that was never written. `is` is
/// total, so a subject missing a field answers `false` and raises nothing.
///
/// The other two are slices. An `array<T>` whose element type no tag decides —
/// `array<Foo>`, an array of shapes, an array of unions — has no tag word for
/// the walk to take. `rule:types/callable-signature`'s written signature asks
/// what a closure's parameters are and not merely whether the value is one, so
/// the marker `callable` walks does not answer it. `as array<Foo>` is refused
/// where it is written (`E0711`) and `is array<Foo>` is not.
fn test_shape(
    tested: TypeId,
    checked_types: &TypeInterner,
    enums: &EnumTable,
) -> Option<TestShape> {
    // `QName` is destructured rather than named, for
    // `super::closure::declared_class`'s reason: `nvs-hir` is a
    // dev-dependency of this crate.
    if let CheckedTy::Class(qname, _) = checked_types.get(tested) {
        return Some(TestShape::Class(qname.to_string()));
    }
    // A union and an intersection are their members' own rows and add no test
    // of their own; `crate::lower` chains them, answering at the first member
    // that decides. Canonicalization has already flattened and deduplicated
    // the members (`nvs_types::ty::TypeInterner::make_union`), so the chain is
    // as short as the written type allows. One member with no row of its own
    // makes the whole type `None`, which keeps `is array<Foo>|int` a single
    // known gap rather than a chain half of which lowers.
    match checked_types.get(tested) {
        CheckedTy::Union(members) => {
            return members
                .iter()
                .map(|member| test_shape(*member, checked_types, enums))
                .collect::<Option<Vec<_>>>()
                .map(TestShape::Any);
        }
        CheckedTy::Intersection(members) => {
            return members
                .iter()
                .map(|member| test_shape(*member, checked_types, enums))
                .collect::<Option<Vec<_>>>()
                .map(TestShape::All);
        }
        // `iterable` holds exactly what `rule:iteration/foreach-subjects`
        // accepts, and each of those is a row that already exists — so this is
        // a chain too, and adds no test of its own. The array tag goes first
        // because it is the one member that reaches no descriptor, then
        // `rule:iteration/two-interfaces`' two interfaces through the walk
        // `instanceof` already emits. Both labels are spelled rather than named
        // for `super::closure::declared_class`'s reason, and a descriptor for
        // each is in every program's class table whether or not the file
        // implements one (`super::lower_program`).
        CheckedTy::Iterable => {
            return Some(TestShape::Any(vec![
                TestShape::Tag(Ty::Array),
                TestShape::Class("Iterable".to_owned()),
                TestShape::Class("Iterator".to_owned()),
            ]));
        }
        // `rule:types/callable-is-a-closure`: a closure satisfies `callable`
        // and no other value does, so the question is whether the subject is an
        // object of one of the environment classes `super::closure`
        // synthesizes — which is what the marker edge on each of them says.
        // That makes this the descriptor walk `instanceof` already emits, with
        // no field read, no tag of its own and no second table.
        CheckedTy::Callable => {
            return Some(TestShape::Class(super::CLOSURE_MARKER.to_owned()));
        }
        _ => {}
    }
    // A literal type carries its own value, so its atom is built here — all
    // but the enum case's, which `rule:types/enum-case-type` deliberately
    // keeps out of the type (§ 3) and which the run's own enum table holds
    // instead.
    let literal = match checked_types.get(tested) {
        CheckedTy::StringLiteral(text) => Some((Ty::Str, LiteralAtom::Str(text.clone()))),
        CheckedTy::IntLiteral(value) => Some((Ty::Int, LiteralAtom::Int(*value))),
        // `rule:types/grammar`'s two `bool` singletons, which are types here
        // and not values: `$x is true` is this row, `$x == true` is not.
        CheckedTy::True => Some((Ty::Bool, LiteralAtom::Bool(true))),
        CheckedTy::False => Some((Ty::Bool, LiteralAtom::Bool(false))),
        CheckedTy::EnumCase(qname, backing, case) => {
            let value = enums.case(qname, case).unwrap_or_else(|| {
                panic!(
                    "nvs-ir: `{qname}::{case}` is an interned enum-case type with no entry in \
                     the run's enum table — `nvs_types` interns one only for a case it \
                     resolved, so the two tables disagree"
                )
            });
            let repr = match backing {
                nvs_types::EnumBacking::Int => Ty::Enum(EnumRepr::Int),
                nvs_types::EnumBacking::Uint => Ty::Enum(EnumRepr::Uint),
            };
            Some((repr, LiteralAtom::EnumCase(value)))
        }
        _ => None,
    };
    if let Some((repr, atom)) = literal {
        return Some(TestShape::Literal { repr, atom });
    }
    Some(TestShape::Tag(match checked_types.get(tested) {
        CheckedTy::Bool => Ty::Bool,
        CheckedTy::Int => Ty::Int,
        CheckedTy::Uint => Ty::Uint,
        CheckedTy::Float => Ty::Float,
        CheckedTy::Decimal => Ty::Decimal,
        CheckedTy::String => Ty::Str,
        CheckedTy::Bytes => Ty::Bytes,
        CheckedTy::Null => Ty::Null,
        // Plain `object` and nothing narrower: `rule:types/grammar`'s opaque
        // top of every class type is exactly "carries `Tag::Object`", while a
        // `Class` erases to the same representation and still owes the
        // descriptor walk.
        CheckedTy::Object => Ty::Object,
        // A bare `array` is `array<mixed>` (`nvs_types::lower::lower_type`),
        // and every value carrying `Tag::Array` holds one.
        CheckedTy::Array(element) if matches!(checked_types.get(*element), CheckedTy::Mixed) => {
            Ty::Array
        }
        // A named element type is the walk instead, and the tag word is the
        // whole of what the helper takes. A `None` here is an element no tag
        // decides — a class, a shape, a `callable`, an enum, a literal, a
        // union — which `as array<T>` refuses where it is written (`E0711`)
        // and `is` does not yet: see this function's own known gap.
        CheckedTy::Array(_) => {
            return Some(TestShape::ArrayOf(array_element_tags(
                tested,
                checked_types,
            )?));
        }
        // A qualified atom never reaches here: `is tainted string` is `E0813`,
        // there being no run-time bit to read.
        _ => return None,
    }))
}

/// The representation one [`TestShape::Literal`]'s payload compare happens at,
/// which is the tested representation itself for every atom but an enum
/// case's.
///
/// A case compares one representation down for
/// [`Lowering::reinterpret_enum_to_backing`]'s reason — `nvs-codegen`'s binary
/// operator table carries no [`Ty::Enum`] row — and the tag is unaffected,
/// `nvs_codegen::ty::tag_of` already answering an enum with its backing type's
/// (`rule:enums/representation`).
fn payload_repr(repr: Ty) -> Ty {
    match repr {
        Ty::Enum(EnumRepr::Int) => Ty::Int,
        Ty::Enum(EnumRepr::Uint) => Ty::Uint,
        other => other,
    }
}

pub(crate) struct NullsafeGuard {
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
