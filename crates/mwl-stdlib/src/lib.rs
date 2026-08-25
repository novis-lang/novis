//! MWL's Tier 0 standard library: every `Core` member, written in native
//! Rust, plus the signature registry the compiler resolves a call against.
//!
//! [ADR 0051](../../../docs/adr/0051-standard-library-tiers.md) § *Tier 0*
//! makes this "compiled into the binary, native, direct heap access, no
//! boundary," and `docs/agent/loop-goal.md` records that it is meant literally:
//! no part of `Core` is written in MWL. [ADR 0063](../../../docs/adr/0063-core-api-conventions.md)
//! fixes every member's *shape* and [docs/spec/01-core-library.md](../../../docs/spec/01-core-library.md)
//! is authoritative for every *signature* — this crate restates neither. It
//! holds the two things a signature on paper cannot be: a resolvable entry in
//! [`registry::CLASSES`], and a callable symbol in [`symbols`].
//!
//! # One module per domain; adding a class is two lines
//!
//! A domain module ([`arr`], [`json`], [`math`], [`regex`], [`str`], [`time`])
//! holds everything about its class: the
//! implementations, each an ADR 0002 helper entry point; a `pub const CLASS`
//! carrying that class's registry rows; and a `pub(crate) fn address` answering
//! for its own symbols and nothing else. A domain with more than one class —
//! [`regex`], [`time`] — names each `CLASS` after it and keeps one `address`
//! arm per class, so the "one file per domain" rule still holds.
//!
//! [`registry`] holds the *shapes* those rows are written in ([`registry::CoreTy`],
//! [`registry::CoreMethod`], …) plus one list naming each domain's `CLASS`. The
//! compiler (`mwl-types`) seeds its signature table from that list, so
//! `Core\Arr::count($a)` resolves through exactly the machinery a user-declared
//! static call already does.
//!
//! The point of the split is collision surface, not file size. Every one of the
//! ~190 members the spec still owes used to edit the same two places — one flat
//! table and one flat match — so two sessions adding two different domains
//! always conflicted. Now **adding a member touches one file**, and adding a
//! *class* adds one line to [`registry::CLASSES`] and one to [`symbols`].
//!
//! Keeping metadata and implementation in one crate is what makes a member
//! impossible to half-add: a registry row naming a symbol nothing defines fails
//! to link, and an implementation nothing registers is dead code the compiler
//! warns about.
//!
//! Three modules are not domains, and all exist for the same reason — more
//! than one domain reaches for what they hold, so no domain may decide it
//! alone. `granularity` holds ADR 0009 § 2's answer to "what unit does a
//! `string` count in"; `ordering` holds what "smaller" means with no
//! comparator given, which `Core\Arr::sort`/`min`/`max` and
//! `Core\Math::min`/`max`/`clamp` would otherwise be free to answer
//! differently; [`instance`] holds what a value of a `Core`-owned class *is*,
//! which § 4's time types, § 9's collections and § 12's `Uri` all answer the
//! same way § 5's `Match` does.
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
//!   status; nothing unwinds. A `Fault::thrown_as` names which of
//!   [docs/spec/01-core-library.md](../../../docs/spec/01-core-library.md)
//!   § 10's classes a `catch` will see — `Core\Json::decode` answers with
//!   `ParseError` — and a bare `Fault::thrown` means `RuntimeError`, which is
//!   what a failure with nothing more specific to say is.
//!
//! # Known gaps
//!
//! 1. **The registry holds part of §§ 1–2, all of §§ 3–4 and § 8, most of
//!    §§ 5–6 and § 9, and none of § 7 or §§ 10–12.**
//!    `Core\Arr::count` was the first, and landed with the mechanism rather
//!    than after it, on this repository's standing "narrow slice, end to end"
//!    rule. Two more members proved the two things the mechanism still had to:
//!    `Arr::filter`, which calls *back* into MWL code through
//!    `mwl_runtime::call_closure`, and `Str::join`, the first with an optional
//!    parameter. Everything registered since is a registry row plus a body and
//!    nothing else. The spec file's §§ 1–12 are the work list, and
//!    `tests/conformance_coverage.rs` is the gate that keeps the *registered*
//!    half honest: a member with no `.mwlt` case that calls it fails
//!    `cargo test -p mwl-stdlib`, which is the check
//!    `docs/agent/loop-goal.md`'s Stage 4 names.
//!
//!    Within § 1, ADR 0009 § 2's granularity question is closed and
//!    [`granularity::DEFAULT`] is its answer, so `length`, `at`, `slice`,
//!    `indexOf`, `lastIndexOf` and `wrap` all count in it. Twelve of that
//!    section's rows are left: `compare`, `chunk`, `lines`, `graphemes`,
//!    `codePoints`, `replaceAll`, `replaceRange`, `fold`, `normalize` and the
//!    two `fromCodePoint` members. `format` is written, over the printf
//!    grammar [`format`] owns and the first variadic parameter in `Core`.
//!    Section 3 is whole: every one of [`math::CLASS`]'s thirty-eight rows
//!    runs, and `abs`/`sign`/`format` take the `int|float|decimal` the spec
//!    writes (see [`math`]'s own gap note for the four rounding rows that do
//!    not yet). Section 5 is six of its eight: [`regex::CLASS`] holds both of
//!    ADR 0056's tiers and the `Core`-owned `Match` they answer with, and that
//!    module's own gap 1 owns `compile`/`replaceWith`, which need `Pattern`.
//!    Section 4 runs but its two component types: [`time`] holds
//!    `Core\Time`'s entry points, `Instant`, `DateTime`, `Duration` and `Zone`
//!    over `jiff` and `cldr`'s pattern grammar — that module's own docs own
//!    why that crate and what it spends, and its gap 1 owns `Date`,
//!    `TimeOfDay` and `Core\Month`. Section 6 is three of its four:
//!    [`json::CLASS`] holds `encode`, `decode` and `isValid` over
//!    `serde_json`, and that module's own gap 2 owns `decodeAs<T>`, which
//!    waits on ADR 0071 and on an explicit type argument at a call site.
//!    Section 8 is whole and needed no dependency at all — [`path`] is `&str`
//!    arithmetic, and its own docs own the one-grammar-on-every-platform rule
//!    that makes a `Core\Path` case pinnable on both legs, the three rows
//!    where it diverges from `pathinfo`/`dirname`, and the two path *shapes*
//!    (UNC, drive-relative) it does not model. Section 9 is two of its three
//!    classes: [`objmap`] and [`objset`] hold every member of their rows over
//!    the one store [`identity_store`] owns, and what is left is `Core\Heap`
//!    and the `Iterable` each of the three also implements — which waits on
//!    ADR 0053's iteration protocol reaching a `Core`-owned class.
//! 3. **Every shape a §§ 1–12 signature writes can now be stated.** The last
//!    one was a **variadic** parameter, and it is
//!    [`registry::CoreTy::Variadic`] — one ABI argument holding a fresh
//!    `array<T>` of the tail, built by `mwl_ir::lower::lower_variadic_tail`,
//!    since a helper's `args: [N]` is a fixed arity. `Core\Str::format` is the
//!    first row to declare one; ADR 0069's
//!    `overlay`/`overlayDeep`/`underlay`/`appendAll`, `Arr::append`,
//!    `Arr::prepend` and `Path::join` need only writing.
//!
//!    What is left is not a *type* but a call shape: `mwl-ir`'s gap 8 still
//!    refuses a **named** or `...spread` argument, which no `Core` signature
//!    needs and every one of those members can be called without.
//!
//!    A **`Core`-owned instance** is no longer one: [`instance`] is the value
//!    behind [`registry::CoreTy::Instance`], and that module's own docs own
//!    what it is and what it spends — so § 9's three collections and § 12's
//!    `Uri` need only their members written, exactly as § 4's `Duration`
//!    already has.
//!
//!    `decimal` is no longer one of them: [`registry::CoreTy::Decimal`] states
//!    it and `mwl_runtime::Decimal` is the value behind it, so `Arr::sum`,
//!    `product` and `average` are written over the
//!    `array<int|float|decimal>` subject the spec gives them.
//!
//!    A class **constant** is no longer among them: [`registry::CoreConst`] is
//!    a roster on [`registry::CoreClass`], resolved by `mwl_types::expr`'s
//!    `ClassConstAccess` arm and lowered as the inlined literal it is, so
//!    `Core\Math`'s eleven are written and `Core\Path::SEPARATOR` needs only
//!    its class. A **user-declared** class's constant is still unmodeled —
//!    `mwl_types`' own known gaps own that half, which nothing in `Core`
//!    depends on.
//!
//!    Everything else the spec writes is expressible: ADR 0063 R2's options
//!    bag ([`registry::CoreTy::Options`], first used by `Core\Arr::range`), a
//!    union in **either** direction ([`registry::CoreTy::Union`], first used
//!    by `Core\Arr::hasKey`), a `Core`-owned enum
//!    ([`registry::CoreTy::Enum`] over [`registry::ENUMS`], first used by
//!    `Core\Arr::sort`), an **absent** option ([`registry::Const::Null`], the
//!    same member), and ADR 0066's `?T`
//!    ([`registry::CoreTy::Nullable`], first used by `Core\Arr::first`) —
//!    including as a **parameter** defaulting to `null`, the spec's most common
//!    optional shape, which `Core\Str::slice`'s `?int $length = null` is the
//!    first row to declare and the first call site to leave out.
//!
//!    Strict identity is no longer among them:
//!    `mwl_runtime::value_identical` defines it and
//!    `mwl_runtime::value_hash` indexes it, so `contains`, `keyOf` and
//!    `unique` are registered and `diff`/`intersect` need only their `SetOn`
//!    enum and their `on`/`by`/`comparator` bag — every one of which
//!    [`registry`] can already state.
//! 4. **`array<T>` is invariant, so a `array<int|string>` parameter takes
//!    only that exact spelling.** `Core\Arr::flip` is the first member whose
//!    spec signature declares one, and `Core\Arr::flip($stringArray)` is
//!    refused today — the caller declares `array<string|int>` instead.
//!    `mwl_types::expr::is_assignable`'s own docs own the rule and say why no
//!    variance was committed to. Widening it later would accept strictly more
//!    programs and break none, so the narrow rule is the safe thing to be
//!    holding while the question is open; the argument *for* widening is that
//!    an MWL array is a copy-on-write **value**, so an element-covariant read
//!    cannot be aliased into an unsound write the way a mutable container's
//!    could.
//! 2. **A type variable is inferred, never declared by user code.** ADR 0007's
//!    *Revisiting* section and `docs/agent/loop-goal.md` both scope `<T>` to
//!    declarations the compiler owns, which is exactly what
//!    [`registry::CoreTy::Var`] is; `mwl-types` owns the unification and
//!    substitution, and its own docs are the home for what that does and does
//!    not do yet.

pub mod arr;
mod bytes;
mod cldr;
mod csv;
mod encoding;
mod format;
pub mod granularity;
mod hash;
mod identity_store;
mod instance;
mod issue;
pub mod json;
pub mod math;
mod objmap;
mod objset;
mod ordering;
pub mod path;
pub mod random;
pub mod regex;
pub mod registry;
pub mod str;
pub mod time;
pub mod uri;
pub mod uuid;
mod validate;

/// ADR 0071's derived-codec field list, re-exported from where it is
/// *consumed*.
///
/// The struct lives in `mwl-runtime` because that is the deepest crate that
/// holds one — `mwl_runtime::ClassDesc` carries the finished list. `mwl-types`
/// and `mwl-ir` each produce a stage of it and neither depends on the runtime
/// directly, so they reach it through this crate, which they already treat as
/// the home of the `Core` contract.
pub use mwl_runtime::{CodecField, CodecTy};

use registry::CoreClass;

/// Every `Core` implementation's symbol and address, for the JIT to resolve
/// against — the same shape and the same purpose as
/// [`mwl_runtime::symbols`], which `mwl-codegen` already registers.
///
/// Deliberately derived from [`registry::CLASSES`] rather than written out a
/// second time: a member is registered once, and this looks its address up by
/// asking each domain module in turn for one of *its* symbols.
///
/// **One `.or_else` per class, never one arm per member.** Each domain owns
/// its own `address` function beside its implementations, so adding a member
/// touches that module alone. Adding a class adds one line here and one in
/// [`registry::CLASSES`].
///
/// # Panics
///
/// Panics naming the symbol if [`registry::CLASSES`] registers one no domain
/// claims. Both halves live in this crate, so that is a build-time oversight
/// rather than anything a program could cause.
#[must_use]
pub fn symbols() -> Vec<(&'static str, *const u8)> {
    registry::CLASSES
        .iter()
        .flat_map(CoreClass::members)
        .map(|method| method.symbol)
        // A constructible class's `new` symbol is not a member of it — see
        // [`registry::CONSTRUCTORS`] and `instance`'s module docs — so the two
        // rosters are chained rather than the constructor being folded into
        // one of them.
        .chain(registry::CONSTRUCTORS.iter().map(|(_, symbol)| *symbol))
        .map(|symbol| (symbol, address_of(symbol)))
        .collect()
}

/// One registered symbol's address, asking each domain module in turn.
///
/// # Panics
///
/// Panics naming the symbol if no domain claims it.
fn address_of(symbol: &'static str) -> *const u8 {
    str::address(symbol)
        .or_else(|| arr::address(symbol))
        .or_else(|| bytes::address(symbol))
        .or_else(|| csv::address(symbol))
        .or_else(|| encoding::address(symbol))
        .or_else(|| hash::address(symbol))
        .or_else(|| json::address(symbol))
        .or_else(|| math::address(symbol))
        .or_else(|| objmap::address(symbol))
        .or_else(|| objset::address(symbol))
        .or_else(|| path::address(symbol))
        .or_else(|| random::address(symbol))
        .or_else(|| regex::address(symbol))
        .or_else(|| time::address(symbol))
        .or_else(|| uri::address(symbol))
        .or_else(|| uuid::address(symbol))
        .or_else(|| validate::address(symbol))
        .unwrap_or_else(|| panic!("mwl-stdlib registers `{symbol}` with no implementation address"))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every registered member — and every constructible class's `new` symbol
    /// — resolves to an address, which is the check the `symbols` panic
    /// exists for, run once rather than left to whichever program first calls
    /// the missing one.
    #[test]
    fn every_registered_member_has_an_implementation_address() {
        let symbols = symbols();
        assert_eq!(
            symbols.len(),
            registry::CLASSES
                .iter()
                .map(|class| class.members().count())
                .sum::<usize>()
                + registry::CONSTRUCTORS.len()
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
            .flat_map(CoreClass::members)
            .map(|method| method.symbol)
            .collect();
        let total = seen.len();
        seen.sort_unstable();
        seen.dedup();
        assert_eq!(seen.len(), total);
    }
}
