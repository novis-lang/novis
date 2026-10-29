//! Interface conformance: every method a class's interfaces declare without
//! a body, the class has to answer.
//!
//! # Why this exists
//!
//! An interface is what a call dispatches through. `rule:iteration/two-interfaces`'s `Iterator<T>` is the case that needs it: its members are bodiless *on purpose*
//! ([`crate::iter_lib`]'s own docs own why — a call resolving to a bodiless
//! declaration has no compiled function to name, so it dispatches on the
//! receiver's runtime class, which is exactly what driving a cursor of
//! unknown concrete class needs). A class claiming `implements Iterator<int>`
//! and forgetting `advance` therefore ends in a dispatch to nothing rather
//! than in a call the author wrote by hand.
//!
//! # What it checks, and what it deliberately does not
//!
//! One walk of every `extends`/`implements` ancestor. Each ancestor's own
//! bodiless, non-`interface_private` methods are the obligation; the class
//! discharges one by having [`resolve_method`] land on a declaration that
//! *does* have a body — its own, an inherited one, or an `rule:classes/interface-default-methods`
//! interface default.
//!
//! The same walk asks one question of the answers it accepts: a member the
//! ancestor declared `: static` has to be answered by a declaration that also
//! writes `static` (`E0404`, [`reject_dropped_static_return`]). The rest of
//! the two signatures is [`check_method_overrides`]'s, which compares every
//! override with the declaration it replaces
//! (`rule:classes/an-override-keeps-its-parents-signature`).
//!
//! These are outside it, each for its own reason:
//!
//! - **An `abstract` class is exempt.** Leaving a member to a subclass is
//!   what the modifier means.
//! - **A member `rule:classes/delegation-by-property`'s `by $field` delegation supplies is exempt, one
//!   member at a time.** Delegation answers a member from a property's own
//!   type, so a synthesized forward discharges the obligation exactly as a
//!   written body does — but only for the members that actually get one.
//!   The exemption is per-member rather than whole-class because the rule's
//!   field requirement is checked here: [`check_delegate_field`] asks whether
//!   `$field` is a declared property of a non-nullable type that satisfies
//!   the interface (`E0720`), so a member no forward covers can be told from
//!   one whose field could not have answered it. [`resolve_delegations`] is
//!   what runs both halves, and its own doc comment owns what a forward is
//!   and which members get one.
//!
//! The compiler-declared global interfaces are *inside* it, and reach it
//! the same way a source-declared one does: [`crate::iter_lib`] seeds every
//! member on [`nvs_hir::interfaces`]'s roster, so `implements Comparable`
//! owes `compareTo` and `implements Stringable` owes `toString` here, rather
//! than only at the use sites (`crate::expr`'s `require_stringable` and the
//! object comparison check).

use nvs_diagnostics::{Diagnostic, Span, code};
use nvs_hir::{QName, SymbolKind, implements_interface};
use nvs_syntax::ast::{ClassDecl, ClassMember, ClassMemberKind, Modifier, Visibility};
use rustc_hash::FxHashSet;

use crate::Env;
use crate::derive::stands_in_for_a_class;
use crate::expr::is_assignable;
use crate::expr_table::{ArgSlot, Delegation};
use crate::signatures::{resolve_method, resolve_property};
use crate::ty::{Ty, TypeId};
use crate::{span_text, strip_sigil};

/// `rule:statements/static-is-a-member-modifier`'s one obligation on a
/// constant redeclaration: a class or interface that redeclares a name a
/// direct ancestor already declares keeps a type assignable to the ancestor's
/// (`E0833`).
///
/// A `static::NAME` read is typed at the declaring class and lowered as a
/// load at that type, and it answers whichever class the call was made on —
/// so a redeclaration at another type would be read at the wrong
/// representation, the `secret` bit included. The test is assignability
/// rather than equality, because a union reads a tagged value whole, and it
/// has no `int` → `float` row, because nothing converts the called class's
/// `int` into the `float` the load expects. Every declaration kind is walked,
/// `abstract` classes and interfaces too: the read may be made through any
/// of them, and an interface's own redeclaration over one it `extends` is the
/// same hazard.
pub(crate) fn check_constant_redeclarations(
    members: &[ClassMember],
    qname: &QName,
    env: &mut Env<'_>,
) {
    let Some(links) = env.graph.get(qname) else {
        return;
    };
    let parents: Vec<QName> = links
        .extends
        .iter()
        .chain(links.implements.iter())
        .cloned()
        .collect();
    for member in members {
        let ClassMemberKind::Const(constant) = &member.kind else {
            continue;
        };
        let name = span_text(env.src, constant.name).to_owned();
        let Some(own) = env
            .signatures
            .get(qname)
            .and_then(|sig| sig.constants.get(&name))
            .map(|sig| sig.ty)
        else {
            continue;
        };
        for parent in &parents {
            let Some((owner, inherited)) =
                crate::signatures::resolve_const_owned(parent, &name, env.signatures, env.graph)
            else {
                continue;
            };
            let inherited = inherited.ty;
            if crate::expr::assign::field_fits(
                own,
                inherited,
                env.interner,
                env.graph,
                env.signatures,
            ) {
                continue;
            }
            let have = env.interner.describe(own);
            let want = env.interner.describe(inherited);
            env.diags.report(
                Diagnostic::error(
                    code::E_CONST_REDECLARED_AT_ANOTHER_TYPE,
                    format!("`{qname}::{name}` is `{have}`, and `{owner}::{name}` is `{want}`"),
                )
                .with_primary(constant.name, "redeclared at another type")
                .with_help(format!(
                    "a `static::{name}` read in `{owner}` has the type `{want}`. It can read the \
                     constant of any subclass, so each subclass must keep a type that fits \
                     `{want}`. Declare it `{want}` here, or a subtype of `{want}`"
                )),
            );
        }
    }
}

/// `rule:classes/a-redeclared-property-keeps-its-parents-type`: an instance
/// property a class declares again over a superclass's keeps the superclass's
/// exact type, and neither declaration is `readonly` (`E0866`).
///
/// Both declarations name one slot ([`crate::layout`] reuses the inherited
/// one), and the superclass's code reads and writes it at its own type. So
/// a narrower type lets the parent store what the subclass reads as another
/// class, and a wider one the reverse. "Exact" is [`field_fits`] both ways,
/// which has no `int` → `float` row. A hooked property is covered by the
/// same test: a `set` hook's parameter already accepts the property's own
/// type (`E0401`). A promoted constructor parameter is a declaration too.
/// A static property is skipped, since each class names its own storage.
///
/// A `covariant` declaration is the one narrower type allowed
/// (`rule:classes/a-covariant-type-is-checked-where-a-value-enters`). It is
/// measured against the topmost declaration, the first superclass that
/// declares the property, and it must also fit the nearest one, so a
/// grandchild never widens what its parent narrowed. It needs a type `is`
/// tests in constant time (`E0883`), and one the topmost type does not fit
/// (`E0882`). A declaration without the keyword is exact against both.
/// Each write that can reach the narrowed slot with a wider value is tested
/// where it is written, which `crate::expr::assign` records.
///
/// [`field_fits`]: crate::expr::assign::field_fits
pub(crate) fn check_property_redeclarations(decl: &ClassDecl, qname: &QName, env: &mut Env<'_>) {
    let Some(links) = env.graph.get(qname) else {
        return;
    };
    let parents = links.extends.clone();
    let mut declared: Vec<Redeclared> = Vec::new();
    for member in &decl.members {
        match &member.kind {
            ClassMemberKind::Property(p) => {
                if p.modifiers.contains(&Modifier::Static) {
                    continue;
                }
                declared.push(Redeclared {
                    name: strip_sigil(span_text(env.src, p.name)).to_owned(),
                    span: p.name,
                    readonly: p.modifiers.contains(&Modifier::Readonly),
                    covariant: p.covariant.then_some(p.ty.span),
                });
            }
            ClassMemberKind::Method(m) if span_text(env.src, m.name) == "constructor" => {
                for p in m.params.iter().filter(|p| p.is_promoted()) {
                    declared.push(Redeclared {
                        name: strip_sigil(span_text(env.src, p.name)).to_owned(),
                        span: p.name,
                        readonly: p.modifiers.contains(&Modifier::Readonly),
                        covariant: p
                            .covariant
                            .then(|| p.ty.as_ref().map_or(p.span, |ty| ty.span)),
                    });
                }
            }
            _ => {}
        }
    }
    for Redeclared {
        name,
        span,
        readonly: own_readonly,
        covariant,
    } in declared
    {
        // `crate::signatures` reports a static or `readonly` one.
        let narrows = covariant.filter(|_| !own_readonly);
        let Some(own) = env
            .signatures
            .get(qname)
            .and_then(|sig| sig.properties.get(&name))
            .copied()
        else {
            continue;
        };
        let mut replaces = false;
        for parent in &parents {
            let Some((owner, inherited)) =
                crate::signatures::resolve_property_owned(parent, &name, env.signatures, env.graph)
            else {
                continue;
            };
            replaces = true;
            let parent_static = env
                .signatures
                .get(&owner)
                .is_some_and(|sig| sig.static_properties.iter().any(|(n, _)| *n == name));
            if parent_static {
                continue;
            }
            let readonly = own_readonly
                || crate::signatures::property_is_readonly(&owner, &name, env.signatures);
            if readonly {
                env.diags.report(
                    Diagnostic::error(
                        code::E_PROPERTY_REDECLARED_AT_ANOTHER_TYPE,
                        format!("`{qname}` declares `${name}` again, and it is `readonly`"),
                    )
                    .with_primary(span, "a `readonly` property declared again")
                    .with_help(format!(
                        "`{owner}` already declares `${name}`. A `readonly` property is written \
                         once, and both constructors would write it. Remove this declaration, or \
                         give the property another name"
                    )),
                );
                continue;
            }
            let (root, top) = topmost_property(&owner, inherited, &name, env);
            if let Some(at) = narrows {
                covariant_property_problem(
                    qname,
                    &name,
                    at,
                    own,
                    (&owner, inherited),
                    (&root, top),
                    env,
                );
                continue;
            }
            let exact =
                |a: TypeId, b: TypeId, env: &mut Env<'_>| fits(a, b, env) && fits(b, a, env);
            if exact(own, inherited, env) && exact(own, top, env) {
                continue;
            }
            // A type narrower than the topmost one is what `covariant` is for.
            let narrower = fits(own, top, env) && fits(inherited, top, env);
            let (owner, want_ty) = if exact(own, inherited, env) {
                (root, top)
            } else {
                (owner, inherited)
            };
            let have = env.interner.describe(own);
            let want = env.interner.describe(want_ty);
            let help = if narrower && !fits(top, own, env) {
                format!(
                    "both classes use the same property, and `{owner}` writes it as `{want}`. \
                     To take a narrower type, write `covariant` before it. Each value is then \
                     tested when it is written"
                )
            } else {
                format!(
                    "both classes use the same property, and `{owner}` reads and writes it as \
                     `{want}`. Declare it `{want}` here"
                )
            };
            env.diags.report(
                Diagnostic::error(
                    code::E_PROPERTY_REDECLARED_AT_ANOTHER_TYPE,
                    format!("`{qname}::${name}` is `{have}`, and `{owner}::${name}` is `{want}`"),
                )
                .with_primary(span, "declared again with another type")
                .with_help(help),
            );
        }
        if let Some(at) = narrows
            && !replaces
        {
            env.diags.report(
                Diagnostic::error(
                    code::E_COVARIANT_MISPLACED,
                    format!(
                        "`covariant` has no effect here, because no parent class of `{qname}` \
                         declares `${name}`"
                    ),
                )
                .with_primary(at, "`covariant` is written here")
                .with_help("`covariant` is for a property that a parent class declares. Remove it"),
            );
        }
    }
}

/// One instance property `check_property_redeclarations` measures.
struct Redeclared {
    /// The name, `$`-sigil not included.
    name: String,
    /// Where the name is written.
    span: Span,
    /// Whether it is `readonly`.
    readonly: bool,
    /// Where its type is written, when it is declared `covariant`.
    covariant: Option<Span>,
}

/// The first superclass that declares the property `name`, and its type,
/// starting from `owner`, which declares it as `ty`.
fn topmost_property(owner: &QName, ty: TypeId, name: &str, env: &Env<'_>) -> (QName, TypeId) {
    let mut top = (owner.clone(), ty);
    let mut seen = FxHashSet::default();
    while seen.insert(top.0.clone()) {
        let parent = env
            .graph
            .get(&top.0)
            .and_then(|links| links.extends.first().cloned());
        let Some(found) = parent.and_then(|parent| {
            crate::signatures::resolve_property_owned(&parent, name, env.signatures, env.graph)
        }) else {
            break;
        };
        top = found;
    }
    top
}

/// Reports what is wrong with `qname`'s `covariant` property `name` of type
/// `own`, written at `at`, against the nearest declaration above it and the
/// topmost one.
fn covariant_property_problem(
    qname: &QName,
    name: &str,
    at: Span,
    own: TypeId,
    (owner, inherited): (&QName, TypeId),
    (root, top): (&QName, TypeId),
    env: &mut Env<'_>,
) {
    let have = env.interner.describe(own);
    if !testable_in_constant_time(own, env) {
        env.diags.report(
            Diagnostic::error(
                code::E_COVARIANT_TYPE_NOT_TESTABLE,
                format!("a `covariant` property cannot have the type `{have}`"),
            )
            .with_primary(at, "each write would test every element of this value")
            .with_help(
                "use the parent's type for this property, and test the value where you read it",
            ),
        );
        return;
    }
    let want = env.interner.describe(top);
    if fits(top, own, env) {
        env.diags.report(
            Diagnostic::error(
                code::E_COVARIANT_MISPLACED,
                format!(
                    "`covariant` has no effect here, because `${name}` is not narrower than in \
                     `{root}`"
                ),
            )
            .with_primary(at, format!("`{have}` accepts every `{want}`"))
            .with_help("remove `covariant`. The property already takes every value a write passes"),
        );
        return;
    }
    let drops_tainted = crate::expr::quals::carries_tainted(top, env.interner)
        && !crate::expr::quals::carries_tainted(own, env.interner);
    let drops_secret = crate::expr::quals::carries_secret(top, env.interner)
        && !crate::expr::quals::carries_secret(own, env.interner);
    let top_want = want.clone();
    let (against, want) = if fits(own, top, env) {
        (owner, env.interner.describe(inherited))
    } else {
        (root, want)
    };
    let message = if drops_tainted || drops_secret {
        let qualifier = if drops_tainted { "tainted" } else { "secret" };
        format!(
            "`{qname}::${name}` is `{have}`, and `{root}::${name}` is `{top_want}`. `covariant` \
             keeps the `{qualifier}` qualifier"
        )
    } else if fits(own, top, env) && fits(own, inherited, env) {
        return;
    } else {
        format!(
            "`{qname}::${name}` is `{have}`, which is not narrower than `{want}` in `{against}`"
        )
    };
    env.diags.report(
        Diagnostic::error(code::E_PROPERTY_REDECLARED_AT_ANOTHER_TYPE, message)
            .with_primary(at, "declared again with another type")
            .with_help(format!(
                "a `covariant` property takes a type that `{against}::${name}` accepts. Declare it \
                 `{want}` or narrower"
            )),
    );
}

/// Reports one `E0449` per member of `decl`'s interfaces that nothing
/// answers, and one `E0404` per member the answer weakens. See the module
/// docs for the exemptions.
pub(crate) fn check_class_conformance(decl: &ClassDecl, qname: &QName, env: &mut Env<'_>) {
    if decl.modifiers.contains(&Modifier::Abstract) {
        return;
    }
    let delegated = if decl.implements.iter().any(|c| c.by_field.is_some()) {
        match resolve_delegations(decl, qname, env) {
            Some(covered) => covered,
            // The clauses could not be lined up with the resolved graph, so
            // there is no delegation to subtract and no program to lower —
            // see `resolve_delegations`' own comment.
            None => return,
        }
    } else {
        FxHashSet::default()
    };

    let mut seen = FxHashSet::default();
    let mut owed: Vec<(QName, String)> = Vec::new();
    collect_obligations(qname, env, &mut seen, &mut owed);

    for (interface, method) in owed {
        if delegated.contains(&method) {
            continue;
        }
        let answer = resolve_method(qname, &method, env.signatures, env.graph)
            .map(|(_, sig)| (sig.has_body, sig.returns_static));
        if let Some((true, keeps_static)) = answer {
            if !keeps_static {
                reject_dropped_static_return(decl, qname, &interface, &method, env);
            }
            continue;
        }
        env.diags.report(
            Diagnostic::error(
                code::E_INTERFACE_METHOD_MISSING,
                format!("`{qname}` does not declare `{method}`, which `{interface}` requires"),
            )
            .with_primary(
                decl.name.span,
                format!("this class implements `{interface}`"),
            )
            .with_help(format!(
                "declare `{method}` here, or give `{interface}` a default body for it"
            )),
        );
    }
}

/// One `E0859` per method `decl` does not declare that two of its own
/// `implements` clauses each supply — `rule:classes/member-conflict-is-an-error`.
///
/// A clause supplies a method in one of two ways: its interface answers it
/// with a default body, or the clause is `by $field` and the interface leaves
/// it bodiless, so the forward answers it. Each clause is asked through
/// [`resolve_method`], so an interface that overrides a default it inherits
/// supplies its own and not both, and a default two clauses reach through one
/// shared ancestor is one source. A default whose interface another source's
/// interface extends is dropped for the same reason.
///
/// The superclass chain is asked first and is never a source: a concrete
/// method it answers wins over every interface, which is the order
/// [`crate::layout`]'s method walk dispatches in. Only the clauses `decl`
/// writes itself are compared, so a subclass inherits its parent's answer and
/// is not reported a second time. An `abstract` class is checked too, because
/// the conflict is in the clauses it wrote.
pub(crate) fn check_member_conflicts(decl: &ClassDecl, qname: &QName, env: &mut Env<'_>) {
    let Some(links) = env.graph.get(qname) else {
        return;
    };
    // The graph drops a clause that resolved to nothing, and that error is
    // already reported — see `resolve_delegations`.
    if links.implements.len() < 2 || links.implements.len() != decl.implements.len() {
        return;
    }
    let interfaces = links.implements.clone();
    let superclasses = links.extends.clone();

    let mut names: Vec<String> = Vec::new();
    let mut seen = FxHashSet::default();
    for interface in &interfaces {
        reachable_method_names(interface, env, &mut seen, &mut names);
    }
    names.sort();
    names.dedup();

    let declared = env.signatures.get(qname);
    for method in names {
        if declared.is_some_and(|sig| sig.methods.contains_key(&method))
            || superclasses.iter().any(|parent| {
                resolve_method(parent, &method, env.signatures, env.graph)
                    .is_some_and(|(_, sig)| sig.has_body)
            })
        {
            continue;
        }
        // `(interface the body is declared on, the delegate field)`.
        let mut sources: Vec<(QName, Option<String>)> = Vec::new();
        for (clause, interface) in decl.implements.iter().zip(&interfaces) {
            let Some((declaring, sig)) =
                resolve_method(interface, &method, env.signatures, env.graph)
            else {
                continue;
            };
            if sig.interface_private {
                continue;
            }
            let source = if sig.has_body {
                (declaring, None)
            } else if let Some(field_span) = clause.by_field {
                let field = strip_sigil(span_text(env.src, field_span)).to_owned();
                (interface.clone(), Some(field))
            } else {
                continue;
            };
            if !sources.contains(&source) {
                sources.push(source);
            }
        }
        let defaults: Vec<QName> = sources
            .iter()
            .filter(|(_, field)| field.is_none())
            .map(|(interface, _)| interface.clone())
            .collect();
        sources.retain(|(interface, field)| {
            field.is_some()
                || !defaults.iter().any(|other| {
                    other != interface && implements_interface(other, interface, env.graph)
                })
        });
        if sources.len() < 2 {
            continue;
        }
        let named: Vec<String> = sources
            .iter()
            .map(|(interface, field)| match field {
                None => format!("the default in `{interface}`"),
                Some(field) => format!("`{interface}` through `${field}`"),
            })
            .collect();
        let calls: Vec<String> = sources
            .iter()
            .map(|(interface, field)| match field {
                None => format!("`{interface}::{method}()`"),
                Some(field) => format!("`$this->{field}->{method}()`"),
            })
            .collect();
        env.diags.report(
            Diagnostic::error(
                code::E_INTERFACE_MEMBER_CONFLICT,
                format!(
                    "`{qname}` gets `{method}` from more than one source: {}",
                    named.join(", ")
                ),
            )
            .with_primary(
                decl.name.span,
                format!("`{qname}` does not declare `{method}`"),
            )
            .with_help(format!(
                "declare `{method}` in `{qname}`. Its body can call {}",
                calls.join(" or ")
            )),
        );
    }
}

/// Every method name `qname` and its ancestors declare, appended to `out`.
fn reachable_method_names(
    qname: &QName,
    env: &Env<'_>,
    seen: &mut FxHashSet<QName>,
    out: &mut Vec<String>,
) {
    if !seen.insert(qname.clone()) {
        return;
    }
    if let Some(sig) = env.signatures.get(qname) {
        out.extend(sig.methods.keys().cloned());
    }
    let Some(links) = env.graph.get(qname) else {
        return;
    };
    for parent in links.extends.iter().chain(links.implements.iter()) {
        reachable_method_names(parent, env, seen, out);
    }
}

/// `E0404` for an answer that drops the `static` its declaration promised.
///
/// A member declared `: static` promises the *called* class, and that promise
/// is what a caller reads: `Slug::parse($text)` is a `Slug` only because
/// [`MethodSig::returns_static`](crate::signatures::MethodSig::returns_static)
/// makes the call site substitute the class the call named. An answer written
/// `: Slug`, `: self` or `: string` keeps the assignability rule — every one of
/// those is assignable to the interface's own class, or is refused elsewhere —
/// and still breaks that substitution for every subclass, so assignability is
/// not the question here and a check of its own is.
///
/// This is the shape `Parses` needs and the only one this reports: `parse` is
/// declared `parse(tainted string $s): static`, and a binding site that hands
/// a route segment to it reads the answer as the class the parameter named.
/// An implementor answering anything else would put a value of one class where
/// the site had already decided another, and every later read of it would be
/// wrong.
///
/// The primary lands on the class's own declaration of the member where it has
/// one; a class that inherits the weakened answer gets its own name instead,
/// which is the only span it wrote.
fn reject_dropped_static_return(
    decl: &ClassDecl,
    qname: &QName,
    interface: &QName,
    method: &str,
    env: &mut Env<'_>,
) {
    let requires_static = env
        .signatures
        .get(interface)
        .and_then(|sig| sig.methods.get(method))
        .is_some_and(|sig| sig.returns_static);
    if !requires_static {
        return;
    }
    let written = decl.members.iter().find_map(|member| match &member.kind {
        ClassMemberKind::Method(m) if span_text(env.src, m.name) == method => Some(
            m.return_type
                .as_ref()
                .map_or(m.name, |declared| declared.span),
        ),
        _ => None,
    });
    env.diags.report(
        Diagnostic::error(
            code::E_INCOMPATIBLE_OVERRIDE,
            format!("`{qname}::{method}` does not return `static`, which `{interface}` declares"),
        )
        .with_primary(
            written.unwrap_or(decl.name.span),
            "this answers a class of its own",
        )
        .with_help(format!(
            "write `: static` here — `{interface}::{method}` promises the class the call named, \
             which a subclass may be"
        )),
    );
}

/// The promises `final` makes, checked where the declaration that breaks
/// one is written: no class extends a `final` class (`E0783`), and no class
/// redeclares a method an ancestor declared `final` (`E0784`).
///
/// Here rather than in a module of its own because it asks this module's own
/// question from the other side — what a class's ancestors impose on it — and
/// reads the same tables ([`resolve_method`] and the class graph) to do
/// it. The modifier itself is recorded per declaration
/// ([`crate::signatures::ClassSignature::final_methods`]), so both halves are
/// a lookup against the declaring class rather than a walk of the source.
///
/// The **class** half asks the direct superclass and nothing further: a
/// `final` class cannot be extended at all, so an ancestor two links up was
/// already refused where it was named, and reporting again here would name a
/// class the author never wrote.
pub(crate) fn check_class_finality(decl: &ClassDecl, qname: &QName, env: &mut Env<'_>) {
    let Some(links) = env.graph.get(qname) else {
        return;
    };
    let parent = links.extends.first().cloned();
    let ancestors: Vec<QName> = links
        .extends
        .iter()
        .chain(links.implements.iter())
        .cloned()
        .collect();

    if let (Some(parent), Some(written)) = (parent, decl.extends.as_ref())
        && crate::signatures::class_is_final(&parent, env.signatures)
    {
        env.diags.report(
            Diagnostic::error(
                code::E_FINAL_CLASS_EXTENDED,
                format!("`{parent}` is `final`, so no class extends it"),
            )
            .with_primary(
                written.span,
                format!("`{qname}` names it as its superclass"),
            )
            .with_help(format!(
                "remove `final` from the declaration of `{parent}` if other classes may extend it. \
                 Or store a `{parent}` in a property of `{qname}`, and write \
                 `implements … by $property` to forward the calls to it"
            )),
        );
    }

    for member in &decl.members {
        let ClassMemberKind::Method(m) = &member.kind else {
            continue;
        };
        let name = span_text(env.src, m.name).to_owned();
        let owner = ancestors
            .iter()
            .find_map(|ancestor| resolve_method(ancestor, &name, env.signatures, env.graph))
            .map(|(owner, _)| owner);
        let Some(owner) = owner else { continue };
        if !crate::signatures::method_is_final(&owner, &name, env.signatures) {
            continue;
        }
        env.diags.report(
            Diagnostic::error(
                code::E_FINAL_METHOD_OVERRIDDEN,
                format!("`{owner}::{name}` is `final`, so no subclass redeclares it"),
            )
            .with_primary(m.name, format!("`{qname}` declares it again here"))
            .with_help(format!(
                "give this method a name of its own, or drop the `final` from `{owner}`'s \
                 declaration of it"
            )),
        );
    }
}

/// `rule:classes/an-override-keeps-its-parents-signature`: every method a
/// class or an interface declares again over one an ancestor declares keeps a
/// signature a call through the ancestor can use (`E0404`).
///
/// A call through the ancestor's type is checked against the ancestor's
/// signature and runs the override, which reads its arguments and returns its
/// result with no conversion between the two. So the return type fits the
/// ancestor's in place, the ancestor's type of each parameter fits the
/// override's in place, and the relation is
/// [`crate::expr::assign::field_fits`], which leaves out the `int` → `float`
/// row that converts where a value is stored. The parameter count, `inout`,
/// the variadic tail, `static` and a visibility no narrower are compared
/// directly. The count is exact, since a call fills a default from the
/// ancestor's signature and never from the override's. A `tainted` or `secret` parameter is kept by the same relation,
/// since the qualifier is part of the parameter's type.
///
/// Each direct ancestor is asked through [`resolve_method`], so the nearest
/// declaration is the one compared, and an interface that redeclares a member
/// of the one it extends is checked here too. A method a class does not
/// declare but answers from its superclass is compared against each interface
/// the class names itself, at the class's name, which is the only span it
/// wrote. A generic interface's type variables are bound from the class's own
/// `implements` clause first, and `Comparable::compareTo` takes the class
/// itself for `self`, as `rule:classes/comparable` declares it.
pub(crate) fn check_method_overrides(
    members: &[ClassMember],
    name_span: Span,
    qname: &QName,
    env: &mut Env<'_>,
) {
    let Some(links) = env.graph.get(qname) else {
        return;
    };
    let ancestors: Vec<QName> = links
        .extends
        .iter()
        .chain(links.implements.iter())
        .cloned()
        .collect();
    let interfaces = links.implements.clone();
    let superclass = links.extends.first().cloned();
    let own_sigs = env.signatures.get(qname).map(|sig| sig.methods.clone());
    let Some(own_sigs) = own_sigs else {
        return;
    };

    for member in members {
        let ClassMemberKind::Method(m) = &member.kind else {
            continue;
        };
        let name = span_text(env.src, m.name).to_owned();
        if name == "constructor" {
            continue;
        }
        let Some(own) = own_sigs.get(&name).cloned() else {
            continue;
        };
        for ancestor in &ancestors {
            let Some((owner, parent)) = resolve_method(ancestor, &name, env.signatures, env.graph)
            else {
                continue;
            };
            let at = m.return_type.as_ref().map_or(m.name, |ty| ty.span);
            compare_override(qname, &owner, &name, &own, &parent, (m.name, at), env);
        }
    }

    let Some(superclass) = superclass else {
        return;
    };
    let mut names = Vec::new();
    let mut seen = FxHashSet::default();
    for interface in &interfaces {
        reachable_method_names(interface, env, &mut seen, &mut names);
    }
    names.sort();
    names.dedup();
    for name in names {
        if own_sigs.contains_key(&name) {
            continue;
        }
        let Some((_, inherited)) = resolve_method(&superclass, &name, env.signatures, env.graph)
        else {
            continue;
        };
        for interface in &interfaces {
            let Some((owner, declared)) =
                resolve_method(interface, &name, env.signatures, env.graph)
            else {
                continue;
            };
            compare_override(
                qname,
                &owner,
                &name,
                &inherited,
                &declared,
                (name_span, name_span),
                env,
            );
        }
    }
}

/// One `E0404` for the first way `own` cannot replace `parent`, which `owner`
/// declares, when `qname`'s method `name` is called through `owner`. `at` is
/// the method's name and its return type, or the class's name twice for an
/// inherited answer.
fn compare_override(
    qname: &QName,
    owner: &QName,
    name: &str,
    own: &crate::signatures::MethodSig,
    parent: &crate::signatures::MethodSig,
    at: (Span, Span),
    env: &mut Env<'_>,
) {
    if parent.interface_private || owner == qname {
        return;
    }
    let parent = bind_interface_args(qname, owner, parent, env);
    let (name_at, return_at) = at;
    let rank = |v: Visibility| match v {
        Visibility::Public => 2,
        Visibility::Protected => 1,
        Visibility::Private => 0,
    };
    let mut narrower = false;
    let problem: Option<(Span, String)> = if own.is_static != parent.is_static {
        let (is, was) = if parent.is_static {
            ("is not `static`", "is")
        } else {
            ("is `static`", "is not")
        };
        Some((
            name_at,
            format!("this method {is}, and `{owner}::{name}` {was}"),
        ))
    } else if rank(own.visibility) < rank(parent.visibility) {
        Some((
            name_at,
            format!(
                "this method is `{}`, and `{owner}::{name}` is `{}`",
                visibility_word(own.visibility),
                visibility_word(parent.visibility)
            ),
        ))
    } else if !crate::expr::assign::field_fits(
        own.return_ty,
        parent.return_ty,
        env.interner,
        env.graph,
        env.signatures,
    ) {
        let have = env.interner.describe(own.return_ty);
        let want = env.interner.describe(parent.return_ty);
        Some((
            return_at,
            format!("this method returns `{have}`, and `{owner}::{name}` returns `{want}`"),
        ))
    } else if parent.returns_static
        && !own.returns_static
        && (parent.has_body
            || env.symbols.get(owner).map(|symbol| symbol.kind) != Some(SymbolKind::Interface))
    {
        // A bodiless interface member is `reject_dropped_static_return`'s,
        // which reports it at every class that answers it.
        Some((
            return_at,
            format!("this method does not return `static`, and `{owner}::{name}` does"),
        ))
    } else if own.params.len() != parent.params.len() {
        // A further parameter is refused even with a default: a call fills
        // a default from the signature it resolved, which is `parent`'s, so
        // the override would read a parameter nothing passed.
        Some((
            name_at,
            format!(
                "this method takes {} parameter(s), and `{owner}::{name}` takes {}",
                own.params.len(),
                parent.params.len()
            ),
        ))
    } else if own.variadic != parent.variadic {
        Some((
            name_at,
            format!(
                "`{owner}::{name}` {} a variadic last parameter, and this method {}",
                if parent.variadic {
                    "takes"
                } else {
                    "does not take"
                },
                if own.variadic { "does" } else { "does not" }
            ),
        ))
    } else {
        parent_params_problem(qname, owner, name, own, &parent, name_at, env).map(
            |(span, message, is_narrower)| {
                narrower = is_narrower;
                (span, message)
            },
        )
    };
    let Some((span, message)) = problem else {
        return;
    };
    report_incompatible_override(qname, owner, name, span, message, narrower, env);
}

/// One `E0404` for `qname`'s method `name`, which a call through `owner`
/// cannot use. `narrower` is set where a parameter takes a narrower type than
/// `owner`'s, and the help then names `covariant`.
fn report_incompatible_override(
    qname: &QName,
    owner: &QName,
    name: &str,
    span: Span,
    message: String,
    narrower: bool,
    env: &mut Env<'_>,
) {
    let mut help = format!(
        "a call through `{owner}` passes what `{owner}::{name}` takes and reads what it \
         returns. Keep its parameters, its `static` and its visibility. Each parameter may \
         accept a wider type, and the return type may be narrower"
    );
    if narrower {
        help.push_str(
            ". To take a narrower type, write `covariant` before it. Each value is then tested \
             when the method is called",
        );
    }
    env.diags.report(
        Diagnostic::error(
            code::E_INCOMPATIBLE_OVERRIDE,
            format!("`{qname}::{name}` cannot replace `{owner}::{name}`"),
        )
        .with_primary(span, message)
        .with_help(help),
    );
}

/// The first parameter of `parent` that `own` cannot take as a call through
/// `owner` passes it: a type that does not fit in place, or `inout` written on
/// one side only. The flag is set where `own`'s type is narrower.
///
/// A `covariant` parameter's type is [`check_covariant_params`]'s, which
/// measures it against the topmost declaration once rather than once for each
/// ancestor.
fn parent_params_problem(
    qname: &QName,
    owner: &QName,
    name: &str,
    own: &crate::signatures::MethodSig,
    parent: &crate::signatures::MethodSig,
    name_at: Span,
    env: &mut Env<'_>,
) -> Option<(Span, String, bool)> {
    // Only `Comparable`'s own `compareTo` takes `self`. A class that narrows
    // it again in a subclass is refused like any other override: a direct
    // `$price->compareTo($other)` through the parent passes the parent's
    // class, and no ordering's run-time class check runs there.
    let comparable = name == "compareTo" && *owner == QName::parse(nvs_hir::interfaces::COMPARABLE);
    for (index, &passed) in parent.params.iter().enumerate() {
        let taken = own.params[index];
        let passed = if comparable {
            env.interner.class(qname.clone())
        } else {
            passed
        };
        let param = own.param_names.get(index).map_or("", String::as_str);
        if own.is_inout(index) != parent.is_inout(index) {
            return Some((
                name_at,
                format!("`${param}` differs from `{owner}::{name}` in `inout`"),
                false,
            ));
        }
        if own.is_covariant(index) {
            continue;
        }
        if !crate::expr::assign::field_fits(passed, taken, env.interner, env.graph, env.signatures)
        {
            let have = env.interner.describe(taken);
            let want = env.interner.describe(passed);
            let narrower = crate::expr::assign::field_fits(
                taken,
                passed,
                env.interner,
                env.graph,
                env.signatures,
            );
            return Some((
                name_at,
                format!("`${param}` is `{have}`, and a call to `{owner}::{name}` passes `{want}`"),
                narrower,
            ));
        }
    }
    None
}

/// `rule:classes/a-covariant-type-is-checked-where-a-value-enters`, at the
/// declaration: each parameter of `members`' methods is measured against the
/// topmost declaration of its method, the first ancestor that declares it.
///
/// A `covariant` parameter needs a method that replaces one (`E0882`), a
/// type `is` tests in constant time (`E0883`), a type strictly narrower than
/// the topmost one's that keeps its `tainted` and `secret` (`E0882` where it
/// is not narrower, `E0404` where it is unrelated or drops a qualifier), and
/// it must accept the default the topmost declaration gives it (`E0884`),
/// since a call through that declaration fills it. A `covariant` type with no
/// value in common with a nearer declaration's narrowed type is `E0404` too
/// ([`reject_covariant_unrelated_to_nearest`]). A parameter without the
/// keyword that is narrower than the topmost declaration's is `E0404`, also
/// when it only repeats its parent's narrowed type. A narrower type is tested
/// on every call, so each level writes the keyword where the test runs.
///
/// A constructor's parameter is not an override, and a promoted one declares
/// a property: [`crate::signatures`] reports one that is not promoted.
pub(crate) fn check_covariant_params(members: &[ClassMember], qname: &QName, env: &mut Env<'_>) {
    let ancestors: Vec<QName> = env.graph.get(qname).map_or_else(Vec::new, |links| {
        links
            .extends
            .iter()
            .chain(links.implements.iter())
            .cloned()
            .collect()
    });
    let own_sigs = env.signatures.get(qname).map(|sig| sig.methods.clone());
    for member in members {
        let ClassMemberKind::Method(m) = &member.kind else {
            continue;
        };
        let name = span_text(env.src, m.name).to_owned();
        if name == "constructor" {
            continue;
        }
        let Some(own) = own_sigs.as_ref().and_then(|sigs| sigs.get(&name)).cloned() else {
            continue;
        };
        let mut nearest = Vec::new();
        let mut roots = Vec::new();
        let mut seen = FxHashSet::default();
        for ancestor in &ancestors {
            let Some((owner, parent)) = resolve_method(ancestor, &name, env.signatures, env.graph)
            else {
                continue;
            };
            if parent.interface_private || owner == *qname {
                continue;
            }
            topmost_declarations(&owner, &parent, &name, env, &mut seen, &mut roots);
            let parent = bind_interface_args(qname, &owner, &parent, env);
            nearest.push((owner, parent));
        }
        if roots.is_empty() {
            for p in m.params.iter().filter(|p| p.covariant) {
                env.diags.report(
                    Diagnostic::error(
                        code::E_COVARIANT_MISPLACED,
                        format!(
                            "`covariant` has no effect here, because `{qname}::{name}` \
                             replaces no method"
                        ),
                    )
                    .with_primary(p.span, "`covariant` is written here")
                    .with_help(
                        "`covariant` is for a parameter of a method that replaces a method of a \
                         parent class or an interface. Remove it",
                    ),
                );
            }
            continue;
        }
        let roots: Vec<(QName, crate::signatures::MethodSig)> = roots
            .into_iter()
            .map(|(owner, sig)| {
                let sig = bind_interface_args(qname, &owner, &sig, env);
                (owner, sig)
            })
            .collect();
        'params: for (index, p) in m.params.iter().enumerate() {
            let Some(&taken) = own.params.get(index) else {
                continue;
            };
            let at = p.ty.as_ref().map_or(p.span, |ty| ty.span);
            if p.covariant && !testable_in_constant_time(taken, env) {
                let have = env.interner.describe(taken);
                env.diags.report(
                    Diagnostic::error(
                        code::E_COVARIANT_TYPE_NOT_TESTABLE,
                        format!("a `covariant` parameter cannot have the type `{have}`"),
                    )
                    .with_primary(at, "each call would test every element of this value")
                    .with_help(
                        "use the parent's type for this parameter, and test the value in the \
                         method body",
                    ),
                );
                continue;
            }
            for (root, sig) in &roots {
                if sig.params.len() != own.params.len() {
                    continue;
                }
                let top = if name == "compareTo"
                    && *root == QName::parse(nvs_hir::interfaces::COMPARABLE)
                {
                    env.interner.class(qname.clone())
                } else {
                    sig.params[index]
                };
                let reported = if p.covariant {
                    covariant_param_problem(qname, root, &name, p, taken, top, sig, index, env)
                } else {
                    let nearest_fits = nearest.iter().all(|(_, parent)| {
                        parent
                            .params
                            .get(index)
                            .is_none_or(|&passed| fits(passed, taken, env))
                    });
                    if nearest_fits && !fits(top, taken, env) {
                        let have = env.interner.describe(taken);
                        let want = env.interner.describe(top);
                        let param = strip_sigil(span_text(env.src, p.name)).to_owned();
                        let narrower = fits(taken, top, env);
                        report_incompatible_override(
                            qname,
                            root,
                            &name,
                            at,
                            format!(
                                "`${param}` is `{have}`, and a call to `{root}::{name}` passes `{want}`"
                            ),
                            narrower,
                            env,
                        );
                        true
                    } else {
                        false
                    }
                };
                if reported {
                    continue 'params;
                }
            }
            if p.covariant {
                reject_covariant_unrelated_to_nearest(qname, &name, p, index, taken, &nearest, env);
            }
        }
    }
}

/// Reports `E0404` when the `covariant` parameter `p`, of type `taken`, has no
/// value in common with the type a nearest declaration in `nearest` gives
/// it. Measured against the topmost declaration alone, a grandchild's
/// `covariant Cat` under its parent's `covariant Dog` would compile, and every
/// call through the parent's type would throw.
fn reject_covariant_unrelated_to_nearest(
    qname: &QName,
    name: &str,
    p: &nvs_syntax::ast::Param,
    index: usize,
    taken: TypeId,
    nearest: &[(QName, crate::signatures::MethodSig)],
    env: &mut Env<'_>,
) {
    let found = nearest.iter().find_map(|(owner, sig)| {
        let &passed = sig.params.get(index)?;
        crate::expr::operators::types_are_disjoint(taken, passed, env).then_some((owner, passed))
    });
    let Some((owner, passed)) = found else {
        return;
    };
    let at = p.ty.as_ref().map_or(p.span, |ty| ty.span);
    let param = strip_sigil(span_text(env.src, p.name)).to_owned();
    let have = env.interner.describe(taken);
    let want = env.interner.describe(passed);
    let message = format!(
        "`${param}` is `{have}`, and `{owner}::{name}` takes `{want}`. No value has both types"
    );
    report_incompatible_override(qname, owner, name, at, message, false, env);
}

/// Reports what is wrong with the `covariant` parameter `p` of `qname`'s
/// method `name`, of type `taken`, measured against `top`, the type `root`'s
/// declaration `sig` gives it. Whether it reported anything.
#[expect(
    clippy::too_many_arguments,
    reason = "each argument is one fact about the parameter the diagnostics name"
)]
fn covariant_param_problem(
    qname: &QName,
    root: &QName,
    name: &str,
    p: &nvs_syntax::ast::Param,
    taken: TypeId,
    top: TypeId,
    sig: &crate::signatures::MethodSig,
    index: usize,
    env: &mut Env<'_>,
) -> bool {
    let at = p.ty.as_ref().map_or(p.span, |ty| ty.span);
    let param = strip_sigil(span_text(env.src, p.name)).to_owned();
    let have = env.interner.describe(taken);
    let want = env.interner.describe(top);
    if fits(top, taken, env) {
        env.diags.report(
            Diagnostic::error(
                code::E_COVARIANT_MISPLACED,
                format!(
                    "`covariant` has no effect here, because `${param}` is not narrower than in \
                     `{root}::{name}`"
                ),
            )
            .with_primary(at, format!("`{have}` accepts every `{want}`"))
            .with_help("remove `covariant`. The parameter already takes every value a call passes"),
        );
        return true;
    }
    let drops_tainted = crate::expr::quals::carries_tainted(top, env.interner)
        && !crate::expr::quals::carries_tainted(taken, env.interner);
    let drops_secret = crate::expr::quals::carries_secret(top, env.interner)
        && !crate::expr::quals::carries_secret(taken, env.interner);
    if !fits(taken, top, env) || drops_tainted || drops_secret {
        let message = if drops_tainted || drops_secret {
            let qualifier = if drops_tainted { "tainted" } else { "secret" };
            format!(
                "`${param}` is `{have}`, and `{root}::{name}` takes `{want}`. `covariant` keeps \
                 the `{qualifier}` qualifier"
            )
        } else {
            format!(
                "`${param}` is `{have}`, which is not narrower than `{want}` in `{root}::{name}`"
            )
        };
        report_incompatible_override(qname, root, name, at, message, false, env);
        return true;
    }
    let Some(Some(default)) = sig.defaults.get(index) else {
        return false;
    };
    if default_passes(default, top, taken, env) {
        return false;
    }
    env.diags.report(
        Diagnostic::error(
            code::E_COVARIANT_DEFAULT_REJECTED,
            format!("`${param}` does not accept the default value that `{root}::{name}` gives it"),
        )
        .with_primary(at, format!("`{have}` does not accept the default"))
        .with_help(format!(
            "a call through `{root}` that leaves out `${param}` passes this default. Use a type \
             that accepts it, or change the default in `{root}`"
        )),
    );
    true
}

/// Whether the default `value` of a parameter declared `top` passes the
/// narrower `taken`. A case of an enum keeps only its backing value here, so
/// it passes when `taken` takes every enum `top` names.
fn default_passes(
    value: &crate::defaults::ConstArg,
    top: TypeId,
    taken: TypeId,
    env: &mut Env<'_>,
) -> bool {
    use crate::defaults::ConstArg;
    let ty = match value {
        ConstArg::Null => env.interner.null(),
        ConstArg::Bool(true) => env.interner.true_ty(),
        ConstArg::Bool(false) => env.interner.false_ty(),
        ConstArg::Int(n) => env.interner.single_value_int(*n),
        ConstArg::Uint(_) => env.interner.uint(),
        ConstArg::Float(_) => env.interner.float(),
        ConstArg::Decimal { .. } => env.interner.decimal(),
        ConstArg::Str(text) => env.interner.single_value_string(text.clone()),
        ConstArg::Bytes(_) => env.interner.bytes(),
        ConstArg::EnumInt(_) | ConstArg::EnumUint(_) => {
            let enums: Vec<TypeId> = env
                .interner
                .union_members(top)
                .into_iter()
                .filter(|&member| matches!(env.interner.get(member), Ty::Enum(..)))
                .collect();
            return enums.into_iter().all(|member| fits(member, taken, env));
        }
        _ => return true,
    };
    fits(ty, taken, env)
}

/// Whether a value of type `from` is a `to` as it is, which is
/// [`crate::expr::assign::field_fits`].
fn fits(from: TypeId, to: TypeId, env: &mut Env<'_>) -> bool {
    crate::expr::assign::field_fits(from, to, env.interner, env.graph, env.signatures)
}

/// Whether `is` tests a value against `ty` in constant time: a class, an
/// interface, an enum, a scalar, `null`, or a union or an intersection of
/// those. A generic class, an array, a shape and a callable are not, since
/// their test reads the value's contents.
fn testable_in_constant_time(ty: TypeId, env: &Env<'_>) -> bool {
    match env.interner.get(ty) {
        Ty::Class(_, args) => args.is_empty(),
        Ty::Union(members) | Ty::Intersection(members) => members
            .iter()
            .all(|&member| testable_in_constant_time(member, env)),
        Ty::Null
        | Ty::Bool
        | Ty::True
        | Ty::False
        | Ty::Int
        | Ty::Uint
        | Ty::Float
        | Ty::Decimal
        | Ty::String
        | Ty::Bytes
        | Ty::TaintedString
        | Ty::TaintedBytes
        | Ty::SecretString
        | Ty::SecretBytes
        | Ty::SecretTaintedString
        | Ty::SecretTaintedBytes
        | Ty::SingleValueString(_)
        | Ty::SingleValueInt(_)
        | Ty::EnumCase(..)
        | Ty::Enum(..)
        | Ty::Object
        | Ty::Mixed
        | Ty::Error => true,
        _ => false,
    }
}

/// Pushes onto `out` each topmost declaration of the method `name`, which
/// `owner` declares as `sig`: `owner` itself when no ancestor of it declares
/// `name`, and otherwise the topmost declarations of each ancestor's.
/// `Comparable::compareTo` is a root only for a class that names `Comparable`
/// itself, because a subclass's `compareTo` is measured against the class
/// that wrote the first one.
fn topmost_declarations(
    owner: &QName,
    sig: &crate::signatures::MethodSig,
    name: &str,
    env: &Env<'_>,
    seen: &mut FxHashSet<QName>,
    out: &mut Vec<(QName, crate::signatures::MethodSig)>,
) {
    let comparable = QName::parse(nvs_hir::interfaces::COMPARABLE);
    let ancestors: Vec<QName> = env.graph.get(owner).map_or_else(Vec::new, |links| {
        links
            .extends
            .iter()
            .chain(links.implements.iter())
            .cloned()
            .collect()
    });
    let mut found = false;
    for ancestor in &ancestors {
        let Some((next, parent)) = resolve_method(ancestor, name, env.signatures, env.graph) else {
            continue;
        };
        if parent.interface_private || next == *owner {
            continue;
        }
        if name == "compareTo" && next == comparable {
            continue;
        }
        found = true;
        topmost_declarations(&next, &parent, name, env, seen, out);
    }
    if !found && seen.insert(owner.clone()) {
        out.push((owner.clone(), sig.clone()));
    }
}

/// `sig` with the type variables of a generic `owner` bound to the arguments
/// `qname`'s own `implements` clauses give it. A variable no clause binds
/// becomes `mixed` ([`crate::generics::substitute`]).
fn bind_interface_args(
    qname: &QName,
    owner: &QName,
    sig: &crate::signatures::MethodSig,
    env: &mut Env<'_>,
) -> crate::signatures::MethodSig {
    if !sig.is_generic(env.interner) {
        return sig.clone();
    }
    let params = nvs_hir::interfaces::type_params(owner.short_name())
        .or_else(|| nvs_stdlib::registry::class_type_params(&owner.to_string()))
        .unwrap_or(&[]);
    let args = crate::signatures::resolve_interface_args(qname, owner, env.signatures, env.graph)
        .unwrap_or_default();
    let bindings: crate::generics::Bindings = params
        .iter()
        .map(|name| (*name).to_owned())
        .zip(args)
        .collect();
    sig.clone().substituted(&bindings, env.interner)
}

fn visibility_word(v: Visibility) -> &'static str {
    match v {
        Visibility::Public => "public",
        Visibility::Protected => "protected",
        Visibility::Private => "private",
    }
}

/// Reports one `E0786` per method of `decl` written without a body, when
/// `decl` itself is not `abstract`.
///
/// The other half of the rule `expr::calls`' `reject_abstract_instantiation`
/// enforces at `new`, and here rather than there because this is where the
/// declaration is: a class that leaves a member open must say so, and a
/// reader fixing this one is editing the class body rather than a call site.
///
/// **The question is the body and not the modifier.** `abstract` is what an
/// author writes, but what breaks is the missing body: `nvs-ir` skips a
/// bodiless method (`lower/mod.rs`), so codegen substitutes
/// `nvs_runtime::nvs_abstract_method` and the call is a run-time `FATAL`
/// naming no source at all. Both spellings therefore report, and only the
/// wording differs.
///
/// An interface never reaches this — its bodiless members are the contract
/// [`check_class_conformance`] holds implementors to — and neither does an
/// `abstract` class, where leaving a member to a subclass is what the
/// modifier means.
pub(crate) fn check_abstract_members(decl: &ClassDecl, qname: &QName, env: &mut Env<'_>) {
    reject_abstract_bodies(decl, qname, env);
    if decl.modifiers.contains(&Modifier::Abstract) {
        return;
    }
    for member in &decl.members {
        let ClassMemberKind::Method(m) = &member.kind else {
            continue;
        };
        if m.body.is_some() {
            continue;
        }
        let name = span_text(env.src, m.name).to_owned();
        let written_abstract = m.modifiers.contains(&Modifier::Abstract);
        let message = if written_abstract {
            format!("`{qname}::{name}` is `abstract`, but `{qname}` is not")
        } else {
            format!("`{qname}::{name}` has no body, but `{qname}` is not `abstract`")
        };
        env.diags.report(
            Diagnostic::error(code::E_ABSTRACT_METHOD_IN_CONCRETE_CLASS, message)
                .with_primary(m.name, "declared here, with nothing to call")
                .with_help(format!(
                    "give it a body, or declare `{qname}` itself `abstract` and instantiate a \
                     subclass that fills this member in"
                )),
        );
    }
}

/// One `E0106` per method of `decl` written `abstract` with a body.
///
/// `abstract` says a subclass writes the body. One written here would make
/// the method an ordinary one that no subclass has to write, and a call would
/// run it, so the modifier would say nothing.
fn reject_abstract_bodies(decl: &ClassDecl, qname: &QName, env: &mut Env<'_>) {
    for member in &decl.members {
        let ClassMemberKind::Method(m) = &member.kind else {
            continue;
        };
        if m.body.is_none() || !m.modifiers.contains(&Modifier::Abstract) {
            continue;
        }
        let name = span_text(env.src, m.name);
        env.diags.report(
            Diagnostic::error(
                code::E_BAD_MODIFIER,
                format!("`{qname}::{name}` is `abstract`, so it cannot have a body"),
            )
            .with_primary(m.name, "this method has a body")
            .with_help(
                "delete the body and end the declaration with `;`, so each subclass writes \
                 it, or delete `abstract` to keep this body",
            ),
        );
    }
}

/// Every `(declaring interface, method)` pair `qname` inherits without a
/// body, in a deterministic order: each ancestor's own methods sorted by
/// name, ancestors in `extends`-then-`implements` order.
///
/// Sorted rather than left in [`rustc_hash::FxHashMap`]'s bucket order for
/// the reason every other ordered walk in this crate is: a diagnostic's
/// position in the report must depend only on the source.
fn collect_obligations(
    qname: &QName,
    env: &Env<'_>,
    seen: &mut FxHashSet<QName>,
    out: &mut Vec<(QName, String)>,
) {
    let Some(links) = env.graph.get(qname) else {
        return;
    };
    let parents: Vec<QName> = links
        .extends
        .iter()
        .chain(links.implements.iter())
        .cloned()
        .collect();
    for parent in parents {
        if !seen.insert(parent.clone()) {
            continue;
        }
        own_obligations(&parent, env, out);
        collect_obligations(&parent, env, seen, out);
    }
}

/// Records one [`Delegation`] per member an `implements I by $field;` clause
/// makes the compiler synthesize a forward for — `rule:classes/delegation-by-property`.
///
/// The obligation set is [`collect_obligations`]', asked of the *interface*
/// rather than of the class: every bodiless, non-`interface_private` member it
/// and its own ancestors declare. A member the class already answers with a
/// body is skipped, which is the rule's "a class may still write the member by
/// hand, which is an ordinary override" — and the same subtraction covers an
/// inherited body and an `rule:classes/interface-default-methods` interface
/// default, both of which `resolve_method` finds and neither of which a
/// forward should displace.
///
/// These member shapes get no forward, and each is a hole rather than a rule:
/// a `static` member has no receiver to forward through (`rule:statements/static-is-a-member-modifier` gives class
/// storage none), and a variadic or `inout` parameter list is packed and
/// written back at the *call site*, so passing it straight on would pack it
/// twice. Each is refused where the clause is written (`E0721`) rather than
/// silently left to reach `nvs_runtime::nvs_abstract_method`, and each counts
/// as *covered* below so that [`check_class_conformance`]'s per-member walk
/// does not then report the same member a second time under `E0449`: one
/// member, one diagnostic, and the one that says what the author can do about
/// it.
///
/// Returns the set of member names the delegation discharges, or [`None`]
/// where the clauses could not be lined up with the resolved graph at all —
/// which the caller reads as "check nothing", the program having a
/// name-resolution error in it already.
fn resolve_delegations(
    decl: &ClassDecl,
    qname: &QName,
    env: &mut Env<'_>,
) -> Option<FxHashSet<String>> {
    let mut covered = FxHashSet::default();
    let links = env.graph.get(qname)?;
    // The graph drops an `implements` entry that resolved to nothing, so the
    // two lists only line up on a program with no name-resolution error in it.
    // One that has one is already reported and will never be lowered, so there
    // is nothing to resolve for it — and guessing which clause is which would
    // synthesize a forward against the wrong interface.
    if links.implements.len() != decl.implements.len() {
        return None;
    }
    let interfaces: Vec<QName> = links.implements.clone();
    let label = qname.to_string();
    for (clause, interface) in decl.implements.iter().zip(interfaces) {
        let Some(field_span) = clause.by_field else {
            continue;
        };
        let field = strip_sigil(span_text(env.src, field_span)).to_owned();
        let (line, _) = env.src.line_col(field_span.start);
        let mut seen = FxHashSet::default();
        seen.insert(qname.clone());
        let mut owed: Vec<(QName, String)> = Vec::new();
        own_obligations(&interface, env, &mut owed);
        seen.insert(interface.clone());
        collect_obligations(&interface, env, &mut seen, &mut owed);
        // The delegation rule's field requirement, asked once per clause rather
        // than once per member: a field that cannot answer the interface at all
        // cannot answer any of it, so every member it owes is marked covered and
        // the author reads one diagnostic about the field instead of one per
        // member.
        if !check_delegate_field(qname, &field, field_span, &interface, env) {
            covered.extend(owed.into_iter().map(|(_, method)| method));
            continue;
        }
        for (_, method) in owed {
            if resolve_method(qname, &method, env.signatures, env.graph)
                .is_some_and(|(_, sig)| sig.has_body)
            {
                continue;
            }
            let Some((_, sig)) = resolve_method(&interface, &method, env.signatures, env.graph)
            else {
                continue;
            };
            let unforwardable = if sig.is_static {
                Some("a `static` member has no receiver to read the delegate off")
            } else if sig.variadic {
                Some(
                    "a variadic parameter list is packed at the call site, so a forward would pack it twice",
                )
            } else if sig.inout.iter().any(|inout| *inout) {
                Some(
                    "an `inout` parameter is written back at the call site, so a forward would stage it twice",
                )
            } else {
                None
            };
            if let Some(why) = unforwardable {
                env.diags.report(
                    Diagnostic::error(
                        code::E_DELEGATE_MEMBER_NOT_FORWARDABLE,
                        format!(
                            "`{interface}::{method}` cannot be forwarded to `${field}`, so \
                             `{qname}` has to declare it"
                        ),
                    )
                    .with_primary(field_span, why.to_owned())
                    .with_help(format!(
                        "write `{method}` in `{qname}` yourself. A method the class declares \
                         replaces the forwarded one"
                    )),
                );
                covered.insert(method);
                continue;
            }
            covered.insert(method.clone());
            env.exprs.record_delegation(Delegation {
                class: label.clone(),
                method: method.clone(),
                field: field.clone(),
                params: sig.params.clone(),
                return_ty: sig.return_ty,
                frame: format!("{label}::{method}() at {}:{}", env.src.name(), line + 1),
                span: field_span,
            });
        }
    }
    Some(covered)
}

/// `rule:classes/delegation-by-property` bullet 1: whether `field` can carry `interface`'s members at
/// all — a declared property of `qname` (its own or an inherited one) whose
/// type is a **non-nullable** class or interface that satisfies `interface`.
/// Reports one `E0720` and answers `false` where it cannot.
///
/// Each half is a failure the synthesized forward has no answer for.
/// `nvs_ir::lower::call::delegation_forward` reads the slot, retains it and
/// dispatches late on whatever class the value's own header names: a name
/// that is no property has no slot to read, a `null` in the slot has no class
/// to dispatch on, and a type that does not reach `interface` names no member
/// for the call to land on. The judgement is
/// [`nvs_hir::implements_interface`]'s, which is reflexive — a field declared
/// *as* the interface satisfies it in zero steps, and that is the most direct
/// way to write a delegate field.
fn check_delegate_field(
    qname: &QName,
    field: &str,
    field_span: nvs_diagnostics::Span,
    interface: &QName,
    env: &mut Env<'_>,
) -> bool {
    let Some(ty) = resolve_property(qname, field, env.signatures, env.graph) else {
        env.diags.report(
            Diagnostic::error(
                code::E_DELEGATE_TYPE_MISMATCH,
                format!("`{qname}` delegates `{interface}` to `${field}`, which is not a property"),
            )
            .with_primary(
                field_span,
                "no property of this name is declared".to_owned(),
            )
            .with_help(format!(
                "declare a property `${field}` of type `{interface}`"
            )),
        );
        return false;
    };
    // A field type that did not resolve has reported already. Answering `true`
    // also keeps the delegated members from being owed a second time
    // (`rule:types/one-mistake-reports-once`).
    if env.interner.is_error(ty) {
        return true;
    }
    let described = env.interner.describe(ty);
    let nullable = env.interner.is_nullable(ty);
    let target = match env.interner.get(ty) {
        Ty::Class(name, _) => Some(name.clone()),
        _ => None,
    };
    if !nullable && target.is_some_and(|name| implements_interface(&name, interface, env.graph)) {
        return true;
    }
    let why = if nullable {
        "a delegate is dispatched on at every forwarded call, so it may not be nullable"
    } else {
        "the delegate's type does not satisfy this interface"
    };
    env.diags.report(
        Diagnostic::error(
            code::E_DELEGATE_TYPE_MISMATCH,
            format!("`${field}` is `{described}`, which cannot satisfy `{interface}`"),
        )
        .with_primary(field_span, why.to_owned())
        .with_help(format!(
            "declare `${field}` as `{interface}`, or as a class that implements it. The type \
             cannot be nullable"
        )),
    );
    false
}

/// One declaration's own bodiless, non-`interface_private` members, appended
/// to `out` in the sorted order [`collect_obligations`] reads them in — the
/// half of that walk that looks at the named declaration itself rather than at
/// its ancestors.
fn own_obligations(qname: &QName, env: &Env<'_>, out: &mut Vec<(QName, String)>) {
    let Some(sig) = env.signatures.get(qname) else {
        return;
    };
    let mut names: Vec<&String> = sig
        .methods
        .iter()
        .filter(|(_, m)| !m.has_body && !m.interface_private)
        .map(|(name, _)| name)
        .collect();
    names.sort();
    out.extend(names.into_iter().map(|n| (qname.clone(), n.clone())));
}

/// Reports `E0827` where what a double names is not an interface, one `E0825`
/// per method that interface requires and the shape of answers leaves out, one
/// `E0826` per field that shape names which the interface does not declare,
/// and one `E0828` per field that names a method it cannot stand in for —
/// `rule:testing/doubles`, checked at the call rather than at a declaration
/// because a double declares nothing.
///
/// The obligation set is [`check_class_conformance`]'s, asked of the interface
/// instead of of a class: what a double owes is exactly what an implementor
/// would have had to write, so an `rule:classes/interface-default-methods`
/// default is owed by neither — the interface answers that one itself — and a
/// `private` interface method is part of no contract, so naming one is the
/// *second* refusal rather than an accepted answer.
///
/// A `partial` owes nothing of the first half: the real implementation behind
/// it answers every method its shape leaves out, which is the whole of what
/// makes it a partial. It is held to the second half unchanged, since a field
/// naming no method of the interface overrides nothing whichever member built
/// it.
///
/// **What is compared is the argument's own inferred type**, which
/// [`crate::expr::literals::check_anon_object`] interned from the fields as
/// written — the declared parameter is `object`
/// (`rule:types/object-top`), because the constraint this position really
/// carries is the interface the *call site* wrote and no registry row can name
/// that. An argument that is not a shape at all therefore names no method,
/// which for a `double` is every method missing at once: there is no answer it
/// could default to.
pub(crate) fn check_double_answers(
    owner: &QName,
    method: &str,
    written: &[TypeId],
    arg_types: &[TypeId],
    slots: &[ArgSlot],
    call_span: Span,
    env: &mut Env<'_>,
) {
    // `Core\Test` is the only owner that stands in for a class rather than
    // building one ([`crate::derive::stands_in_for_a_class`]), and its two
    // members are the whole roster — every other `Core` call reaches this with
    // nothing to ask. The parameter index is the member's own: `$answers` is
    // `double`'s first and `partial`'s second, behind the `$real` it delegates
    // to.
    let answers_at = match (stands_in_for_a_class(&owner.to_string()), method) {
        (true, "double") => 0,
        (true, "partial") => 1,
        _ => return,
    };
    let Some(first) = written.first().copied() else {
        return;
    };
    // A type argument that names no declaration at all is
    // `E_TYPE_ARG_NOT_A_CLASS`, reported where the written class is read
    // (`crate::expr::args::written_class_of`), so this asks only the narrower
    // question that refusal leaves open: the name resolved, and to what kind.
    let Ty::Class(interface, _) = env.interner.get(first) else {
        return;
    };
    let interface = interface.clone();
    if env.symbols.get(&interface).map(|symbol| symbol.kind) != Some(SymbolKind::Interface) {
        env.diags.report(
            Diagnostic::error(
                code::E_DOUBLE_TYPE_ARG_NOT_AN_INTERFACE,
                format!(
                    "`{owner}::{method}` stands in for an interface, and `{interface}` is not one"
                ),
            )
            .with_primary(call_span, format!("`{interface}` written here"))
            .with_help(
                "a test double is an object whose fields are callables. It can replace the \
                 methods of an interface, and nothing else. A class also has properties and \
                 method bodies, and a double has neither",
            ),
        );
        return;
    }
    let named = answered_methods(answers_at, arg_types, slots, env);

    // Every field first, so a shape that both misnames one method and omits
    // another reads as the two mistakes it is, in the order `rule:testing/doubles`'s
    // own example prints them.
    let unknown: Vec<String> = named
        .iter()
        .filter(|(name, _)| {
            !resolve_method(&interface, name, env.signatures, env.graph)
                .is_some_and(|(_, sig)| !sig.interface_private)
        })
        .map(|(name, _)| name.clone())
        .collect();
    for name in unknown {
        env.diags.report(
            Diagnostic::error(
                code::E_DOUBLE_METHOD_UNKNOWN,
                format!("`{interface}` declares no method `{name}`"),
            )
            .with_primary(call_span, format!("this stands in for `{interface}`"))
            .with_help(format!(
                "remove the `{name}:` field, or declare `{name}` in `{interface}`"
            )),
        );
    }
    check_answer_signatures(&interface, &named, call_span, env);
    if method == "partial" {
        return;
    }

    let mut seen = FxHashSet::default();
    seen.insert(interface.clone());
    let mut owed: Vec<(QName, String)> = Vec::new();
    own_obligations(&interface, env, &mut owed);
    collect_obligations(&interface, env, &mut seen, &mut owed);
    // One diagnostic per *method*, not per declaration that names it: an
    // interface redeclaring a parent's member owes it once, and the author has
    // one field to write either way.
    let mut reported = FxHashSet::default();
    for (declaring, name) in owed {
        if named.iter().any(|(field, _)| *field == name) || !reported.insert(name.clone()) {
            continue;
        }
        env.diags.report(
            Diagnostic::error(
                code::E_DOUBLE_METHOD_MISSING,
                format!("`{declaring}::{name}` is not implemented by this double"),
            )
            .with_primary(call_span, format!("this stands in for `{interface}`"))
            .with_help(format!(
                "add a `{name}:` field whose value is a callable for this method. Or use \
                 `Core\\Test::partial` with a real implementation"
            )),
        );
    }
}

/// One `E0828` per field of `named` that answers a method `interface` declares
/// with a value that cannot stand in for it — the third of
/// `rule:testing/doubles`' three questions, asked of both members, since a
/// `partial`'s override answers its method exactly as a `double`'s field does.
///
/// A field naming no method of the interface is not asked: `E0826` has already
/// named it, and there is no signature to compare it against.
///
/// # Why a code of its own rather than the argument's own mismatch
///
/// The position a bad answer is written in is `$answers`, and that parameter
/// is declared `object` (`rule:types/object-top`), so
/// [`crate::expr::assign::report_mismatch`] there would report that a shape is
/// not an object — which is false, and names neither the field nor the method
/// it was supposed to answer. The constraint is the interface the *call site*
/// wrote, and this walk is the only place holding both halves of it.
///
/// The relation is ordinary assignability over [`Ty::CallableSig`], so
/// `rule:types/callable-arity`'s prefix arity and
/// `rule:types/callable-variance`'s contravariant parameters and covariant
/// return are what a double is held to, with no comparison of its own: a
/// callable declaring fewer parameters than the method is accepted here for the
/// reason `nvs_runtime::callable` accepts it at the call.
fn check_answer_signatures(
    interface: &QName,
    named: &[(String, TypeId)],
    call_span: Span,
    env: &mut Env<'_>,
) {
    for (name, field) in named {
        let Some((declaring, sig)) = resolve_method(interface, name, env.signatures, env.graph)
        else {
            continue;
        };
        if sig.interface_private {
            continue;
        }
        // Bare `callable` carries no parameter list to compare
        // (`rule:types/callable-values`), and that is the accepted case
        // rather than a refusal: the field promises a callable and nothing more,
        // which is what `nvs_runtime::callable` checks a tag at a time.
        if matches!(env.interner.get(*field), Ty::Callable) {
            continue;
        }
        let expected = env.interner.callable_sig(sig.params.clone(), sig.return_ty);
        if is_assignable(*field, expected, env.interner, env.graph, env.signatures) {
            continue;
        }
        let found = env.interner.describe(*field);
        let wanted = env.interner.describe(expected);
        env.diags.report(
            Diagnostic::error(
                code::E_DOUBLE_METHOD_SIGNATURE,
                format!("`{name}:` answers `{declaring}::{name}` with `{found}`"),
            )
            .with_primary(call_span, format!("`{declaring}::{name}` is `{wanted}`"))
            .with_help(format!(
                "the field must be a callable that a caller of `{name}` can use. It may declare \
                 fewer parameters than `{name}`. Each parameter it declares must accept the \
                 method's argument, and its return type must fit the method's return type"
            )),
        );
    }
}

/// The method names one double's shape of answers implements paired with the
/// type of the field naming each, in the order the shape interned them — which
/// [`crate::ty::TypeInterner::shape`] sorts, so the refusals below it come out
/// in one order for one program.
///
/// The field's own type is carried rather than its name alone because
/// [`check_answer_signatures`] is the one reader that needs both: what a double
/// owes is a method's *signature*, and the shape is where the answer's is
/// written.
///
/// The argument is found through its [`ArgSlot`] rather than by position, so a
/// call writing `answers:` by name is read exactly as the positional spelling
/// is. An argument that filled no parameter, or one whose type is not a shape,
/// answers nothing — see [`check_double_answers`] for what that then means.
fn answered_methods(
    param: usize,
    arg_types: &[TypeId],
    slots: &[ArgSlot],
    env: &Env<'_>,
) -> Vec<(String, TypeId)> {
    let Some(index) = slots.iter().position(|slot| *slot == ArgSlot::Param(param)) else {
        return Vec::new();
    };
    let Some(ty) = arg_types.get(index) else {
        return Vec::new();
    };
    match env.interner.get(*ty) {
        Ty::Shape(fields) => fields
            .iter()
            .map(|field| (field.name.clone(), field.ty))
            .collect(),
        _ => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use nvs_diagnostics::{Diagnostics, SourceMap, code};
    use nvs_hir::resolve_file;
    use nvs_syntax::parse_file;

    use crate::check::check_program;
    use crate::expr_table::ExprTypeTable;
    use crate::ty::TypeInterner;

    fn check_src(src: &str) -> Diagnostics {
        let mut map = SourceMap::new();
        let file = map.add("t.nvs", src);
        let mut diags = Diagnostics::new();
        let stmts = parse_file(map.file(file), &mut diags);
        assert!(!diags.has_errors(), "fixture failed to parse: {diags:?}");
        let module = resolve_file(&stmts, map.file(file), &mut diags);
        assert!(!diags.has_errors(), "fixture failed to resolve: {diags:?}");
        let mut interner = TypeInterner::new();
        let mut exprs = ExprTypeTable::new();
        check_program(
            &[crate::ProgramFile {
                src: map.file(file),
                stmts: &stmts,
            }],
            &module,
            &mut interner,
            &mut exprs,
            &mut diags,
        );
        diags
    }

    fn missing(diags: &Diagnostics) -> bool {
        diags
            .iter()
            .any(|d| d.code == Some(code::E_INTERFACE_METHOD_MISSING))
    }

    /// The case this module exists for: `rule:iteration/two-interfaces`'s members are bodiless
    /// by design, so a cursor missing one dispatches to nothing.
    #[test]
    fn a_cursor_class_missing_advance_is_diagnosed() {
        let diags = check_src(
            "<?nvs\n\
             class Nums implements Iterator<int> {\n\
             \x20 function current(): int { return 1; }\n\
             }\n",
        );
        assert!(missing(&diags), "{diags:?}");
    }

    /// ...and one declaring both is clean.
    #[test]
    fn a_cursor_class_declaring_both_members_is_clean() {
        let diags = check_src(
            "<?nvs\n\
             class Nums implements Iterator<int> {\n\
             \x20 function advance(): bool { return false; }\n\
             \x20 function current(): int { return 1; }\n\
             }\n",
        );
        assert!(!diags.has_errors(), "{diags:?}");
    }

    /// A user-declared interface is checked the same way.
    #[test]
    fn a_user_interfaces_member_is_owed_too() {
        let diags = check_src(
            "<?nvs\n\
             interface Shape { function area(): int; }\n\
             class Square implements Shape {}\n",
        );
        assert!(missing(&diags), "{diags:?}");
    }

    /// `rule:classes/interface-default-methods`: a default body discharges the obligation.
    #[test]
    fn an_interface_default_body_discharges_the_obligation() {
        let diags = check_src(
            "<?nvs\n\
             interface Shape { public function area(): int { return 0; } }\n\
             class Square implements Shape {}\n",
        );
        assert!(!diags.has_errors(), "{diags:?}");
    }

    /// Leaving a member to a subclass is what `abstract` means — the
    /// subclass is what owes it.
    #[test]
    fn an_abstract_class_is_exempt_and_its_concrete_subclass_is_not() {
        let clean = check_src(
            "<?nvs\n\
             interface Shape { function area(): int; }\n\
             abstract class Partial implements Shape {}\n\
             class Done extends Partial { function area(): int { return 4; } }\n",
        );
        assert!(!clean.has_errors(), "{clean:?}");

        let owed = check_src(
            "<?nvs\n\
             interface Shape { function area(): int; }\n\
             abstract class Partial implements Shape {}\n\
             class Undone extends Partial {}\n",
        );
        assert!(missing(&owed), "{owed:?}");
    }

    /// A compiler-declared interface owes its members like any other: once
    /// [`crate::iter_lib`] seeds `compareTo`, a class claiming `Comparable`
    /// and declaring nothing is the same `E0449` as a source-declared
    /// interface left unanswered.
    #[test]
    fn a_reserved_interface_owes_its_seeded_members() {
        let owed = check_src("<?nvs\nclass Money implements Comparable {}\n");
        assert!(missing(&owed), "{owed:?}");

        let answered = check_src(
            "<?nvs\n\
             class Money implements Comparable {\n\
             public function compareTo(self $other): int { return 0; }\n\
             }\n",
        );
        assert!(!answered.has_errors(), "{answered:?}");
    }

    /// `Parses` is one required member, so a class claiming it and declaring
    /// nothing owes `parse` by exactly `Comparable`'s route above.
    #[test]
    fn a_class_implementing_parses_and_declaring_no_parse_names_the_member() {
        let owed = check_src("<?nvs\nclass Slug implements Parses {}\n");
        assert!(missing(&owed), "{owed:?}");
        assert!(
            owed.iter().any(|d| d.message.contains("`parse`")),
            "the diagnostic names the member it owes: {owed:?}"
        );
    }

    /// `rule:expressions/try-parse`: `tryParse` is a default body on the
    /// interface, not a second required member, so declaring `parse` alone is
    /// a complete implementation.
    #[test]
    fn a_class_implementing_parses_inherits_try_parse_without_declaring_it() {
        let diags = check_src(
            "<?nvs\n\
             class Slug implements Parses {\n\
             public static function parse(tainted string $s): static { return new static(); }\n\
             }\n",
        );
        assert!(!diags.has_errors(), "{diags:?}");
    }

    /// ...and writing the second implementation that rule warns about is
    /// latitude, not a diagnostic — the same latitude `Comparable::compareTo`
    /// has.
    #[test]
    fn an_implementor_may_override_try_parse() {
        let diags = check_src(
            "<?nvs\n\
             class Slug implements Parses {\n\
             public static function parse(tainted string $s): static { return new static(); }\n\
             public static function tryParse(tainted string $s): ?Slug { return null; }\n\
             }\n",
        );
        assert!(!diags.has_errors(), "{diags:?}");
    }

    /// `static` is what makes an implementor's `parse` answer its own class at
    /// the binding site — see [`super::reject_dropped_static_return`].
    #[test]
    fn a_parse_returning_something_other_than_static_is_refused() {
        for written in ["Slug", "self", "string"] {
            let refused = check_src(&format!(
                "<?nvs\n\
                 class Slug implements Parses {{\n\
                 public static function parse(tainted string $s): {written} \
                 {{ return new Slug(); }}\n\
                 }}\n",
            ));
            assert!(
                refused
                    .iter()
                    .any(|d| d.code == Some(code::E_INCOMPATIBLE_OVERRIDE)),
                "`{written}` is not `static`: {refused:?}"
            );
        }
    }

    /// The three members of `Core\Router\Gate`, as a program's own class
    /// writes them — `nvs_stdlib::registry::CoreContract`'s first entry.
    const GATE_MEMBERS: [&str; 3] = [
        "public function allows(Core\\Router\\Match $route): bool { return true; }\n",
        "public function refused(Core\\Router\\Refusal $reason, ?Core\\Router\\Match $route): \
         void {}\n",
        "public function failed(?Core\\Router\\Match $route, ?Throwable $error): void {}\n",
    ];

    /// A `Core` interface a program implements owes each of its members, and
    /// one implementing all three is clean — including where a parameter typed
    /// by the interface is handed the class.
    #[test]
    fn a_class_implementing_the_gate_owes_its_three_members() {
        let whole = check_src(&format!(
            "<?nvs\n\
             class Shop implements Core\\Router\\Gate {{\n{}{}{}}}\n\
             class Site {{\n\
             public static function serve(Core\\Router\\Gate $gate): void {{}}\n\
             }}\n\
             Site::serve(new Shop());\n",
            GATE_MEMBERS[0], GATE_MEMBERS[1], GATE_MEMBERS[2],
        ));
        assert!(!whole.has_errors(), "{whole:?}");
        for left_out in 0..GATE_MEMBERS.len() {
            let written: String = GATE_MEMBERS
                .iter()
                .enumerate()
                .filter(|(index, _)| *index != left_out)
                .map(|(_, member)| *member)
                .collect();
            let diags = check_src(&format!(
                "<?nvs\nclass Shop implements Core\\Router\\Gate {{\n{written}}}\n"
            ));
            assert!(missing(&diags), "member {left_out} left out: {diags:?}");
        }
    }
}
