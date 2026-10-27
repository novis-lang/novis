//! The keystone test of `rule:security/taint-propagation`: no `Core` signature loses a mark.
//!
//! The checker carries `tainted` from an argument to a result in two ways. A parameter classified
//! [`Qual::Contagious`] or [`Qual::Reveal`] makes the whole result tainted, and a type variable
//! carries the bound type, mark included, to wherever the return writes it. What neither covers is
//! a plain `string` or `bytes` in the return of a member whose only way in for outside text is a
//! parameter the registry does not classify: an `array<T>`, `mixed`, an `object` or a variable.
//! `Core\Arr::keys` was that shape, and its keys came back plain from a tainted array.
//!
//! Fixing such members one at a time is how the bug got in, so this test reads every row and is
//! the list. A member it names either carries the mark, or is written in [`ALLOWED`] with the rule
//! that says why its plain answer is right.

use nvs_stdlib::registry::{CLASSES, CoreMethod, CoreTy, Qual};

/// The members that answer plain text from a parameter that can carry outside text, each with the
/// rule that says why. Two kinds stand here. A launderer encodes what it is handed for one sink
/// (`rule:security/launderers-are-sink-named`). An encoder's row says `mixed`, and the checker
/// carries the mark at the call from the argument's own type: `nvs_types::expr::quals`'
/// `encodes_outside_text` is that list, and a member added here as an encoder is added there.
const ALLOWED: &[(&str, &str, &str)] = &[
    (r"Core\Json", "encode", "rule:security/taint-propagation"),
    (
        r"Core\Serialize",
        "encode",
        "rule:security/taint-propagation",
    ),
    (r"Core\Str", "format", "rule:security/taint-propagation"),
    (r"Core\Bytes", "pack", "rule:security/taint-propagation"),
    (r"Core\Uri", "buildQuery", "rule:security/taint-propagation"),
    (
        r"Core\Router",
        "url",
        "rule:routing/link-target-and-params-are-checked",
    ),
    (
        r"Core\Router",
        "urlAbsolute",
        "rule:routing/link-target-and-params-are-checked",
    ),
    (
        r"Core\Router",
        "urlSigned",
        "rule:routing/link-target-and-params-are-checked",
    ),
    (r"Core\Jwt", "signObject", "rule:security/jws-issued-subset"),
];

/// Whether a parameter of this type can be handed a value carrying `tainted` without the registry
/// classifying it: a container or a variable whose contents the row does not describe.
fn holds_outside_text(ty: &CoreTy) -> bool {
    match ty {
        CoreTy::Mixed
        | CoreTy::Object
        | CoreTy::Var(_)
        | CoreTy::Bounded(..)
        | CoreTy::Written(_)
        | CoreTy::WrittenReturning(..) => true,
        CoreTy::Array(elem) => {
            !matches!(ty.classification(), Some(Qual::Contagious)) && holds_outside_text(elem)
        }
        CoreTy::Iterated(elem) | CoreTy::Nullable(elem) | CoreTy::Variadic(elem) => {
            holds_outside_text(elem)
        }
        CoreTy::Union(members) => members.iter().any(holds_outside_text),
        CoreTy::InstanceAt(_, args) => args.iter().any(holds_outside_text),
        CoreTy::Shape(arms) => arms
            .iter()
            .any(|fields| fields.iter().any(|field| holds_outside_text(&field.ty))),
        CoreTy::Options(options) => options.iter().any(|option| holds_outside_text(&option.ty)),
        _ => false,
    }
}

/// Whether this return type has a plain `string` or `bytes` anywhere a caller can read it.
fn answers_plain_text(ty: &CoreTy) -> bool {
    match ty {
        CoreTy::Str | CoreTy::Bytes => true,
        CoreTy::Array(elem) | CoreTy::Iterated(elem) | CoreTy::Nullable(elem) => {
            answers_plain_text(elem)
        }
        CoreTy::Union(members) => members.iter().any(answers_plain_text),
        CoreTy::InstanceAt(_, args) => args.iter().any(answers_plain_text),
        CoreTy::Shape(arms) => arms
            .iter()
            .any(|fields| fields.iter().any(|field| answers_plain_text(&field.ty))),
        _ => false,
    }
}

/// Whether a callback parameter is handed plain text while the member walks outside text: the key
/// of an array, which the array's type says nothing about.
fn calls_back_with_plain_text(ty: &CoreTy) -> bool {
    match ty {
        CoreTy::CallableSig(params, _) => params.iter().any(answers_plain_text),
        CoreTy::Nullable(inner) => calls_back_with_plain_text(inner),
        CoreTy::Union(members) => members.iter().any(calls_back_with_plain_text),
        CoreTy::Options(options) => options
            .iter()
            .any(|option| calls_back_with_plain_text(&option.ty)),
        _ => false,
    }
}

/// What is wrong with this row, or `None` when every mark it can be handed reaches its answer.
fn lost_mark(member: &CoreMethod) -> Option<&'static str> {
    if !member.params.iter().any(holds_outside_text) {
        return None;
    }
    if answers_plain_text(&member.return_ty) {
        return Some("answers plain text");
    }
    if member.params.iter().any(calls_back_with_plain_text) {
        return Some("hands its callback plain text");
    }
    None
}

#[test]
fn every_core_signature_keeps_a_tainted_input_s_mark() {
    let mut lost = Vec::new();
    for class in CLASSES {
        for member in class.members() {
            let Some(why) = lost_mark(member) else {
                continue;
            };
            let allowed = ALLOWED
                .iter()
                .any(|(name, found, _)| *name == class.name && *found == member.name);
            if !allowed {
                lost.push(format!("`{}::{}` {why}", class.name, member.name));
            }
        }
    }
    assert!(
        lost.is_empty(),
        "each member below can be handed outside text and answers it without `tainted`. Carry the \
         mark into its return, or name it in `ALLOWED` with the rule that makes it one \
         (rule:security/launderers-are-sink-named):\n{}",
        lost.join("\n")
    );
}

#[test]
fn every_allowed_row_names_a_member_and_a_rule() {
    for (class_name, member, rule) in ALLOWED {
        let class = CLASSES
            .iter()
            .find(|class| class.name == *class_name)
            .unwrap_or_else(|| panic!("`{class_name}` is not a Core class"));
        let found = class
            .members()
            .find(|found| found.name == *member)
            .unwrap_or_else(|| panic!("`{class_name}::{member}` is not a member of that class"));
        assert!(
            lost_mark(found).is_some(),
            "`{class_name}::{member}` keeps every mark now, so its `ALLOWED` row is stale"
        );
        assert!(
            rule.starts_with("rule:"),
            "`{class_name}::{member}`'s row names `{rule}`, which is not a rule id"
        );
    }
}
