//! `rule:enums/an-enum-lives-in-the-class-that-uses-it`, held against the
//! registry: an enum exactly one `Core` class takes, returns or holds is
//! declared in that class, and an enum several classes use, or none, stays in
//! its namespace.
//!
//! A class *uses* an enum when a parameter, an option, a field of a settings
//! shape, a return type or a constant of one of its members names it. An enum no class names is one an
//! attribute takes — `Core\Http\Method` through `#[Route]`, `Core\Audience`
//! through `#[Access]` — and that has no class to live in.
//!
//! An error class has no `CLASSES` row, so it uses an enum through an own
//! property, and [`ERROR_PROPERTIES`] lists each one. The property's type is
//! seeded in `nvs_types::error_lib`, which this crate cannot reach, so the
//! table restates it and its own test holds it against
//! `nvs_hir::errors::OWN_PROPERTIES` and [`ENUMS`].

use nvs_stdlib::registry::{CLASSES, Const, CoreEnum, CoreTy, ENUMS};

/// Every error class's own property that holds a `Core` enum: the class, the
/// property and the enum.
const ERROR_PROPERTIES: &[(&str, &str, &str)] = &[
    (r"Core\Db\DbError", "kind", r"Core\Db\DbError::Kind"),
    (r"Core\Ldap\LdapError", "kind", r"Core\Ldap\LdapError::Kind"),
];

/// Every enum `ty` names, appended to `into`.
fn walk(ty: &CoreTy, into: &mut Vec<&'static str>) {
    match ty {
        CoreTy::Enum(name) | CoreTy::EnumCase(name, _) => into.push(name),
        CoreTy::InstanceAt(_, args) | CoreTy::Union(args) => {
            args.iter().for_each(|arg| walk(arg, into));
        }
        CoreTy::Array(inner)
        | CoreTy::Nullable(inner)
        | CoreTy::Variadic(inner)
        | CoreTy::Iterated(inner) => walk(inner, into),
        CoreTy::CallableSig(params, ret) => {
            params.iter().for_each(|param| walk(param, into));
            walk(ret, into);
        }
        CoreTy::Shape(arms) => {
            for field in arms.iter().flat_map(|arm| arm.iter()) {
                walk(&field.ty, into);
                if let Some(Const::EnumCase(name, _)) = field.default {
                    into.push(name);
                }
            }
        }
        CoreTy::Options(options) => {
            for option in *options {
                walk(&option.ty, into);
                if let Const::EnumCase(name, _) = option.default {
                    into.push(name);
                }
            }
        }
        _ => {}
    }
}

/// The classes that use `core`, sorted.
fn users(core: &CoreEnum) -> Vec<&'static str> {
    let mut found: Vec<&'static str> = CLASSES
        .iter()
        .filter(|class| {
            let mut named = Vec::new();
            for member in class.members() {
                member
                    .params
                    .iter()
                    .for_each(|param| walk(param, &mut named));
                walk(&member.return_ty, &mut named);
            }
            for constant in class.constants {
                walk(&constant.ty, &mut named);
                if let Const::EnumCase(name, _) = constant.value {
                    named.push(name);
                }
            }
            named.contains(&core.name)
        })
        .map(|class| class.name)
        .collect();
    found.extend(
        ERROR_PROPERTIES
            .iter()
            .filter(|(_, _, named)| *named == core.name)
            .map(|(class, _, _)| *class),
    );
    found.sort_unstable();
    found
}

/// The last segment of a class name: `Arr` for `Core\Arr`.
fn short(class: &str) -> &str {
    class.rsplit('\\').next().unwrap_or(class)
}

#[test]
fn every_error_property_names_a_declared_property_and_a_registered_enum() {
    for (class, property, named) in ERROR_PROPERTIES {
        assert!(
            nvs_hir::errors::own_properties(class).contains(property),
            "`{class}` declares no own property `{property}`"
        );
        assert!(
            ENUMS.iter().any(|core| core.name == *named),
            "`{named}` is not a registered enum"
        );
    }
}

#[test]
fn every_core_enum_one_class_uses_is_owned_by_it() {
    let mut misplaced = Vec::new();
    for core in ENUMS {
        if let [only] = users(core).as_slice()
            && core.owner() != Some(*only)
        {
            misplaced.push(format!("{} is used by `{only}` alone", core.name));
        }
    }
    assert!(
        misplaced.is_empty(),
        "declare each in its class:\n{}",
        misplaced.join("\n")
    );
}

#[test]
fn every_owned_core_enum_is_used_by_its_owner() {
    let mut wrong = Vec::new();
    for core in ENUMS {
        let Some(owner) = core.owner() else {
            continue;
        };
        let users = users(core);
        if users != [owner] {
            wrong.push(format!(
                "{} is owned by `{owner}` and used by {users:?}: an enum several classes use, \
                 or none, is declared in its namespace",
                core.name
            ));
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

#[test]
fn no_owned_core_enum_repeats_its_owners_word() {
    let mut repeated = Vec::new();
    for core in ENUMS {
        let Some(owner) = core.owner() else {
            continue;
        };
        if core.short_name().contains(short(owner)) {
            repeated.push(core.name);
        }
    }
    assert!(
        repeated.is_empty(),
        "an owned enum drops its owner's word from its name: {repeated:?}"
    );
}
