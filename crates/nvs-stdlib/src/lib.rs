//! Novis's Tier 0 standard library: every `Core` member, written in native
//! Rust, plus the signature registry the compiler resolves a call against.
//!
//! [ADR 0051](/docs/adr/0051-standard-library-tiers.md) § *Tier 0*
//! makes this "compiled into the binary, native, direct heap access, no
//! boundary," and `docs/agent/loop-goal.md` records that it is meant literally:
//! no part of `Core` is written in Novis. [ADR 0063](/docs/adr/0063-core-api-conventions.md)
//! fixes every member's *shape* and [docs/spec/01-core-library.md](/docs/spec/01-core-library.md)
//! is authoritative for every *signature* — this crate restates neither. It
//! holds the two things a signature on paper cannot be: a resolvable entry in
//! [`registry::CLASSES`], and a callable symbol in [`symbols`].
//!
//! # One module per domain; adding a class is two lines
//!
//! A domain module ([`arr`], [`json`], [`math`], [`regex`], [`mod@str`], [`time`])
//! holds everything about its class: the
//! implementations, each an ADR 0002 helper entry point; a `pub const CLASS`
//! carrying that class's registry rows; and a `pub(crate) fn address` answering
//! for its own symbols and nothing else. A domain with more than one class —
//! [`regex`], [`time`] — names each `CLASS` after it and keeps one `address`
//! arm per class, so the "one file per domain" rule still holds.
//!
//! [`registry`] holds the *shapes* those rows are written in ([`registry::CoreTy`],
//! [`registry::CoreMethod`], …) plus one list naming each domain's `CLASS`. The
//! compiler (`nvs-types`) seeds its signature table from that list, so
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
//! [ADR 0002](/docs/adr/0002-error-propagation.md) makes normative for
//! a runtime helper — `extern "C" fn(*mut Ctx, *const Value, *mut Value) -> i32`
//! — reached through [`nvs_runtime::nvs_helper!`], so `nvs-codegen` emits a
//! `Core` call through the *same* path it already emits
//! `nvs_ir::ir::InstKind::HelperCall` through, with no per-member Cranelift
//! signature anywhere. The consequences are the helper convention's, not new
//! policy:
//!
//! * **Arguments are borrowed, never consumed.** A helper body receives
//!   `&[Value]` and releases nothing, so the caller keeps owning every
//!   reference it passed. This is the opposite of an Novis method call, whose
//!   callee owns its parameters — and it is what ADR 0063's R3 purity rule
//!   makes safe: no `Core` member stores its argument.
//! * **A returned heap value carries one fresh reference**, which the caller
//!   owns, exactly like `nvs_str_concat`'s result.
//! * **Failure is a `Fault`**, which becomes ADR 0002's `THROWN` or `FATAL`
//!   status; nothing unwinds. A `Fault::thrown_as` names which of
//!   [docs/spec/01-core-library.md](/docs/spec/01-core-library.md)
//!   § 10's classes a `catch` will see — `Core\Json::decode` answers with
//!   `ParseError` — and a bare `Fault::thrown` means `RuntimeError`, which is
//!   what a failure with nothing more specific to say is.
//!
//! # Known gaps
//!
//! 1. **The registry holds spec §§ 1–12 whole; §§ 13–20 hold whatever the
//!    goals since have written, and nothing measures the remainder.**
//!    `Core\Arr::count` was the first, and landed with the mechanism rather
//!    than after it, on this repository's standing "narrow slice, end to end"
//!    rule. Two more members proved the two things the mechanism still had to:
//!    `Arr::filter`, which calls *back* into Novis code through
//!    `nvs_runtime::call_closure`, and `Str::join`, the first with an optional
//!    parameter. Everything registered since is a registry row plus a body and
//!    nothing else.
//!
//!    **Two gates say so, and both stop at § 12.**
//!    `tests/spec_registry_coverage.rs` fails on a §§ 1–12 row the registry
//!    does not declare, and `tests/spec-members-outstanding.txt` — the file it
//!    reads, which only ever shrinks — holds no keys, which is this project's
//!    definition of *registered whole*. `tests/conformance_coverage.rs` then
//!    keeps that half honest: a member with no `.nvst` case that calls it fails
//!    `cargo test -p nvs-stdlib`.
//!
//!    **Past § 12 there is no outstanding-members file and no coverage gate**,
//!    so a row nobody has written fails nothing — `Core\Db`'s `stream`,
//!    `streamAs` and `Connection::close` are spec § 18 rows in exactly that
//!    position. What each of §§ 13–20 still owes is its own module's known
//!    gaps; `docs/agent/carried-gaps.md` is where the ones no chain goal owns
//!    are kept, and widening either gate past § 12 is an entry on it.
//! 3. **Every shape a §§ 1–12 signature writes can now be stated.** The last
//!    one was a **variadic** parameter, and it is
//!    [`registry::CoreTy::Variadic`] — one ABI argument holding a fresh
//!    `array<T>` of the tail, built by `nvs_ir::lower::lower_variadic_tail`,
//!    since a helper's `args: [N]` is a fixed arity. `Core\Str::format` is the
//!    first row to declare one; ADR 0069's
//!    `overlay`/`overlayDeep`/`underlay`/`appendAll`, `Arr::append`,
//!    `Arr::prepend` and `Path::join` need only writing.
//!
//!    A **named** or `...spread` argument at such a call is no longer a gap
//!    either: both lower, and ADR 0063 R2's "every parameter is callable by
//!    the spec's `$name`" holds end to end. Every parameter's name is on the
//!    row — [`registry::CoreMethod::names`], one per positional slot, the
//!    trailing bag under [`registry::OPTIONS_NAME`] — taken from the signature
//!    column of `docs/spec/01-core-library.md` and held there by
//!    `every_registry_rows_names_are_the_specs_signature_column`, which is
//!    that rule's "a parameter's name is compatibility surface, versioned
//!    where its type is". [`registry::ParamDoc::name`] is a *key* into that
//!    list rather than a second copy of it.
//!
//!    `nvs_types::core_lib` reads them into `MethodSig::param_names`, so a
//!    `name:` at a `Core` call site resolves through the machinery a
//!    user-declared method's does — the same slot mapping, the same skipped
//!    default, and the same refusal for a name that reaches no parameter or
//!    reaches a variadic tail. No member's shape changed to get there.
//!
//!    A **`Core`-owned instance** is no longer one: [`instance`] is the value
//!    behind [`registry::CoreTy::Instance`], and that module's own docs own
//!    what it is and what it spends — so § 9's three collections and § 12's
//!    `Uri` need only their members written, exactly as § 4's `Duration`
//!    already has.
//!
//!    A **sequence** parameter — whatever `foreach` accepts, ADR 0053 § 3's
//!    three shapes at once — is [`registry::CoreTy::Iterated`], first
//!    declared by `Core\Arr::from`, and `nvs_runtime::sequence` is the one
//!    place such an argument is read: an array walked directly, a cursor
//!    driven by name through its class descriptor's own method table.
//!
//!    `decimal` is no longer one of them: [`registry::CoreTy::Decimal`] states
//!    it and `nvs_runtime::Decimal` is the value behind it, so `Arr::sum`,
//!    `product` and `average` are written over the
//!    `array<int|float|decimal>` subject the spec gives them.
//!
//!    A class **constant** is no longer among them: [`registry::CoreConst`] is
//!    a roster on [`registry::CoreClass`], resolved by `nvs_types::expr`'s
//!    `ClassConstAccess` arm and lowered as the inlined literal it is, so
//!    `Core\Math`'s eleven are written and `Core\Path::SEPARATOR` needs only
//!    its class. A **user-declared** class's constant is the same shape one
//!    crate over — `nvs_types::signatures::ConstSig` — and nothing in `Core`
//!    depends on that half.
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
//!    `nvs_runtime::value_identical` defines it and
//!    `nvs_runtime::value_hash` indexes it, so `contains`, `keyOf` and
//!    `unique` are registered and `diff`/`intersect` need only their `SetOn`
//!    enum and their `on`/`by`/`comparator` bag — every one of which
//!    [`registry`] can already state.
//! 4. **`array<T>` is invariant, so a `array<int|string>` parameter takes
//!    only that exact spelling.** `Core\Arr::flip` is the first member whose
//!    spec signature declares one, and `Core\Arr::flip($stringArray)` is
//!    refused today — the caller declares `array<string|int>` instead.
//!    `nvs_types::expr::is_assignable`'s own docs own the rule and say why no
//!    variance was committed to. Widening it later would accept strictly more
//!    programs and break none, so the narrow rule is the safe thing to be
//!    holding while the question is open; the argument *for* widening is that
//!    an Novis array is a copy-on-write **value**, so an element-covariant read
//!    cannot be aliased into an unsound write the way a mutable container's
//!    could.
//! 2. **A type variable is inferred, never declared by user code.** ADR 0007's
//!    *Revisiting* section and `docs/agent/loop-goal.md` both scope `<T>` to
//!    declarations the compiler owns, which is exactly what
//!    [`registry::CoreTy::Var`] is; `nvs-types` owns the unification and
//!    substitution, and its own docs are the home for what that does and does
//!    not do yet.

pub mod arr;
mod ast;
mod attributes;
mod bus;
mod bytes;
mod cache;
// `pub` for the two predicates `nvs_types::capability` reads — the class's own
// spelling and ADR 0112 § 8's roster — so that the checker's refusal holds no
// copy of either. `cldr` is `pub` for the same reason one grammar over.
pub mod cap;
mod channel;
pub mod cldr;
pub mod cli;
mod command;
mod config;
mod crypto;
mod csrf;
mod csv;
mod cursor;
// `pub` for [`db::check_literal_query`], which is ADR 0067 § 10's half of the
// intrinsic pass and the only thing `nvs-types` reads here — for `cap`'s reason
// exactly: the checker's refusal holds no second copy of a grammar this crate
// already owns.
pub mod db;
mod debug;
mod decimal;
mod encoding;
mod env;
mod fatal;
pub mod format;
pub mod granularity;
mod hash;
mod heap;
pub mod html;
mod http;
mod identity_store;
mod instance;
pub mod io;
mod issue;
pub mod json;
mod jwt;
mod log;
mod mail;
pub mod math;
mod multipart;
mod objmap;
mod objset;
mod ordering;
mod out;
mod password;
pub mod path;
mod process;
mod program;
// `pub` for [`queue::MIGRATION_POSTGRES`] and its MySQL sibling alone: ADR 0084 § 2's schema is written
// beside the statements that read its columns, and `nvs queue migrate` in `nvs-cli` is a second
// crate that has to run it — in whichever dialect the block it was pointed at speaks. The
// members themselves are reached the way every other class's are, through [`registry`].
pub mod queue;
pub mod random;
mod ratelimit;
mod reflect;
pub mod regex;
pub mod registry;
// `pub` for [`request::is_known_verb`] alone: ADR 0097 § 2's door refuses a verb outside
// `Core\Http\Method`'s eight with a `501` before an isolate exists, and the roster is this
// module's — a second copy of it in the crate that accepts connections would be a second
// answer. The members themselves are reached the way every other class's are, through
// [`registry`].
pub mod request;
mod response;
pub mod router;
pub mod script;
mod secret;
mod serialize;
mod server;
mod session;
mod signed_cookie;
mod socket;
mod sse;
mod storage;
pub mod str;
mod taint;
mod task;
mod test;
pub mod time;
mod topic;
mod totp;
pub mod uri;
pub mod uuid;
mod validate;

/// ADR 0071's derived-codec field list, re-exported from where it is
/// *consumed*.
///
/// The struct lives in `nvs-runtime` because that is the deepest crate that
/// holds one — `nvs_runtime::ClassDesc` carries the finished list. `nvs-types`
/// and `nvs-ir` each produce a stage of it and neither depends on the runtime
/// directly, so they reach it through this crate, which they already treat as
/// the home of the `Core` contract.
pub use nvs_runtime::{CodecField, CodecTy, EnumCases, FieldDefault};

use registry::CoreClass;

/// Every `Core` implementation's symbol and address, for the JIT to resolve
/// against — the same shape and the same purpose as
/// [`nvs_runtime::symbols`], which `nvs-codegen` already registers.
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
        .chain(registry::CONSTRUCTORS.iter().map(|(_, new)| new.symbol))
        // ADR 0077 § 4's two prepared link implementations, which no member row
        // names on purpose — `router::link`'s own docs own why one member has
        // two entry points.
        .chain(router::link::SYMBOLS)
        // ADR 0006's two constructs, which are syntax rather than members and
        // so have no row to be found through — `script`'s own module doc owns
        // why a `spawn script`/`await` symbol may not be callable by name.
        .chain([
            script::SPAWN_SYMBOL,
            script::SPAWN_METHOD_SYMBOL,
            script::AWAIT_SYMBOL,
        ])
        // The sink carriers' three row-less symbols, which are constructs
        // rather than members for the same reason those two are: ADR 0024 § 5's
        // lift is `as` on a source literal, and § 5's `Markup + Markup` and ADR
        // 0086 § 2's `Text + Text` are an operator. Each is reachable only from
        // the lowering of the construct that spells it, which is what a row
        // would undo — `html`'s and `cli`'s own module docs own why.
        .chain([
            html::MARKUP_SYMBOL,
            html::MARKUP_CONCAT_SYMBOL,
            cli::TEXT_CONCAT_SYMBOL,
        ])
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
        .or_else(|| ast::address(symbol))
        .or_else(|| attributes::address(symbol))
        .or_else(|| bytes::address(symbol))
        .or_else(|| cache::address(symbol))
        .or_else(|| cap::address(symbol))
        .or_else(|| channel::address(symbol))
        .or_else(|| cldr::address(symbol))
        .or_else(|| cli::address(symbol))
        .or_else(|| command::address(symbol))
        .or_else(|| config::address(symbol))
        .or_else(|| crypto::address(symbol))
        .or_else(|| csrf::address(symbol))
        .or_else(|| csv::address(symbol))
        .or_else(|| cursor::address(symbol))
        .or_else(|| db::address(symbol))
        .or_else(|| debug::address(symbol))
        .or_else(|| decimal::address(symbol))
        .or_else(|| encoding::address(symbol))
        .or_else(|| env::address(symbol))
        .or_else(|| fatal::address(symbol))
        .or_else(|| io::address(symbol))
        .or_else(|| hash::address(symbol))
        .or_else(|| heap::address(symbol))
        .or_else(|| html::address(symbol))
        .or_else(|| http::address(symbol))
        .or_else(|| json::address(symbol))
        .or_else(|| jwt::address(symbol))
        .or_else(|| log::address(symbol))
        .or_else(|| mail::address(symbol))
        .or_else(|| math::address(symbol))
        .or_else(|| objmap::address(symbol))
        .or_else(|| objset::address(symbol))
        .or_else(|| out::address(symbol))
        .or_else(|| password::address(symbol))
        .or_else(|| path::address(symbol))
        .or_else(|| process::address(symbol))
        .or_else(|| program::address(symbol))
        .or_else(|| queue::address(symbol))
        .or_else(|| random::address(symbol))
        .or_else(|| ratelimit::address(symbol))
        .or_else(|| reflect::address(symbol))
        .or_else(|| request::address(symbol))
        .or_else(|| response::address(symbol))
        .or_else(|| router::address(symbol))
        .or_else(|| regex::address(symbol))
        .or_else(|| script::address(symbol))
        .or_else(|| secret::address(symbol))
        .or_else(|| serialize::address(symbol))
        .or_else(|| server::address(symbol))
        .or_else(|| session::address(symbol))
        .or_else(|| signed_cookie::address(symbol))
        .or_else(|| socket::address(symbol))
        .or_else(|| sse::address(symbol))
        .or_else(|| storage::address(symbol))
        .or_else(|| taint::address(symbol))
        .or_else(|| task::address(symbol))
        .or_else(|| test::address(symbol))
        .or_else(|| time::address(symbol))
        .or_else(|| topic::address(symbol))
        .or_else(|| totp::address(symbol))
        .or_else(|| uri::address(symbol))
        .or_else(|| uuid::address(symbol))
        .or_else(|| validate::address(symbol))
        .unwrap_or_else(|| panic!("nvs-stdlib registers `{symbol}` with no implementation address"))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A snapshot built from the text an operator would have written, for the
    /// reason `nvs_runtime::capability`'s own cases state: the boot path
    /// deserializes, so a case that constructed the typed tree directly would
    /// pin a grant no configuration file can express.
    ///
    /// Here rather than in each module that grants something, because a
    /// capability case in `process` and one in `http` ask the same question of
    /// the same loader, and a second copy would be a second reading of what a
    /// configuration file can say. Inside this module rather than beside it
    /// because `nvs_stdlib_reaches_the_os_only_through_the_gate` scans each file
    /// down to its *first* `#[cfg(test)]` and asserts there is only one.
    pub(crate) fn granting(written: &str) -> std::sync::Arc<nvs_config::Snapshot> {
        let table: toml::Table = written.parse().expect("the case writes valid TOML");
        std::sync::Arc::new(nvs_config::Snapshot {
            config: table
                .clone()
                .try_into()
                .expect("the case writes a block this tree has"),
            table,
            ..nvs_config::Snapshot::default()
        })
    }

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
                // ADR 0077 § 4's two prepared link entry points, which belong
                // to `Core\Router::url`/`::urlAbsolute` and to no row of their
                // own — see `router::link`.
                + router::link::SYMBOLS.len()
                // ADR 0006's `spawn script` — in its two entry forms, which
                // are two symbols and one construct — and `await`, ADR 0024
                // § 5's `as Markup` and `Markup + Markup`, and ADR 0086 § 2's
                // `Text + Text`: six symbols behind five constructs, each
                // syntax rather than a call, so none of them has a row
                // either — see `script`'s, `html`'s and `cli`'s module docs.
                + 6
        );
        assert!(symbols.iter().all(|(_, address)| !address.is_null()));
    }

    /// A symbol names one *member*, and more than one row may carry it only
    /// where they are that member declared on two classes.
    ///
    /// The symbol is what the JIT resolves against, so two unrelated members
    /// sharing one would silently call the same code — which is what this
    /// still refuses, and the whole of what it refused when it was written.
    /// What it now admits is
    /// [ADR 0043](/docs/adr/0043-interface-default-methods-and-delegation-replace-traits.md)'s
    /// `by` delegation, whose entire content is that the two rows *are* one
    /// member: `Core\Db\Transaction implements Queryable by $connection`
    /// ([ADR 0067](/docs/adr/0067-core-db.md) § 7, and the first
    /// `Core` type to use the construct) declares the interface once on the
    /// connection and forwards it, so a second body under a second symbol
    /// would be exactly the drift the delegation exists to prevent. The price
    /// is that a shared symbol has to carry the same spelling and the same
    /// signature at every row — so a paste error between two different
    /// members fails here as it always did.
    #[test]
    fn a_shared_symbol_is_one_member_declared_more_than_once() {
        let mut rows: Vec<&registry::CoreMethod> = registry::CLASSES
            .iter()
            .flat_map(CoreClass::members)
            .collect();
        rows.sort_unstable_by_key(|row| row.symbol);
        for pair in rows.windows(2) {
            let (left, right) = (pair[0], pair[1]);
            if left.symbol != right.symbol {
                continue;
            }
            assert_eq!(
                left.name, right.name,
                "`{}` is carried by two differently named members",
                left.symbol
            );
            assert_eq!(
                (
                    left.names,
                    format!("{:?}{:?}{:?}", left.params, left.defaults, left.return_ty)
                ),
                (
                    right.names,
                    format!(
                        "{:?}{:?}{:?}",
                        right.params, right.defaults, right.return_ty
                    )
                ),
                "`{}` is carried by two different signatures of `{}`",
                left.symbol,
                left.name
            );
        }
    }
}
