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
        r"Core\Json",
        "encodeEach",
        "rule:security/taint-propagation",
    ),
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
        CoreTy::Iterated(elem)
        | CoreTy::Nullable(elem)
        | CoreTy::Variadic(elem)
        | CoreTy::Dispatched(elem)
        | CoreTy::Keys(_, elem) => holds_outside_text(elem),
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

/// Where the keys of a member's answer come from, when no parameter carries the key mark
/// ([`CoreTy::Keys`]).
#[derive(Clone, Copy)]
enum KeysFrom {
    /// The answer is a list: its keys are positions, `"0"`, `"1"` and on.
    Positions,
    /// The keys are copied from an array argument's own keys, which were checked when they were
    /// written.
    Subject,
    /// The keys are read out of text the member parses: an argument, the request or the
    /// configuration. A text parameter refuses a `secret` argument unless it is a `Reveal` one, and
    /// outside text makes the answer's elements `tainted` or `mixed`, so the checker reads the keys
    /// as outside text too.
    Text,
}

/// The members that return an array without a key mark, each with where its keys come from.
const KEYS_FROM: &[(&str, &str, KeysFrom)] = {
    use KeysFrom::{Positions, Subject, Text};
    &[
        (r"Core\Str", "split", Positions),
        (r"Core\Str", "chunk", Positions),
        (r"Core\Str", "lines", Positions),
        (r"Core\Str", "graphemes", Positions),
        (r"Core\Str", "codePoints", Positions),
        (r"Core\Arr", "filter", Subject),
        (r"Core\Arr", "map", Subject),
        (r"Core\Arr", "keys", Positions),
        (r"Core\Arr", "values", Positions),
        (r"Core\Arr", "slice", Subject),
        (r"Core\Arr", "replaceRange", Subject),
        (r"Core\Arr", "chunk", Subject),
        (r"Core\Arr", "append", Subject),
        (r"Core\Arr", "prepend", Subject),
        (r"Core\Arr", "withoutFirst", Subject),
        (r"Core\Arr", "withoutLast", Subject),
        (r"Core\Arr", "withoutNull", Subject),
        (r"Core\Arr", "padStart", Subject),
        (r"Core\Arr", "padEnd", Subject),
        (r"Core\Arr", "reverse", Subject),
        (r"Core\Arr", "flatten", Positions),
        (r"Core\Arr", "flattenDeep", Positions),
        (r"Core\Arr", "sort", Subject),
        (r"Core\Arr", "sortByKey", Subject),
        (r"Core\Arr", "fill", Positions),
        (r"Core\Arr", "from", Positions),
        (r"Core\Arr", "overlay", Subject),
        (r"Core\Arr", "overlayDeep", Subject),
        (r"Core\Arr", "underlay", Subject),
        (r"Core\Arr", "appendAll", Subject),
        (r"Core\Arr", "diff", Subject),
        (r"Core\Arr", "intersect", Subject),
        (r"Core\Arr", "unique", Subject),
        (r"Core\Attributes", "all", Positions),
        (r"Core\Regex", "matchAll", Positions),
        (r"Core\Regex", "split", Positions),
        (r"Core\Bytes", "unpack", Positions),
        (r"Core\Path", "split", Positions),
        (r"Core\IO", "list", Positions),
        (r"Core\Random", "sample", Positions),
        (r"Core\Random", "shuffle", Positions),
        (r"Core\Random\Seeded", "sample", Positions),
        (r"Core\Random\Seeded", "shuffle", Positions),
        (r"Core\Uri", "parseQuery", Text),
        (r"Core\Router", "methodsFor", Positions),
        (r"Core\Csv", "parse", Text),
        (r"Core\Task", "map", Subject),
        (r"Core\Program", "implementingWith", Positions),
        (r"Core\Cli", "multiSelect", Positions),
        (r"Core\Config", "setting", Text),
        (r"Core\Request", "queryArray", Text),
        (r"Core\Request", "postArray", Text),
        (r"Core\Storage", "list", Positions),
        (r"Core\Jwt", "verify", Text),
        (r"Core\Signature", "verify", Text),
        (r"Core\Zip", "entries", Positions),
        (r"Core\Http\Response", "headers", Positions),
        (r"Core\Http\Stream", "headers", Positions),
        (r"Core\Db\Rows", "column", Positions),
        (r"Core\Ldap\Entry", "strings", Positions),
        (r"Core\Ldap\Entry", "sids", Positions),
    ]
};

/// Whether this type, or any type inside it, carries the key mark.
fn marks_keys(ty: &CoreTy) -> bool {
    match ty {
        CoreTy::Keys(..) => true,
        CoreTy::Array(inner)
        | CoreTy::Iterated(inner)
        | CoreTy::Nullable(inner)
        | CoreTy::Variadic(inner)
        | CoreTy::Dispatched(inner)
        | CoreTy::Alias(_, inner) => marks_keys(inner),
        CoreTy::CallableSig(params, ret) => params.iter().any(marks_keys) || marks_keys(ret),
        CoreTy::Union(members) | CoreTy::InstanceAt(_, members) => members.iter().any(marks_keys),
        CoreTy::Options(options) => options.iter().any(|option| marks_keys(&option.ty)),
        _ => false,
    }
}

/// Whether a parameter of this type can hand the member values it could make keys of: a type
/// variable, text or a callable, at any depth.
fn gives_key_values(ty: &CoreTy) -> bool {
    match ty {
        CoreTy::Var(_)
        | CoreTy::Bounded(..)
        | CoreTy::Written(_)
        | CoreTy::WrittenReturning(..)
        | CoreTy::Callable
        | CoreTy::CallableSig(..)
        | CoreTy::Mixed => true,
        _ if ty.is_text_like() => true,
        CoreTy::Array(inner)
        | CoreTy::Iterated(inner)
        | CoreTy::Nullable(inner)
        | CoreTy::Variadic(inner)
        | CoreTy::Dispatched(inner)
        | CoreTy::Keys(_, inner)
        | CoreTy::Alias(_, inner) => gives_key_values(inner),
        CoreTy::Union(members) | CoreTy::InstanceAt(_, members) => {
            members.iter().any(gives_key_values)
        }
        CoreTy::Options(options) => options.iter().any(|option| gives_key_values(&option.ty)),
        _ => false,
    }
}

/// Whether this return type is an array, whose keys a program reads back as plain text.
fn returns_an_array(ty: &CoreTy) -> bool {
    match ty {
        CoreTy::Array(_) => true,
        CoreTy::Nullable(inner) | CoreTy::Alias(_, inner) => returns_an_array(inner),
        CoreTy::Union(members) => members.iter().any(returns_an_array),
        _ => false,
    }
}

/// `rule:security/taint-propagation`'s key mark: a member that turns values into keys says so
/// on the position those values come from, and the checker refuses a `secret` one there. This
/// test is the list, so a new member cannot skip the question.
#[test]
fn every_member_that_returns_a_plain_keyed_array_says_where_its_keys_come_from() {
    let mut open = Vec::new();
    for class in CLASSES {
        for member in class.members() {
            if !returns_an_array(&member.return_ty)
                || !member.params.iter().any(gives_key_values)
                || member.params.iter().any(marks_keys)
            {
                continue;
            }
            let answered = KEYS_FROM
                .iter()
                .any(|(name, found, _)| *name == class.name && *found == member.name);
            if !answered {
                let params: Vec<String> = member.params.iter().map(CoreTy::spelled).collect();
                open.push(format!(
                    "{}::{}({}): {}",
                    class.name,
                    member.name,
                    params.join(", "),
                    member.return_ty.spelled()
                ));
            }
        }
    }
    assert!(
        open.is_empty(),
        "each member below returns an array and does not say where its keys come from. Mark the \
         parameter whose values become keys with `CoreTy::Keys`, or name the member in \
         `KEYS_FROM`:\n{}",
        open.join("\n")
    );
    for (class_name, member, _) in KEYS_FROM {
        let found = CLASSES
            .iter()
            .find(|class| class.name == *class_name)
            .and_then(|class| class.members().find(|found| found.name == *member));
        let found = found
            .unwrap_or_else(|| panic!("`{class_name}::{member}` in `KEYS_FROM` is not a member"));
        assert!(
            !found.params.iter().any(marks_keys),
            "`{class_name}::{member}` carries the key mark now, so its `KEYS_FROM` row is stale"
        );
    }
}

/// `rule:security/sink-predicate` over `Core\Reflect`: text that picks a class, a method, a
/// property, a constant or a case by name is an instruction, so the parameter is a sink. A member
/// that returns a `bool` only checks that the name exists and runs nothing, so it may stay neutral.
#[test]
fn every_reflection_member_that_picks_by_name_refuses_tainted_text() {
    let mut open = Vec::new();
    for class in CLASSES
        .iter()
        .filter(|class| class.name.starts_with(r"Core\Reflect"))
    {
        for member in class.members() {
            if matches!(member.return_ty, CoreTy::Bool) {
                continue;
            }
            for (index, param) in member.params.iter().enumerate() {
                let picks_by_text = matches!(param, CoreTy::Text(_) | CoreTy::ClassName(_));
                if picks_by_text && !matches!(param.classification(), Some(Qual::Sink)) {
                    let name = member.names.get(index).copied().unwrap_or("?");
                    open.push(format!("{}::{}(${name})", class.name, member.name));
                }
            }
        }
    }
    assert!(
        open.is_empty(),
        "each parameter below picks code by its text and is not `Qual::Sink`. Make it a sink, or \
         return a `bool` that runs nothing:\n{}",
        open.join("\n")
    );
}
