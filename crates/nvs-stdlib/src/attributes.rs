//! `Core\Attributes` — [ADR 0046](../../../../docs/adr/0046-attributes-shape-literal-metadata.md)
//! §§ 4-5's structural retrieval, and the one `Core` class whose members never
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
//! string, which is not a member name any declaration can have — ADR 0029's
//! casing rule needs at least one character — and so is the one value that can
//! mean "the target itself" without shadowing a real name.

use crate::registry::{Const, CoreClass, CoreMethod, CoreTy, Qual};

/// This class's fully-qualified name, in one place so the registry row and
/// every consumer that matches on it cannot drift apart.
pub(crate) const NAME: &str = "Core\\Attributes";

/// `T` — the shape a retrieval is asked for, written at the call site.
/// [`CoreTy::Written`] owns why a variable appearing in no parameter position
/// has to be supplied there, which is ADR 0046 § 6's whole subject.
const T: CoreTy = CoreTy::Written("T");

/// `$member` — the optional declaration name both members take, carrying
/// [ADR 0088](../../../../docs/adr/0088-a-sink-is-an-instruction-and-the-default-refuses.md)
/// § 2's classification.
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
    methods: &[
        CoreMethod {
            name: "get",
            params: &[CoreTy::Callable, MEMBER],
            defaults: &[Const::Str("")],
            return_ty: CoreTy::Nullable(&T),
            symbol: "nvs_core_attributes_get",
        },
        CoreMethod {
            name: "all",
            params: &[CoreTy::Callable, MEMBER],
            defaults: &[Const::Str("")],
            return_ty: CoreTy::Array(&T),
            symbol: "nvs_core_attributes_all",
        },
    ],
    instance: &[],
    slots: &[],
    constants: &[],
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
/// ADR 0046 § 5 resolves a retrieval in `nvs check`, so a call reaching a
/// helper means `nvs-ir` lowered one it should have replaced.
extern "C" fn folded_at_compile_time() {
    // Written through the handle rather than with `eprintln!`, which this
    // crate's lints refuse: a `Core` member's own output goes through
    // `nvs_runtime::Ctx`'s sinks, and this is not output — it is the last
    // thing a process does before aborting.
    use std::io::Write as _;
    let _ = std::io::stderr().write_all(
        b"nvs: a `Core\\Attributes` retrieval reached a runtime helper - ADR 0046 \xc2\xa7 5 \
          folds every one of them in `nvs check`, so this is a bug in `nvs-ir`'s lowering \
          rather than in the program\n",
    );
    std::process::abort();
}
