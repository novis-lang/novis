//! An `extend` block's shape: what its target may be, and which members it
//! may add (`rule:classes/an-extend-block-adds-members-to-a-type`).
//!
//! `nvs_hir::hierarchy` resolves the target and refuses a declared name of the
//! wrong kind. What is left here needs what only this crate holds: the `Core`
//! registry, to tell a `Core` class, interface or enum from a `Core`
//! attribute or namespace, and the [`crate::package::Packages`] of the
//! program, to refuse a block of a type its own package declares. Each
//! refused member is one [`code::E_EXTEND_MEMBER_NOT_ADDABLE`] at the member,
//! and the rest of the block is still checked. An added method named like a
//! member of a compiler-declared interface is [`code::E_ADDED_NAME_RUNTIME_CALLS`]
//! and an added name that meets another member is
//! [`code::E_ADDED_MEMBER_COLLIDES`], both in [`check_collisions`].
//!
//! Cost: one walk of a block's members, and one scan of the `Core` type roster
//! per block whose target is under `Core`. All of it is compile time.

use nvs_diagnostics::{Diagnostic, Span, code};
use nvs_hir::{ClassGraph, QName, SymbolKind};
use nvs_syntax::ast::{
    ClassMember, ClassMemberKind, ExtendDecl, Modifier, PropertyHookKind, PropertyMember, Type,
    TypeAtom, TypeKind,
};
use rustc_hash::FxHashMap;

use crate::signatures::{MethodSig, SignatureTable};
use crate::{Env, span_text};

/// Checks one `extend` block declared at file scope under `namespace`.
pub(crate) fn check_block(decl: &ExtendDecl, namespace: &[String], env: &mut Env<'_>) {
    let block = QName::join(namespace, span_text(env.src, decl.name.span));
    // A target that did not resolve was reported where it was written.
    let Some(target) = env.graph.block_target(&block).cloned() else {
        return;
    };
    let kind = target_kind(&target, env);
    let Some(kind) = kind else {
        env.diags.report(
            Diagnostic::error(
                code::E_EXTEND_TARGET_NOT_A_TYPE,
                format!(
                    "`{target}` is not a class, an interface or an enum, and an `extend` block \
                     extends one of these"
                ),
            )
            .with_primary(decl.target.span, "not a class, an interface or an enum"),
        );
        return;
    };
    if is_own_type(&target, decl.span, env) {
        env.diags.report(
            Diagnostic::error(
                code::E_EXTEND_OWN_TYPE,
                format!("`{target}` is declared in your own code"),
            )
            .with_primary(decl.target.span, "a type of this package")
            .with_help(format!(
                "add the member to `{target}` itself. An `extend` block adds members to a type \
                 from `Core` or from another package"
            )),
        );
    }
    check_type_params(decl, &target, env);
    let sealed = nothing_extends(&target, kind);
    for member in &decl.members {
        check_member(member, kind, env);
        if let ClassMemberKind::Property(p) = &member.kind {
            refuse_stored(&target, kind, p, env);
        }
        if sealed && let ClassMemberKind::Attribute(nested) = &member.kind {
            refuse_sealed_attribute(&target, nested.name.span, env);
        }
    }
    check_collisions(decl, &block, &target, env);
}

/// Whether no class of a program can extend or implement `target`: a `Core`
/// class (`rule:core-api/core-classes-are-sealed`), or a `Core` interface only
/// `Core` classes implement. An attribute a block adds to such a type could be
/// used nowhere, since an owned attribute is used on the owner's descendants.
fn nothing_extends(target: &QName, kind: SymbolKind) -> bool {
    if !target.is_core() {
        return false;
    }
    let written = target.to_string();
    match kind {
        SymbolKind::Class => true,
        SymbolKind::Interface => nvs_stdlib::registry::INTERFACES
            .iter()
            .any(|i| i.name.eq_ignore_ascii_case(&written)),
        _ => false,
    }
}

/// `E0338` for a nested attribute on a type [`nothing_extends`].
fn refuse_sealed_attribute(target: &QName, at: Span, env: &mut Env<'_>) {
    env.diags.report(
        Diagnostic::error(
            code::E_EXTEND_MEMBER_NOT_ADDABLE,
            format!(
                "an `extend` block cannot add an attribute to `{target}`, because no class can \
                 extend or implement it"
            ),
        )
        .with_primary(at, "this attribute could not be used anywhere")
        .with_help(
            "An attribute declared on a type is used only on the classes that extend or \
             implement it. Declare the attribute at file scope instead."
                .to_owned(),
        ),
    );
}

/// Which name table an added member is looked up in: a method, a constant
/// (an enum case is one), or a property.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Method,
    Const,
    Property,
}

impl Kind {
    fn word(self) -> &'static str {
        match self {
            Self::Method => "method",
            Self::Const => "constant",
            Self::Property => "property",
        }
    }
}

/// The added name of `member`, with the table it is looked up in. A nested
/// enum or attribute is named through the type, and is not a member here.
fn added_name(
    member: &ClassMember,
    src: &nvs_diagnostics::SourceFile,
) -> Option<(Kind, String, Span)> {
    let (kind, span) = match &member.kind {
        ClassMemberKind::Method(m) => (Kind::Method, m.name),
        ClassMemberKind::Const(c) => (Kind::Const, c.name),
        ClassMemberKind::Property(p) => (Kind::Property, p.name),
        _ => return None,
    };
    let name = span_text(src, span).trim_start_matches('$').to_owned();
    Some((kind, name, span))
}

/// Whether the signatures of `class` itself declare `name` of `kind`.
fn declares(class: &QName, kind: Kind, name: &str, env: &Env<'_>) -> bool {
    let enum_case = kind == Kind::Const && env.enums.case(class, name).is_some();
    enum_case
        || env.signatures.get(class).is_some_and(|sig| match kind {
            Kind::Method => sig.methods.contains_key(name),
            Kind::Const => sig.constants.contains_key(name),
            Kind::Property => {
                sig.properties.contains_key(name)
                    || sig.static_properties.iter().any(|(own, _)| own == name)
            }
        })
}

/// Refuses every added name that meets another member, `E1011`: a member of
/// the target or an ancestor, a member another block adds to a related type,
/// and a member a class or interface of the program declares below the
/// target. Both places are in the error. Two blocks that meet are reported
/// once, at the block whose name sorts last.
///
/// The whole program is in [`Env::graph`] and [`Env::signatures`] before any
/// body is checked, so this never depends on which file asks. Cost: per added
/// member, one ancestor walk, one pass over the program's blocks and one over
/// its declared classes. All of it is compile time.
fn check_collisions(decl: &ExtendDecl, block: &QName, target: &QName, env: &mut Env<'_>) {
    for member in &decl.members {
        let Some((kind, name, at)) = added_name(member, env.src) else {
            continue;
        };
        let what = kind.word();
        if kind == Kind::Method
            && let Some(interface) = runtime_calls(&name, env)
        {
            env.diags.report(
                Diagnostic::error(
                    code::E_ADDED_NAME_RUNTIME_CALLS,
                    format!("an `extend` block cannot add a method named `{name}`"),
                )
                .with_primary(at, format!("`{name}` is a method of `{interface}`"))
                .with_help(format!(
                    "give the method a different name. Novis calls `{name}` by its name, and it \
                     never calls an added method that way. To give a class of your own this \
                     method, write `implements {interface}`"
                )),
            );
            continue;
        }
        let own = nvs_hir::find_in_ancestors(target, env.graph, |class| {
            declares(class, kind, &name, env).then(|| class.clone())
        })
        .or_else(|| declares(target, kind, &name, env).then(|| target.clone()));
        if let Some(owner) = own {
            let found = member_span(&owner, kind, &name, env);
            collide(
                at,
                format!("`{owner}` already has a {what} named `{name}`"),
                found.map(|span| (span, format!("`{owner}::{name}` is declared here"))),
                env,
            );
            continue;
        }
        let other = env
            .graph
            .blocks()
            .filter(|(other, other_target)| {
                *other != block
                    && other.to_string() < block.to_string()
                    && (reaches(target, other_target, env.graph)
                        || reaches(other_target, target, env.graph))
                    && declares(other, kind, &name, env)
            })
            .map(|(other, _)| other.clone())
            .min_by_key(ToString::to_string);
        if let Some(other) = other {
            let found = member_span(&other, kind, &name, env);
            collide(
                at,
                format!("the `extend` block `{other}` already adds a {what} named `{name}`"),
                found.map(|span| (span, format!("`{other}` adds `{name}` here"))),
                env,
            );
            continue;
        }
        let below = env
            .signatures
            .iter()
            .filter(|(class, _)| {
                !class.is_core()
                    && *class != target
                    && env.graph.block_target(class).is_none()
                    && reaches(class, target, env.graph)
                    && declares(class, kind, &name, env)
            })
            .map(|(class, _)| class.clone())
            .min_by_key(ToString::to_string);
        if let Some(class) = below {
            let found = member_span(&class, kind, &name, env);
            collide(
                at,
                format!("`{class}` extends `{target}` and declares a {what} named `{name}`"),
                found.map(|span| (span, format!("`{class}::{name}` is declared here"))),
                env,
            );
        }
    }
}

/// The compiler-declared interface that has a method named `name`, if one
/// has. An operator, a conversion, `foreach` and a route call these methods by
/// name, so an added method may not take one (`E1012`). The names are read
/// from the members [`crate::iter_lib`] seeds for each interface on
/// [`nvs_hir::interfaces::RESERVED`], so the list has one home.
fn runtime_calls(name: &str, env: &Env<'_>) -> Option<&'static str> {
    nvs_hir::interfaces::RESERVED
        .iter()
        .map(|(interface, _)| *interface)
        .find(|interface| {
            env.signatures
                .get(&QName::parse(interface))
                .is_some_and(|sig| sig.methods.contains_key(name))
        })
}

fn collide(at: Span, message: String, other: Option<(Span, String)>, env: &mut Env<'_>) {
    let mut diag = Diagnostic::error(code::E_ADDED_MEMBER_COLLIDES, message)
        .with_primary(at, "this name is already used")
        .with_help(
            "give the added member a different name. An added member never replaces or \
             overrides another member"
                .to_owned(),
        );
    if let Some((span, label)) = other {
        diag = diag.with_secondary(span, label);
    }
    env.diags.report(diag);
}

/// Where `owner`, a class, interface, enum or block of the program, declares
/// `name` of `kind`, or `None` for a `Core` type, which has no source.
fn member_span(owner: &QName, kind: Kind, name: &str, env: &Env<'_>) -> Option<Span> {
    let at = env.symbols.get(owner)?.decl_span;
    let file = env.files.iter().find(|file| file.src.id() == at.file)?;
    find_member(file.stmts, at, kind, name, file.src)
}

fn find_member(
    stmts: &[nvs_syntax::ast::Stmt],
    at: Span,
    kind: Kind,
    name: &str,
    src: &nvs_diagnostics::SourceFile,
) -> Option<Span> {
    use nvs_syntax::ast::StmtKind;
    stmts.iter().find_map(|stmt| {
        let (decl_name, members, cases) = match &stmt.kind {
            StmtKind::NamespaceDecl(ns) => {
                return find_member(&ns.body.as_ref()?.stmts, at, kind, name, src);
            }
            StmtKind::ClassDecl(d) => (d.name.span, &d.members, &[][..]),
            StmtKind::InterfaceDecl(d) => (d.name.span, &d.members, &[][..]),
            StmtKind::EnumDecl(d) => (d.name.span, &d.members, &d.cases[..]),
            StmtKind::ExtendDecl(d) => (d.name.span, &d.members, &[][..]),
            _ => return None,
        };
        if decl_name != at {
            return None;
        }
        let case = cases
            .iter()
            .find(|case| kind == Kind::Const && span_text(src, case.name.span) == name)
            .map(|case| case.name.span);
        case.or_else(|| {
            members
                .iter()
                .filter_map(|member| added_name(member, src))
                .find(|(k, n, _)| *k == kind && n == name)
                .map(|(_, _, span)| span)
        })
    })
}

/// What `target` is: a class, an interface or an enum, or `None` for a `Core`
/// name that is none of them, such as an attribute.
fn target_kind(target: &QName, env: &Env<'_>) -> Option<SymbolKind> {
    if target.is_core() {
        use nvs_stdlib::registry as core;
        let written = target.to_string();
        let same = |name: &str| name.eq_ignore_ascii_case(&written);
        return if core::ENUMS.iter().any(|e| same(e.name)) {
            Some(SymbolKind::Enum)
        } else if core::INTERFACES.iter().any(|i| same(i.name))
            || core::DERIVE_INTERFACES.iter().any(|name| same(name))
            || core::CONTRACTS.iter().any(|c| same(c.name))
        {
            Some(SymbolKind::Interface)
        } else if core::class(&written).is_some() {
            Some(SymbolKind::Class)
        } else {
            None
        };
    }
    if target.is_reserved_global_interface() {
        return Some(SymbolKind::Interface);
    }
    Some(
        env.symbols
            .get(target)
            .map_or(SymbolKind::Class, |symbol| symbol.kind),
    )
}

/// Whether `target` is declared in the package the block at `at` is in, or,
/// for a block of the application, in the application. `Core`, the reserved
/// global types and a class a loaded extension declares are nobody's own.
fn is_own_type(target: &QName, at: Span, env: &Env<'_>) -> bool {
    if target.is_core()
        || target.is_reserved_global_interface()
        || target.is_reserved_global_class()
    {
        return false;
    }
    let Some(symbol) = env.symbols.get(target) else {
        return false;
    };
    let package_of = |file| env.packages.and_then(|packages| packages.of(file));
    match (package_of(at.file), package_of(symbol.decl_span.file)) {
        (None, None) => true,
        (Some(a), Some(b)) => std::ptr::eq(a, b),
        _ => false,
    }
}

/// Refuses one member a block cannot add. `kind` is the target's kind.
fn check_member(member: &ClassMember, kind: SymbolKind, env: &mut Env<'_>) {
    let (modifiers, what): (&[Modifier], &str) = match &member.kind {
        ClassMemberKind::Method(m) => (&m.modifiers, "method"),
        ClassMemberKind::Property(p) => (&p.modifiers, "property"),
        ClassMemberKind::Const(c) => (&c.modifiers, "constant"),
        ClassMemberKind::TypeAlias(_) => {
            refuse(
                member.span,
                "a `type` alias",
                "declare the alias at file scope",
                env,
            );
            return;
        }
        ClassMemberKind::Enum(_) | ClassMemberKind::Attribute(_) if kind == SymbolKind::Enum => {
            refuse(
                member.span,
                "a nested declaration on an enum",
                "an enum has no nested enum and no nested attribute. Declare it in a class of \
                 your own",
                env,
            );
            return;
        }
        ClassMemberKind::Enum(_) | ClassMemberKind::Attribute(_) | ClassMemberKind::Error => {
            return;
        }
        _ => return,
    };
    for (word, modifier) in [
        ("protected", Modifier::Protected),
        ("abstract", Modifier::Abstract),
        ("final", Modifier::Final),
    ] {
        if modifiers.contains(&modifier) {
            refuse(
                member.span,
                &format!("a `{word}` {what}"),
                "write a `public` or a `private` member. An added member is never inherited \
                 or overridden",
                env,
            );
            return;
        }
    }
    match &member.kind {
        ClassMemberKind::Method(m) if span_text(env.src, m.name) == "constructor" => refuse(
            member.span,
            "a constructor",
            "write a `static` method that creates and returns the object",
            env,
        ),
        // A generator keeps its receiver in a state object. An added static
        // method takes `0` there, and an enum case is an integer.
        ClassMemberKind::Method(m)
            if (kind == SymbolKind::Enum || m.modifiers.contains(&Modifier::Static))
                && m.body
                    .as_ref()
                    .is_some_and(nvs_syntax::ast::is_generator_body) =>
        {
            refuse(
                m.name,
                "a method that uses `yield` here",
                "return an array from the method. An added static method, and an added method \
                 of an enum, cannot use `yield`",
                env,
            );
        }
        ClassMemberKind::Property(p) => check_property(member.span, p, env),
        _ => {}
    }
}

/// Refuses an added property that is `static` or `readonly`, or stored with
/// no value to read before its first write.
fn check_property(at: Span, p: &PropertyMember, env: &mut Env<'_>) {
    if p.modifiers.contains(&Modifier::Static) {
        refuse(
            at,
            "a `static` property",
            "declare a `static` property in a class of your own",
            env,
        );
        return;
    }
    if p.modifiers.contains(&Modifier::Readonly) {
        refuse(
            at,
            "a `readonly` property",
            "remove `readonly`. An added property is never `readonly`",
            env,
        );
        return;
    }
    let computed = is_computed_decl(p);
    let has_start =
        p.default.is_some() || p.modifiers.contains(&Modifier::Lateinit) || is_nullable(&p.ty);
    if !computed && !has_start {
        refuse(
            at,
            "a stored property with no default",
            "give it a default, a `?` type or `lateinit`. An added property has no constructor \
             to set it",
            env,
        );
    }
}

/// Whether an added property is stored: not `static` and not computed. Such a
/// property lives in its object's added slot, so [`crate::layout`] gives the
/// block's target the slot when a block declares one.
pub(crate) fn is_stored_added(p: &PropertyMember) -> bool {
    !p.modifiers.contains(&Modifier::Static) && !is_computed_decl(p)
}

/// Whether an added property is computed: hooks, every one of them `get`.
fn is_computed_decl(p: &PropertyMember) -> bool {
    p.hooks.as_ref().is_some_and(|hooks| {
        !hooks.is_empty() && hooks.iter().all(|hook| hook.kind == PropertyHookKind::Get)
    })
}

/// `E1014` for a stored property added to `target`, whose objects have no
/// added slot (`rule:classes/an-added-property-lives-in-one-hidden-slot`). An
/// interface and an enum never have one. No class takes the slot yet, so a
/// stored property is refused on every type until the runtime gives it.
fn refuse_stored(target: &QName, kind: SymbolKind, p: &PropertyMember, env: &mut Env<'_>) {
    // A `static`, a `readonly` and a property with no start are refused as
    // `E0338` already (`check_property`), one mistake reported once.
    let has_start =
        p.default.is_some() || p.modifiers.contains(&Modifier::Lateinit) || is_nullable(&p.ty);
    if is_computed_decl(p)
        || !has_start
        || p.modifiers.contains(&Modifier::Static)
        || p.modifiers.contains(&Modifier::Readonly)
    {
        return;
    }
    let what = match kind {
        SymbolKind::Interface => "an interface",
        SymbolKind::Enum => "an enum",
        _ => "this class",
    };
    env.diags.report(
        Diagnostic::error(
            code::E_ADDED_STORED_PROPERTY_NO_SLOT,
            format!("`{target}` cannot store a property an `extend` block adds"),
        )
        .with_primary(p.name, format!("{what} has no place for an added value"))
        .with_help(
            "give the property only a `get` hook, so it computes its value on each read. To keep \
             data with the value, write a class of your own with a property for each"
                .to_owned(),
        ),
    );
}

/// Whether `ty` takes `null`: `?T`, or a union with `null` in it.
fn is_nullable(ty: &Type) -> bool {
    match &ty.kind {
        TypeKind::Nullable(_) => true,
        TypeKind::Union(parts) => parts.iter().any(is_nullable),
        TypeKind::Paren(inner) => is_nullable(inner),
        TypeKind::Atom(TypeAtom::Null) => true,
        _ => false,
    }
}

/// The names a block on a generic `Core` class binds its type parameters to,
/// in order: each `<…>` entry written as one bare name. Anything else is left
/// out, and [`check_type_params`] reports it.
pub(crate) fn type_param_names(
    decl: &ExtendDecl,
    src: &nvs_diagnostics::SourceFile,
) -> Vec<String> {
    decl.type_params
        .iter()
        .filter_map(|ty| match &ty.kind {
            TypeKind::Atom(TypeAtom::Name(name, args)) if args.is_empty() => {
                let text = span_text(src, name.span);
                (!text.contains('\\')).then(|| text.to_owned())
            }
            _ => None,
        })
        .collect()
}

/// `E0338` for a `<…>` list that does not bind a generic `Core` class's
/// parameters: a list on a class that is not generic, a list of the wrong
/// length, no list on a generic class, and an entry that is not one bare
/// name. A block of one instance, such as `Core\ObjectMap<int, string>`, is
/// the last case.
fn check_type_params(decl: &ExtendDecl, target: &QName, env: &mut Env<'_>) {
    let params = nvs_stdlib::registry::class_type_params(&target.to_string()).unwrap_or(&[]);
    let names = type_param_names(decl, env.src);
    let written = decl.type_params.len();
    // A scalar such as `int` is not a name, so `Core\ObjectMap<Tag, int>` is
    // refused here. A bare name is always a new one, inside the block only.
    if written == params.len() && names.len() == written {
        return;
    }
    let list = params.join(", ");
    let help = if params.is_empty() {
        format!("`{target}` has no type parameters. Remove the `<…>` list")
    } else {
        format!(
            "write `extend {target}<{list}> as Name {{ … }}`. Each name in the list is a new \
             name for one type parameter, and a call fills it in from the value's type"
        )
    };
    let at = match (decl.type_params.first(), decl.type_params.last()) {
        (Some(first), Some(last)) => first.span.to(last.span),
        _ => decl.target.span,
    };
    env.diags.report(
        Diagnostic::error(
            code::E_EXTEND_MEMBER_NOT_ADDABLE,
            format!("this `extend` block does not name the type parameters of `{target}`"),
        )
        .with_primary(at, "the type parameters are not named here")
        .with_help(help),
    );
}

/// `imports` with the block `block` added under its short name: what a
/// block's own body sees, since its members are visible inside it without a
/// `use`. Cost: one copy of the file's `use` map per block, at compile time.
pub(crate) fn imports_with_block(
    imports: &FxHashMap<String, QName>,
    block: &QName,
) -> FxHashMap<String, QName> {
    let mut imports = imports.clone();
    imports.insert(block.short_name().to_owned(), block.clone());
    imports
}

/// The blocks `imports` names that add members to `qname`: a block whose
/// target is `qname` or one of its ancestors. A file reaches an added member
/// only through a block its `use` list names.
pub(crate) fn used_blocks<'a>(
    qname: &'a QName,
    imports: &'a FxHashMap<String, QName>,
    graph: &'a ClassGraph,
) -> impl Iterator<Item = &'a QName> + 'a {
    imports.values().filter(move |block| {
        graph
            .block_target(block)
            .is_some_and(|target| reaches(qname, target, graph))
    })
}

/// Whether `qname` is `target` or has it among its ancestors.
fn reaches(qname: &QName, target: &QName, graph: &ClassGraph) -> bool {
    qname == target
        || nvs_hir::find_in_ancestors(qname, graph, |class| (class == target).then_some(()))
            .is_some()
}

/// The method `name` a block used in this file adds to `qname`, with the block
/// that adds it. The type's own members are looked up first; this is the
/// fallback when none of them has the name.
pub(crate) fn added_method<'a>(
    qname: &'a QName,
    name: &str,
    imports: &'a FxHashMap<String, QName>,
    table: &'a SignatureTable,
    graph: &'a ClassGraph,
) -> Option<(&'a QName, &'a MethodSig)> {
    used_blocks(qname, imports, graph)
        .find_map(|block| Some((block, table.get(block)?.methods.get(name)?)))
}

/// The block used in this file that adds the constant `name` to `qname`.
pub(crate) fn added_const<'a>(
    qname: &'a QName,
    name: &str,
    imports: &'a FxHashMap<String, QName>,
    table: &'a SignatureTable,
    graph: &'a ClassGraph,
) -> Option<&'a QName> {
    used_blocks(qname, imports, graph).find(|block| {
        table
            .get(block)
            .is_some_and(|sig| sig.constants.contains_key(name))
    })
}

/// The property `name` a block used in this file adds to `qname`, with the
/// block and its type. Like [`added_method`], it is the fallback when the
/// type's own properties do not have the name.
pub(crate) fn added_property<'a>(
    qname: &'a QName,
    name: &str,
    imports: &'a FxHashMap<String, QName>,
    table: &'a SignatureTable,
    graph: &'a ClassGraph,
) -> Option<(&'a QName, crate::ty::TypeId)> {
    used_blocks(qname, imports, graph)
        .find_map(|block| Some((block, *table.get(block)?.properties.get(name)?)))
}

/// Whether the property `name` that `block` adds is computed: it has a `get`
/// hook and no other hook, so it has no stored value.
pub(crate) fn is_computed(block: &QName, name: &str, table: &SignatureTable) -> bool {
    let hooks = crate::signatures::hooks_of(block, name, table);
    hooks.get && !hooks.set
}

/// The block in the program that adds the member `name` to `qname`, for the
/// help of an unknown member: a block the file does not use.
pub(crate) fn unused_block_adding(
    qname: &QName,
    name: &str,
    table: &SignatureTable,
    graph: &ClassGraph,
) -> Option<QName> {
    // The first by name, so the help does not depend on the map's order.
    graph
        .blocks()
        .filter(|(block, target)| {
            reaches(qname, target, graph)
                && table.get(block).is_some_and(|sig| {
                    sig.methods.contains_key(name)
                        || sig.constants.contains_key(name)
                        || sig.properties.contains_key(name)
                })
        })
        .map(|(block, _)| block.to_string())
        .min()
        .map(|block| QName::parse(&block))
}

fn refuse(at: Span, what: &str, help: &str, env: &mut Env<'_>) {
    env.diags.report(
        Diagnostic::error(
            code::E_EXTEND_MEMBER_NOT_ADDABLE,
            format!("an `extend` block cannot add {what}"),
        )
        .with_primary(at, "not allowed in an `extend` block")
        .with_help(help.to_owned()),
    );
}
