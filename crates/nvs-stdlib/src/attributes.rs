//! `Core\Attributes` — `rule:attributes/structural-retrieval` and `rule:attributes/retrieval-folds-while-checking`'s structural retrieval, and the one `Core` class whose members never
//! run.
//!
//! Every other class in this crate registers a signature *and* an
//! implementation. This one registers a signature and nothing else, because §
//! 5 says what the members do: a declaration's attached-attribute list is
//! fully static and every payload is already a compile-time literal
//! (§ 2), so `nvs check` **replaces the call with its answer** — a compiled-in
//! `null`, the matched literal itself, or the array of them. There is no
//! lookup left for a helper to perform, and `nvs_types::attributes` is where
//! the answer is computed.
//!
//! The two symbols below therefore name a body that exists only so
//! [`crate::symbols`] can hand the JIT an address for every registered member
//! without a second rule about which rows are exempt. Reaching one is a
//! compiler bug — `nvs-ir` lowers the call to the folded constant, so no
//! `InstKind::CoreCall` naming either symbol is ever emitted — and the body
//! says so and aborts rather than answering plausibly.
//!
//! # Why `$member` is one optional parameter rather than two overloads
//!
//! § 4 writes each member twice, once with a `$member` name and once without.
//! The registry has no overloading and wants none: what the two spellings
//! differ in is *which* declaration is named, which is one argument's worth of
//! information. So the parameter is a single `string` defaulting to the empty
//! string, which is not a member name any declaration can have — `rule:core-api/identifier-casing`'s
//! casing rule needs at least one character — and so is the one value that can
//! mean "the target itself" without shadowing a real name.

use crate::registry::{Const, CoreClass, CoreMethod, CoreTy, MethodDoc, ParamDoc, Qual};

/// This class's fully-qualified name, in one place so the registry row and
/// every consumer that matches on it cannot drift apart.
pub(crate) const NAME: &str = "Core\\Attributes";

/// `T` — the shape a retrieval is asked for, written at the call site.
/// [`CoreTy::Written`] owns why a variable appearing in no parameter position
/// has to be supplied there, which is `rule:attributes/call-site-type-argument`'s whole subject.
const T: CoreTy = CoreTy::Written("T");

/// `$member` — the optional declaration name both members take, carrying
/// `rule:security/unclassified-parameter-refuses-tainted`
/// 's classification.
///
/// [`Qual::Neutral`] by the first bullet of [`Qual`]'s own rule, which is where
/// that rule is written: neither member's answer can carry a byte of either
/// argument. What comes back is a payload literal read out of a *declaration*
/// — § 5 folds the call to that constant before anything runs — so the name
/// selects which declaration is read rather than flowing into what is read.
const MEMBER: CoreTy = CoreTy::Text(Qual::Neutral);

/// The registry row. See [`crate::registry::CLASSES`].
pub(crate) const CLASS: CoreClass = CoreClass {
    name: NAME,
    doc: None,
    methods: &[
        CoreMethod {
            name: "get",
            names: &["target", "member"],
            params: &[CoreTy::Callable, MEMBER],
            defaults: &[Const::Str("")],
            return_ty: CoreTy::Nullable(&T),
            symbol: "nvs_core_attributes_get",
            doc: Some(&GET_DOC),
        },
        CoreMethod {
            name: "all",
            names: &["target", "member"],
            params: &[CoreTy::Callable, MEMBER],
            defaults: &[Const::Str("")],
            return_ty: CoreTy::Array(&T),
            symbol: "nvs_core_attributes_all",
            doc: Some(&ALL_DOC),
        },
    ],
    instance: &[],
    slots: &[],
    constants: &[],
};

/// `$target`, documented once — both members take the same one.
const TARGET_DOC: ParamDoc = ParamDoc {
    name: "target",
    desc: "The declaration whose attributes are read — a class or function reference.",
    shape: &[],
};

/// See [`TARGET_DOC`].
const MEMBER_DOC: ParamDoc = ParamDoc {
    name: "member",
    desc: "The name of a member of `$target` to read instead of `$target` itself; the empty \
           string, which is the default, means the target.",
    shape: &[],
};

/// `Core\Attributes::get`'s reference card — `rule:core-api/reference-card`.
const GET_DOC: MethodDoc = MethodDoc {
    short: "Answers the one attribute attached to `$target` — or to its member `$member` — whose \
            literal structurally satisfies the shape `T` written at the call site, resolved in \
            `nvs check` so that the call is replaced by its answer and nothing runs.",
    params: &[TARGET_DOC, MEMBER_DOC],
    ret: "The matching attribute's payload literal as `T`, or `null` when none satisfies `T`; \
          more than one is a compile error naming `all<T>` as the fix. Matching is width \
          subtyping, so the empty shape `{}` is satisfied by every attached literal: a marker \
          type with no fields asks for any attribute at all, and beside a second attribute it is \
          that compile error rather than the marker.",
    errors: &[],
};

/// `Core\Attributes::all`'s reference card — `rule:core-api/reference-card`.
const ALL_DOC: MethodDoc = MethodDoc {
    short: "Answers every attribute attached to `$target` — or to its member `$member` — whose \
            literal structurally satisfies the shape `T` written at the call site, resolved in \
            `nvs check` so that the call is replaced by its answer and nothing runs.",
    params: &[TARGET_DOC, MEMBER_DOC],
    ret: "An `array<T>` of the matching payload literals in declaration order, empty when none \
          satisfies `T`. Matching is width subtyping, so the empty shape `{}` answers every \
          attached literal rather than the markers among them.",
    errors: &[],
};

/// The address of one of *this* module's symbols, or `None` for a symbol that
/// belongs to another domain. See [`crate::symbols`].
pub(crate) fn address(symbol: &str) -> Option<*const u8> {
    Some(match symbol {
        "nvs_core_attributes_get" | "nvs_core_attributes_all" => {
            (folded_at_compile_time as *const ()).cast()
        }
        _ => return None,
    })
}

/// The body both rows name, and the one this crate hopes is never entered:
/// `rule:attributes/retrieval-folds-while-checking` resolves a retrieval in `nvs check`, so a call reaching a
/// helper means `nvs-ir` lowered one it should have replaced.
extern "C" fn folded_at_compile_time() {
    // Written through the handle rather than with `eprintln!`, which this
    // crate's lints refuse: a `Core` member's own output goes through
    // `nvs_runtime::Ctx`'s sinks, and this is not output — it is the last
    // thing a process does before aborting.
    use std::io::Write as _;
    let _ = std::io::stderr().write_all(
        b"nvs: a `Core\\Attributes` retrieval reached a runtime helper - `rule:attributes/inert-metadata` \xc2\xa7 5 \
          folds every one of them in `nvs check`, so this is a bug in `nvs-ir`'s lowering \
          rather than in the program\n",
    );
    std::process::abort();
}
