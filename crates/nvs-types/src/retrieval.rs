//! `rule:attributes/structural-retrieval` and `rule:attributes/retrieval-folds-while-checking`: `Core\Attributes::get<T>` and `::all<T>`, answered here rather than
//! at run time.
//!
//! § 5 is what makes this a checker pass at all. A declaration's attached
//! attributes are fixed by its source, and [`crate::attributes`] has already
//! proved every payload value is a compile-time constant (§ 2), so the whole
//! question — *which attached literals structurally satisfy `T`* — has an
//! answer before the program starts. The call is therefore **replaced** with
//! that answer: a compiled-in `null` where nothing matches, the matched
//! literal itself where exactly one does, and a compile-time diagnostic
//! (`E0728`) where more than one does, naming `::all<T>` as the fix. There is
//! no runtime lookup, no reflection table in the compiled unit, and
//! `nvs_stdlib::attributes`' two symbols name a body that aborts if it is ever
//! reached.
//!
//! # Structural, not nominal
//!
//! § 4's rule is that retrieval matches on *shape*: an attached literal is an
//! answer to `get<T>` exactly when it satisfies `T` under
//! [`crate::expr::is_assignable`] — `rule:types/shape-type`'s width subtyping, the same
//! test a shape-typed binding goes through — regardless of whether it was
//! written bare or under a name, and regardless of what that name was. An
//! attribute's optional name exists to check the literal where it is
//! *written* ([`crate::attributes`]), and is never part of how a caller asks
//! for it. That is what keeps attributes from growing a second namespace of
//! kind names for unrelated frameworks to collide in.
//!
//! # Which declaration `$target` names
//!
//! § 4 fixes four spellings and this pass inspects them **syntactically**,
//! exactly as `rule:security/secret-sinks-refuse`'s sinks inspect a literal argument: the reference
//! is never evaluated, so `Foo::bar(...)` here is a written name rather than a
//! closure value. A method is its own first-class-callable reference; a class
//! is its `constructor`'s; a parameter is its method's reference plus
//! `$member`; a property is its class's `constructor` reference plus
//! `$member`.
//!
//! The last two overlap, and deliberately: `constructor` plus a `$member` is
//! both "the property named `$member`" and "the constructor parameter named
//! `$member`", which are the *same declaration* for an `rule:classes/delegation-by-field` promoted
//! parameter. So both rosters are consulted and their attributes joined, which
//! is the one arrangement that reads a promoted parameter's attribute once and
//! an unpromoted one's at all.
//!
//! A *written* `$member` is checked against the target's real declarations
//! ([`declares_member`]) and a name reaching neither roster is `E0798`. It has
//! to be: the answer a misspelling would otherwise fold to — `null`, or the
//! empty array — is the very answer a correct retrieval of an absent attribute
//! gives, so nothing downstream could ever tell the two apart. A `$member`
//! that is not a string literal has no name to check, and § 4's *Consequences*
//! already fixes that case as an empty result rather than a diagnostic: `get`
//! folds to `null` and `all` to the empty array.
//!
//! # What a matched payload has to be
//!
//! § 5 replaces the call with the payload, so every value in a *matched*
//! payload needs a constant form ([`ConstArg`]), and **every shape § 2 admits
//! has one** — a class constant and `Foo::class` through
//! [`crate::signatures::resolve_const`], which is the same entry a *read* of
//! the name inlines, and an enum case through [`crate::enums`].
//!
//! What that costs is a [`Scope`] per attach site, and it is not optional: a
//! payload's `Mode::Fast` is resolved through the namespace and the `use` table
//! of the file it was **written** in, which is the declaration's, while the
//! retrieval asking for it may sit in another file importing another `Mode`.
//! The same scope types the payload in [`matching`], so what a name meant when
//! it was checked and what it folds to here cannot come apart.
//!
//! `E0731` is what is left over: a class constant whose *own* declaration
//! folded to no value (`E0792`'s gap, one position along) has nothing to
//! compile in. It is reported at the retrieval and never where the attribute is
//! *attached*, because § 2 admits the spelling and an attribute nobody
//! retrieves costs nothing.

use nvs_diagnostics::{Diagnostic, SourceFile, code};
use nvs_hir::QName;
use nvs_syntax::ast::{
    Attribute, AttributeGroup, CallArgs, ClassMember, ClassMemberKind, Expr, ExprKind, MemberName,
    NamespaceDecl, Stmt, StmtKind,
};
use rustc_hash::{FxHashMap, FxHashSet};

use crate::defaults::ConstArg;
use crate::expr::{check_object_literal, is_assignable, resolve_class_expr};
use crate::expr_table::ExprInfo;
use crate::locals::LocalScope;
use crate::signatures::{resolve_method, resolve_property};
use crate::ty::{Ty, TypeId};
use crate::{Ctx, Env, span_text};

/// The class `rule:attributes/structural-retrieval`'s two members live on, spelled as
/// [`nvs_hir::QName`] renders it.
const OWNER: &str = "Core\\Attributes";

/// One attached attribute, with the file it was written in — a payload's own
/// field names and string literals are read out of *that* source rather than
/// out of whichever file the retrieval was written in.
#[derive(Clone, Copy)]
struct Site<'a> {
    src: &'a SourceFile,
    attr: &'a Attribute,
    /// Which [`Scope`] in [`AttributeTable::scopes`] this was written under.
    ///
    /// An index rather than a reference because the scope is *owned* by the
    /// table and the sites are copied out of it, and a whole scope per site
    /// would be one clone of the file's `use` table per attached attribute.
    scope: usize,
}

/// The namespace, `use` table and enclosing class one attach site was written
/// under — everything a class name inside its payload resolves through.
///
/// The retrieval's own scope cannot answer that: `#[{mode: Mode::Fast}]` is
/// written in the file that declares the class, and the `Core\Attributes` call
/// asking for it may be in another file, in another namespace, importing
/// another `Mode`. Kept per class declaration rather than per site, since every
/// attribute on a class and on its members shares one.
struct Scope {
    namespace: Vec<String>,
    imports: FxHashMap<String, QName>,
    class: QName,
}

/// The scope a site was written under, as the [`Ctx`] every resolver takes.
///
/// `current_class` is the declaration the attribute is attached to, so a
/// payload may write `self::MAX`; the three body-only fields are what a
/// payload has no notion of — an attribute is not inside a hook, a constructor
/// or a generator.
fn site_ctx(scope: &Scope) -> Ctx<'_> {
    Ctx {
        namespace: &scope.namespace,
        imports: &scope.imports,
        current_class: Some(&scope.class),
        current_hook: None,
        in_constructor: false,
        generator_elem: None,
    }
}

/// Every attach site in the program, indexed by the declaration `rule:attributes/structural-retrieval`'s
/// `$target` spellings name.
///
/// Built once, before any body is checked, for the reason
/// [`crate::consts::build_const_table`] is: a retrieval may be written above
/// the declaration it asks about, in the same file or in another one, so a
/// table filled as the walk descends would answer differently depending on
/// source order.
#[derive(Default)]
pub(crate) struct AttributeTable<'a> {
    classes: FxHashMap<QName, ClassAttrs<'a>>,
    /// Every class declaration's own [`Scope`], indexed by [`Site::scope`].
    scopes: Vec<Scope>,
}

#[derive(Default)]
struct ClassAttrs<'a> {
    /// The declaration's own `#[...]` groups.
    own: Vec<Site<'a>>,
    /// Each method's own groups, by method name.
    methods: FxHashMap<String, Vec<Site<'a>>>,
    /// Each declared property's groups, by name with no `$` sigil.
    properties: FxHashMap<String, Vec<Site<'a>>>,
    /// Each parameter's groups, keyed by its method's name and its own, with
    /// no `$` sigil.
    params: FxHashMap<(String, String), Vec<Site<'a>>>,
}

/// Collects every attach site in the program. See [`AttributeTable`].
pub(crate) fn build_attribute_table<'a>(files: &[crate::ProgramFile<'a>]) -> AttributeTable<'a> {
    let mut table = AttributeTable::default();
    for file in files {
        collect(file.stmts, file.src, &[], &FxHashMap::default(), &mut table);
    }
    table
}

/// The namespace and `use` walk `crate::check::check_stmts` makes, made a
/// second time and a whole pass earlier — a braced `namespace {}` opens a fresh
/// import set and an unbraced one clears it, exactly as it does there, because
/// what a name meant is a property of the text it was written in.
fn collect<'a>(
    stmts: &'a [Stmt],
    src: &'a SourceFile,
    namespace: &[String],
    imports: &FxHashMap<String, QName>,
    table: &mut AttributeTable<'a>,
) {
    let mut current_ns = namespace.to_vec();
    let mut current_imports = imports.clone();
    for stmt in stmts {
        let (name, attributes, members) = match &stmt.kind {
            StmtKind::NamespaceDecl(NamespaceDecl { name, body, .. }) => {
                let new_ns = name.as_ref().map_or_else(Vec::new, |n| {
                    QName::parse(span_text(src, n.span)).segments().to_vec()
                });
                match body {
                    Some(block) => {
                        collect(&block.stmts, src, &new_ns, &FxHashMap::default(), table);
                    }
                    None => {
                        current_ns = new_ns;
                        current_imports.clear();
                    }
                }
                continue;
            }
            StmtKind::UseDecl(use_decl) => {
                let target = QName::parse(span_text(src, use_decl.path.span));
                current_imports.insert(target.short_name().to_owned(), target);
                continue;
            }
            StmtKind::ClassDecl(decl) => (&decl.name, &decl.attributes, &decl.members),
            StmtKind::InterfaceDecl(decl) => (&decl.name, &decl.attributes, &decl.members),
            _ => continue,
        };
        let qname = QName::join(&current_ns, span_text(src, name.span));
        let scope = table.scopes.len();
        table.scopes.push(Scope {
            namespace: current_ns.clone(),
            imports: current_imports.clone(),
            class: qname.clone(),
        });
        let entry = table.classes.entry(qname).or_default();
        push(&mut entry.own, attributes, src, scope);
        collect_members(entry, members, src, scope);
    }
}

fn collect_members<'a>(
    entry: &mut ClassAttrs<'a>,
    members: &'a [ClassMember],
    src: &'a SourceFile,
    scope: usize,
) {
    for member in members {
        match &member.kind {
            ClassMemberKind::Property(p) => {
                let name = crate::strip_sigil(span_text(src, p.name)).to_owned();
                push(
                    entry.properties.entry(name).or_default(),
                    &p.attributes,
                    src,
                    scope,
                );
            }
            ClassMemberKind::Method(m) => {
                let method = span_text(src, m.name).to_owned();
                push(
                    entry.methods.entry(method.clone()).or_default(),
                    &m.attributes,
                    src,
                    scope,
                );
                for param in &m.params {
                    let name = crate::strip_sigil(span_text(src, param.name)).to_owned();
                    push(
                        entry.params.entry((method.clone(), name)).or_default(),
                        &param.attributes,
                        src,
                        scope,
                    );
                }
            }
            _ => {}
        }
    }
}

fn push<'a>(
    out: &mut Vec<Site<'a>>,
    groups: &'a [AttributeGroup],
    src: &'a SourceFile,
    scope: usize,
) {
    for group in groups {
        for attr in &group.attributes {
            out.push(Site { src, attr, scope });
        }
    }
}

/// Whether `owner::member` is one of `rule:attributes/structural-retrieval`'s two retrievals — the cheap
/// test [`crate::expr::calls`] makes before reaching for anything here.
pub(crate) fn is_retrieval(owner: &QName, member: &str) -> bool {
    owner.to_string() == OWNER && matches!(member, "get" | "all")
}

/// Resolves one retrieval and records its folded answer against the call's own
/// span, as the [`ExprInfo::CoreConst`] `nvs-ir` materializes any other
/// compile-time constant from.
///
/// Records nothing where the call is refused: a diagnostic has been reported,
/// and `nvs-ir` never reaches a unit that failed to check.
pub(crate) fn fold_retrieval(
    call: &Expr,
    member: &str,
    written: &[TypeId],
    args: &CallArgs,
    ctx: &Ctx<'_>,
    env: &mut Env<'_>,
) {
    let CallArgs::List(list) = args else {
        return;
    };
    let Some(want) = written.first().copied() else {
        // The type-argument count is `check_written_type_args`' refusal and
        // has already been made; a second one names the same mistake twice.
        return;
    };
    if !matches!(env.interner.get(want), Ty::Shape(_)) {
        let found = env.interner.describe(want);
        env.diags.report(
            Diagnostic::error(
                code::E_ATTRIBUTE_TYPE_ARG_NOT_A_SHAPE,
                format!("`Core\\Attributes::{member}` retrieves a shape, and `{found}` is not one"),
            )
            .with_primary(call.span, format!("`{found}` written here"))
            .with_help(
                "`rule:attributes/structural-retrieval`: retrieval is structural — an attached literal is an answer \
                 exactly when it satisfies `T` under `rule:types/shape-type`'s width subtyping, so `T` \
                 is an inline `{...}` or a `type` alias naming one",
            ),
        );
        return;
    }
    let Some(target) = list.first().map(|arg| &arg.value) else {
        // A missing argument is the arity check's refusal, already made.
        return;
    };
    let Some((class, method)) = target_declaration(target, ctx, env) else {
        env.diags.report(
            Diagnostic::error(
                code::E_ATTRIBUTE_TARGET_NOT_A_DECLARATION,
                "a `Core\\Attributes` retrieval names no declaration",
            )
            .with_primary(target.span, "this is not a declaration reference")
            .with_help(
                "`rule:attributes/structural-retrieval`: the target is written as a first-class-callable reference and \
                 inspected where it is written — `Foo::bar(...)` for a method, and \
                 `Foo::constructor(...)` for the class itself, plus a literal member name for \
                 one of its properties or parameters",
            ),
        );
        return;
    };
    let member_arg = list.get(1).map(|arg| &arg.value);
    let member_name = member_arg.and_then(|value| match &value.kind {
        ExprKind::Str(span) => Some((
            value.span,
            crate::string_lit::cook_string_literal(env.src, *span),
        )),
        _ => None,
    });
    // A `$member` written as anything but a string literal names no roster to
    // select, and the module doc above fixes that case as the empty result. It
    // is not the same as no second argument at all, which selects the target's
    // own roster, so the two are told apart here rather than by `sites_for`.
    let computed_member = member_arg.is_some() && member_name.is_none();
    if let Some((span, name)) = &member_name
        && !declares_member(&class, &method, name, env)
    {
        env.diags.report(
            Diagnostic::error(
                code::E_ATTRIBUTE_MEMBER_NOT_DECLARED,
                format!(
                    "a `Core\\Attributes` retrieval names `${name}`, which `{class}` does not \
                     declare"
                ),
            )
            .with_primary(*span, "no parameter or property of that name")
            .with_help(
                "`rule:attributes/structural-retrieval`: a *written* member name is checked against the target's real \
                 declarations here, because the answer a misspelling would fold to — `null`, or \
                 the empty array — is the same one a correct retrieval of an absent attribute \
                 gives, and nothing later can tell them apart; only a computed `$member` falls \
                 back to that empty result",
            ),
        );
        return;
    }
    let sites = if computed_member {
        Vec::new()
    } else {
        sites_for(
            env,
            &class,
            &method,
            member_name.as_ref().map(|(_, name)| name.as_str()),
        )
    };
    let matched = matching(&sites, want, env);
    let value = match (member, matched.len()) {
        ("get", 0) => ConstArg::Null,
        ("get", 1) => match fold_payload(matched[0], env) {
            Some(value) => value,
            None => return,
        },
        ("get", _) => {
            env.diags.report(
                Diagnostic::error(
                    code::E_ATTRIBUTE_RETRIEVAL_AMBIGUOUS,
                    format!(
                        "`{class}::{method}` carries {} attached literals satisfying this shape",
                        matched.len()
                    ),
                )
                .with_primary(call.span, "`get` answers at most one")
                .with_help(
                    "`rule:attributes/retrieval-folds-while-checking`: an attached-attribute list is static, so this is decided \
                     here rather than by a test run — write `Core\\Attributes::all<T>(…)`, \
                     which answers every match",
                ),
            );
            return;
        }
        _ => {
            let mut entries = Vec::with_capacity(matched.len());
            for (index, site) in matched.iter().enumerate() {
                let Some(value) = fold_payload(*site, env) else {
                    return;
                };
                entries.push((index.to_string(), value));
            }
            ConstArg::Array(entries)
        }
    };
    env.exprs.record(call.span, ExprInfo::CoreConst { value });
}

/// `rule:attributes/structural-retrieval`'s `$target` spelling, read syntactically: the class it names
/// and the method whose reference was written.
fn target_declaration(target: &Expr, ctx: &Ctx<'_>, env: &Env<'_>) -> Option<(QName, String)> {
    let ExprKind::StaticCall {
        class,
        method: MemberName::Ident(name),
        args: CallArgs::FirstClassCallable,
        ..
    } = &target.kind
    else {
        return None;
    };
    let qname = resolve_class_expr(class, ctx, env)?;
    Some((qname, span_text(env.src, *name).to_owned()))
}

/// Whether `member` names a real declaration of the target — § 4's validation
/// of a *written* member name, and the whole of what `E0798` reports on.
///
/// Asked of [`crate::signatures`] rather than of [`AttributeTable`], which
/// carries the same two rosters: a declaration is real whether or not anything
/// is attached to it, and one inherited from an ancestor is as real as an own
/// one, neither of which the attach table can answer. Both rosters are
/// consulted for the same reason [`sites_for`] consults both — `constructor`
/// plus a name is both a property and a constructor parameter, and for an ADR
/// 0043 § 4 promoted parameter it is one declaration reached two ways.
///
/// [`MethodSig::param_names`] is read directly rather than through
/// [`MethodSig::param_index`]: a variadic tail is a parameter that can carry an
/// attribute even though no call may fill it *by name*, which is the one thing
/// that index excludes.
///
/// [`MethodSig::param_names`]: crate::signatures::MethodSig::param_names
/// [`MethodSig::param_index`]: crate::signatures::MethodSig::param_index
fn declares_member(class: &QName, method: &str, member: &str, env: &Env<'_>) -> bool {
    if let Some((_, sig)) = resolve_method(class, method, env.signatures, env.graph)
        && sig.param_names.iter().any(|name| name == member)
    {
        return true;
    }
    method == "constructor" && resolve_property(class, member, env.signatures, env.graph).is_some()
}

/// The attach sites `$target` plus `$member` names, joined where § 4's two
/// spellings overlap — see this module's own docs.
fn sites_for<'a>(
    env: &Env<'a>,
    class: &QName,
    method: &str,
    member: Option<&str>,
) -> Vec<Site<'a>> {
    let Some(entry) = env.attributes.classes.get(class) else {
        return Vec::new();
    };
    let Some(member) = member.filter(|name| !name.is_empty()) else {
        return if method == "constructor" {
            // A class with no written `constructor` still names itself this
            // way — `rule:classes/definite-property-initialization` synthesizes one, so § 4 uses the spelling for
            // every class rather than only for the ones that declare it, and
            // a class that *does* write one may carry attributes on both.
            let mut sites = entry.own.clone();
            if let Some(found) = entry.methods.get(method) {
                sites.extend(found.iter().copied());
            }
            sites
        } else {
            entry.methods.get(method).cloned().unwrap_or_default()
        };
    };
    let mut sites = Vec::new();
    if method == "constructor"
        && let Some(found) = entry.properties.get(member)
    {
        sites.extend(found.iter().copied());
    }
    if let Some(found) = entry.params.get(&(method.to_owned(), member.to_owned())) {
        sites.extend(found.iter().copied());
    }
    sites
}

/// The sites whose payload satisfies `want`, in attach order.
///
/// Each payload is inferred under **its own** file's source and its own
/// [`Scope`], because a field name, a string literal and a `Mode::Fast` are all
/// read out of the text they were written in — the retrieval's file and
/// imports have no bearing on what the payload says.
fn matching<'a>(sites: &[Site<'a>], want: TypeId, env: &mut Env<'a>) -> Vec<Site<'a>> {
    let outer = env.src;
    let table = env.attributes;
    let mut matched = Vec::new();
    for site in sites {
        env.src = site.src;
        let mut live = FxHashSet::default();
        let scope = LocalScope::new();
        let written = site_ctx(&table.scopes[site.scope]);
        let actual =
            check_object_literal(&site.attr.fields, None, &mut live, &scope, &written, env);
        if is_assignable(actual, want, env.interner, env.graph, env.signatures) {
            matched.push(*site);
        }
    }
    env.src = outer;
    matched
}

/// One matched payload as the constant § 5 replaces the call with, or `None`
/// after reporting `E0731` for a value with no constant form.
fn fold_payload<'a>(site: Site<'a>, env: &mut Env<'a>) -> Option<ConstArg> {
    let outer = env.src;
    env.src = site.src;
    let written = site_ctx(&env.attributes.scopes[site.scope]);
    let mut fields = Vec::with_capacity(site.attr.fields.len());
    let mut ok = true;
    for field in &site.attr.fields {
        let name = span_text(env.src, field.name).to_owned();
        match crate::defaults::fold_constant_value(&field.value, Some(&written), env) {
            Some(value) => fields.push((name, value)),
            None => {
                report_unfoldable(&field.value, env);
                ok = false;
            }
        }
    }
    env.src = outer;
    ok.then_some(ConstArg::Shape(fields))
}

/// The `E0731` half of [`fold_payload`], which owns why.
fn report_unfoldable(value: &Expr, env: &mut Env<'_>) {
    env.diags.report(
        Diagnostic::error(
            code::E_ATTRIBUTE_PAYLOAD_UNFOLDABLE,
            "this attribute satisfies the retrieved shape, but holds a value with no \
             compile-time form",
        )
        .with_primary(value.span, "no constant form")
        .with_help(
            "`rule:attributes/retrieval-folds-while-checking` replaces the retrieval with the payload itself, so every value in \
             it has to be materializable — a class constant, `Foo::class` and an enum case \
             all are, so what is left is a constant whose own declaration folds to nothing \
             (`E0792`): write the value out here, or give that constant a foldable one",
        ),
    );
}
