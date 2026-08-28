//! `Core\Program` — [ADR 0061](../../../../docs/adr/0061-compile-time-autoload-and-program-discovery.md)
//! § 3's program enumeration, and the second `Core` class whose members never
//! run.
//!
//! [`crate::attributes`] is the first, and the reason is the same one stated
//! twice: § 3 says `implementing<T>()` "expands, at compile time, to an array
//! literal of `new` expressions — one per non-abstract class in the program
//! implementing `T`". The answer is decided by `nvs_hir::implementors` over a
//! graph the whole program is already in, so there is no lookup left for a
//! helper to perform and `nvs_types::program` is where the expansion happens.
//! The symbol below therefore names a body that exists only so
//! [`crate::symbols`] — `crate::address_of`, in fact — can hand the JIT an
//! address for every registered member without a second rule about which rows
//! are exempt. Reaching it is a compiler bug, and the body says so and aborts
//! rather than answering plausibly.
//!
//! # Why the row declares no parameter at all
//!
//! `implementing<T>()` takes nothing. Its whole input is the type argument,
//! which is why [`CoreTy::Written`] rather than [`crate::registry::CoreTy::Var`]
//! is the variant: a variable inferred from an argument needs a parameter
//! position holding the answer, and this member has none. That makes it the
//! only registered member whose `params` is empty *and* whose signature is
//! generic, so the call spelling a program writes is
//! `Core\Program::implementing<Module>()` and never the bare one — a written
//! type-argument list is not optional on a member that declares one
//! (`E_TYPE_ARGS_MISSING`).
//!
//! It is **not** one of [`crate::registry::WRITTEN_CLASS_MEMBERS`], which
//! `Core\Json::decodeAs` is: that roster puts the *written class's* descriptor
//! in argument slot 0 for a helper to hydrate against, and this member has no
//! helper to hand anything to. What `T` names here is an interface, which has
//! no descriptor at all.

use crate::registry::{CoreClass, CoreMethod, CoreTy};

/// This class's fully-qualified name, in one place so the registry row and
/// every consumer that matches on it cannot drift apart.
pub(crate) const NAME: &str = "Core\\Program";

/// `T` — the interface the enumeration is asked for, written at the call site.
/// [`CoreTy::Written`] owns why a variable appearing in no parameter position
/// has to be supplied there.
const T: CoreTy = CoreTy::Written("T");

/// The registry row. See [`crate::registry::CLASSES`].
pub(crate) const CLASS: CoreClass = CoreClass {
    name: NAME,
    methods: &[CoreMethod {
        name: "implementing",
        params: &[],
        defaults: &[],
        return_ty: CoreTy::Array(&T),
        symbol: "nvs_core_program_implementing",
    }],
    instance: &[],
    slots: &[],
    constants: &[],
};

/// The address of one of *this* module's symbols, or `None` for a symbol that
/// belongs to another domain. See [`crate::symbols`].
pub(crate) fn address(symbol: &str) -> Option<*const u8> {
    Some(match symbol {
        "nvs_core_program_implementing" => (expanded_at_compile_time as *const ()).cast(),
        _ => return None,
    })
}

/// The body the row names, and the one this crate hopes is never entered:
/// ADR 0061 § 3 expands the call in `nvs check`, so a call reaching a helper
/// means `nvs-ir` lowered one it should have replaced.
extern "C" fn expanded_at_compile_time() {
    // Written through the handle rather than with `eprintln!`, which this
    // crate's lints refuse — and this is not a `Core` member's output anyway,
    // it is the last thing a process does before aborting.
    use std::io::Write as _;
    let _ = std::io::stderr().write_all(
        b"nvs: `Core\\Program::implementing` reached a runtime helper - ADR 0061 \xc2\xa7 3 \
          expands every one of them in `nvs check`, so this is a bug in `nvs-ir`'s lowering \
          rather than in the program\n",
    );
    std::process::abort();
}
