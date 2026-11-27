//! `rule:classes/a-compared-property-name-is-a-use`'s warning: a string a
//! property callback compares with its name parameter, where no property of
//! the callback's class has that name.
//!
//! [`nvs_hir::compared_names`] reads which strings a callback compares. This
//! module raises [`code::W_COMPARED_NAME_IS_NO_PROPERTY`] at each one that no
//! property of the class, of a class above it or of a class below it has. It
//! is a warning and never an error, because a class below may be declared in
//! another program. A callback that writes its name parameter is not read,
//! since a comparison after the write may not compare a property's name.
//!
//! **What it spends:** checking time only. The property names of the class's
//! lineage are collected once per class, and only when it has a callback that
//! compares a name.

use nvs_diagnostics::{Diagnostic, code};
use nvs_hir::QName;
use nvs_hir::hierarchy::{descendants, find_in_ancestors, implements_interface};
use nvs_syntax::ast::ClassDecl;
use rustc_hash::FxHashSet;

use crate::Env;

/// Warns at each name a callback `decl` declares compares with its name
/// parameter that no property of the class's lineage has.
pub(crate) fn check_compared_names(decl: &ClassDecl, qname: &QName, env: &mut Env<'_>) {
    let source = env.src.text();
    let callbacks = nvs_hir::compared_names::callbacks_in(&decl.members, source);
    let mut known: Option<FxHashSet<String>> = None;
    let mut unknown = Vec::new();
    for (_, interface, names) in callbacks {
        if names.assigned.is_some()
            || !implements_interface(qname, &QName::parse(interface), env.graph)
        {
            continue;
        }
        for span in names.compared {
            let name = source.get(span.range()).unwrap_or_default();
            let known = known.get_or_insert_with(|| properties_of_lineage(qname, env));
            if !known.contains(name) {
                unknown.push((span, name.to_owned()));
            }
        }
    }
    for (span, name) in unknown {
        env.diags.report(
            Diagnostic::warning(
                code::W_COMPARED_NAME_IS_NO_PROPERTY,
                format!("`'{name}'` is not a property of `{qname}`"),
            )
            .with_primary(span, "no class above or below it has this property either")
            .with_help("check the spelling of the property name"),
        );
    }
}

/// The name of every property `qname`, a class above it and a class below it
/// declare, without the `$`.
fn properties_of_lineage(qname: &QName, env: &Env<'_>) -> FxHashSet<String> {
    let mut names = FxHashSet::default();
    let mut add = |class: &QName| {
        if let Some(signature) = env.signatures.get(class) {
            names.extend(signature.properties.keys().cloned());
        }
    };
    find_in_ancestors(qname, env.graph, |class| {
        add(class);
        None::<()>
    });
    for below in descendants(qname, env.graph) {
        add(&below);
    }
    names
}
