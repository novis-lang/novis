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
//! * The per-domain modules ([`arr`], [`str`]) hold the implementations, each
//!   an ADR 0002 helper entry point.
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
//! 1. **The registry holds part of §§ 1–2 and none of §§ 3–12.**
//!    `Core\Arr::count` was the first, and landed with the mechanism rather
//!    than after it, on this repository's standing "narrow slice, end to end"
//!    rule. Two more members proved the two things the mechanism still had to:
//!    `Arr::filter`, which calls *back* into MWL code through
//!    `mwl_runtime::call_closure`, and `Str::join`, the first with an optional
//!    parameter. Everything registered since is a registry row plus a body and
//!    nothing else. The spec file's §§ 1–12 are the work list, and
//!    `.claude/loop-goal.md`'s Stage 4 holds the coverage gate that will name
//!    every member still missing a conformance case.
//!
//!    Within § 1, the members still absent are the ones waiting on something:
//!    `length`/`at`/`slice` on
//!    [ADR 0009](../../../docs/adr/0009-string-and-bytes.md)'s open
//!    granularity question — which [`str`]'s own docs record as the one thing
//!    that can still change an already-registered member's answer.
//! 3. **`Const` has no `null` variant**, so a member whose spec signature
//!    defaults to `null` — `Core\Str::slice`'s `?int $length = null` and the
//!    rest of the spec's most common optional shape — waits on
//!    `mwl_types::defaults` growing one, and on the IR constant under it.
//!    ADR 0063 R2's options bag is *not* on that list any more:
//!    [`registry::CoreTy::Options`] expresses it and `Core\Arr::range` is the
//!    first member registered with one.
//! 2. **A type variable is inferred, never declared by user code.** ADR 0007's
//!    *Revisiting* section and `.claude/loop-goal.md` both scope `<T>` to
//!    declarations the compiler owns, which is exactly what
//!    [`registry::CoreTy::Var`] is; `mwl-types` owns the unification and
//!    substitution, and its own docs are the home for what that does and does
//!    not do yet.

pub mod arr;
pub mod registry;
pub mod str;

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
                "mwl_core_arr_range" => (arr::mwl_core_arr_range as *const ()).cast(),
                "mwl_core_str_is_empty" => (str::mwl_core_str_is_empty as *const ()).cast(),
                "mwl_core_str_contains" => (str::mwl_core_str_contains as *const ()).cast(),
                "mwl_core_str_starts_with" => (str::mwl_core_str_starts_with as *const ()).cast(),
                "mwl_core_str_ends_with" => (str::mwl_core_str_ends_with as *const ()).cast(),
                "mwl_core_str_join" => (str::mwl_core_str_join as *const ()).cast(),
                "mwl_core_str_pad_start" => (str::mwl_core_str_pad_start as *const ()).cast(),
                "mwl_core_str_pad_end" => (str::mwl_core_str_pad_end as *const ()).cast(),
                "mwl_core_str_repeat" => (str::mwl_core_str_repeat as *const ()).cast(),
                "mwl_core_str_lower" => (str::mwl_core_str_lower as *const ()).cast(),
                "mwl_core_str_upper" => (str::mwl_core_str_upper as *const ()).cast(),
                "mwl_core_str_upper_first" => (str::mwl_core_str_upper_first as *const ()).cast(),
                "mwl_core_str_lower_first" => (str::mwl_core_str_lower_first as *const ()).cast(),
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
