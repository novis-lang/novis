//! A parameter's default value — the constant a call site materializes when
//! it leaves that parameter out.
//!
//! # Why a constant, and not the expression
//!
//! PHP evaluates a parameter default *in the callee*, once per call, from an
//! arbitrary constant expression. Novis evaluates it **here, once, at signature
//! collection**, and records the resulting [`ConstArg`] in
//! [`MethodSig::defaults`](crate::signatures::MethodSig::defaults) so the
//! *caller* can emit it as an ordinary literal argument. Three things fall out
//! of that, all of them the reason to do it this way:
//!
//! * **The callee needs no second entry point.** Every compiled function keeps
//!   exactly one arity, so nothing in `nvs-ir`, `nvs-codegen` or the ADR 0002
//!   call ABI learns about defaults at all — `nvs_ir::lower::lower_call_args`
//!   pushes one more `ConstInt`/`ConstStr` and stops.
//! * **A default cannot observe anything.** It is a constant, so it cannot
//!   read a global (there are none — ADR 0008), call a function, or differ
//!   between two calls that both omitted it.
//! * **A `Core` member's default and a user-declared one are one mechanism.**
//!   `nvs_stdlib::registry` states a `Core` default as data
//!   (`nvs_stdlib::registry::Const`); [`crate::core_lib`] translates it into
//!   the same [`ConstArg`] this module evaluates a written `= expr` into, the
//!   same way it already translates `CoreTy` into [`crate::ty::Ty`].
//!
//! # What a default may be
//!
//! A literal of the parameter's own declared type — `bool`, `int`, `uint`,
//! `float`, `string` — optionally negated (`= -1`). Anything else is
//! `E_PARAM_DEFAULT_NOT_LITERAL`, naming what is accepted.
//!
//! # A *property*'s default is the same decoder, one position along
//!
//! [`eval_property_default`] evaluates `public int $n = 4;` with exactly the
//! same literal grammar, plus `= []` and plus the two *named* constants
//! [ADR 0046](../../../docs/adr/0046-attributes-shape-literal-metadata.md) § 2
//! puts in the compile-time constant set beside a literal — another class's
//! `const` and an enum case ([`const_reference_default`]) — and reports
//! `E_PROPERTY_DEFAULT_NOT_LITERAL` instead. Where it *goes* is the whole
//! difference: a parameter default is emitted by the caller, while a property
//! default is copied onto `nvs_runtime::ClassDesc` and written into every
//! fresh instance's slot by `nvs_runtime::NvsObj::new`. `nvs_ir` emits no
//! instruction for it at all — `nvs_ir::ir::InstKind::New` carries the
//! constructor call, so there is no site between allocation and construction
//! for an initializer to be spliced into, and a constructor prologue would run
//! the *declaring* class's defaults rather than the instantiated class's.
//!
//! **Known gap:** a *written* `= null` is refused along with the rest. The
//! constant itself now exists — [`ConstArg::Null`], over
//! `nvs_ir::ir::InstKind::ConstNull` — but the thing a written one would
//! declare is a `?T` parameter, and `nvs_ir::ty::Ty`'s own docs record why a
//! type that admits both `null` and a `T` has no IR representation yet. So
//! this stays refused until that lands, and the constant is reached only from
//! [`crate::core_lib`], where the *declared* type is the option's own and
//! `null` means "not given".
//!
//! **Known gap, and it is the parameter half only:** a named constant — an
//! enum case (`Mode $m = Mode::Fast`), another class's `const` — is accepted
//! at a *property* default and still refused at a *parameter* one. The
//! difference is where the constant lands. A property default becomes a
//! `nvs_runtime::FieldDefault` materialized straight into a slot, where an
//! enum case *is* the integer it was folded to and no IR type carries the
//! distinction; a parameter default is emitted at the omitting call site by
//! `nvs_ir::lower::emit_const_arg`, which would have to hand a
//! [`ConstArg::Int`] to a `nvs_ir::ty::Ty::Enum` position. Widening
//! [`literal_default`] itself is what closes that, once the emitter carries
//! the position's own IR type rather than the constant's.
//!
//! A class constant is also only as wide as [`crate::consts`] folds it: an
//! integer whose magnitude no `int` holds has no folded value at all
//! (ADR 0047 § 1), so `uint $n = Limits::MAX;` above `i64::MAX` is refused
//! here even though the literal `= 18446744073709551615` is accepted.
//!
//! **A `decimal` default is refused, and is now the shortest thing on this
//! list to build:** `nvs_ir::ir::InstKind::ConstDecimal` exists, so
//! [`ConstArg`]'s one-variant-per-instruction rule above is satisfied by a
//! variant carrying that instruction's own three parts. Until one is written,
//! `decimal $vat = 0.19` is `E_PARAM_DEFAULT_NOT_LITERAL` — a clean refusal,
//! not a wrong constant.

use nvs_diagnostics::{Diagnostic, Span, code};
use nvs_syntax::ast::{Expr, ExprKind, UnaryOp};

use crate::ty::{Ty, TypeId};
use crate::{Ctx, Env};

/// A parameter default's already-evaluated value, in the parameter's own
/// declared type.
///
/// Deliberately one variant per `nvs_ir::ir::InstKind` constant rather than a
/// general value: this exists to be *emitted*, and a shape with no instruction
/// under it could be recorded here and then fail at lowering, which is exactly
/// the class of failure `nvs_stdlib::registry::CoreTy`'s own docs give as the
/// reason that enum is closed too.
#[derive(Clone, Debug, PartialEq)]
pub enum ConstArg {
    /// `null` — what an **absent** argument is.
    ///
    /// Produced only by [`crate::core_lib`], from
    /// `nvs_stdlib::registry::Const::Null`, and only for an ADR 0063 R2
    /// option whose spec signature gives it no "not given" spelling of its
    /// own: `Core\Arr::sort`'s `by` and `comparator` are the first two. A
    /// written `= null` is still refused — see this module's own known gap,
    /// which is about the `?T` *parameter type* that would declare it, not
    /// about the constant.
    Null,
    /// `bool`
    Bool(bool),
    /// `int`
    Int(i64),
    /// `uint`
    Uint(u64),
    /// `float`
    Float(f64),
    /// `string`, already cooked — escapes resolved by
    /// [`crate::string_lit::cook_string_literal`], the same routine every
    /// other string literal in the program goes through, so a default is never
    /// a second escape grammar.
    Str(String),
    /// `bytes`, as the octets themselves.
    ///
    /// Produced only by [`crate::core_lib`], from
    /// `nvs_stdlib::registry::Const::Bytes`: there is no `bytes` literal in
    /// the language ([ADR 0009](../../../docs/adr/0009-string-and-bytes.md)
    /// § 1), so no *written* default can reach this variant, and
    /// [`literal_default`] does not produce it. It is kept apart from
    /// [`Self::Str`] because the two materialize under different runtime tags,
    /// which is the whole difference between the types.
    Bytes(Vec<u8>),
    /// The empty array, `[]`.
    ///
    /// Produced by [`crate::core_lib`], from
    /// `nvs_stdlib::registry::Const::EmptyArray`, which owns why the *only*
    /// array constant is the empty one — and by [`eval_property_default`],
    /// which is the one written position that accepts it. A *parameter*
    /// default of `= []` is still refused: it would have to be materialized
    /// afresh at every call site that omitted it, which is a cost the caller
    /// cannot see, whereas a property default is written once into a slot the
    /// instance already owns.
    EmptyArray,
    /// ADR 0063 R2's options bag, wholly omitted at the call site: one entry
    /// per declared option, in the bag's own declared order, each holding that
    /// option's default.
    ///
    /// The one variant with no single `nvs_ir::ir::InstKind` constant under
    /// it, and deliberately so — a bag has no runtime representation at all.
    /// `nvs_ir::lower::lower_call_args` expands it into one ordinary constant
    /// per entry, which is why the "one variant per instruction" rule above
    /// still holds one level down. Never produced by
    /// [`eval_param_default`]: user code cannot declare a bag, so this only
    /// ever comes from [`crate::core_lib`].
    Options(Vec<(String, ConstArg)>),
    /// A `Core`-owned instance, named by the symbol that builds it and the
    /// constant arguments that symbol takes —
    /// `nvs_stdlib::registry::Const::Built`, which owns the rule that this is
    /// a class *constant*'s value and never a default.
    ///
    /// The second variant with no single `nvs_ir::ir::InstKind` constant under
    /// it, and unlike [`Self::Options`] it does reach `nvs-ir`: an instance
    /// has no constant form at all, so what is inlined at the use site is the
    /// call, which is the same `InstKind::CoreCall` a written
    /// `Zone::of("UTC")` lowers to. Never produced by [`eval_param_default`],
    /// for the same reason [`Self::Options`] is not: user code cannot declare
    /// one.
    Built {
        /// The `Core` symbol that builds the value.
        symbol: &'static str,
        /// Its arguments, positional.
        args: Vec<ConstArg>,
    },
    /// An [ADR 0036](../../../docs/adr/0036-anonymous-object-shapes.md) § 2
    /// shape value, its fields in the order they were written.
    ///
    /// Produced only by [`crate::attributes`], for ADR 0046 § 5's fold: a
    /// retrieval's answer *is* an attached literal, and § 5 replaces the call
    /// with that value rather than looking one up. There is no written
    /// position that reaches this — a shape literal is an expression with a
    /// lowering of its own, so nothing else needs a constant form of one.
    Shape(Vec<(String, ConstArg)>),
    /// An `array<T>` value, each entry as its own already-resolved `string`
    /// key ([ADR 0007](../../../docs/adr/0007-explicit-type-system.md) § 5:
    /// every key is a `string`) and its constant value.
    ///
    /// Produced only by [`crate::attributes`], beside [`Self::Shape`] and for
    /// the same reason. The keys are resolved *here* rather than left implicit
    /// because everything in a folded payload is constant, so an array
    /// literal's auto-index run has one answer and this is the last place it
    /// is cheap to compute. [`Self::EmptyArray`] stays the spelling of `[]`
    /// wherever a default reaches one.
    Array(Vec<(String, ConstArg)>),
}

/// Evaluates a written `= expr` parameter default against the parameter's own
/// declared type `declared`, reporting `E_PARAM_DEFAULT_NOT_LITERAL` and
/// returning `None` for anything this module does not accept (see its docs).
///
/// The declared type drives the decoding rather than the literal's own shape,
/// which is what makes `uint $n = 3` and `float $f = 1` work without a
/// widening rule of their own: the digits are read once, into the type the
/// parameter actually holds.
pub(crate) fn eval_param_default(
    expr: &Expr,
    declared: TypeId,
    env: &mut Env<'_>,
) -> Option<ConstArg> {
    let value = literal_default(expr, declared, env);
    if value.is_none() {
        let want = env.interner.describe(declared);
        env.diags.report(
            Diagnostic::error(
                code::E_PARAM_DEFAULT_NOT_LITERAL,
                format!("a parameter default must be a `{want}` literal"),
            )
            .with_primary(expr.span, "not a literal of the declared type")
            .with_help(
                "a default is evaluated once, at the call site that omits it — write a \
                 `bool`/`int`/`uint`/`float`/`string` literal, optionally negated",
            ),
        );
    }
    value
}

/// Evaluates a written `= expr` **property** default against the property's
/// own declared type `declared`, reporting `E_PROPERTY_DEFAULT_NOT_LITERAL`
/// and returning `None` for anything this module does not accept.
///
/// The same decoder [`eval_param_default`] uses, plus two shapes a parameter
/// has no use for. `= []` becomes [`ConstArg::EmptyArray`]: a property is the
/// position where the empty array is worth having — ADR 0022 obliges a
/// constructor to assign every non-defaulted property, so without it a class
/// accumulating into an `array<T>` has to write the assignment by hand in
/// every constructor it declares. And a *named* constant — `Mode::Fast`,
/// `Limits::MAX` — is folded by [`const_reference_default`], which is ADR 0046
/// § 2's constant set arriving one position along from the attribute payload
/// it was written for.
///
/// Unlike a parameter default, this constant is never emitted at a *call
/// site*: it is copied onto the class descriptor and written into the fresh
/// instance's slot by `nvs_runtime::NvsObj::new`, which is why an array
/// constant is reachable here at all — and why a folded enum case is, since
/// what the slot receives is the integer ADR 0010 § 3 says the case already
/// is.
pub(crate) fn eval_property_default(
    expr: &Expr,
    declared: TypeId,
    ctx: &Ctx<'_>,
    env: &mut Env<'_>,
) -> Option<ConstArg> {
    let empty_array = matches!(&expr.kind, ExprKind::ArrayLiteral(items) if items.is_empty())
        && matches!(env.interner.get(declared), Ty::Array(_));
    let mut reported = false;
    let value = if empty_array {
        Some(ConstArg::EmptyArray)
    } else if matches!(&expr.kind, ExprKind::ClassConstAccess { .. }) {
        const_reference_default(expr, declared, ctx, &mut reported, env)
    } else {
        literal_default(expr, declared, env)
    };
    if value.is_none() && !reported {
        let want = env.interner.describe(declared);
        env.diags.report(
            Diagnostic::error(
                code::E_PROPERTY_DEFAULT_NOT_LITERAL,
                format!("a property default must be a `{want}` constant"),
            )
            .with_primary(expr.span, "not a constant of the declared type")
            .with_help(
                "a property default is evaluated once, at compile time, and written into \
                 every fresh instance's slot — write a `bool`/`int`/`uint`/`float`/`string` \
                 literal, optionally negated, `[]`, an enum case or another class's `const`; \
                 anything else belongs in `constructor`",
            ),
        );
    }
    value
}

/// `Mode::Fast` or `Limits::MAX` at a property default — the two *named*
/// members of ADR 0046 § 2's compile-time constant set, folded to the same
/// [`ConstArg`] a written literal produces.
///
/// Three sources, in the order the name can mean them, and each already
/// resolved once by a pass that runs before this one: [`crate::enums`] holds
/// every case's integer (a `Core` enum's included, seeded from
/// `nvs_stdlib::registry::ENUMS`), [`crate::core_lib::constant`] holds a
/// `Core` class constant as a `ConstArg` already, and [`crate::consts`] holds
/// every declared class constant this crate folds. Nothing is evaluated here
/// that was not evaluated there — ADR 0046 § 2's reason for a closed list is
/// exactly that a second constant evaluator is what it refuses to grow.
///
/// The declared type still decides, as it does for a literal: a case is
/// accepted where the property declares that enum, an `int` constant widens
/// into a `float` property under ADR 0007 § 2's one implicit conversion, and
/// nothing else crosses. A `secret` constant ([ADR 0033](../../../docs/adr/0033-secret-qualifier-for-confidential-values.md)
/// § 1) is refused into a non-`secret` slot with its own `E_TYPE_MISMATCH`,
/// which is what the qualifier would otherwise be laundered by: the declared
/// type is the only thing carrying it, and a folded value has already lost it.
/// `reported` says whether that refusal already spoke, so the caller does not
/// add a second, vaguer one.
fn const_reference_default(
    expr: &Expr,
    declared: TypeId,
    ctx: &Ctx<'_>,
    reported: &mut bool,
    env: &mut Env<'_>,
) -> Option<ConstArg> {
    let ExprKind::ClassConstAccess { class, name } = &expr.kind else {
        return None;
    };
    let qname = crate::expr::resolve_class_expr(class, ctx, env)?;
    let member = crate::span_text(env.src, *name).to_owned();
    if let Some(case) = env.enums.case(&qname, &member) {
        let names_it = match env.interner.get(declared) {
            Ty::Enum(declared_enum, _) => *declared_enum == qname,
            Ty::EnumCase(declared_enum, _, declared_case) => {
                *declared_enum == qname && *declared_case == member
            }
            _ => false,
        };
        if !names_it {
            return None;
        }
        return Some(match case {
            crate::enums::EnumValue::Int(value) => ConstArg::Int(value),
            crate::enums::EnumValue::Uint(value) => ConstArg::Uint(value),
        });
    }
    let folded = if qname.is_core() {
        crate::core_lib::constant(&qname, &member, env.interner).map(|(_, value)| value)
    } else {
        if env.consts.is_secret(&qname, &member, env.graph)
            && !crate::expr::type_is_secret(declared, env.interner)
        {
            let want = env.interner.describe(declared);
            env.diags.report(
                Diagnostic::error(
                    code::E_TYPE_MISMATCH,
                    format!("`{qname}::{member}` is `secret`, and `{want}` is not"),
                )
                .with_primary(expr.span, "a `secret` constant in a non-`secret` slot")
                .with_help(
                    "declare the property `secret` too, so the qualifier ADR 0033 § 1 puts on \
                     the constant still travels with the value it initializes",
                ),
            );
            *reported = true;
            return None;
        }
        match env.consts.get(&qname, &member, env.graph) {
            Some(crate::consts::ConstValue::Bool(value)) => Some(ConstArg::Bool(*value)),
            Some(crate::consts::ConstValue::Int(value)) => Some(ConstArg::Int(*value)),
            Some(crate::consts::ConstValue::Float(value)) => Some(ConstArg::Float(*value)),
            Some(crate::consts::ConstValue::Str(value)) => Some(ConstArg::Str(value.clone())),
            Some(crate::consts::ConstValue::Ineligible) | None => None,
        }
    }?;
    place_const(folded, declared, env)
}

/// One already-folded constant against the property's own declared type —
/// [`literal_default`]'s grid, with the literal's syntax already gone.
///
/// The `int`-into-`float` row is the same widening ADR 0007 § 2 allows at any
/// ordinary assignment and [`literal_default`] already applies to a written
/// integer literal in a `float` position; every other pairing is refused
/// rather than converted, because `as` is the only conversion spelling and a
/// default has nowhere to write one.
fn place_const(value: ConstArg, declared: TypeId, env: &Env<'_>) -> Option<ConstArg> {
    match (env.interner.get(declared), &value) {
        (Ty::Bool, ConstArg::Bool(_))
        | (Ty::Int, ConstArg::Int(_))
        | (Ty::Uint, ConstArg::Uint(_))
        | (Ty::Float, ConstArg::Float(_))
        | (
            Ty::String | Ty::TaintedString | Ty::SecretString | Ty::SecretTaintedString,
            ConstArg::Str(_),
        ) => Some(value),
        (Ty::Uint, ConstArg::Int(n)) => u64::try_from(*n).ok().map(ConstArg::Uint),
        (Ty::Float, ConstArg::Int(n)) => {
            #[expect(
                clippy::cast_precision_loss,
                reason = "the same widening ADR 0007 § 2 already allows at an \
                          ordinary int-to-float assignment, applied to a constant \
                          the author wrote by hand"
            )]
            let widened = *n as f64;
            Some(ConstArg::Float(widened))
        }
        _ => None,
    }
}

/// The shared literal decoder behind both entry points above: the value, or
/// `None` for a shape neither accepts. Reports nothing — each caller names its
/// own position in its own diagnostic.
pub(crate) fn literal_default(
    expr: &Expr,
    declared: TypeId,
    env: &mut Env<'_>,
) -> Option<ConstArg> {
    let (negated, inner) = match &expr.kind {
        ExprKind::Unary {
            op: UnaryOp::Neg,
            expr: inner,
        } => (true, &**inner),
        ExprKind::Unary {
            op: UnaryOp::Plus,
            expr: inner,
        } => (false, &**inner),
        _ => (false, expr),
    };
    match (env.interner.get(declared).clone(), &inner.kind) {
        (Ty::Bool, ExprKind::Bool(b)) if !negated => Some(ConstArg::Bool(*b)),
        (Ty::Int, ExprKind::Int(span)) => int_magnitude(*span, env)
            .and_then(|m| {
                if negated {
                    negate_int(m)
                } else {
                    i64::try_from(m).ok()
                }
            })
            .map(ConstArg::Int),
        (Ty::Uint, ExprKind::Int(span)) if !negated => {
            int_magnitude(*span, env).map(ConstArg::Uint)
        }
        (Ty::Float, ExprKind::Float(span)) => {
            float_value(*span, env.src).map(|f| ConstArg::Float(if negated { -f } else { f }))
        }
        // An integer literal in a `float` position is the one cross-type
        // spelling accepted, for ADR 0007 § 4's reason: `int` widens to
        // `float` at any ordinary assignment, and a default is one.
        (Ty::Float, ExprKind::Int(span)) => int_magnitude(*span, env).map(|m| {
            #[expect(
                clippy::cast_precision_loss,
                reason = "the same widening ADR 0007 § 4 already allows at an \
                          ordinary int-to-float assignment, and the literal is \
                          written by hand"
            )]
            let f = m as f64;
            ConstArg::Float(if negated { -f } else { f })
        }),
        (
            Ty::String | Ty::TaintedString | Ty::SecretString | Ty::SecretTaintedString,
            ExprKind::Str(span),
        ) if !negated => Some(ConstArg::Str(crate::string_lit::cook_string_literal(
            env.src, *span,
        ))),
        _ => None,
    }
}

/// An integer literal's magnitude, in whatever radix it was written —
/// [`crate::expr::int_literal_digits`]'s job, reused so a default never grows
/// a second integer grammar. `None` for a magnitude that does not fit `u64`,
/// which the caller reports as "not a literal of the declared type" along with
/// every other rejection: an out-of-range default is the same authoring
/// mistake, at the same span.
pub(crate) fn int_magnitude(span: Span, env: &Env<'_>) -> Option<u64> {
    let (radix, digits) = crate::expr::int_literal_digits(env.src, span);
    u64::from_str_radix(&digits, radix).ok()
}

/// `-magnitude` as an `i64`, or `None` when the magnitude is too large to
/// negate — `i64::MIN`'s own magnitude included, which `i64::try_from` alone
/// would refuse.
fn negate_int(magnitude: u64) -> Option<i64> {
    if magnitude == (i64::MAX as u64) + 1 {
        return Some(i64::MIN);
    }
    i64::try_from(magnitude).ok().and_then(i64::checked_neg)
}

/// A float literal's value. PHP's own `strtod`-shaped grammar is a superset of
/// Rust's, but every spelling the *lexer* produces a `Float` token for parses
/// here; a `_` digit separator is stripped first, exactly as
/// [`crate::expr::int_literal_digits`] strips one.
pub(crate) fn float_value(span: Span, src: &nvs_diagnostics::SourceFile) -> Option<f64> {
    crate::span_text(src, span).replace('_', "").parse().ok()
}
