//! `rule:attributes/an-attribute-is-declared-with-its-sites`,
//! `rule:attributes/attach-sites-and-forms` and
//! `rule:attributes/payload-is-a-compile-time-constant`: every `#[Name(...)]`
//! names an `attribute` declaration, is used on a site that declaration lists,
//! and carries a payload of compile-time constants that fits its fields.
//!
//! # A use, checked in order
//!
//! [`lookup`] resolves `Name` to its declaration. A short name is looked up in
//! two scopes: the owner chain of the class the attribute is written in,
//! walked through [`nvs_hir::find_in_ancestors`], and the namespace and `use`
//! scope. A name both scopes give is `E1007`, never a silent choice. `Owner::Name`
//! is the owned declaration from anywhere. A class, an interface, an enum or a
//! `type` alias after `#[` is `E1005`. A name that resolves to nothing is the
//! ordinary undeclared-name error. A name on `Core`'s compiler roster resolves
//! to its row of [`crate::derive::ATTRIBUTE_DOCS`], the same declaration value
//! with its sites and `repeatable`.
//!
//! [`check_attribute`] then makes four checks, each its own diagnostic, and
//! stops at the first that fails, so one mistake is reported once: the site is
//! one the declaration lists (`E0874`), an owned declaration is used on its
//! owner's descendants (`E1006`), a second use on one declaration needs
//! `repeatable` (`E0763`), and the payload fits the declared fields (`E0401`).
//! A `Core` declaration makes the first and the third the same way. Its
//! payload is checked by the pass that owns the name, against that pass's
//! option roster, and so is what that pass asks beyond the site, such as
//! `#[Route]` needing `#[Access]`.
//! [`check_attribute_decl`] checks the declaration itself: a site no retrieval
//! reads is `E1004` there, so no use of it has to be refused one by one.
//!
//! A use costs one table lookup while compiling, at most one walk up its
//! class's parents for a short name, and one comparison per earlier attribute
//! on the same declaration. Nothing of it reaches run time.
//!
//! `rule:attributes/payload-is-a-compile-time-constant` is one sentence, and the
//! reason it is *this* sentence rather than a taste is in that rule: the whole
//! literal is resolved once, at compile time, into the compiled unit's constant
//! pool — the storage class an enum case's backing integer and a class constant
//! already use — so there is no moment at which a variable could be read, a call
//! made or a `new` constructed. PHP's attributes do have that moment (a real object is built
//! the first time `ReflectionAttribute::newInstance()` is called) and Novis's
//! deliberately do not, so the refusal here is what keeps the two
//! halves of that decision from disagreeing.
//!
//! The payload is checked against the declaration's fields by
//! `rule:types/shape-type`'s width subtyping, the rule an ordinary shape-typed
//! binding is checked by. A payload is folded into a new value wherever it is
//! read, so its field types are compared covariantly
//! ([`crate::expr::built_fits`]).
//!
//! One consequence of the constant payload is checked from here but owned
//! elsewhere: `rule:security/secret-sinks-refuse`'s sink for a `secret` class
//! constant reaching a payload is `crate::expr::quals`', where every other sink
//! already lives.
//! [`check_value`] calls it at each value it reaches, that walk being the one
//! place every payload value passes.

use nvs_diagnostics::{Diagnostic, Span, code};
use nvs_syntax::ast::{
    AnonObjectField, ArrayItem, Attribute, AttributeDecl, AttributeGroup, AttributeSite,
    AttributeSiteWord, ClassMember, ClassMemberKind, EnumCase, Expr, ExprKind, Name, Param, Type,
    TypeAtom, TypeKind, UnaryOp,
};

use crate::expr::{built_fits, check_anon_object, check_expr, report_mismatch};
use crate::locals::{Live, LocalScope};
use crate::ty::{Ty, TypeId};
use crate::{Ctx, Env, span_text};

/// Every attribute attached anywhere under one class/interface/enum body:
/// the declaration's own groups, each member's, each method parameter's and
/// each property hook's. One entry point per declaration kind would be a copy
/// of this walk each, so the caller hands over the lists it has, and says
/// which of the three declarations they came from.
pub(crate) fn check_declaration(
    decl: crate::deprecated::Decl,
    groups: &[AttributeGroup],
    members: &[ClassMember],
    cases: &[EnumCase],
    ctx: &Ctx<'_>,
    env: &mut Env<'_>,
) {
    let own_site = match decl {
        crate::deprecated::Decl::Class => AttributeSite::Class,
        crate::deprecated::Decl::Interface => AttributeSite::Interface,
        crate::deprecated::Decl::Enum => AttributeSite::Enum,
    };
    check_groups(groups, UseSite::One(own_site), ctx, env);
    // `#[Core\Path]`'s and `#[Core\Deprecated]`'s attach rules ask the same
    // question of every site this walk visits, so each walks the same lists
    // once, beside it.
    crate::paths::check_marker_sites(members, ctx, env);
    crate::route_params::check_marker_sites(members, ctx, env);
    crate::deprecated::check_sites(decl, groups, members, cases, ctx, env);
    for member in members {
        match &member.kind {
            ClassMemberKind::Property(p) => {
                check_groups(
                    &p.attributes,
                    UseSite::One(AttributeSite::Property),
                    ctx,
                    env,
                );
                for hook in p.hooks.iter().flatten() {
                    check_groups(
                        &hook.attributes,
                        UseSite::One(AttributeSite::Hook),
                        ctx,
                        env,
                    );
                    // No retrieval reads a hook's parameter, so only a
                    // declaration that lists `hook` fits there.
                    if let Some(param) = &hook.param {
                        check_groups(
                            &param.attributes,
                            UseSite::One(AttributeSite::Hook),
                            ctx,
                            env,
                        );
                    }
                }
            }
            ClassMemberKind::Const(c) => {
                check_groups(
                    &c.attributes,
                    UseSite::One(AttributeSite::Constant),
                    ctx,
                    env,
                );
            }
            ClassMemberKind::Attribute(declared) => {
                check_attribute_decl(declared, ctx, env);
            }
            ClassMemberKind::Method(m) => {
                check_groups(&m.attributes, UseSite::One(AttributeSite::Method), ctx, env);
                // The markers that mean nothing away from the attribute that
                // reads them — `rule:routing/a-query-parameter-is-declared-like-a-capture`'s `#[Query]`, `rule:attributes/api-adds-and-cannot-contradict`'s
                // `#[Api]`, `rule:attributes/access-is-a-required-sibling`'s `#[Access]` and `rule:tooling/commands-are-compiled`'s
                // `#[Option]`. Asked here because each owning pass walks only
                // the methods its own attribute selects, so a stray marker is
                // invisible to the pass that would refuse it, and this is the
                // walk that visits every method.
                crate::routes::check_stray_query(m, ctx, env);
                crate::routes::check_stray_api(m, ctx, env);
                crate::routes::check_stray_access(m, ctx, env);
                crate::commands::check_stray_options(m, ctx, env);
                check_params(&m.params, ctx, env);
            }
            _ => {}
        }
    }
    for case in cases {
        check_groups(
            &case.attributes,
            UseSite::One(AttributeSite::Case),
            ctx,
            env,
        );
    }
}

/// The sites a declaration in a program may list: the four a retrieval reads.
/// Only `Core`'s declarations list the other five.
const READ_SITES: [AttributeSite; 4] = [
    AttributeSite::Class,
    AttributeSite::Method,
    AttributeSite::Property,
    AttributeSite::Parameter,
];

/// One `attribute` declaration, at namespace level or in the body of
/// `ctx.current_class`: a site word written twice and a field declared twice
/// are each an error, a site no retrieval reads is `E1004`, and the body is
/// lowered as a shape type is, which reports what a shape's fields report.
pub(crate) fn check_attribute_decl(decl: &AttributeDecl, ctx: &Ctx<'_>, env: &mut Env<'_>) {
    for (index, word) in decl.sites.iter().enumerate() {
        if decl.sites[..index]
            .iter()
            .any(|seen| seen.site == word.site)
        {
            env.diags.report(
                Diagnostic::error(
                    code::E_DUPLICATE_DECLARATION,
                    format!("`{}` is listed twice after `on`", word.site.word()),
                )
                .with_primary(word.span, "listed again here")
                .with_help("Delete one of the two."),
            );
            continue;
        }
        if READ_SITES.contains(&word.site) {
            continue;
        }
        env.diags.report(
            Diagnostic::error(
                code::E_ATTRIBUTE_SITE_NOT_READ,
                format!(
                    "an attribute declared in a program may list only `class`, `method`, \
                     `property` and `parameter`, and `{}` is not read by any retrieval",
                    word.site.word()
                ),
            )
            .with_primary(word.span, "no retrieval reads an attribute here")
            .with_help(
                "`Core\\Attributes` reads an attribute on a class, a method, a property or a \
                 parameter. Delete this word from the `on` list.",
            ),
        );
    }
    for (index, field) in decl.fields.iter().enumerate() {
        let name = span_text(env.src, field.name);
        if decl.fields[..index]
            .iter()
            .any(|seen| span_text(env.src, seen.name) == name)
        {
            env.diags.report(
                Diagnostic::error(
                    code::E_DUPLICATE_DECLARATION,
                    format!("the field `{name}` is declared twice in this attribute"),
                )
                .with_primary(field.name, "declared again here")
                .with_help("Delete one of the two, or give the second field another name."),
            );
        }
    }
    let written = span_text(env.src, decl.name.span);
    let qname = match ctx.current_class {
        Some(owner) => nvs_hir::QName::member(owner, written),
        None => nvs_hir::QName::join(ctx.namespace, written),
    };
    if let Some(def) = env.aliases.attribute(&qname) {
        let shape = def.shape.clone();
        crate::lower::lower_type(&shape, ctx, env);
    }
}

/// Where one attribute is written. A promoted constructor parameter is a
/// property and a parameter, and a declaration that lists either fits there.
#[derive(Clone, Copy)]
enum UseSite {
    One(AttributeSite),
    Promoted,
}

impl UseSite {
    fn fits(self, sites: &[AttributeSite]) -> bool {
        sites.iter().any(|&listed| match self {
            Self::One(site) => listed == site,
            Self::Promoted => {
                matches!(listed, AttributeSite::Property | AttributeSite::Parameter)
            }
        })
    }

    /// The word an `on` list writes for this site.
    const fn word(self) -> &'static str {
        match self {
            Self::One(site) => site.word(),
            Self::Promoted => "property",
        }
    }

    /// The site as a message names it: "a class", "a parameter".
    const fn noun(self) -> &'static str {
        match self {
            Self::One(AttributeSite::Class) => "a class",
            Self::One(AttributeSite::Interface) => "an interface",
            Self::One(AttributeSite::Enum) => "an enum",
            Self::One(AttributeSite::Method) => "a method",
            Self::One(AttributeSite::Property) => "a property",
            Self::One(AttributeSite::Hook) => "a property hook",
            Self::One(AttributeSite::Constant) => "a class constant",
            Self::One(AttributeSite::Case) => "an enum case",
            Self::One(AttributeSite::Parameter) => "a parameter",
            Self::Promoted => "a promoted constructor parameter",
        }
    }
}

/// Site words in backticks, joined for a message: "`class`, `method` and
/// `property`".
fn site_list(sites: &[AttributeSite]) -> String {
    let words: Vec<String> = sites
        .iter()
        .map(|site| format!("`{}`", site.word()))
        .collect();
    match words.split_last() {
        None => String::new(),
        Some((last, [])) => last.clone(),
        Some((last, rest)) => format!("{} and {last}", rest.join(", ")),
    }
}

/// The sites a program's declaration lists, without their spans.
fn listed(words: &[AttributeSiteWord]) -> Vec<AttributeSite> {
    words.iter().map(|word| word.site).collect()
}

/// Check 1, the same for a program's declaration and a `Core` one: `E0874`
/// when `site` is not one of `sites`. `shown` is the name as the message
/// writes it. Whether the use may go on.
fn site_fits(
    attr: &Attribute,
    site: UseSite,
    shown: &str,
    sites: &[AttributeSite],
    env: &mut Env<'_>,
) -> bool {
    if site.fits(sites) {
        return true;
    }
    env.diags.report(
        Diagnostic::error(
            code::E_ATTRIBUTE_WRONG_SITE,
            format!(
                "`{shown}` is used on {}, and its declaration allows only {}",
                site.noun(),
                site_list(sites)
            ),
        )
        .with_primary(attr.span, "not allowed here")
        .with_help(format!(
            "Use `#[{shown}]` only on {}, or delete it here.",
            site_list(sites)
        )),
    );
    false
}

/// Check 3, the same for a program's declaration and a `Core` one: `E0763`
/// for a second use of `name` on one declaration when it is not
/// `repeatable`. Each later use is reported against the first. `program` is
/// whether a program declares it, so the help may say to add `repeatable`.
/// Whether the use may go on.
fn count_fits(
    attr: &Attribute,
    shown: &str,
    name: &nvs_hir::QName,
    repeatable: bool,
    program: bool,
    used: &mut Vec<(nvs_hir::QName, Span)>,
    env: &mut Env<'_>,
) -> bool {
    if !repeatable && let Some((_, first)) = used.iter().find(|(seen, _)| seen == name) {
        env.diags.report(
            Diagnostic::error(
                code::E_ACCESS_REPEATED,
                format!("`{shown}` is used twice here, and its declaration is not `repeatable`"),
            )
            .with_primary(attr.span, "second use")
            .with_secondary(*first, "first use")
            .with_help(if program {
                format!("Delete one of the two, or write `repeatable` before `attribute {shown}`.")
            } else {
                format!("Delete one of the two, or write all of it in one `#[{shown}]`.")
            }),
        );
        return false;
    }
    used.push((name.clone(), attr.span));
    true
}

/// Every attribute on one parameter list — `rule:attributes/attach-sites-and-forms`'s parameter attach site.
fn check_params(params: &[Param], ctx: &Ctx<'_>, env: &mut Env<'_>) {
    for param in params {
        let site = if param.is_promoted() {
            UseSite::Promoted
        } else {
            UseSite::One(AttributeSite::Parameter)
        };
        check_groups(&param.attributes, site, ctx, env);
    }
}

/// Every attribute on one declaration. Each declaration this list has
/// accepted so far is kept, so a second use of one that is not `repeatable`
/// is reported where it is written.
fn check_groups(groups: &[AttributeGroup], site: UseSite, ctx: &Ctx<'_>, env: &mut Env<'_>) {
    let mut used: Vec<(nvs_hir::QName, Span)> = Vec::new();
    for group in groups {
        for attr in &group.attributes {
            check_attribute(attr, site, &mut used, ctx, env);
        }
    }
}

/// What an attribute's name resolves to ([`lookup`]).
pub(crate) enum Lookup<'e> {
    /// An `attribute` declaration.
    Declared(&'e nvs_hir::AttributeDef),
    /// A name on `Core`'s compiler roster: its row of
    /// [`crate::derive::ATTRIBUTE_DOCS`], and the name it resolved to.
    Recognized(&'static crate::derive::AttributeDoc, nvs_hir::QName),
    /// Something that is not an attribute. The text is what it is, for a
    /// message: "a class", "a `type` alias".
    NotAnAttribute(&'static str),
    /// A diagnostic has been reported already.
    Reported,
}

/// Resolves the name `name`, or `name::member` when `member` is written, to the
/// declaration it names. See this module's docs for the two scopes. An
/// undeclared name and a name two scopes give are reported here. What is
/// reported for something that is not an attribute depends on where it is
/// written, so the caller reports it.
pub(crate) fn lookup<'e>(
    name: &Name,
    member: Option<Span>,
    ctx: &Ctx<'_>,
    env: &mut Env<'e>,
) -> Lookup<'e> {
    let aliases: &'e nvs_hir::AliasTable = env.aliases;
    let text = span_text(env.src, name.span).to_owned();
    if let Some(member) = member {
        let owner = nvs_hir::resolve_ref(&text, ctx.namespace, ctx.imports);
        let short = span_text(env.src, member);
        if let Some(def) = aliases.attribute(&nvs_hir::QName::member(&owner, short)) {
            return Lookup::Declared(def);
        }
        if aliases.get_member(&owner, short).is_some() {
            return Lookup::NotAnAttribute("a `type` alias");
        }
        // The type position's lowering reports an owner or a member that
        // resolves to nothing. Anything else is a class constant or an enum
        // case.
        let ty = Type {
            kind: TypeKind::Atom(TypeAtom::Member(*name, member)),
            span: name.span.to(member),
        };
        let id = crate::lower::lower_type(&ty, ctx, env);
        if env.interner.is_error(id) {
            return Lookup::Reported;
        }
        return Lookup::NotAnAttribute("a class constant or an enum case");
    }
    let qname = nvs_hir::resolve_ref(&text, ctx.namespace, ctx.imports);
    let recognized = crate::derive::ATTRIBUTE_DOCS
        .iter()
        .find(|doc| qname == nvs_hir::QName::parse(doc.name));
    let scoped = aliases.attribute(&qname);
    // Only a short name is looked up on the owner chain.
    let owned = ctx
        .current_class
        .filter(|_| !text.contains('\\'))
        .and_then(|class| {
            nvs_hir::find_in_ancestors(class, env.graph, |ancestor| {
                aliases.attribute(&nvs_hir::QName::member(ancestor, &text))
            })
        });
    match (owned, scoped.is_some() || recognized.is_some()) {
        (Some(owned), true) => {
            report_two_scopes(name.span, owned, &qname, env);
            return Lookup::Reported;
        }
        (Some(owned), false) => return Lookup::Declared(owned),
        (None, true) => {
            return match (scoped, recognized) {
                (Some(def), _) => Lookup::Declared(def),
                (None, Some(doc)) => Lookup::Recognized(doc, qname),
                (None, None) => Lookup::Reported,
            };
        }
        (None, false) => {}
    }
    if let Some(symbol) = env.symbols.get(&qname) {
        return Lookup::NotAnAttribute(match symbol.kind {
            nvs_hir::SymbolKind::Class => "a class",
            nvs_hir::SymbolKind::Interface => "an interface",
            nvs_hir::SymbolKind::Enum => "an enum",
            nvs_hir::SymbolKind::TypeAlias => "a `type` alias",
            // An attribute declaration is in the table above, so this is one
            // whose declaration did not parse.
            nvs_hir::SymbolKind::Attribute => return Lookup::Reported,
        });
    }
    if aliases.get(&qname).is_some() {
        return Lookup::NotAnAttribute("a `type` alias");
    }
    if qname.is_reserved_global_interface() {
        return Lookup::NotAnAttribute("an interface");
    }
    if qname.is_reserved_global_class() {
        return Lookup::NotAnAttribute("a class");
    }
    let core_types = nvs_stdlib::registry::type_names();
    if core_types.contains(&qname.to_string().as_str()) {
        return Lookup::NotAnAttribute("a `Core` type");
    }
    if let Some(diagnostic) = nvs_hir::undeclared_name(
        nvs_hir::Undeclared {
            qname: &qname,
            text: &text,
            span: name.span,
            namespace: ctx.namespace,
            imports: ctx.imports,
            stmts: env.stmts,
            src: env.src,
        },
        env.symbols,
        &core_types,
    ) {
        env.diags.report(diagnostic);
    }
    Lookup::Reported
}

/// Which written type argument of `owner::member` names an attribute
/// declaration: the `T` of `Core\Attributes::get` and `::all`, of
/// `Core\Router\Match::attribute` and of `Core\Program::implementingWith`.
/// [`lower_retrieval_arg`] lowers that argument.
pub(crate) fn declaration_arg(owner: &nvs_hir::QName, member: &str) -> Option<usize> {
    if crate::retrieval::is_retrieval(owner, member)
        || nvs_stdlib::registry::takes_route_attributes(&owner.to_string(), member)
    {
        return Some(0);
    }
    (crate::program::is_enumeration(owner, member) && crate::program::is_joined(member))
        .then_some(1)
}

/// The full name of the attribute declaration a type argument names, a
/// program's or a `Core` row, or `None` where it names none. It reports nothing: [`lower_retrieval_arg`] has reported every
/// mistake in the same argument already.
pub(crate) fn declaration_of(
    ty: &Type,
    ctx: &Ctx<'_>,
    env: &mut Env<'_>,
) -> Option<nvs_hir::QName> {
    let (name, member) = match &ty.kind {
        TypeKind::Atom(TypeAtom::Name(name, args)) if args.is_empty() => (name, None),
        TypeKind::Atom(TypeAtom::Member(owner, member)) => (owner, Some(*member)),
        _ => return None,
    };
    let before = env.diags.len();
    let found = lookup(name, member, ctx, env);
    env.diags.truncate(before);
    match found {
        Lookup::Declared(def) => Some(def.name.clone()),
        Lookup::Recognized(_, qname) => Some(qname),
        Lookup::NotAnAttribute(_) | Lookup::Reported => None,
    }
}

/// The declaration an attribute written at a site resolves to, read in the
/// file and scope it was written in. It reports nothing: the site's own check
/// has reported every mistake in the name already.
pub(crate) fn declaration_at(
    attr: &Attribute,
    ctx: &Ctx<'_>,
    env: &mut Env<'_>,
) -> Option<nvs_hir::QName> {
    let name = attr.name.as_ref()?;
    let before = env.diags.len();
    let found = match lookup(name, attr.member, ctx, env) {
        Lookup::Declared(def) => Some(def.name.clone()),
        Lookup::Recognized(_, qname) => Some(qname),
        Lookup::NotAnAttribute(_) | Lookup::Reported => None,
    };
    env.diags.truncate(before);
    found
}

/// One type argument that names an attribute declaration
/// ([`declaration_arg`]). The answer is the declaration's fields as a shape
/// type. Anything else is `E1008`, and the answer is the error type.
pub(crate) fn lower_retrieval_arg(ty: &Type, ctx: &Ctx<'_>, env: &mut Env<'_>) -> TypeId {
    let found = match &ty.kind {
        TypeKind::Atom(TypeAtom::Name(name, args)) if args.is_empty() => {
            Some(lookup(name, None, ctx, env))
        }
        TypeKind::Atom(TypeAtom::Member(owner, member)) => {
            Some(lookup(owner, Some(*member), ctx, env))
        }
        _ => None,
    };
    let written = |env: &Env<'_>| {
        let span = match &ty.kind {
            TypeKind::Atom(TypeAtom::Member(owner, member)) => owner.span.to(*member),
            _ => ty.span,
        };
        span_text(env.src, span).to_owned()
    };
    let (written, what) = match found {
        Some(Lookup::Declared(def)) => {
            if let TypeKind::Atom(TypeAtom::Name(name, _) | TypeAtom::Member(name, _)) = &ty.kind {
                env.exprs.record_attribute_name(name.span, def.name.clone());
            }
            let shape = def.shape.clone();
            return crate::lower::lower_type(&shape, ctx, env);
        }
        Some(Lookup::Reported) => return env.interner.error(),
        Some(Lookup::Recognized(doc, qname)) => {
            if let TypeKind::Atom(TypeAtom::Name(name, _) | TypeAtom::Member(name, _)) = &ty.kind {
                env.exprs.record_attribute_name(name.span, qname);
            }
            return core_payload_type(doc, env);
        }
        Some(Lookup::NotAnAttribute(what)) => (written(env), format!("is {what}")),
        None => {
            let id = crate::lower::lower_type(ty, ctx, env);
            if env.interner.is_error(id) {
                return id;
            }
            let what = if matches!(env.interner.get(id), Ty::Shape(_)) {
                "is a shape type"
            } else {
                "is not an attribute declaration"
            };
            (env.interner.describe(id), what.to_owned())
        }
    };
    env.diags.report(
        Diagnostic::error(
            code::E_ATTRIBUTE_TYPE_ARG_NOT_DECLARED,
            format!("a retrieval's type argument must name an attribute declaration, and `{written}` {what}"),
        )
        .with_primary(ty.span, "not an attribute declaration")
        .with_help(
            "declare the attribute with `attribute Name on … { … }`, and write its name here. \
             The call then returns the attributes written with that name",
        ),
    );
    env.interner.error()
}

/// The payload type a retrieval of a `Core` declaration returns: its row's
/// fields as a shape. A `Core` row is the same declaration value a program's
/// `attribute` builds (`rule:attributes/a-retrieval-matches-the-declaration-it-names`),
/// so `get<Core\Command>` reads `#[Core\Command]`'s payloads as `get<Job>`
/// reads `#[Job]`'s.
fn core_payload_type(doc: &crate::derive::AttributeDoc, env: &mut Env<'_>) -> TypeId {
    let fields = doc
        .fields
        .iter()
        .map(|field| crate::ty::ShapeField {
            name: field.name.to_owned(),
            ty: spelled_type(field.ty, env),
            required: field.required,
        })
        .collect();
    env.interner.shape(fields)
}

/// One of the type spellings [`crate::derive::ATTRIBUTE_DOCS`] writes:
/// `string`, `bool`, `int`, `mixed`, `object`, `array<T>`, a shape
/// `{a: T, b?: T}` and a qualified `Core` enum name. A spelling outside that list is a bug in the table, and
/// its type is `mixed`, so no program fails to compile for it.
fn spelled_type(text: &str, env: &mut Env<'_>) -> TypeId {
    let text = text.trim();
    if let Some(elem) = text
        .strip_prefix("array<")
        .and_then(|rest| rest.strip_suffix('>'))
    {
        let elem = spelled_type(elem, env);
        return env.interner.array(elem);
    }
    if let Some(body) = text
        .strip_prefix('{')
        .and_then(|rest| rest.strip_suffix('}'))
    {
        let mut fields = Vec::new();
        for part in split_top_level(body) {
            let Some((name, ty)) = part.split_once(':') else {
                continue;
            };
            let (name, required) = match name.trim().strip_suffix('?') {
                Some(name) => (name, false),
                None => (name.trim(), true),
            };
            let ty = spelled_type(ty, env);
            fields.push(crate::ty::ShapeField {
                name: name.trim().to_owned(),
                ty,
                required,
            });
        }
        return env.interner.shape(fields);
    }
    match text {
        "string" => env.interner.string(),
        "bool" => env.interner.bool_ty(),
        "int" => env.interner.int(),
        "object" => env.interner.object(),
        // A `Core` enum, such as `Core\Http\Method`.
        name if name.contains('\\') => {
            let qname = nvs_hir::QName::parse(name);
            if env.enums.get(&qname).is_some() {
                let backing = env.enums.backing_of(&qname);
                env.interner.enum_(qname, backing)
            } else {
                env.interner.class(qname)
            }
        }
        _ => {
            debug_assert!(
                text == "mixed",
                "`{text}` is not a spelling `Core`'s attribute table uses"
            );
            env.interner.mixed()
        }
    }
}

/// `body` split at each `,` outside `<…>` and `{…}`.
fn split_top_level(body: &str) -> Vec<&str> {
    let mut parts = Vec::new();
    let (mut depth, mut start) = (0usize, 0usize);
    for (at, ch) in body.char_indices() {
        match ch {
            '<' | '{' => depth += 1,
            '>' | '}' => depth = depth.saturating_sub(1),
            ',' if depth == 0 => {
                parts.push(&body[start..at]);
                start = at + 1;
            }
            _ => {}
        }
    }
    if !body[start..].trim().is_empty() {
        parts.push(&body[start..]);
    }
    parts
}

/// `E1007`: a short name that the owner chain and the namespace scope both
/// give. `scoped` is the namespace scope's declaration.
fn report_two_scopes(
    span: Span,
    owned: &nvs_hir::AttributeDef,
    scoped: &nvs_hir::QName,
    env: &mut Env<'_>,
) {
    let short = owned.short_name();
    let owner = owned
        .owner
        .as_ref()
        .map(ToString::to_string)
        .unwrap_or_default();
    let owner_short = owned
        .owner
        .as_ref()
        .map(|owner| owner.short_name().to_owned())
        .unwrap_or_default();
    let segments = scoped.segments();
    let namespace = segments[..segments.len() - 1].join("\\");
    // A qualified name is already absolute, so only a global declaration
    // needs the leading `\` to differ from the short name.
    let written = if namespace.is_empty() {
        format!("\\{scoped}")
    } else {
        scoped.to_string()
    };
    let place = if namespace.is_empty() {
        "the global namespace".to_owned()
    } else {
        format!("`{namespace}`")
    };
    env.diags.report(
        Diagnostic::error(
            code::E_ATTRIBUTE_NAME_IN_TWO_SCOPES,
            format!(
                "`{short}` is declared in `{owner}` and in {place}, so write \
                 `{owner_short}::{short}` or `{written}`"
            ),
        )
        .with_primary(span, "two declarations have this name")
        .with_help(format!(
            "`{owner_short}::{short}` names the attribute declared in `{owner}`. `{written}` \
             names the other one."
        )),
    );
}

/// `E1005`'s fix: the shape alias the attribute's name resolves to, rewritten
/// from `type Name = { … };` to `attribute Name on <site> { … }`, as the span
/// of the alias's declaration and its replacement
/// (`rule:ide/a-code-action-ships-only-a-fix-a-diagnostic-already-knows`).
///
/// `None` unless the alias is declared in this file, has no type parameters,
/// and its right-hand side is a shape, and unless `site` is one a program's
/// declaration may list. The alias's doc comment stays above it, so it
/// documents the attribute. Costs one alias lookup and a scan of the
/// declaration's text, and only where `E1005` is reported.
fn alias_as_attribute(
    name: &Name,
    attr: &Attribute,
    site: UseSite,
    ctx: &Ctx<'_>,
    env: &Env<'_>,
) -> Option<(Span, String)> {
    let readable = match site {
        UseSite::Promoted => true,
        UseSite::One(site) => matches!(
            site,
            AttributeSite::Class
                | AttributeSite::Method
                | AttributeSite::Property
                | AttributeSite::Parameter
        ),
    };
    if !readable {
        return None;
    }
    let text = span_text(env.src, name.span);
    let qname = nvs_hir::resolve_ref(text, ctx.namespace, ctx.imports);
    let (rhs, short) = match attr.member {
        Some(member) => {
            let short = span_text(env.src, member);
            (env.aliases.get_member(&qname, short)?, short)
        }
        None => (env.aliases.get(&qname)?, qname.short_name()),
    };
    if rhs.span.file != attr.span.file || !matches!(rhs.kind, TypeKind::Atom(TypeAtom::Shape(_))) {
        return None;
    }
    let src = env.src.text();
    let start = rhs.span.start as usize;
    let end = rhs.span.end as usize;
    // `type Name =` in front of the shape, and `;` after it.
    let head = src.get(..start)?.trim_end().strip_suffix('=')?.trim_end();
    let head = head.strip_suffix(short)?;
    let keyword = head.trim_end();
    if keyword.len() == head.len() {
        return None;
    }
    let keyword = keyword.strip_suffix("type")?;
    if keyword
        .chars()
        .next_back()
        .is_some_and(|ch| ch.is_ascii_alphanumeric() || ch == '_')
    {
        return None;
    }
    let tail = src.get(end..)?;
    let semicolon = tail.len() - tail.trim_start().len();
    if !tail[semicolon..].starts_with(';') {
        return None;
    }
    let from = u32::try_from(keyword.len()).ok()?;
    let to = u32::try_from(end + semicolon + 1).ok()?;
    let body = src.get(start..end)?;
    let replacement = format!("attribute {short} on {} {body}", site.word());
    Some((Span::new(attr.span.file, from, to), replacement))
}

fn check_attribute(
    attr: &Attribute,
    site: UseSite,
    used: &mut Vec<(nvs_hir::QName, Span)>,
    ctx: &Ctx<'_>,
    env: &mut Env<'_>,
) {
    let mut payload = Payload {
        constant: true,
        clean: true,
    };
    for field in &attr.fields {
        payload = payload.and(check_value(&field.value, ctx, env));
    }
    let constant = payload.constant;
    let Some(name) = &attr.name else {
        // `rule:attributes/attach-sites-and-forms`'s bare form names no shape to check against, so the
        // literal is checked as a well-formed literal and nothing more —
        // which the constant walk above has just done — and its values are
        // then inferred so that what each names is on record.
        if constant && payload.clean {
            infer_fields(&attr.fields, false, ctx, env);
        }
        return;
    };
    let def = match lookup(name, attr.member, ctx, env) {
        Lookup::Declared(def) => def,
        // `rule:core-classes/derive-attribute`'s compiler-recognized
        // attributes: checks 1 and 3 as for a program's declaration, and the
        // payload by the pass that owns the name.
        Lookup::Recognized(doc, qname) => {
            env.exprs.record_attribute_name(name.span, qname.clone());
            let shown = span_text(env.src, name.span).to_owned();
            if site_fits(attr, site, &shown, doc.sites, env)
                && count_fits(attr, &shown, &qname, doc.repeatable, false, used, env)
            {
                check_recognized(attr, &qname, payload, ctx, env);
            }
            return;
        }
        Lookup::NotAnAttribute(what) => {
            let written = span_text(env.src, attr.member.map_or(name.span, |m| name.span.to(m)));
            let written = written.to_owned();
            let mut diagnostic = Diagnostic::error(
                code::E_NOT_AN_ATTRIBUTE_DECLARATION,
                format!("only an attribute declaration may follow `#[`, and `{written}` is {what}"),
            )
            .with_primary(attr.span, "this is not an attribute declaration")
            .with_help(format!(
                "declare the attribute with `attribute Name on {} {{ … }}`, and write its \
                 name after `#[`",
                site.word()
            ));
            if let Some((span, replacement)) = alias_as_attribute(name, attr, site, ctx, env) {
                let head = replacement
                    .split_once(" {")
                    .map_or(replacement.as_str(), |(head, _)| head);
                let title = format!("rewrite the `type` alias as `{head}`");
                diagnostic = diagnostic.with_fix(span, replacement, title);
            }
            env.diags.report(diagnostic);
            return;
        }
        Lookup::Reported => return,
    };
    env.exprs.record_attribute_name(name.span, def.name.clone());
    let short = def.short_name();
    // 1. The site is one the declaration lists.
    if !site_fits(attr, site, short, &listed(&def.sites), env) {
        return;
    }
    // 2. An owned declaration is used on its owner's descendants only.
    if let Some(owner) = &def.owner {
        let descends = ctx.current_class.is_some_and(|class| {
            class != owner && nvs_hir::implements_interface(class, owner, env.graph)
        });
        if !descends {
            env.diags.report(
                Diagnostic::error(
                    code::E_ATTRIBUTE_OUTSIDE_ITS_OWNER,
                    format!(
                        "`{short}` is declared in `{owner}`, so it is used only on classes that \
                         extend or implement `{owner}` and their members"
                    ),
                )
                .with_primary(attr.span, "this class does not extend or implement it")
                .with_help(format!(
                    "Use `#[{short}]` in a class that extends or implements `{owner}`, or \
                     declare the attribute outside `{owner}`."
                )),
            );
            return;
        }
    }
    // 3. A second use needs `repeatable`.
    if !count_fits(attr, short, &def.name, def.repeatable, true, used, env) {
        return;
    }
    // 4. The payload fits the declared fields.
    let shape = def.shape.clone();
    let shape = crate::lower::lower_type(&shape, ctx, env);
    if !matches!(env.interner.get(shape), Ty::Shape(_)) {
        return;
    }
    env.exprs.record_attribute_shape(attr.span, shape);
    // A payload with a computed value has already been reported once, and
    // inferring that value would report it a second time — as an undeclared
    // variable, or as a mismatch against the field the shape declares. The
    // author is told about the value they wrote before they are told what it
    // failed to satisfy.
    if !payload.constant {
        return;
    }
    // Deliberately the ordinary shape-typed position's check, run over an
    // empty scope: the constant-payload check
    // (`rule:attributes/payload-is-a-compile-time-constant`) has just proved
    // there is no variable in this payload, so no binding can be read and none
    // can be captured. Each field is then read at the type its alias names for
    // it, so `'clear'` at `'clear'|'keep'` is that single-value type, the check
    // a retrieval makes, and a payload accepted here is the payload
    // `Core\Attributes::get<Name>` finds.
    let mut live = Live::default();
    let scope = LocalScope::new();
    let actual = check_anon_object(attr.span, &attr.fields, None, &mut live, &scope, ctx, env);
    let actual = crate::retrieval::with_requested_literals(
        &attr.fields,
        actual,
        shape,
        &mut live,
        &scope,
        ctx,
        env,
    );
    // A payload that is not clean has had its `secret` constant reported as
    // `E0727`. A field declared `mixed` refuses a `secret` value, and that
    // would report the same constant again. So the fit reads each `mixed` as
    // `secret mixed`, and every other mismatch is still reported
    // (`rule:types/one-mistake-reports-once`).
    let want = if payload.clean {
        shape
    } else {
        crate::expr::quals::mixed_admitting_secret(shape, env.interner)
    };
    if !built_fits(actual, want, env.interner, env.graph, env.signatures) {
        report_mismatch(attr.payload, shape, actual, env);
    }
}

/// One attribute whose name resolved to `rule:core-classes/derive-attribute`'s
/// roster, its payload checked by the pass that owns that name.
fn check_recognized(
    attr: &Attribute,
    qname: &nvs_hir::QName,
    payload: Payload,
    ctx: &Ctx<'_>,
    env: &mut Env<'_>,
) {
    if payload.constant {
        // Only the rosters that have a payload to check appear here; a
        // recognized name whose payload is empty by construction
        // (`#[Fixture]`) has nothing to say and says nothing.
        let recognized = |want: &str| *qname == nvs_hir::QName::parse(want);
        if recognized(crate::derive::TEST) {
            crate::testing::check_payload(&attr.fields, ctx, env);
        } else if recognized(crate::derive::BENCH) {
            crate::testing::check_bench_payload(&attr.fields, ctx, env);
        } else if recognized(crate::derive::NO_MUTATE) {
            crate::testing::check_no_mutate_payload(attr, ctx, env);
        } else if recognized(crate::derive::PROPERTY) {
            crate::property::check_payload(&attr.fields, ctx, env);
        } else if recognized(crate::derive::COMMAND) {
            check_roster(
                "Command",
                crate::commands::COMMAND_OPTIONS,
                &attr.fields,
                ctx,
                env,
            );
        } else if recognized(crate::derive::OPTION) {
            check_roster(
                "Option",
                crate::commands::OPTION_OPTIONS,
                &attr.fields,
                ctx,
                env,
            );
        } else if recognized(crate::derive::ROUTE) {
            check_roster("Route", crate::routes::OPTIONS, &attr.fields, ctx, env);
        } else if recognized(crate::derive::ACCESS) {
            // Two calls, because `rule:attributes/access-payload` states two kinds of rule:
            // the roster answers what is asked of every payload, and the
            // pass that owns the attribute answers what is about
            // `#[Access]` alone.
            check_roster(
                "Access",
                crate::routes::ACCESS_OPTIONS,
                &attr.fields,
                ctx,
                env,
            );
            crate::routes::check_access(attr, env);
        } else if recognized(crate::derive::API) {
            // Only the roster here. `rule:attributes/api-adds-and-cannot-contradict`'s contradictions are
            // every one of them a comparison against the *declaration* —
            // its `#[Route]`, its return type, the classes the program
            // declares — so they are asked by the per-class walk that
            // holds those, exactly as `#[Route]`'s own path checks are.
            check_roster("Api", crate::routes::API_OPTIONS, &attr.fields, ctx, env);
        } else if recognized(crate::derive::API_SCHEME) {
            // The roster is every field some `type` takes. Which fields one
            // `type` takes is `crate::routes::collect_schemes`' question.
            check_roster(
                r"Api\Scheme",
                crate::routes::SCHEME_OPTIONS,
                &attr.fields,
                ctx,
                env,
            );
        } else if recognized(crate::derive::KEEP_KEY_ORDER) {
            // The roster answers what `reason` may hold. That it is written,
            // and that the class derives a JSON codec, is
            // `crate::derive::check_class_derive`'s question.
            check_roster(
                r"Core\Json\KeepKeyOrder",
                crate::derive::KEEP_KEY_ORDER_OPTIONS,
                &attr.fields,
                ctx,
                env,
            );
        } else if recognized(crate::derive::DEPRECATED) {
            // The roster admits `construct` everywhere, because it cannot see
            // the declaration; `crate::deprecated::check_sites` refuses it on
            // everything but a class.
            check_roster(
                r"Core\Deprecated",
                crate::deprecated::OPTIONS,
                &attr.fields,
                ctx,
                env,
            );
        } else if [
            crate::derive::DERIVE,
            crate::derive::DB_DERIVE,
            crate::derive::QUERY,
            crate::derive::ROUTE_TARGET,
            crate::derive::ROUTE_PARAMS,
            crate::derive::PATH,
        ]
        .into_iter()
        .any(recognized)
        {
            // `#[Core\Json\Derive()]` is `#[Core\Json\Derive]`, and a field in
            // the list is an option no pass reads. The test hooks and
            // `#[Fixture]` report theirs in their own passes.
            report_no_options(attr, env);
        } else if payload.clean {
            // A recognized name with no roster — `#[Json\Field]`,
            // `#[TestWith]` — has its payload read by its own pass, which
            // reads the literal and records nothing about the values. A
            // `#[TestWith]` row is judged there against the parameter it
            // names, a `uint` one included, so an integer only a `uint`
            // holds is inferred as one here and left for that pass to
            // place.
            let rows = recognized(crate::derive::TEST_WITH);
            infer_fields(&attr.fields, rows, ctx, env);
        }
    }
}

/// `E_UNKNOWN_OPTION` for each field of a recognized attribute that takes
/// none.
fn report_no_options(attr: &Attribute, env: &mut Env<'_>) {
    let Some(name) = &attr.name else {
        return;
    };
    let written = span_text(env.src, name.span).to_owned();
    for field in &attr.fields {
        let option = span_text(env.src, field.name).to_owned();
        env.diags.report(
            Diagnostic::error(
                code::E_UNKNOWN_OPTION,
                format!("`{option}` is not an option of `#[{written}]`"),
            )
            .with_primary(field.span, "no such option")
            .with_help(format!("`#[{written}]` takes no options.")),
        );
    }
}

/// What [`check_value`] found out about one value, or a whole payload: whether
/// it is a compile-time constant, and whether it is **clean** — no `secret`
/// constant refused inside it.
///
/// Two flags rather than one, because they gate different things. A payload
/// that is not constant is not checked against any shape; a payload that is
/// constant and not clean still is, so the author sees what the value failed
/// to satisfy — and is not *inferred* afterwards, because the ordinary
/// inference of a refused value reports a second refusal for the same bytes
/// (`E0824` for a `secret` inside an array literal) after `E0727` has already
/// said the one thing that matters.
#[derive(Clone, Copy)]
struct Payload {
    constant: bool,
    clean: bool,
}

impl Payload {
    /// Both findings, over this and one more value.
    const fn and(self, other: Self) -> Self {
        Self {
            constant: self.constant && other.constant,
            clean: self.clean && other.clean,
        }
    }
}

/// Every value of a payload no roster and no shape places, inferred over an
/// empty scope so that what each names is recorded: `Status::Live` in a bare
/// `#[{ state: Status::Live }]` is an enum case an editor can follow, exactly as
/// it is in a payload a roster reads through [`check_roster`].
///
/// Nothing is placed and nothing new is reported for a constant that resolves:
/// [`check_value`] has proved every value here constant and clean, and a
/// constant infers over no scope at all. What a name that resolves to nothing
/// gets is the ordinary `E0303`, which is the answer every other position
/// gives it.
///
/// `uint_magnitudes` is set for a payload whose own pass places each value
/// against a declared type afterwards: an integer literal no `int` holds is
/// then inferred at `uint`, where a payload nothing places gets the ordinary
/// `E0429` for it.
fn infer_fields(
    fields: &[AnonObjectField],
    uint_magnitudes: bool,
    ctx: &Ctx<'_>,
    env: &mut Env<'_>,
) {
    let mut live = Live::default();
    let scope = LocalScope::new();
    for field in fields {
        let expected = if uint_magnitudes && exceeds_int(&field.value, env) {
            Some(env.interner.uint())
        } else {
            None
        };
        check_expr(&field.value, expected, &mut live, &scope, ctx, env);
    }
}

/// Whether `expr` is a bare integer literal whose magnitude a `uint` holds and
/// an `int` does not.
fn exceeds_int(expr: &Expr, env: &Env<'_>) -> bool {
    matches!(
        &expr.kind,
        ExprKind::Int(span)
            if crate::defaults::int_magnitude(*span, env)
                .is_some_and(|magnitude| i64::try_from(magnitude).is_err())
    )
}

/// Each option a `Core` attribute no longer takes: the attribute as
/// [`check_roster`] names it, the option, and the help sentence that names
/// its replacement. `rule:core-api/no-deprecation-window-before-a-release`
/// keeps a removed option an error, and this list is how its error says what
/// to write instead. A row stays while code written before the removal may
/// still be compiled, and costs one comparison per unknown option.
const REMOVED_OPTIONS: &[(&str, &str, &str)] = &[(
    "Route",
    "name",
    "A link names the route method itself, as in `Core\\Router::url(Home::show)`",
)];

/// One recognized attribute's payload, checked against its roster of options.
///
/// The walk every `rule:core-classes/derive-attribute` name that carries a payload shares, because a
/// recognized name is matched *nominally* and so has no shape to be checked
/// against: what it may hold is a roster its own module declares — `rule:testing/test-attribute`'s is [`crate::testing`]'s, `rule:tooling/commands-are-compiled`'s are [`crate::commands`]',
/// `rule:routing/route-attribute`'s is [`crate::routes`]'. The answers per field are deliberately
/// these: a name no row declares, a value at the wrong type, and a name given
/// twice.
///
/// Called only for a payload [`check_attribute`] has already proved constant,
/// for this module's own reason: the author is told about a value they wrote
/// before they are told what it failed to satisfy. A rule *between* two
/// options — `rule:testing/runner-is-strict`'s `retries` requiring `because` — is one this walk
/// cannot state, and stays with the roster that declares it.
///
/// `attribute` is the name as a diagnostic writes it, without its `#[]`.
/// A name on [`REMOVED_OPTIONS`] is answered with what replaced it.
pub(crate) fn check_roster(
    attribute: &str,
    options: &[(&str, crate::testing::OptionTy)],
    fields: &[AnonObjectField],
    ctx: &Ctx<'_>,
    env: &mut Env<'_>,
) {
    // The scope is empty and stays empty: the constant-payload check has just
    // proved this payload
    // reads no variable, so there is no binding to mark live and none to
    // capture.
    let mut live = Live::default();
    let scope = LocalScope::new();
    let mut seen: Vec<String> = Vec::with_capacity(fields.len());
    for field in fields {
        let name = span_text(env.src, field.name).to_owned();
        let declared = options
            .iter()
            .find(|(option, _)| *option == name)
            .map(|(_, ty)| *ty);
        let expected = declared.and_then(|ty| ty.intern(env));
        check_expr(&field.value, expected, &mut live, &scope, ctx, env);
        if declared.is_none() {
            let removed = REMOVED_OPTIONS
                .iter()
                .find(|(on, option, _)| *on == attribute && *option == name);
            let help = match (removed, crate::routes::option_home(attribute, &name)) {
                (Some((_, _, instead)), _) => {
                    format!("`{name}` was removed from `#[{attribute}]`. {instead}")
                }
                (None, Some(home)) => format!(
                    "`{name}` is an option of `#[{home}]`. Write it in the `#[{home}]` of the same method"
                ),
                (None, None) => format!("the options are: {}", roster_names(options)),
            };
            env.diags.report(
                Diagnostic::error(
                    code::E_UNKNOWN_OPTION,
                    format!("`{name}` is not an option of `#[{attribute}]`"),
                )
                .with_primary(field.span, "no such option")
                .with_help(help),
            );
        } else if seen.iter().any(|already| already == &name) {
            env.diags.report(
                Diagnostic::error(
                    code::E_DUPLICATE_DECLARATION,
                    format!("the option `{name}` is given twice"),
                )
                .with_primary(field.span, "already set above"),
            );
        }
        seen.push(name);
    }
}

/// A roster rendered for a help text — `skip: string, at: string, …` — so a
/// typo is answered with the options themselves rather than with a type
/// spelling nobody wrote.
fn roster_names(options: &[(&str, crate::testing::OptionTy)]) -> String {
    options
        .iter()
        .map(|(option, ty)| format!("{option}: {}", ty.describe()))
        .collect::<Vec<_>>()
        .join(", ")
}

/// One payload value, reported where it is written rather than at the
/// attribute — a payload with two bad fields is two diagnostics, in source
/// order, because each is its own mistake. Answers whether this value (and
/// every value nested inside it) is a constant, which is what tells
/// [`check_attribute`] the literal is worth checking against a shape, and
/// whether it is clean ([`Payload`]).
fn check_value(expr: &Expr, ctx: &Ctx<'_>, env: &mut Env<'_>) -> Payload {
    if is_constant(expr) {
        // `rule:security/secret-sinks-refuse`'s payload sink, asked of every value this walk reaches
        // and not only of a payload's top level: a `secret` constant nested
        // inside an array or an anonymous object is folded into the same
        // constant pool. It answers for a `Class::CONST` and for nothing
        // else — see [`crate::expr::reject_secret_attribute_constant`] for
        // why one expression kind is the whole of it.
        let mut found = Payload {
            constant: true,
            clean: !crate::expr::reject_secret_attribute_constant(expr, ctx, env),
        };
        // A container's own elements are values in their own right, so a
        // constant-shaped container is descended into rather than trusted.
        match &expr.kind {
            ExprKind::ArrayLiteral(items) => {
                for ArrayItem { key, value, .. } in items {
                    if let Some(key) = key {
                        found = found.and(check_value(key, ctx, env));
                    }
                    found = found.and(check_value(value, ctx, env));
                }
            }
            ExprKind::AnonObject(fields) => {
                for field in fields {
                    found = found.and(check_value(&field.value, ctx, env));
                }
            }
            ExprKind::Paren(inner) | ExprKind::Unary { expr: inner, .. } => {
                found = found.and(check_value(inner, ctx, env));
            }
            _ => {}
        }
        return found;
    }
    env.diags.report(
        Diagnostic::error(
            code::E_ATTRIBUTE_VALUE_NOT_CONSTANT,
            "an attribute's field value is not a compile-time constant",
        )
        .with_primary(expr.span, "this is computed when the program runs")
        .with_help(
            "an attribute's value is a literal, a class constant or an enum case. It is fixed \
             when the program compiles, so it cannot be a variable, a call or a `new`",
        ),
    );
    Payload {
        constant: false,
        clean: true,
    }
}

/// The closed list of shapes `rule:attributes/payload-is-a-compile-time-constant` admits. Closed on purpose: an
/// expression kind this does not name is refused, so a shape the grammar
/// grows is refused until someone decides it belongs in a constant pool,
/// rather than accepted because nothing said otherwise.
pub(crate) fn is_constant(expr: &Expr) -> bool {
    match &expr.kind {
        ExprKind::Null
        | ExprKind::Bool(_)
        | ExprKind::Int(_)
        | ExprKind::Float(_)
        | ExprKind::Duration(_)
        | ExprKind::Str(_)
        // `Class::CONST` — a class constant, and `rule:enums/closed-integer-type`'s enum case with it,
        // which is the one spelling `rule:attributes/payload-is-a-compile-time-constant`
        // names beside a literal.
        | ExprKind::ClassConstAccess { .. }
        // `Class::class`, which that rule's list already covers: it *is* a class
        // constant, and the only one whose value the compiler resolves rather
        // than reads — `crate::expr::members::check_class_name_const` folds it
        // to the fully qualified name with no runtime step at all. Admitted
        // because `rule:attributes/api-adds-and-cannot-contradict` writes an `errors` entry's `type` as a name,
        // and a name is either this spelling or a magic string; `rule:programs/no-runtime-autoload`'s
        // whole premise is that a name resolves, so the string form would be
        // the one thing the constant-payload rule exists to refuse.
        | ExprKind::ClassNameConst { .. }
        | ExprKind::ArrayLiteral(_)
        | ExprKind::AnonObject(_) => true,
        // An interpolated string reads a variable by definition, whatever it
        // interpolates, so it is not the `Str` row one syntax along.
        ExprKind::Paren(inner) => is_constant(inner),
        // The sign on a numeric literal, and `~` on an integer one: an
        // operator over a constant operand is folded with it.
        ExprKind::Unary { op, expr } => {
            matches!(op, UnaryOp::Neg | UnaryOp::Plus | UnaryOp::BitNot) && is_constant(expr)
        }
        // Error recovery already reported something; a second diagnostic on
        // the same span names one mistake twice.
        ExprKind::Error(_) => true,
        _ => false,
    }
}
