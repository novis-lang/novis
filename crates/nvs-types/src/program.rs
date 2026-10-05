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
//!
//! `rule:programs/constructors`' `constructors<T, C>()` is the third member,
//! and the one that lifts the second refusal: it builds nothing, so it lists
//! the same classes with one closure each, and a constructor is refused only
//! where `C`'s parameters cannot call it (`E0839`), or where `C` is not a
//! callable returning `T` (`E0838`).

use nvs_diagnostics::{Diagnostic, code};
use nvs_hir::{QName, SymbolKind};
use nvs_syntax::ast::{CallArgs, Expr, ExprKind};

use crate::defaults::ConstArg;
use crate::expr::args::check_args_typed;
use crate::expr::calls::resolved_call;
use crate::expr::is_assignable;
use crate::expr::members::{check_method_visibility, class_qname_of};
use crate::expr_table::{ArgSlot, ExprInfo, ResolvedCall};
use crate::locals::{Live, LocalScope};
use crate::retrieval::{fold_payload, matching, sites_for};
use crate::signatures::{resolve_method, resolve_property};
use crate::ty::{ShapeField, Ty, TypeId};
use crate::{Ctx, Env};

/// The one class this pass answers for.
const OWNER: &str = r"Core\Program";

/// The joined enumeration, `rule:programs/implementing-with`'s member.
const WITH: &str = "implementingWith";

/// The typed constructors, `rule:programs/constructors`' member.
const CONSTRUCTORS: &str = "constructors";

/// Whether `owner::member` is an enumeration — the same nominal test
/// [`crate::retrieval::is_retrieval`] makes, against a resolved [`QName`]
/// rather than against what the call site spelled. Three members answer:
/// `implementing`, expanded by [`expand`], `implementingWith`, expanded by
/// [`expand_with`], and `constructors`, expanded by [`expand_constructors`].
pub(crate) fn is_enumeration(owner: &QName, member: &str) -> bool {
    owner.to_string() == OWNER && matches!(member, "implementing" | WITH | CONSTRUCTORS)
}

/// Whether `member` is the typed-constructor form, which
/// [`expand_constructors`] answers with a type of its own.
pub(crate) fn is_constructors(member: &str) -> bool {
    member == CONSTRUCTORS
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
    live: &mut Live,
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
    live: &mut Live,
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

/// `rule:programs/constructors`: [`expand`]'s list, each class beside the
/// constructor its `make` closure calls, recorded as
/// [`ExprInfo::ProgramConstructors`].
///
/// `C` must be a `callable(...)` type naming its parameters, with a return
/// type a `T` is assignable to — anything else is `E0838`. Each listed class
/// is then checked as `fn(<C's parameters>): T => new Class(<the same
/// arguments>)` would be at the call: the constructor visible here, every
/// parameter of `C` assignable to the constructor's parameter at its place,
/// and every constructor parameter past them defaulted. A class that fails is
/// `E0839` naming it, with the reason as a note, and every such class is
/// reported before the call is refused.
///
/// Returns `array<{class: string, make: C}>`, refused or not, so a refusal is
/// reported once rather than again by whatever reads the answer. The
/// closures' own signature, `callable(<C's parameters>): T`, is recorded
/// against the call's span so a `$f is callable(...)` test sees the classes
/// `nvs-ir` builds for them.
pub(crate) fn expand_constructors(
    call: &Expr,
    written: &[TypeId],
    ctx: &Ctx<'_>,
    env: &mut Env<'_>,
) -> TypeId {
    let (Some(want), Some(maker)) = (written.first().copied(), written.get(1).copied()) else {
        // The type-argument count is `check_written_type_args`' refusal.
        return env.interner.mixed();
    };
    let text = env.interner.string();
    let row = env.interner.shape(vec![
        ShapeField {
            name: "class".to_owned(),
            ty: text,
            required: true,
        },
        ShapeField {
            name: "make".to_owned(),
            ty: maker,
            required: true,
        },
    ]);
    let answer = env.interner.array(row);
    let Some(selector) = selector_of(call, CONSTRUCTORS, want, env) else {
        return answer;
    };
    let shape = match env.interner.get(maker) {
        Ty::CallableSig { params, ret } => Some((params.clone(), *ret)),
        _ => None,
    };
    let params = match shape {
        Some((params, ret))
            if is_assignable(want, ret, env.interner, env.graph, env.signatures) =>
        {
            params
        }
        _ => {
            let found = env.interner.describe(maker);
            env.diags.report(
                Diagnostic::error(
                    code::E_PROGRAM_CONSTRUCTORS_TYPE_ARG_NOT_A_MAKER,
                    format!(
                        "`Core\\Program::{CONSTRUCTORS}` needs a `callable(...)` type returning \
                         `{selector}`, and `{found}` is not one"
                    ),
                )
                .with_primary(call.span, format!("`{found}` written here"))
                .with_help(format!(
                    "`rule:programs/constructors`: each row's `make` takes the parameters this \
                     type names and returns a new `{selector}` — write it as \
                     `callable(<the constructor's parameters>): {selector}`"
                )),
            );
            return answer;
        }
    };

    let classes = nvs_hir::implementors(&selector, env.graph);
    let mut ctors = Vec::with_capacity(classes.len());
    let mut refused = false;
    for class in &classes {
        match constructor_fitting(class, &params, maker, call, ctx, env) {
            Ok(ctor) => ctors.push(ctor),
            Err(reason) => {
                let shown = env.interner.describe(maker);
                env.diags.report(
                    Diagnostic::error(
                        code::E_PROGRAM_CONSTRUCTOR_DOES_NOT_FIT,
                        format!(
                            "`{class}`'s constructor cannot be called with `{shown}`'s parameters"
                        ),
                    )
                    .with_primary(
                        call.span,
                        format!("this lists `{class}`, which is a `{selector}`"),
                    )
                    .with_note(reason)
                    .with_help(format!(
                        "`rule:programs/constructors`: each row's `make` is \
                         `fn(<{shown}'s parameters>): {selector} => new {class}(<the same \
                         arguments>)`, checked as that closure would be here — change \
                         `{class}`'s constructor, or the parameters `{shown}` names",
                    )),
                );
                refused = true;
            }
        }
    }
    if refused {
        return answer;
    }
    env.exprs.record(
        call.span,
        ExprInfo::ProgramConstructors {
            classes,
            ctors,
            params: params.clone(),
        },
    );
    let made = env.interner.callable_sig(params, want);
    env.exprs.record_callable_value(call.span, made);
    answer
}

/// The constructor `make` calls for one listed class, already resolved as
/// `new class(<one argument per parameter of C>)` would resolve it, or the
/// reason it cannot be called that way.
///
/// `None` inside the `Ok` is a class declaring no constructor, which fits
/// exactly when `C` takes no parameters. A variadic constructor takes nothing
/// into its tail here: `C`'s parameters fill the fixed ones, so one more
/// parameter than those is refused like any other.
fn constructor_fitting(
    class: &QName,
    params: &[TypeId],
    maker: TypeId,
    call: &Expr,
    ctx: &Ctx<'_>,
    env: &mut Env<'_>,
) -> Result<Option<ResolvedCall>, String> {
    let shown = env.interner.describe(maker);
    let Some((owner, sig)) = resolve_method(class, "constructor", env.signatures, env.graph) else {
        if params.is_empty() {
            return Ok(None);
        }
        return Err(format!(
            "`{class}` declares no constructor, so `new {class}()` takes no arguments, and \
             `{shown}` passes {}",
            params.len()
        ));
    };
    // The visibility `new` faces at this call, reported by the same check a
    // written `new` runs and then moved into the note: one diagnostic per
    // class, with the class named first.
    let before = env.diags.len();
    check_method_visibility(&owner, "constructor", &sig, call.span, ctx, env);
    if env.diags.len() > before {
        let reason = env
            .diags
            .iter()
            .nth(before)
            .map(|d| d.message.clone())
            .unwrap_or_default();
        env.diags.truncate(before);
        return Err(reason);
    }
    let fixed = sig.params.len() - usize::from(sig.variadic);
    if params.len() > fixed {
        return Err(format!(
            "`{owner}::constructor` takes {fixed} argument(s), and `{shown}` passes {}",
            params.len()
        ));
    }
    if sig.required() > params.len() {
        return Err(format!(
            "`{owner}::constructor` needs {} argument(s), and `{shown}` passes {}",
            sig.required(),
            params.len()
        ));
    }
    for (index, &given) in params.iter().enumerate() {
        let name = &sig.param_names[index];
        if sig.inout.get(index).copied().unwrap_or(false) {
            return Err(format!(
                "`{owner}::constructor`'s `${name}` is `inout`, and a closure has no variable \
                 to pass there"
            ));
        }
        let wanted = sig.params[index];
        if !is_assignable(given, wanted, env.interner, env.graph, env.signatures) {
            let (given, wanted) = (env.interner.describe(given), env.interner.describe(wanted));
            return Err(format!(
                "parameter {} of `{shown}` is `{given}`, and `{owner}::constructor`'s `${name}` \
                 is `{wanted}`",
                index + 1
            ));
        }
    }
    let slots = (0..params.len()).map(ArgSlot::Param).collect();
    Ok(Some(resolved_call(
        owner,
        "constructor".to_owned(),
        &sig,
        slots,
        env.signatures,
    )))
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
    live: &mut Live,
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
                     zero-argument `constructor`, or list the classes with \
                     `Core\\Program::constructors<{selector}, callable(...): {selector}>()`, \
                     which passes the arguments you give it (`rule:programs/constructors`)",
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

    /// The codes of every error `diags` holds, in report order.
    fn codes(diags: &Diagnostics) -> Vec<&'static str> {
        diags
            .iter()
            .filter_map(|d| d.code.map(|code| code.as_str()))
            .collect()
    }

    /// `rule:programs/constructors`: the call records the list rather than a
    /// call, one resolved constructor per class, and `C`'s parameter list —
    /// which is every closure's own, so `nvs-ir` builds one closure per class
    /// from it.
    // covers: Core\Program::constructors
    #[test]
    fn program_constructors_expands_to_one_typed_closure_per_class() {
        let (exprs, span, diags) = check(
            "<?nvs\n\
             interface Module { public function tag(): string; }\n\
             class Beta implements Module {\n\
                 public function constructor(public int $port) {}\n\
                 public function tag(): string { return \"b\"; }\n\
             }\n\
             class Alpha implements Module {\n\
                 public function constructor(public int $port) {}\n\
                 public function tag(): string { return \"a\"; }\n\
             }\n\
             Core\\Program::constructors<Module, callable(int): Module>();\n",
        );
        assert!(!diags.has_errors(), "the call was refused: {diags:?}");
        let Some(ExprInfo::ProgramConstructors {
            classes,
            ctors,
            params,
        }) = exprs.lookup(span)
        else {
            panic!("the call recorded no expansion: {:?}", exprs.lookup(span));
        };
        assert_eq!(params.len(), 1, "one parameter, `C`'s `int`");
        assert_eq!(
            classes.iter().map(ToString::to_string).collect::<Vec<_>>(),
            ["Alpha", "Beta"]
        );
        let labels: Vec<String> = ctors
            .iter()
            .map(|ctor| {
                let ctor = ctor.as_ref().expect("both classes declare a constructor");
                format!("{}::{}", ctor.class, ctor.method)
            })
            .collect();
        assert_eq!(labels, ["Alpha::constructor", "Beta::constructor"]);
    }

    /// `rule:programs/constructors`: "one row per class `implementing<T>()`
    /// would list, in the same order". The two lists are asked of one program
    /// and compared whole, so a member that sorted or filtered on its own
    /// fails here while still looking right alone.
    #[test]
    fn program_constructors_lists_the_classes_implementing_lists_in_its_order() {
        let declarations = "interface Module { public function tag(): string; }\n\
             class Zulu implements Module { public function tag(): string { return \"z\"; } }\n\
             class Mike implements Module { public function tag(): string { return \"m\"; } }\n\
             abstract class Base implements Module {}\n\
             class Alpha extends Base { public function tag(): string { return \"a\"; } }\n";
        let (exprs, span, diags) = check(&format!(
            "<?nvs\n{declarations}Core\\Program::implementing<Module>();\n"
        ));
        assert!(!diags.has_errors(), "{diags:?}");
        let Some(ExprInfo::ProgramInstances { classes, .. }) = exprs.lookup(span) else {
            panic!(
                "`implementing` recorded no expansion: {:?}",
                exprs.lookup(span)
            );
        };
        let listed: Vec<String> = classes.iter().map(ToString::to_string).collect();

        let (exprs, span, diags) = check(&format!(
            "<?nvs\n{declarations}Core\\Program::constructors<Module, callable(): Module>();\n"
        ));
        assert!(!diags.has_errors(), "{diags:?}");
        let Some(ExprInfo::ProgramConstructors { classes, .. }) = exprs.lookup(span) else {
            panic!(
                "`constructors` recorded no expansion: {:?}",
                exprs.lookup(span)
            );
        };
        let made: Vec<String> = classes.iter().map(ToString::to_string).collect();
        assert_eq!(listed, ["Alpha", "Mike", "Zulu"]);
        assert_eq!(made, listed, "the two lists can be zipped");
    }

    /// The limit `implementing` has and this member lifts: a constructor with
    /// a required parameter is enumerated, and its resolved call fills one
    /// slot per parameter of `C`, leaving the defaulted one after it to the
    /// constructor's own default.
    #[test]
    fn program_constructors_accepts_a_constructor_that_takes_arguments() {
        let (exprs, span, diags) = check(
            "<?nvs\n\
             interface Module { public function tag(): string; }\n\
             class Needy implements Module {\n\
                 public function constructor(public int $port, public string $host = \"local\") {}\n\
                 public function tag(): string { return \"n\"; }\n\
             }\n\
             Core\\Program::constructors<Module, callable(int): Module>();\n",
        );
        assert!(
            !diags.has_errors(),
            "a required argument `C` passes is accepted: {diags:?}"
        );
        let Some(ExprInfo::ProgramConstructors { ctors, .. }) = exprs.lookup(span) else {
            panic!("the call recorded no expansion: {:?}", exprs.lookup(span));
        };
        let [Some(ctor)] = ctors.as_slice() else {
            panic!("one resolved constructor: {ctors:?}");
        };
        assert_eq!(
            ctor.arg_slots,
            [crate::expr_table::ArgSlot::Param(0)],
            "one slot per parameter of `C`"
        );
        assert_eq!(ctor.param_tys.len(), 2, "the default fills the second");
    }

    /// `C`'s refusal, `E0838`, for each way it can be wrong: bare `callable`
    /// names no parameters, a return type a `T` is not assignable to, and a
    /// type that is not callable at all. Nothing is recorded for `nvs-ir`.
    #[test]
    fn program_constructors_needs_a_callable_returning_t() {
        for maker in ["callable", "callable(int): string", "int"] {
            let (exprs, span, diags) = check(&format!(
                "<?nvs\n\
                 interface Module {{ public function tag(): string; }}\n\
                 Core\\Program::constructors<Module, {maker}>();\n",
            ));
            assert_eq!(codes(&diags), ["E0838"], "for `{maker}`: {diags:?}");
            assert!(
                !matches!(
                    exprs.lookup(span),
                    Some(ExprInfo::ProgramConstructors { .. })
                ),
                "a refused call of `{maker}` must record nothing for `nvs-ir` to lower"
            );
        }
    }

    /// `E0839`, one per class that does not fit, each naming its class: one
    /// argument too few, one too many and one of the wrong type. The class
    /// that fits between them is not named, so a check that refused the whole
    /// list for one class fails here.
    #[test]
    fn program_constructors_names_the_class_whose_constructor_does_not_fit() {
        let (exprs, span, diags) = check(
            "<?nvs\n\
             interface Job { public function run(): void; }\n\
             class Mailer implements Job {\n\
                 public function constructor(public string $host, public int $port) {}\n\
                 public function run(): void {}\n\
             }\n\
             class Sweeper implements Job { public function run(): void {} }\n\
             class Timer implements Job {\n\
                 public function constructor(public int $seconds) {}\n\
                 public function run(): void {}\n\
             }\n\
             class Fits implements Job {\n\
                 public function constructor(public string $name) {}\n\
                 public function run(): void {}\n\
             }\n\
             Core\\Program::constructors<Job, callable(string): Job>();\n",
        );
        assert_eq!(codes(&diags), ["E0839", "E0839", "E0839"], "{diags:?}");
        let named: Vec<&str> = diags
            .iter()
            .map(|d| {
                ["Mailer", "Sweeper", "Timer", "Fits"]
                    .into_iter()
                    .find(|class| d.message.starts_with(&format!("`{class}`")))
                    .unwrap_or("")
            })
            .collect();
        assert_eq!(named, ["Mailer", "Sweeper", "Timer"]);
        assert!(
            diags
                .iter()
                .all(|d| d.notes.iter().any(|note| !note.starts_with("help:"))),
            "each carries its reason as a note: {diags:?}"
        );
        assert!(
            !matches!(
                exprs.lookup(span),
                Some(ExprInfo::ProgramConstructors { .. })
            ),
            "a refused call must record nothing for `nvs-ir` to lower"
        );
    }

    /// "the visibility `new` faces there": a private constructor is refused
    /// from outside the class, with the ordinary visibility error as the
    /// note, and accepted from the class's own body, where `new` reaches it.
    #[test]
    fn program_constructors_respects_a_private_constructor() {
        let vault = "interface Job { public function run(): void; }\n\
             class Vault implements Job {\n\
                 private function constructor(public string $key) {}\n\
                 public function run(): void {}\n\
                 public static function makers(): array<{class: string, make: callable(string): Job}> {\n\
                     return Core\\Program::constructors<Job, callable(string): Job>();\n\
                 }\n\
             }\n";
        let (_, _, diags) = check(&format!(
            "<?nvs\n{vault}Core\\Program::constructors<Job, callable(string): Job>();\n"
        ));
        assert_eq!(
            codes(&diags),
            ["E0839"],
            "refused outside `Vault`: {diags:?}"
        );
        let refused = diags.iter().next().expect("one diagnostic");
        assert!(refused.message.contains("Vault"), "{}", refused.message);
        assert!(
            refused.notes.iter().any(|note| note.contains("private")),
            "the note is the visibility error: {:?}",
            refused.notes
        );

        let (_, _, diags) = check(&format!("<?nvs\n{vault}Vault::makers();\n"));
        assert!(!diags.has_errors(), "accepted inside `Vault`: {diags:?}");
    }
}
