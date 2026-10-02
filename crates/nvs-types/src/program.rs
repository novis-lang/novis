//! `rule:programs/implementing`: `Core\Program::implementing<T>()`, answered here rather than at run
//! time.
//!
//! The sibling of [`crate::retrieval`], and the same pass shape for the same
//! reason: the question has an answer before the program starts, so the call
//! is **replaced** with it. What differs is that the answer is not a constant.
//! § 3 expands the call "to an array literal of `new` expressions — one per
//! non-abstract class in the program implementing `T`, sorted by
//! fully-qualified name", and a `new` allocates. So this records
//! [`ExprInfo::ProgramInstances`] — the list — where retrieval records
//! [`crate::expr_table::ExprInfo::CoreConst`], and `nvs-ir` emits the `new`s
//! and the array. The instances are therefore per-request like any other
//! object and nothing crosses an isolate boundary
//! (`rule:security/isolate-shares-nothing`).
//!
//! # The scan already happened
//!
//! Nothing here walks a directory. `nvs_hir::AutoloadMap::enumerate` names
//! every file the autoload roots declare a name in, `nvs_hir::requires` loads
//! them — and *only* for a program that writes this call, which is § 3's
//! "a program containing no `implementing<T>()` call never performs the scan"
//! — and `nvs_hir::implementors` filters the resulting class graph. By the
//! time this pass runs, `Env::graph` holds every declaration the answer is
//! drawn from, so the whole of this module is a lookup and two refusals.
//!
//! The two refusals are each reported at the call: `T` must be an interface
//! or a class (`E0743`), and every class the enumeration would instantiate
//! needs a no-argument constructor (`E0744`). Their own `Code` doc comments
//! own why each is refused rather than worked around.

use nvs_diagnostics::{Diagnostic, code};
use nvs_hir::{QName, SymbolKind};
use nvs_syntax::ast::{CallArgs, Expr, ExprKind};
use rustc_hash::FxHashSet;

use crate::defaults::ConstArg;
use crate::expr::args::check_args_typed;
use crate::expr::calls::resolved_call;
use crate::expr::members::class_qname_of;
use crate::expr_table::{ExprInfo, ResolvedCall};
use crate::locals::LocalScope;
use crate::retrieval::{fold_payload, matching, sites_for};
use crate::signatures::{resolve_method, resolve_property};
use crate::ty::{ShapeField, Ty, TypeId};
use crate::{Ctx, Env};

/// The one class this pass answers for.
const OWNER: &str = r"Core\Program";

/// The joined enumeration, `rule:programs/implementing-with`'s member.
const WITH: &str = "implementingWith";

/// Whether `owner::member` is an enumeration — the same nominal test
/// [`crate::retrieval::is_retrieval`] makes, against a resolved [`QName`]
/// rather than against what the call site spelled. Two members answer:
/// `implementing`, expanded by [`expand`], and `implementingWith`, expanded by
/// [`expand_with`].
pub(crate) fn is_enumeration(owner: &QName, member: &str) -> bool {
    owner.to_string() == OWNER && (member == "implementing" || member == WITH)
}

/// Whether `member` is the joined form, which [`expand_with`] answers with a
/// type of its own rather than the registry row's.
pub(crate) fn is_joined(member: &str) -> bool {
    member == WITH
}

/// Resolves one enumeration and records its answer against the call's own
/// span, as the [`ExprInfo::ProgramInstances`] `nvs-ir` materializes the
/// array literal from.
///
/// Records nothing where the call is refused: a diagnostic has been reported,
/// and `nvs-ir` never reaches a unit that failed to check.
pub(crate) fn expand(
    call: &Expr,
    written: &[TypeId],
    live: &mut FxHashSet<String>,
    scope: &LocalScope,
    ctx: &Ctx<'_>,
    env: &mut Env<'_>,
) {
    let Some(want) = written.first().copied() else {
        // The type-argument count is `check_written_type_args`' refusal and
        // has already been made; a second one names the same mistake twice.
        return;
    };
    let Some(selector) = selector_of(call, "implementing", want, env) else {
        return;
    };
    let classes = nvs_hir::implementors(&selector, env.graph);
    let Some(ctors) = constructors_of(&classes, &selector, call, live, scope, ctx, env) else {
        return;
    };
    env.exprs
        .record(call.span, ExprInfo::ProgramInstances { classes, ctors });
}

/// `rule:programs/implementing-with`: [`expand`]'s list, joined with one
/// `rule:attributes/retrieval-folds-while-checking` retrieval per class, and
/// recorded as [`ExprInfo::ProgramInstancesWith`].
///
/// Per class the retrieval is exactly what `Core\Attributes::get<T>` would
/// answer for that class's own declaration of `$member`: the class itself for
/// an empty name, its method for a method name, its property or constructor
/// parameter otherwise — [`sites_for`]'s two rosters, joined the same way. A
/// name no class declares is `E0798` naming the class, and two matches on one
/// class are `E0728` naming it, both the retrieval's own codes for the same
/// mistakes. A `$member` that is not a string literal names no roster and
/// every row's `attribute` is `null`, as `get<T>`'s computed member folds.
///
/// Returns the call's type — `array<{instance: I, attribute: ?T}>` over the
/// written arguments — because no registry row can state it: the row spells
/// the shape for the card, and its return type is not lowered for this
/// member. `mixed` where the call was refused.
pub(crate) fn expand_with(
    call: &Expr,
    written: &[TypeId],
    args: &CallArgs,
    live: &mut FxHashSet<String>,
    scope: &LocalScope,
    ctx: &Ctx<'_>,
    env: &mut Env<'_>,
) -> TypeId {
    let (Some(want_selector), Some(want_shape)) =
        (written.first().copied(), written.get(1).copied())
    else {
        // The type-argument count is `check_written_type_args`' refusal.
        return env.interner.mixed();
    };
    let Some(selector) = selector_of(call, WITH, want_selector, env) else {
        return env.interner.mixed();
    };
    if !matches!(env.interner.get(want_shape), Ty::Shape(_)) {
        let found = env.interner.describe(want_shape);
        env.diags.report(
            Diagnostic::error(
                code::E_ATTRIBUTE_TYPE_ARG_NOT_A_SHAPE,
                format!("`Core\\Program::{WITH}` retrieves a shape, and `{found}` is not one"),
            )
            .with_primary(call.span, format!("`{found}` written here"))
            .with_help(
                "`rule:attributes/structural-retrieval`: retrieval is structural — an attached literal is an \
                 answer exactly when it satisfies `T` under `rule:types/shape-type`'s width subtyping, so \
                 the second type argument is an inline `{...}` or a `type` alias naming one",
            ),
        );
        return env.interner.mixed();
    }
    let member_arg = match args {
        CallArgs::List(list) => list.first().map(|arg| &arg.value),
        _ => None,
    };
    let member_name = member_arg.and_then(|value| match &value.kind {
        ExprKind::Str(span) => Some((
            value.span,
            crate::string_lit::cook_string_literal(env.src, *span),
        )),
        _ => None,
    });
    let computed_member = member_arg.is_some() && member_name.is_none();

    let classes = nvs_hir::implementors(&selector, env.graph);
    let Some(ctors) = constructors_of(&classes, &selector, call, live, scope, ctx, env) else {
        return env.interner.mixed();
    };
    let mut payloads = Vec::with_capacity(classes.len());
    for class in &classes {
        // Which of `sites_for`'s two rosters the name selects on *this* class:
        // a method's own sites, or the constructor's joined with a property's.
        let (method, property) = match &member_name {
            Some((_, name)) if !name.is_empty() => {
                if resolve_method(class, name, env.signatures, env.graph).is_some() {
                    (name.clone(), None)
                } else if resolve_property(class, name, env.signatures, env.graph).is_some() {
                    ("constructor".to_owned(), Some(name.as_str()))
                } else {
                    let span = member_name.as_ref().map_or(call.span, |(span, _)| *span);
                    env.diags.report(
                        Diagnostic::error(
                            code::E_ATTRIBUTE_MEMBER_NOT_DECLARED,
                            format!(
                                "`Core\\Program::{WITH}` names `{name}`, which `{class}` does not \
                                 declare"
                            ),
                        )
                        .with_primary(span, "no method, parameter or property of that name")
                        .with_help(format!(
                            "`rule:programs/implementing-with`: the member is read on every class listed \
                             for `{selector}`, so it is checked against each one's real declarations — \
                             name a member `{selector}` declares, or the empty string for the \
                             class's own attributes"
                        )),
                    );
                    return env.interner.mixed();
                }
            }
            _ => ("constructor".to_owned(), None),
        };
        let sites = if computed_member {
            Vec::new()
        } else {
            sites_for(env, class, &method, property)
        };
        let matched = matching(&sites, want_shape, env);
        let value = match matched.len() {
            0 => ConstArg::Null,
            1 => match fold_payload(matched[0], env) {
                Some(value) => value,
                None => return env.interner.mixed(),
            },
            count => {
                env.diags.report(
                    Diagnostic::error(
                        code::E_ATTRIBUTE_RETRIEVAL_AMBIGUOUS,
                        format!("`{class}::{method}` carries {count} attached literals satisfying this shape"),
                    )
                    .with_primary(call.span, "each row's `attribute` holds at most one")
                    .with_help(format!(
                        "`rule:programs/implementing-with`: an attached-attribute list is static, so this is \
                         decided here rather than by a test run — narrow the shape until one literal on \
                         `{class}` satisfies it, or read that class with `Core\\Attributes::all<T>(…)`"
                    )),
                );
                return env.interner.mixed();
            }
        };
        payloads.push(value);
    }
    env.exprs.record(
        call.span,
        ExprInfo::ProgramInstancesWith {
            classes,
            ctors,
            payloads,
        },
    );
    let null = env.interner.null();
    let attribute = env.interner.make_union([want_shape, null]);
    let row = env.interner.shape(vec![
        ShapeField {
            name: "instance".to_owned(),
            ty: want_selector,
            required: true,
        },
        ShapeField {
            name: "attribute".to_owned(),
            ty: attribute,
            required: true,
        },
    ]);
    env.interner.array(row)
}

/// The interface or class a written type argument names, or `None` after
/// reporting `E0743` — the selector's one refusal, shared by both
/// enumerations.
///
/// A class is accepted abstract or not. What the enumeration then lists is
/// [`nvs_hir::implementors`]' answer for it: every non-abstract class that is
/// a `T`, the class itself included when it is concrete.
fn selector_of(call: &Expr, member: &str, want: TypeId, env: &mut Env<'_>) -> Option<QName> {
    let found = class_qname_of(want, env.interner).filter(|qname| {
        env.symbols
            .get(qname)
            .is_some_and(|sym| matches!(sym.kind, SymbolKind::Interface | SymbolKind::Class))
    });
    if found.is_none() {
        let found = env.interner.describe(want);
        env.diags.report(
            Diagnostic::error(
                code::E_PROGRAM_TYPE_ARG_NOT_A_CLASS_OR_INTERFACE,
                format!(
                    "`Core\\Program::{member}` enumerates an interface or a class, and `{found}` \
                     is neither"
                ),
            )
            .with_primary(call.span, format!("`{found}` written here"))
            .with_help(
                "`rule:programs/implementing`: the type argument is what gives the enumerated instances \
                 a static type — `object` is opaque and a shape describes data rather than methods, \
                 so an array of anything else is one nothing can be called on",
            ),
        );
    }
    found
}

/// Each class's resolved `constructor` call, or `None` after reporting
/// `E0744` — § 3's second refusal, shared by both enumerations.
///
/// The entry is the one a written `new C()` records ([`ExprInfo::New`]'s
/// `ctor`): the constructor's signature plus the slots an empty argument list
/// leaves to its defaults, which `nvs-ir` materializes through its ordinary
/// argument lowering. A bare label allocated with no arguments at all, which
/// left every optional parameter unset — a `float $value = 1013.0` read `0`.
fn constructors_of(
    classes: &[QName],
    selector: &QName,
    call: &Expr,
    live: &mut FxHashSet<String>,
    scope: &LocalScope,
    ctx: &Ctx<'_>,
    env: &mut Env<'_>,
) -> Option<Vec<Option<ResolvedCall>>> {
    let mut ctors = Vec::with_capacity(classes.len());
    for class in classes {
        // A class declaring no `constructor` at all is the common case and the
        // one § 3 is written for; `ExprInfo::New`'s own `ctor` is `None` there
        // too, and `nvs-ir` allocates without calling anything.
        let Some((owner, sig)) = resolve_method(class, "constructor", env.signatures, env.graph)
        else {
            ctors.push(None);
            continue;
        };
        if sig.required() > 0 {
            env.diags.report(
                Diagnostic::error(
                    code::E_PROGRAM_IMPLEMENTOR_NEEDS_NO_ARGUMENT_CONSTRUCTOR,
                    format!(
                        "`{class}` is a `{selector}` and its constructor takes {} \
                         required argument(s)",
                        sig.required()
                    ),
                )
                .with_primary(
                    call.span,
                    format!("this enumeration would write `new {class}()`"),
                )
                .with_help(format!(
                    "`rule:programs/implementing`: every class the enumeration instantiates needs a \
                     no-argument constructor, and dependencies arrive through \
                     `{selector}`'s own methods instead — give `{class}` a \
                     zero-argument `constructor`",
                )),
            );
            return None;
        }
        let (_, slots, checked) = check_args_typed(
            &CallArgs::List(Vec::new()),
            Some(sig.clone()),
            call.span,
            live,
            scope,
            ctx,
            env,
        );
        let Some(checked) = checked else {
            ctors.push(None);
            continue;
        };
        ctors.push(Some(resolved_call(
            owner,
            "constructor".to_owned(),
            &checked,
            slots,
            env.signatures,
        )));
    }
    Some(ctors)
}

#[cfg(test)]
mod tests {
    use nvs_diagnostics::{Diagnostics, SourceMap};
    use nvs_syntax::ast::StmtKind;

    use crate::expr_table::{ExprInfo, ExprTypeTable};
    use crate::ty::TypeInterner;

    /// Checks `src` and hands back the table plus the span of its **first
    /// expression statement** — which every fixture below writes as the
    /// enumeration and nothing else, so there is no ambiguity about which
    /// entry is being read.
    fn check(src: &str) -> (ExprTypeTable, nvs_diagnostics::Span, Diagnostics) {
        let mut map = SourceMap::new();
        let file = map.add("t.nvs", src);
        let mut diags = Diagnostics::new();
        let stmts = nvs_syntax::parse_file(map.file(file), &mut diags);
        assert!(!diags.has_errors(), "fixture failed to parse: {diags:?}");
        let module = nvs_hir::resolve_file(&stmts, map.file(file), &mut diags);
        let mut interner = TypeInterner::new();
        let mut exprs = ExprTypeTable::new();
        crate::check_program(
            &[crate::ProgramFile {
                src: map.file(file),
                stmts: &stmts,
            }],
            &module,
            &mut interner,
            &mut exprs,
            &mut diags,
        );
        let span = stmts
            .iter()
            .find_map(|stmt| match &stmt.kind {
                StmtKind::Expr(expr) => Some(expr.span),
                _ => None,
            })
            .expect("fixture must write the call as an expression statement");
        (exprs, span, diags)
    }

    /// `rule:programs/implementing`, all four clauses of it at once: the call records the
    /// expansion rather than a call, the entries are the **non-abstract**
    /// classes reaching the interface, they are **sorted by fully-qualified
    /// name**, and each carries the constructor `nvs-ir` is to name — the
    /// declaring one, or `None` where the class declares none.
    // covers: Core\Program::implementing
    #[test]
    fn program_implementing_expands_to_new_expressions() {
        let (exprs, span, diags) = check(
            "<?nvs\n\
             interface Module { public function tag(): string; }\n\
             class Zulu implements Module { public function tag(): string { return \"z\"; } }\n\
             class Alpha implements Module {\n\
                 public function constructor() {}\n\
                 public function tag(): string { return \"a\"; }\n\
             }\n\
             abstract class Base implements Module { public function tag(): string { return \"b\"; } }\n\
             Core\\Program::implementing<Module>();\n",
        );
        assert!(!diags.has_errors(), "fixture failed to check: {diags:?}");
        let Some(ExprInfo::ProgramInstances { classes, ctors }) = exprs.lookup(span) else {
            panic!("the call recorded no expansion: {:?}", exprs.lookup(span));
        };
        let names: Vec<String> = classes.iter().map(ToString::to_string).collect();
        // Source order is `Zulu`, `Alpha`; the answer is not.
        assert_eq!(names, ["Alpha", "Zulu"]);
        let labels: Vec<Option<String>> = ctors
            .iter()
            .map(|ctor| {
                ctor.as_ref()
                    .map(|call| format!("{}::{}", call.class, call.method))
            })
            .collect();
        assert_eq!(labels, [Some("Alpha::constructor".to_owned()), None]);
    }

    /// `rule:programs/implementing`'s class selector over an abstract base: it
    /// lists its concrete descendants, a concrete class in the middle is
    /// listed, and a class two levels down under an abstract one is reached.
    /// The answer is asserted whole, so a walk that listed the abstract
    /// classes or stopped at the first level fails here.
    #[test]
    fn program_implementing_takes_an_abstract_class_and_lists_its_concrete_subclasses() {
        let (exprs, span, diags) = check(
            "<?nvs\n\
             abstract class Exporter {}\n\
             class Csv extends Exporter {}\n\
             class Tsv extends Csv {}\n\
             abstract class Partial extends Csv {}\n\
             class Wide extends Partial {}\n\
             class Loose {}\n\
             Core\\Program::implementing<Exporter>();\n",
        );
        assert!(!diags.has_errors(), "fixture failed to check: {diags:?}");
        let Some(ExprInfo::ProgramInstances { classes, .. }) = exprs.lookup(span) else {
            panic!("the call recorded no expansion: {:?}", exprs.lookup(span));
        };
        let names: Vec<String> = classes.iter().map(ToString::to_string).collect();
        assert_eq!(names, ["Csv", "Tsv", "Wide"]);
    }

    /// `rule:programs/implementing`: a concrete `T` is in its own list, ahead
    /// of its subclasses only because its name sorts first.
    #[test]
    fn program_implementing_takes_a_concrete_class_and_lists_it_with_its_subclasses() {
        let (exprs, span, diags) = check(
            "<?nvs\n\
             class Csv {}\n\
             class Tsv extends Csv {}\n\
             Core\\Program::implementing<Csv>();\n",
        );
        assert!(!diags.has_errors(), "fixture failed to check: {diags:?}");
        let Some(ExprInfo::ProgramInstances { classes, .. }) = exprs.lookup(span) else {
            panic!("the call recorded no expansion: {:?}", exprs.lookup(span));
        };
        let names: Vec<String> = classes.iter().map(ToString::to_string).collect();
        assert_eq!(names, ["Csv", "Tsv"], "a concrete `T` is in its own list");
    }

    /// The selector's one refusal, reported where the type argument is
    /// written, for each kind of type that is neither an interface nor a
    /// class: an enum is the named type a check for "any named type" would
    /// let through, a shape is a type with fields, and `int` names no
    /// declaration at all.
    #[test]
    fn program_implementing_refuses_an_enum_a_shape_and_a_scalar() {
        for selector in ["Suit", "{name: string}", "int"] {
            let (exprs, span, diags) = check(&format!(
                "<?nvs\n\
                 enum Suit {{ Hearts = 1 }}\n\
                 Core\\Program::implementing<{selector}>();\n",
            ));
            assert!(
                diags.iter().any(|d| d.code
                    == Some(nvs_diagnostics::code::E_PROGRAM_TYPE_ARG_NOT_A_CLASS_OR_INTERFACE)),
                "expected E0743 for `{selector}`, got: {diags:?}"
            );
            assert!(
                !matches!(exprs.lookup(span), Some(ExprInfo::ProgramInstances { .. })),
                "a refused enumeration of `{selector}` must record nothing for `nvs-ir` to lower"
            );
        }
    }

    /// § 3's second refusal, and the bound it rests on asserted from both
    /// sides: what the expansion cannot write is `new Needy(8080)`, so the
    /// test is `required() > 0` rather than "declares a constructor" — the
    /// same class with the same parameter *defaulted* is enumerated and
    /// carries its constructor. A member that read the parameter count
    /// instead would refuse both and still look right on the first half
    /// alone. The class is named in the message because the fix is in that
    /// class rather than at the call the diagnostic points at.
    #[test]
    fn an_implementor_without_a_no_argument_constructor_is_named() {
        let (exprs, span, diags) = check(
            "<?nvs\n\
             interface Module { public function tag(): string; }\n\
             class Needy implements Module {\n\
                 public function constructor(int $port) {}\n\
                 public function tag(): string { return \"n\"; }\n\
             }\n\
             Core\\Program::implementing<Module>();\n",
        );
        let named = diags.iter().find(|d| {
            d.code
                == Some(nvs_diagnostics::code::E_PROGRAM_IMPLEMENTOR_NEEDS_NO_ARGUMENT_CONSTRUCTOR)
        });
        let named = named.unwrap_or_else(|| panic!("expected E0744, got: {diags:?}"));
        assert!(
            named.message.contains("Needy"),
            "the diagnostic must name the class to fix: {}",
            named.message
        );
        assert!(
            !matches!(exprs.lookup(span), Some(ExprInfo::ProgramInstances { .. })),
            "a refused enumeration must record nothing for `nvs-ir` to lower"
        );

        let (exprs, span, diags) = check(
            "<?nvs\n\
             interface Module { public function tag(): string; }\n\
             class Needy implements Module {\n\
                 public function constructor(int $port = 8080) {}\n\
                 public function tag(): string { return \"n\"; }\n\
             }\n\
             Core\\Program::implementing<Module>();\n",
        );
        assert!(
            !diags.has_errors(),
            "an optional argument is not one: {diags:?}"
        );
        let Some(ExprInfo::ProgramInstances { classes, ctors }) = exprs.lookup(span) else {
            panic!("the call recorded no expansion: {:?}", exprs.lookup(span));
        };
        assert_eq!(
            classes.iter().map(ToString::to_string).collect::<Vec<_>>(),
            ["Needy"]
        );
        let [Some(ctor)] = ctors.as_slice() else {
            panic!("one resolved constructor: {ctors:?}");
        };
        assert_eq!(
            (ctor.class.to_string().as_str(), ctor.method.as_str()),
            ("Needy", "constructor")
        );
        // The optional parameter is one the constructor's own default fills:
        // the call carries the signature `new Needy()` would carry, and no
        // written slot, which is what makes the instance carry `1` rather
        // than an unset slot.
        assert!(
            ctor.arg_slots.is_empty(),
            "nothing was written: {:?}",
            ctor.arg_slots
        );
        assert_eq!(
            ctor.param_tys.len(),
            1,
            "one parameter for the default to fill: {:?}",
            ctor.param_names
        );
    }

    /// `rule:programs/implementing-with`: the joined form records the
    /// enumeration's list beside one folded payload per class — the matching
    /// literal as a shape constant, `null` where the class's member carries
    /// none — in the enumeration's order.
    // covers: Core\Program::implementingWith
    #[test]
    fn program_implementing_with_records_one_payload_per_class() {
        let (exprs, span, diags) = check(
            "<?nvs\n\
             interface Module { public function tag(): string; }\n\
             class Beta implements Module {\n\
                 #[{order: 2}]\n\
                 public function tag(): string { return \"b\"; }\n\
             }\n\
             class Alpha implements Module {\n\
                 public function tag(): string { return \"a\"; }\n\
             }\n\
             Core\\Program::implementingWith<Module, {order: int}>(\"tag\");\n",
        );
        assert!(!diags.has_errors(), "the join was refused: {diags:?}");
        let Some(ExprInfo::ProgramInstancesWith {
            classes,
            ctors,
            payloads,
        }) = exprs.lookup(span)
        else {
            panic!("the call recorded no join: {:?}", exprs.lookup(span));
        };
        assert_eq!(
            classes.iter().map(ToString::to_string).collect::<Vec<_>>(),
            ["Alpha", "Beta"]
        );
        assert!(
            ctors.iter().all(Option::is_none),
            "no class declares a constructor: {ctors:?}"
        );
        let [alpha, beta] = payloads.as_slice() else {
            panic!("one payload per class: {payloads:?}");
        };
        assert!(
            matches!(alpha, crate::defaults::ConstArg::Null),
            "a class whose member carries no match folds to `null`: {alpha:?}"
        );
        let crate::defaults::ConstArg::Shape(fields) = beta else {
            panic!("the matched literal folds to a shape: {beta:?}");
        };
        assert_eq!(fields.len(), 1);
        assert_eq!(fields[0].0, "order");
        assert!(matches!(fields[0].1, crate::defaults::ConstArg::Int(2)));
    }

    /// `rule:programs/implementing-with`: `I` selects exactly as
    /// `implementing<I>` does, so a class selects its concrete classes, itself
    /// included, and the abstract one in between is not a row.
    #[test]
    fn program_implementing_with_takes_a_class() {
        let (exprs, span, diags) = check(
            "<?nvs\n\
             class Page {}\n\
             abstract class Section extends Page {}\n\
             #[{order: 1}]\n\
             class About extends Section {}\n\
             Core\\Program::implementingWith<Page, {order: int}>();\n",
        );
        assert!(!diags.has_errors(), "the join was refused: {diags:?}");
        let Some(ExprInfo::ProgramInstancesWith {
            classes, payloads, ..
        }) = exprs.lookup(span)
        else {
            panic!("the call recorded no join: {:?}", exprs.lookup(span));
        };
        assert_eq!(
            classes.iter().map(ToString::to_string).collect::<Vec<_>>(),
            ["About", "Page"]
        );
        let [about, page] = payloads.as_slice() else {
            panic!("one payload per class: {payloads:?}");
        };
        assert!(
            matches!(about, crate::defaults::ConstArg::Shape(_)),
            "`About`'s own attribute is its row's: {about:?}"
        );
        assert!(
            matches!(page, crate::defaults::ConstArg::Null),
            "`Page` carries no attribute: {page:?}"
        );
    }
}
