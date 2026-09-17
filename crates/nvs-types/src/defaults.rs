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
//!   exactly one arity, so nothing in `nvs-ir`, `nvs-codegen` or the `rule:errors/propagation`
//!   call ABI learns about defaults at all — `nvs_ir::lower::lower_call_args`
//!   pushes one more `ConstInt`/`ConstStr` and stops.
//! * **A default cannot observe anything.** It is a constant, so it cannot
//!   read a global (there are none — `rule:statements/static-is-a-member-modifier`), call a function, or differ
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
//! `rule:attributes/payload-is-a-compile-time-constant`
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
//! **A written `= null` is accepted exactly where the declared type admits
//! one** — `?int $rank = null` at a property and at a parameter alike, as
//! [`ConstArg::Null`] over `nvs_ir::ir::InstKind::ConstNull`. The decoder asks
//! [`crate::ty::TypeInterner::is_nullable`] rather than naming a shape, so `?T`,
//! `T|null` and a bare `null` are one rule and a type that admits no `null`
//! refuses it with every other constant of the wrong type. This is the same
//! constant [`crate::core_lib`] reaches for a `Core` member's own
//! `?int $length = null`, so a written default and a registered one are one
//! mechanism here too.
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
//! (`rule:types/literal-types`), so `uint $n = Limits::MAX;` above `i64::MAX` is refused
//! here even though the literal `= 18446744073709551615` is accepted.
//! Decided: Allow it: the call-site emitter carries the parameter's own IR type — The property and
//! parameter surfaces match, and every omitting call site's emitter changes.
//! — owner: decided-closures
//!
//! **A `decimal` default is refused, and is now the shortest thing on this
//! list to build:** `nvs_ir::ir::InstKind::ConstDecimal` exists, so
//! [`ConstArg`]'s one-variant-per-instruction rule above is satisfied by a
//! variant carrying that instruction's own three parts. Until one is written,
//! `decimal $vat = 0.19` is `E_PARAM_DEFAULT_NOT_LITERAL` — a clean refusal,
//! not a wrong constant.

use nvs_diagnostics::{Diagnostic, Span, code};
use nvs_syntax::ast::{ArrayItem, Expr, ExprKind, UnaryOp};

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
    /// `nvs_stdlib::registry::Const::Null`, and only for an `rule:core-api/shape-rules` R2
    /// option whose spec signature gives it no "not given" spelling of its
    /// own: `Core\Arr::sort`'s `by` and `comparator` are the first two. A
    /// written `= null` is still refused — see this module's own known gap,
    /// which is about the `?T` *parameter type* that would declare it, not
    /// about the constant.
    Null,
    /// **Not a value**: the never-written marker an omitting call site
    /// materializes for a **nullable** option or shape field —
    /// `rule:core-api/a-nullable-field-omits-as-the-never-written-marker`.
    ///
    /// Produced only by [`crate::core_lib`], from
    /// `nvs_stdlib::registry::Const::NeverWritten`, and lowered to
    /// `nvs_ir::ir::InstKind::ConstUnset`, which is `nvs_runtime::Tag::Unset`
    /// under a zero payload. It is the one constant here that has no type and
    /// that no program can write, which is exactly what lets the helper tell
    /// an omitted key from a written `null`
    /// (`rule:core-api/omission-is-not-a-written-null`).
    NeverWritten,
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
    /// the language (`rule:types/bytes`), so no *written* default can reach this variant, and
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
    /// `rule:core-api/shape-rules` R2's options bag, wholly omitted at the call site: one entry
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
    /// `rule:core-api/shape-flattens-at-the-abi`'s fill list for a **required** shape parameter: one entry per field
    /// of the merged arm list, in the order that list flattens, holding what
    /// the call site passes for a key the written literal does not carry — the
    /// field's own default where it has one, [`Self::Null`] for a field
    /// belonging to an arm the caller did not write.
    ///
    /// The one variant that is **not a default for its parameter**, and the
    /// distinction is the whole reason it is a second variant rather than an
    /// [`Self::Options`] entry: a shape parameter is written at every call
    /// site, so recording its fills the way a bag records its own would make
    /// [`crate::signatures::MethodSig::required`] count the parameter as
    /// optional and let a call omit it entirely. `required` skips this entry
    /// for exactly that reason, and it is the one reader of `defaults` that
    /// asks the optionality question. An optional shape parameter, which no
    /// registry row declares yet, would record `Options` instead — the
    /// variants split on *omittable or not*, not on which registry spelling
    /// produced them.
    ///
    /// Like [`Self::Options`] it has no single `nvs_ir::ir::InstKind` constant
    /// under it: `nvs_ir::lower::Lowering::lower_options_arg` expands it into
    /// one ordinary constant per slot. Never produced by [`eval_param_default`]
    /// — user code cannot declare a shape parameter, so this only ever comes
    /// from [`crate::core_lib`].
    RequiredShape(Vec<(String, ConstArg)>),
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
    /// An `rule:types/object-literal`
    /// shape value, its fields in the order they were written.
    ///
    /// Produced only by [`crate::attributes`], for `rule:attributes/retrieval-folds-while-checking`'s fold: a
    /// retrieval's answer *is* an attached literal, and § 5 replaces the call
    /// with that value rather than looking one up. There is no written
    /// position that reaches this — a shape literal is an expression with a
    /// lowering of its own, so nothing else needs a constant form of one.
    Shape(Vec<(String, ConstArg)>),
    /// An `array<T>` value, each entry as its own already-resolved `string`
    /// key (`rule:types/arrays`:
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
/// position where the empty array is worth having — `rule:classes/definite-property-initialization` obliges a
/// constructor to assign every non-defaulted property, so without it a class
/// accumulating into an `array<T>` has to write the assignment by hand in
/// every constructor it declares. And a *named* constant — `Mode::Fast`,
/// `Limits::MAX` — is folded by [`const_reference_default`], which is `rule:attributes/payload-is-a-compile-time-constant`'s constant set arriving one position along from the attribute payload
/// it was written for.
///
/// Unlike a parameter default, this constant is never emitted at a *call
/// site*: it is copied onto the class descriptor and written into the fresh
/// instance's slot by `nvs_runtime::NvsObj::new`, which is why an array
/// constant is reachable here at all — and why a folded enum case is, since
/// what the slot receives is the integer `rule:enums/no-class-machinery` says the case already
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
/// members of `rule:attributes/payload-is-a-compile-time-constant`'s compile-time constant set, folded to the same
/// [`ConstArg`] a written literal produces.
///
/// Three sources, in the order the name can mean them, and each already
/// resolved once by a pass that runs before this one: [`crate::enums`] holds
/// every case's integer (a `Core` enum's included, seeded from
/// `nvs_stdlib::registry::ENUMS`), [`crate::core_lib::constant`] holds a
/// `Core` class constant as a `ConstArg` already, and [`crate::consts`] holds
/// every declared class constant this crate folds. Nothing is evaluated here
/// that was not evaluated there — `rule:attributes/payload-is-a-compile-time-constant`'s reason for a closed list is
/// exactly that a second constant evaluator is what it refuses to grow.
///
/// The declared type still decides, as it does for a literal: a case is
/// accepted where the property declares that enum, an `int` constant widens
/// into a `float` property under `rule:types/conversion`'s one implicit conversion, and
/// nothing else crosses. A `secret` constant (`rule:security/secret-qualifier`
/// ) is refused into a non-`secret` slot with its own `E_TYPE_MISMATCH`,
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
                    "declare the property `secret` too, so the qualifier `rule:security/secret-qualifier` puts on \
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

/// `Mode::Fast`, `Limits::MAX` or `Foo::class` as the value each already
/// resolved to elsewhere — `rule:attributes/payload-is-a-compile-time-constant`'s three *named* constants, folded for
/// the § 5 payload that has to compile one in.
///
/// [`const_reference_default`] is the sibling, and which table each reads is
/// the whole of the difference. That one runs while signatures are still being
/// collected, so it asks [`crate::consts`], built early and folding no array;
/// this one runs in the checking pass, where
/// [`crate::signatures::resolve_const`] holds the same constant with its whole
/// value — the very entry a *read* of `Foo::CONST` inlines
/// ([`crate::expr::members`]), which is what makes a payload and a read agree
/// on what the name means. A constant whose own declaration folded to nothing
/// is `None` here, and that residue is the whole of what `E0731` still reports.
///
/// There is no `secret` gate, because there is nothing left to launder: ADR
/// 0033 § 4's fifth sink already refused a `secret` constant where the
/// attribute was *written* ([`crate::attributes`]), one whole pass earlier.
pub(crate) fn fold_const_reference(
    expr: &Expr,
    ctx: &Ctx<'_>,
    env: &mut Env<'_>,
) -> Option<ConstArg> {
    match &expr.kind {
        // `static::class` would resolve here to the *declaring* class, which
        // `rule:statements/static-is-a-member-modifier`'s late static binding makes the wrong answer: `rule:types/class-constant`
        // reads it off the frame's called class instead, and a constant
        // initializer has no frame to read. Left unfolded rather than answered
        // differently in two places — the read then takes `E0792`, the same
        // code every other constant with no compile-time form takes.
        ExprKind::ClassNameConst { class } if !matches!(class.kind, ExprKind::StaticExpr) => {
            let qname = crate::expr::resolve_class_expr(class, ctx, env)?;
            Some(ConstArg::Str(qname.to_string()))
        }
        ExprKind::ClassConstAccess { class, name } => {
            let qname = crate::expr::resolve_class_expr(class, ctx, env)?;
            let member = crate::span_text(env.src, *name).to_owned();
            if let Some(case) = env.enums.case(&qname, &member) {
                return Some(match case {
                    crate::enums::EnumValue::Int(value) => ConstArg::Int(value),
                    crate::enums::EnumValue::Uint(value) => ConstArg::Uint(value),
                });
            }
            if qname.is_core() {
                return crate::core_lib::constant(&qname, &member, env.interner)
                    .map(|(_, value)| value);
            }
            crate::signatures::resolve_const(&qname, &member, env.signatures, env.graph)
                .and_then(|sig| sig.value.clone())
        }
        _ => None,
    }
}

/// One already-folded constant against the property's own declared type —
/// [`literal_default`]'s grid, with the literal's syntax already gone.
///
/// The `int`-into-`float` row is the same widening `rule:types/conversion` allows at any
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
                reason = "the same widening `rule:types/conversion` already allows at an \
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
        // spelling accepted, for `rule:types/arithmetic`'s reason: `int` widens to
        // `float` at any ordinary assignment, and a default is one.
        (Ty::Float, ExprKind::Int(span)) => int_magnitude(*span, env).map(|m| {
            #[expect(
                clippy::cast_precision_loss,
                reason = "the same widening `rule:types/arithmetic` already allows at an \
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
        // A written `null` against a type that admits one. The guard asks
        // [`crate::ty::TypeInterner::is_nullable`] rather than matching `Ty::Union`
        // here, because `?T`, `A|B|null` and a bare `null` are three spellings
        // of the same admission and that question already has one answer.
        (_, ExprKind::Null) if !negated && env.interner.is_nullable(declared) => {
            Some(ConstArg::Null)
        }
        _ => None,
    }
}

/// A **class constant**'s written value, folded to the [`ConstArg`] `rule:classes/no-free-functions-or-constants`'s
/// inlining rule needs: [`literal_default`]'s type-directed grid first, and an
/// array literal where that grid has nothing to say.
///
/// The extra shape is here rather than in [`literal_default`] because the
/// position is what makes it affordable. A class constant is inlined at its use
/// sites, exactly as `nvs_ir::lower::emit_const_arg` already materializes
/// [`ConstArg::Array`] for `rule:attributes/retrieval-folds-while-checking`'s folded retrieval — while a *parameter*
/// default of `= [1, 2]` would be built afresh at every call site that omitted
/// it, which is [`ConstArg::EmptyArray`]'s own doc's reason for refusing even
/// the empty one there.
///
/// **The declared type still drives the decoding**, one level at a time: an
/// `array<float>` constant written `[1, 2]` folds to two [`ConstArg::Float`]s,
/// because each element is placed by the same [`literal_default`] the whole
/// value would have been, and an `array<array<float>>` recurses through here
/// rather than falling to a decoder with no type left to place against. Nothing
/// is accepted at a *wrong* type on the way: an element the grid refuses is the
/// whole constant refused, since a half-folded array has no value to inline.
///
/// `None` for a value with no constant form, which the *read* refuses with
/// `E0792` — [`crate::expr::members`] owns why that is the position rather than
/// this one. Two shapes reach it: the named constants of `rule:attributes/payload-is-a-compile-time-constant` that
/// [`const_reference_default`] resolves one position along and this pass does
/// not, and a shape-typed constant, whose fields would each need placing
/// against the declared shape's own.
pub(crate) fn eval_const_value(
    expr: &Expr,
    declared: TypeId,
    env: &mut Env<'_>,
) -> Option<ConstArg> {
    if let Some(value) = literal_default(expr, declared, env) {
        return Some(value);
    }
    let Ty::Array(element) = *env.interner.get(declared) else {
        return None;
    };
    let ExprKind::ArrayLiteral(items) = &expr.kind else {
        return None;
    };
    if items.is_empty() {
        // `[]` keeps its one spelling wherever a constant reaches one — see
        // [`ConstArg::EmptyArray`]. Both lower to the same empty
        // `nvs_ir::ir::InstKind::ArrayNew`.
        return Some(ConstArg::EmptyArray);
    }
    fold_const_array(items, Some(element), None, env)
}

/// One array literal's entries under the `string` keys `rule:types/arrays` gives them,
/// with a keyless run taking its position in that run — every value here is
/// constant, so the auto-index has one answer and this is the last place it is
/// cheap to compute.
///
/// `element` is the declared element type where the position has one, and each
/// value is placed in it; `None` is the untyped fold `rule:attributes/retrieval-folds-while-checking`'s payload
/// takes. A spread is refused rather than expanded: `...$rows` names a binding,
/// and `...[1, 2]` inside a constant is a shape nothing writes.
fn fold_const_array(
    items: &[ArrayItem],
    element: Option<TypeId>,
    ctx: Option<&Ctx<'_>>,
    env: &mut Env<'_>,
) -> Option<ConstArg> {
    let mut out = Vec::with_capacity(items.len());
    let mut next = 0_u64;
    for ArrayItem {
        key, value, spread, ..
    } in items
    {
        if *spread {
            return None;
        }
        let key = match key {
            Some(key) => match fold_constant_value(key, ctx, env)? {
                ConstArg::Str(s) => s,
                ConstArg::Int(n) => n.to_string(),
                ConstArg::Uint(n) => n.to_string(),
                _ => return None,
            },
            None => {
                let key = next.to_string();
                next += 1;
                key
            }
        };
        let value = match element {
            Some(element) => eval_const_value(value, element, env)?,
            None => fold_constant_value(value, ctx, env)?,
        };
        out.push((key, value));
    }
    Some(ConstArg::Array(out))
}

/// One constant value with **no position to place it in**: the decoder `rule:attributes/retrieval-folds-while-checking`'s payload fold reaches, an attached literal being checked structurally
/// rather than declared. Reports nothing — each caller names its own position in
/// its own diagnostic.
///
/// [`literal_default`] is the type-directed sibling and is preferred wherever a
/// declared type exists, because that is what makes `uint`, `float` and the
/// three qualified `string`s reachable at all. Here the value's own shape is the
/// whole of the evidence: an integer is an `int` where one holds it and a `uint`
/// above that, which is the same order a written annotation would have narrowed
/// it in.
///
/// `ctx` is the scope a **named** constant in this value resolves through, and
/// `None` says the caller has none to offer: the declaration folds
/// ([`eval_const_value`]) run before the pass that would answer, so they hand
/// `None` and a `Mode::Fast` written there is left to
/// [`const_reference_default`] one position along. `rule:attributes/retrieval-folds-while-checking`'s payload fold
/// is the caller that does have one — the attach site's own, never the
/// retrieval's — and [`fold_const_reference`] is what it buys.
pub(crate) fn fold_constant_value(
    expr: &Expr,
    ctx: Option<&Ctx<'_>>,
    env: &mut Env<'_>,
) -> Option<ConstArg> {
    let mut negated = false;
    let mut inner = expr;
    loop {
        match &inner.kind {
            ExprKind::Paren(next) => inner = next,
            ExprKind::Unary {
                op: UnaryOp::Neg,
                expr: next,
            } => {
                negated = !negated;
                inner = next;
            }
            ExprKind::Unary {
                op: UnaryOp::Plus,
                expr: next,
            } => inner = next,
            _ => break,
        }
    }
    match &inner.kind {
        ExprKind::Null if !negated => Some(ConstArg::Null),
        ExprKind::Bool(b) if !negated => Some(ConstArg::Bool(*b)),
        ExprKind::Str(span) if !negated => Some(ConstArg::Str(
            crate::string_lit::cook_string_literal(env.src, *span),
        )),
        ExprKind::Float(span) => {
            float_value(*span, env.src).map(|f| ConstArg::Float(if negated { -f } else { f }))
        }
        ExprKind::Int(span) => {
            let magnitude = int_magnitude(*span, env)?;
            if negated {
                i64::try_from(magnitude)
                    .ok()
                    .map(|v| ConstArg::Int(-v))
                    .or_else(|| (magnitude == 1 << 63).then_some(ConstArg::Int(i64::MIN)))
            } else {
                Some(i64::try_from(magnitude).map_or(ConstArg::Uint(magnitude), ConstArg::Int))
            }
        }
        ExprKind::ArrayLiteral(items) if !negated => fold_const_array(items, None, ctx, env),
        ExprKind::ObjectLiteral(fields) if !negated => {
            let mut out = Vec::with_capacity(fields.len());
            for field in fields {
                out.push((
                    crate::span_text(env.src, field.name).to_owned(),
                    fold_constant_value(&field.value, ctx, env)?,
                ));
            }
            Some(ConstArg::Shape(out))
        }
        // `Mode::Fast`, `Limits::MAX` and `Foo::class`, for a caller that
        // carries the scope they were written in. A negated one is not folded:
        // `-Foo::MAX` is a spelling no position admits today, and inventing an
        // answer here would be the second constant evaluator `rule:attributes/payload-is-a-compile-time-constant`
        // exists to refuse.
        ExprKind::ClassConstAccess { .. } | ExprKind::ClassNameConst { .. } if !negated => {
            fold_const_reference(inner, ctx?, env)
        }
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
