//! An `extend` block's shape: what its target may be, and which members it
//! may add (`rule:classes/an-extend-block-adds-members-to-a-type`).
//!
//! `nvs_hir::hierarchy` resolves the target and refuses a declared name of the
//! wrong kind. What is left here needs what only this crate holds: the `Core`
//! registry, to tell a `Core` class, interface or enum from a `Core`
//! attribute or namespace, and the [`crate::package::Packages`] of the
//! program, to refuse a block of a type its own package declares. Each
//! refused member is one [`code::E_EXTEND_MEMBER_NOT_ADDABLE`] at the member,
//! and the rest of the block is still checked.
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
    for member in &decl.members {
        check_member(member, kind, env);
    }
    check_collisions(decl, &block, &target, env);
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
    let computed = p.hooks.as_ref().is_some_and(|hooks| {
        !hooks.is_empty() && hooks.iter().all(|hook| hook.kind == PropertyHookKind::Get)
    });
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
                    sig.methods.contains_key(name) || sig.constants.contains_key(name)
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
