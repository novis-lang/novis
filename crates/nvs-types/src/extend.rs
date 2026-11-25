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
