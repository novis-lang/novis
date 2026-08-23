//! MWL's Tier 0 standard library: every `Core` member, written in native
//! Rust, plus the signature registry the compiler resolves a call against.
//!
//! [ADR 0051](../../../docs/adr/0051-standard-library-tiers.md) § *Tier 0*
//! makes this "compiled into the binary, native, direct heap access, no
//! boundary," and `.claude/loop-goal.md` records that it is meant literally:
//! no part of `Core` is written in MWL. [ADR 0063](../../../docs/adr/0063-core-api-conventions.md)
//! fixes every member's *shape* and [docs/spec/01-core-library.md](../../../docs/spec/01-core-library.md)
//! is authoritative for every *signature* — this crate restates neither. It
//! holds the two things a signature on paper cannot be: a resolvable entry in
//! [`registry::CLASSES`], and a callable symbol in [`symbols`].
//!
//! # The two halves, and why they are one crate
//!
//! * [`registry`] is pure metadata — a member's name, its parameter and
//!   return types, and the symbol its implementation is reachable at. The
//!   compiler (`mwl-types`) seeds its own signature table from it, so
//!   `Core\Arr::count($a)` resolves through exactly the machinery a
//!   user-declared static call already does.
//! * The per-domain modules ([`arr`]) hold the implementations, each an
//!   ADR 0002 helper entry point.
//!
//! Keeping them together is what makes a member impossible to half-add: a
//! registry entry naming a symbol nothing defines fails to link, and an
//! implementation nothing registers is dead code the compiler will warn about.
//!
//! # A `Core` call is a helper call
//!
//! Every member has the one signature
//! [ADR 0002](../../../docs/adr/0002-error-propagation.md) makes normative for
//! a runtime helper — `extern "C" fn(*mut Ctx, *const Value, *mut Value) -> i32`
//! — reached through [`mwl_runtime::mwl_helper!`], so `mwl-codegen` emits a
//! `Core` call through the *same* path it already emits
//! `mwl_ir::ir::InstKind::HelperCall` through, with no per-member Cranelift
//! signature anywhere. The consequences are the helper convention's, not new
//! policy:
//!
//! * **Arguments are borrowed, never consumed.** A helper body receives
//!   `&[Value]` and releases nothing, so the caller keeps owning every
//!   reference it passed. This is the opposite of an MWL method call, whose
//!   callee owns its parameters — and it is what ADR 0063's R3 purity rule
//!   makes safe: no `Core` member stores its argument.
//! * **A returned heap value carries one fresh reference**, which the caller
//!   owns, exactly like `mwl_str_concat`'s result.
//! * **Failure is a `Fault`**, which becomes ADR 0002's `THROWN` or `FATAL`
//!   status; nothing unwinds.
//!
//! # Known gaps
//!
//! 1. **The registry holds three members.** `Core\Arr::count` was the first,
//!    and landed with the mechanism rather than after it, on this
//!    repository's standing "narrow slice, end to end" rule — it compiles and
//!    runs, so every later member is a registry row plus a body rather than
//!    more machinery. `filter` is the second thing the mechanism had to
//!    prove: a member that calls *back* into MWL code, through
//!    `mwl_runtime::call_closure`. The spec file's §§ 1–12 are the work list,
//!    and `.claude/loop-goal.md`'s Stage 4 holds the coverage gate that will
//!    name every member still missing one.
//! 3. **No member takes an options shape or an optional parameter yet.**
//!    `registry::CoreTy` can express neither, which is why `Core\Arr::range`
//!    — a two-argument member with a `{step?: int}` bag — is not registered
//!    alongside `filter`: registering it without the bag would put a
//!    signature in the compiler that the spec does not describe.
//! 2. **A type variable is inferred, never declared by user code.** ADR 0007's
//!    *Revisiting* section and `.claude/loop-goal.md` both scope `<T>` to
//!    declarations the compiler owns, which is exactly what
//!    [`registry::CoreTy::Var`] is; `mwl-types` owns the unification and
//!    substitution, and its own docs are the home for what that does and does
//!    not do yet.

pub mod arr;
pub mod registry;

/// Every `Core` implementation's symbol and address, for the JIT to resolve
/// against — the same shape and the same purpose as
/// [`mwl_runtime::symbols`], which `mwl-codegen` already registers.
///
/// Deliberately derived from [`registry::CLASSES`] rather than written out a
/// second time: a member is registered once, and this looks its address up
/// through one match that the compiler makes exhaustive by failing to build
/// when a registered symbol has no arm.
///
/// # Panics
///
/// Panics naming the symbol if [`registry::CLASSES`] registers one this has no
/// address for. Both tables live in this crate, so that is a build-time
/// oversight rather than anything a program could cause.
#[must_use]
pub fn symbols() -> Vec<(&'static str, *const u8)> {
    registry::CLASSES
        .iter()
        .flat_map(|class| class.methods)
        .map(|method| {
            let address: *const u8 = match method.symbol {
                "mwl_core_arr_count" => (arr::mwl_core_arr_count as *const ()).cast(),
                "mwl_core_arr_filter" => (arr::mwl_core_arr_filter as *const ()).cast(),
                "mwl_core_arr_is_empty" => (arr::mwl_core_arr_is_empty as *const ()).cast(),
                other => panic!("mwl-stdlib registers `{other}` with no implementation address"),
            };
            (method.symbol, address)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every registered member resolves to an address — the check the
    /// `symbols` panic exists for, run once rather than left to whichever
    /// program first calls the missing member.
    #[test]
    fn every_registered_member_has_an_implementation_address() {
        let symbols = symbols();
        assert_eq!(
            symbols.len(),
            registry::CLASSES
                .iter()
                .map(|class| class.methods.len())
                .sum::<usize>()
        );
        assert!(symbols.iter().all(|(_, address)| !address.is_null()));
    }

    /// A member's symbol is unique across the whole registry: it is what the
    /// JIT resolves against, so two members sharing one would silently call
    /// the same code.
    #[test]
    fn no_two_members_share_a_symbol() {
        let mut seen: Vec<&str> = registry::CLASSES
            .iter()
            .flat_map(|class| class.methods)
            .map(|method| method.symbol)
            .collect();
        let total = seen.len();
        seen.sort_unstable();
        seen.dedup();
        assert_eq!(seen.len(), total);
    }
}
