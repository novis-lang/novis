//! Novis's Tier 0 standard library: every `Core` member, written in native
//! Rust, plus the signature registry the compiler resolves a call against.
//!
//! `rule:core-api/five-placements`
//! makes this "compiled into the binary, native, direct heap access, no
//! boundary," and `docs/agent/loop-goal.md` records that it is meant literally:
//! no part of `Core` is written in Novis. `rule:core-api/shape-rules`
//! fixes every member's *shape* and [docs/spec/01-core-library.md](/docs/spec/01-core-library.md)
//! is authoritative for every *signature* — this crate restates neither. It
//! holds the two things a signature on paper cannot be: a resolvable entry in
//! [`registry::CLASSES`], and a callable symbol in [`symbols`].
//!
//! # One module per domain; adding a class is two lines
//!
//! A domain module ([`arr`], [`json`], [`math`], [`regex`], [`mod@str`], [`time`])
//! holds everything about its class: the
//! implementations, each an `rule:errors/propagation` helper entry point; a `pub const CLASS`
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
//! alone. `granularity` holds `rule:types/string-is-utf8`'s answer to "what unit does a
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
//! `rule:errors/propagation` makes normative for
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
//!   callee owns its parameters — and it is what `rule:core-api/shape-rules`'s R3 purity rule
//!   makes safe: no `Core` member stores its argument.
//! * **A returned heap value carries one fresh reference**, which the caller
//!   owns, exactly like `nvs_str_concat`'s result.
//! * **Failure is a `Fault`**, which becomes `rule:errors/propagation`'s `THROWN` or `FATAL`
//!   status; nothing unwinds. A `Fault::thrown_as` names which of
//!   [docs/spec/01-core-library.md](/docs/spec/01-core-library.md)
//!   § 10's classes a `catch` will see — `Core\Json::decode` answers with
//!   `ParseError` — and a bare `Fault::thrown` means `RuntimeError`, which is
//!   what a failure with nothing more specific to say is.
//!
//! # Every shape a §§ 1–12 signature writes can be stated
//!
//! [`registry::CoreTy`] spells the whole of what those sections' signature
//! column writes, and each variant's own doc comment is the home for what it
//! is and what it costs: a union in either direction, a `Core`-owned enum, a
//! `Core`-owned instance ([`instance`] is the value behind it), a sequence, a
//! `decimal`, `rule:core-api/shape-rules` R2's options bag, a shape parameter
//! carrying its arms, a variadic tail, and
//! `rule:expressions/nullable-conversion`'s `?T` — including as a parameter
//! defaulting to `null`, which `Core\Str::slice`'s `?int $length = null`
//! declares. A class constant sits beside them on [`registry::CoreConst`]
//! rather than in the enum, since a constant has a value and no signature.
//!
//! A **named** or `...spread` argument at such a call is spelled too. Every
//! parameter's name is on the row — [`registry::CoreMethod::names`], one per
//! positional slot, the trailing bag under [`registry::OPTIONS_NAME`] — taken
//! from the signature column of `docs/spec/01-core-library.md` and held there
//! by `every_registry_rows_names_are_the_specs_signature_column`, which is
//! `rule:core-api/shape-rules` R2's "a parameter's name is compatibility
//! surface, versioned where its type is". `nvs_types::core_lib` reads them
//! into `MethodSig::param_names`, so a `name:` at a `Core` call site resolves
//! through the machinery a user-declared method's does — the same slot
//! mapping, the same skipped default, and the same refusal for a name that
//! reaches no parameter or reaches a variadic tail.
//! [`registry::ParamDoc::name`] is a *key* into that list rather than a second
//! copy of it.
//!
//! So a member the spec writes and this crate has not registered needs its row
//! and its body, never a spelling — and which members those are is counted
//! rather than listed here, by the walks below.
//!
//! **A type variable is inferred, never declared by user code.**
//! `rule:types/declaration`'s *Revisiting* section scopes `<T>` to
//! declarations the compiler owns, which is exactly what
//! [`registry::CoreTy::Var`] is; `nvs-types` owns the unification and
//! substitution, and its own docs are the home for what that does and does not
//! do yet.
//!
//! # What measures the roster
//!
//! `tests/spec_registry_coverage.rs` walks `docs/spec/01-core-library.md` and
//! asks this registry for a row, in each shape the spec writes one: §§ 1–12's
//! `| Member | Signature | … |` tables, § 13's `| Class | Owns | ADR |` rows,
//! and §§ 14–19's bullets and tables. Every walk carries a checked-in
//! outstanding-members file — `spec-members-outstanding.txt`,
//! `spec-members-compiler-facing-outstanding.txt`,
//! `spec-members-part-two-outstanding.txt` — which **only ever shrinks**:
//! registering a member and striking its line are one edit, and the gate fails
//! on a stale line as loudly as on an unlisted one. A file holding no keys is
//! this project's definition of *registered whole* for the sections it walks,
//! and every key that remains names its owner in a column, read against
//! `docs/agent/goals/`. `tests/conformance_coverage.rs` keeps the other half
//! honest: a registered member with no `.nvst` case that calls it fails
//! `cargo test -p nvs-stdlib`.
//!
//! **What no walk reads is §§ 16–17's rosters**, which the spec states inside
//! an English cell rather than as a shape — `compiler_facing_members`' own doc
//! is where that bound and the reason § 13's otherwise identical table can be
//! read are written down. Those two sections get a class-level walk instead,
//! over `spec-classes-part-two-outstanding.txt`, so a class the spec names and
//! nobody has written is still caught; that a class the registry does hold is
//! missing a member the prose gives it, nothing checks.

pub mod arr;
mod ast;
mod attributes;
mod budget;
mod bus;
mod bytes;
mod cache;
// `pub` for the two predicates `nvs_types::capability` reads — the class's own
// spelling and `rule:security/capability-roster-is-closed`'s roster — so that the checker's refusal holds no
// copy of either. `cldr` is `pub` for the same reason one grammar over.
pub mod cap;
mod channel;
pub mod cldr;
pub mod cli;
mod command;
mod compress;
mod config;
mod crypto;
mod csrf;
mod csv;
mod cursor;
// `pub` for [`db::check`]'s three refusals, which are `rule:core-classes/db-literal-query-checking`'s half of
// the intrinsic pass and the only things `nvs-types` reads here — for `cap`'s
// reason exactly: the checker's refusal holds no second copy of a grammar this
// crate already owns.
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
mod jwe;
mod jwt;
mod keyring;
mod log;
mod mail;
pub mod math;
pub mod metrics;
mod mime;
mod multipart;
mod net;
mod objmap;
mod objset;
mod ordering;
mod os;
mod out;
mod password;
pub mod path;
// `pub` because the whole module is for a reader outside this crate:
// `rule:php-migration/every-php-builtin-is-a-completion-candidate` is an editor
// feature, and `nvs-lsp` is the one that offers the candidates. Nothing here is
// a `Core` member, so it takes no row in [`registry`] and no address below.
pub mod php_names;
mod process;
mod program;
// `pub` for [`queue::schema`] and the statements beside it: `rule:core-classes/queue-storage-is-a-table`'s schema is written
// beside the statements that read its columns, and `nvs queue migrate` in `nvs-cli` is a second
// crate that has to run it — in whichever dialect the block it was pointed at speaks. The
// members themselves are reached the way every other class's are, through [`registry`].
pub mod queue;
pub mod random;
mod ratelimit;
mod reflect;
pub mod regex;
pub mod registry;
// `pub` for [`request::is_known_verb`] alone: `nvs_server::Reply::not_implemented`'s door refuses a verb outside
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
// `pub` for [`session::cookie_in`] alone, and for [`request`]'s reason one notch along: the door
// verifies a CSRF token against the session a request rides under
// (`rule:security/csrf-is-on-by-default`), before an isolate exists, and the cookie's name and its
// default are this module's. The members themselves are reached the way every other class's are,
// through [`registry`].
pub mod session;
mod signal;
mod signature;
mod signed_cookie;
mod socket;
mod sort;
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
mod xml;
mod zip;

/// `rule:core-classes/derive-attribute`'s derived-codec field list, re-exported from where it is
/// *consumed*.
///
/// The struct lives in `nvs-runtime` because that is the deepest crate that
/// holds one — `nvs_runtime::ClassDesc` carries the finished list. `nvs-types`
/// and `nvs-ir` each produce a stage of it and neither depends on the runtime
/// directly, so they reach it through this crate, which they already treat as
/// the home of the `Core` contract.
pub use nvs_runtime::{CodecElement, CodecField, CodecTy, EnumCases, FieldDefault};

/// `rule:core-classes/html-literal`'s folded constant needs a `Core` class's
/// descriptor address while it is being compiled, and `instance` is where the
/// process's one table lives. Re-exported rather than moved, so the roster
/// stays beside the table it reads.
pub use instance::class_descriptors;

/// The checker's question about a `Core` class's identity: whether a value can
/// be an instance of one at all, which is what `nvs_types` admits a written
/// name as `instanceof`'s right-hand side on. Re-exported beside the roster
/// above because it is the same skip asked one class at a time.
pub use instance::class_has_instances;

/// The resolver an embedder installs on a context at boot, so a serialized
/// `Core` instance resolves on the way back in — `nvs_runtime::Ctx`'s
/// `set_core_classes`, whose one caller is `nvs_codegen::Unit::install_in`.
/// Re-exported beside the roster above because both are the same leaked table
/// read from outside this crate.
pub use instance::core_class_desc;

/// `Core\Cache::process()`'s tier, created before the cores that will share it —
/// `nvs serve` calls this as its fleet starts. The function's own doc owns why a
/// tier that no caller armed is still correct.
pub use cache::arm_process_tier;

/// `rule:config/a-fleet-entry-fires-at-most-once-under-a-lease`'s lease over the
/// shared tier, which `nvs serve` is the only caller of — its own doc owns why
/// the ticker cannot reach this store itself. Re-exported rather than moved, for
/// [`arm_process_tier`]'s reason: it is two commands on a connection this crate
/// owns, and it belongs beside that connection.
pub use cache::Lease;

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
        // `rule:routing/link-name-and-params-are-checked`'s two prepared link implementations, which no member row
        // names on purpose — `router::link`'s own docs own why one member has
        // two entry points.
        .chain(router::link::SYMBOLS)
        // `rule:security/isolate-shares-nothing`'s two constructs, which are syntax rather than members and
        // so have no row to be found through — `script`'s own module doc owns
        // why a `spawn script`/`await` symbol may not be callable by name.
        .chain([
            script::SPAWN_SYMBOL,
            script::SPAWN_METHOD_SYMBOL,
            script::AWAIT_SYMBOL,
        ])
        // The sink carriers' row-less symbols, which are constructs rather than
        // members for the same reason those two are: `rule:core-classes/html-auto-escape`'s
        // lift is `as` on a source literal, § 5's `Markup + Markup` and ADR
        // 0086 § 2's `Text + Text` are an operator, and
        // `rule:core-classes/html-literal`'s pair is a markup literal's segment
        // and hole. Each is reachable only from the lowering of the construct
        // that spells it, which is what a row would undo — `html`'s and
        // `cli`'s own module docs own why.
        .chain([
            html::MARKUP_SYMBOL,
            html::MARKUP_CONCAT_SYMBOL,
            html::ESCAPE_TEXT_SYMBOL,
            html::MARKUP_TEXT_SYMBOL,
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
        .or_else(|| budget::address(symbol))
        .or_else(|| bytes::address(symbol))
        .or_else(|| cache::address(symbol))
        .or_else(|| cap::address(symbol))
        .or_else(|| channel::address(symbol))
        .or_else(|| cldr::address(symbol))
        .or_else(|| cli::address(symbol))
        .or_else(|| command::address(symbol))
        .or_else(|| compress::address(symbol))
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
        .or_else(|| jwe::address(symbol))
        .or_else(|| jwt::address(symbol))
        .or_else(|| log::address(symbol))
        .or_else(|| mail::address(symbol))
        .or_else(|| math::address(symbol))
        .or_else(|| metrics::address(symbol))
        .or_else(|| mime::address(symbol))
        .or_else(|| net::address(symbol))
        .or_else(|| objmap::address(symbol))
        .or_else(|| objset::address(symbol))
        .or_else(|| os::address(symbol))
        .or_else(|| out::address(symbol))
        .or_else(|| password::address(symbol))
        .or_else(|| path::address(symbol))
        .or_else(|| process::address(symbol))
        .or_else(|| program::address(symbol))
        .or_else(|| queue::address(symbol))
        .or_else(|| zip::address(symbol))
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
        .or_else(|| signal::address(symbol))
        .or_else(|| signature::address(symbol))
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
        .or_else(|| xml::address(symbol))
        .unwrap_or_else(|| panic!("nvs-stdlib registers `{symbol}` with no implementation address"))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The frozen WebCrypto vector set, declared here for [`granting`]'s reason
    /// and stated once in its own module doc: a helper every module's cases
    /// share is declared inside this module rather than beside the modules that
    /// ship, because the capability guard scans a file down to its first
    /// `#[cfg(test)]`.
    pub(crate) mod vectors;

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

    /// The process's one outbound TLS client, and the self-signed `localhost`
    /// certificate it trusts — declared here for [`granting`]'s reason and one
    /// of its own.
    ///
    /// `rule:security/one-tls-client` is one client per process, and
    /// `nvs_host::tls::configure` settles exactly that: it answers a second call
    /// `AlreadyExists`, and it refuses outright once any session has run. So a
    /// case in `http::transport` that needs a trusted origin and one in `cache`
    /// that needs a handshake to go out at all cannot each build one — whichever
    /// ran first would decide the other's outcome from another module. This
    /// function is the one place it is built, and every case that needs a
    /// session calls it before opening one.
    ///
    /// The `OnceLock` is what makes the order not matter. The file is written
    /// rather than the certificate handed over directly, because the seam under
    /// test starts at the path an operator wrote and a case that skipped the
    /// encoding would be asserting against a path nothing runs.
    pub(crate) fn outbound_client() -> &'static (rustls::pki_types::CertificateDer<'static>, Vec<u8>)
    {
        static TRUSTED: std::sync::OnceLock<(rustls::pki_types::CertificateDer<'static>, Vec<u8>)> =
            std::sync::OnceLock::new();
        TRUSTED.get_or_init(|| {
            let issued = rcgen::generate_simple_self_signed(vec!["localhost".to_owned()])
                .expect("the loopback certificate could not be generated");
            let path =
                std::env::temp_dir().join(format!("nvs-http-roots-{}.pem", std::process::id()));
            std::fs::write(&path, issued.cert.pem()).expect("the roots file could not be written");
            nvs_host::tls::configure(&nvs_host::tls::ClientPolicy {
                roots: vec![path.to_string_lossy().into_owned()],
                ..nvs_host::tls::ClientPolicy::default()
            })
            .expect("the process's outbound client had already been built");
            (
                issued.cert.der().clone(),
                issued.signing_key.serialize_der(),
            )
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
                // `rule:routing/link-name-and-params-are-checked`'s two prepared link entry points, which belong
                // to `Core\Router::url`/`::urlAbsolute` and to no row of their
                // own — see `router::link`.
                + router::link::SYMBOLS.len()
                // `rule:security/isolate-shares-nothing`'s `spawn script` — in its two entry forms, which
                // are two symbols and one construct — and `await`, `rule:core-classes/html-auto-escape`'s `as Markup` and `Markup + Markup`, `rule:tooling/styling-is-a-value-not-a-grammar`'s
                // `Text + Text`, and `rule:core-classes/html-literal`'s markup
                // literal, whose segment and hole are a second pair behind one
                // construct: eight symbols behind six constructs, each syntax
                // rather than a call, so none of them has a row either — see
                // `script`'s, `html`'s and `cli`'s module docs.
                + 8
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
    /// `rule:classes/no-traits`'s
    /// `by` delegation, whose entire content is that the two rows *are* one
    /// member: `Core\Db\Transaction implements Queryable by $connection`
    /// (`rule:core-classes/db-transactions`, and the first
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

    /// `held` as a `string` [`nvs_runtime::Value`] — every argument the members
    /// below take except `Core\Regex::compile`'s four option slots and
    /// `Core\Time::parse`'s zone.
    fn text(held: &str) -> nvs_runtime::Value {
        nvs_runtime::Value::str(nvs_runtime::NvsStr::new(held.as_bytes()))
    }

    /// Runs `member` at the `rule:errors/propagation` boundary compiled code
    /// reaches it at, over `leading` and then the `strings` built and released
    /// here and then `rest`, both of which stay the caller's — and answers the
    /// sentence a `catch` would read, since the verdict and its words are what
    /// these cases compare rather than the object.
    ///
    /// `leading` is what a roster puts ahead of everything a call site wrote:
    /// [`registry::PREPARED_MEMBERS`]' word is the one these cases pass, and it
    /// is empty for a member on no roster at all.
    fn ran(
        ctx: &mut nvs_runtime::Ctx,
        member: nvs_runtime::NvsFn,
        leading: &[nvs_runtime::Value],
        strings: &[&str],
        rest: &[nvs_runtime::Value],
    ) -> Result<(), String> {
        let mut args: Vec<nvs_runtime::Value> = leading.to_vec();
        args.extend(strings.iter().copied().map(text));
        args.extend_from_slice(rest);
        let answer = nvs_runtime::call(member, ctx, &args);
        #[expect(
            unsafe_code,
            reason = "this frame built the `string` arguments and owns whatever the \
                      member answered with, and every member here borrows rather \
                      than consumes"
        )]
        unsafe {
            for held in &args[leading.len()..leading.len() + strings.len()] {
                held.release();
            }
            if let Ok(value) = answer {
                value.release();
            }
        }
        answer.map(|_| ()).map_err(|_| {
            ctx.take_pending()
                .map(std::borrow::Cow::into_owned)
                .unwrap_or_default()
        })
    }

    /// `rule:expressions/preparation-preserves-behaviour` over all four
    /// grammars the checker prepares — the format template, the regex pattern,
    /// the URI and the CLDR date pattern — each asserted where the two paths
    /// could actually diverge: the entry point `nvs_types::intrinsics` calls,
    /// beside the member a request reaches.
    ///
    /// Asserted by **agreement**, not against a second list of expected
    /// refusals, which would pass while both halves drifted together. A
    /// malformed literal is refused on both sides in the same words, the
    /// runtime's carrying only the `Core\Class::member(): ` prefix a throw adds
    /// and a diagnostic does not; a well-formed one is accepted on both, and
    /// the regex half also pins the [`regex::Tier`] the fold records, which is
    /// the one prepared artifact a later stage reads back.
    /// [`format`]'s own `a_prepared_literal_and_its_runtime_twin_share_one_implementation`
    /// is this taken deeper on the one grammar read against the call's other
    /// arguments.
    ///
    /// Two asymmetries are asserted as what they are rather than smoothed over.
    /// `Core\Regex::compile`'s options are not folded, so a flagged call's
    /// *verdict* is compared and its wording is not: the flags reach the engine
    /// as a balanced `(?ims:…)` wrapper, which moves no pattern across the
    /// accept boundary but does move what the second engine's own error quotes.
    /// And `Core\Time::parse` refuses a zonal field a well-formed pattern may
    /// carry, which is a rule about that member rather than about the grammar
    /// (`nvs_types::intrinsics`' gap 3), so civil patterns are what the
    /// accepted half offers it.
    #[test]
    fn every_intrinsic_literal_prepares_the_artifact_the_runtime_builds() {
        use nvs_runtime::{Ctx, Fault, Value};

        let mut ctx = Ctx::buffered();

        // The format template, whose runtime half is a plain call rather than a
        // member: the renderer and `placeholders` walk one template parser.
        for template in ["%q", "%1$", "%'"] {
            let checked = format::placeholders(template).expect_err("a refusal");
            let Fault::Thrown(_, thrown) = format::format(template, &[]).expect_err("a refusal")
            else {
                panic!("`{template}` refused as something other than a throw");
            };
            assert_eq!(checked.message, thrown, "for `{template}`");
        }
        for (template, arity) in [("%s", 1), ("%2$s %1$s", 2), ("100%% of %d", 1)] {
            format::placeholders(template).expect("a template");
            let given: Vec<Value> = (0..arity).map(|_| Value::int(1)).collect();
            format::format(template, &given).expect("a rendering");
        }

        // The regex pattern, under each option the member carries as well as
        // under none, which is what the fold reads.
        const OPTIONS: [[bool; 4]; 5] = [
            [false, false, false, false],
            [true, false, false, false],
            [false, true, false, false],
            [false, false, true, false],
            [false, false, false, true],
        ];
        for pattern in ["(", "[a-", "*"] {
            let checked = regex::validate(pattern).expect_err("a refusal");
            for options in OPTIONS {
                let thrown = ran(
                    &mut ctx,
                    regex::nvs_core_regex_compile,
                    // A refused literal prepared nothing, which is the word a
                    // call site whose pattern never folded carries too.
                    &[Value::int(regex::PREPARED_NONE)],
                    &[pattern],
                    &options.map(Value::bool),
                )
                .expect_err("a refusal");
                if options == OPTIONS[0] {
                    assert_eq!(
                        thrown,
                        format!("Core\\Regex::compile(): {checked}"),
                        "for `{pattern}`"
                    );
                }
            }
        }
        for (pattern, tier) in [
            (r"^\d+$", regex::Tier::Linear),
            (r"(?<=a)b", regex::Tier::Backtracking),
        ] {
            assert_eq!(
                regex::validate(pattern).expect("a pattern"),
                tier,
                "for `{pattern}`"
            );
            for options in OPTIONS {
                ran(
                    &mut ctx,
                    regex::nvs_core_regex_compile,
                    // Told what the fold settled, which is the whole of what
                    // `registry::PREPARED_MEMBERS` carries: the runtime half is
                    // asserted against the *prepared* call and not only against
                    // the untold one.
                    &[Value::int(tier.prepared_code())],
                    &[pattern],
                    &options.map(Value::bool),
                )
                .expect("the runtime compiles what the checker prepared");
            }
        }

        // The URI, both of whose throwing steps the fold runs: the grammar and
        // the port's range.
        for written in [
            "http://[::1/",
            "http://example.com:70000/",
            "http://exa mple.com/",
        ] {
            let checked = uri::validate(written).expect_err("a refusal");
            let thrown = ran(&mut ctx, uri::nvs_core_uri_parse, &[], &[written], &[])
                .expect_err("a refusal");
            assert_eq!(
                thrown,
                format!("Core\\Uri::parse(): {checked}"),
                "for `{written}`"
            );
        }
        for written in [
            "https://example.com/a?b=c#d",
            "mailto:novis@example.com",
            "/relative/path",
        ] {
            uri::validate(written).expect("a URI reference");
            ran(&mut ctx, uri::nvs_core_uri_parse, &[], &[written], &[])
                .expect("the runtime parses what the checker prepared");
        }

        // The CLDR pattern, through the member that takes one as a `string`
        // beside the text it reads.
        let named = text("UTC");
        let zone = nvs_runtime::call(time::nvs_core_time_zone_of, &mut ctx, &[named])
            .expect("`UTC` is an identifier the zone database carries");
        #[expect(
            unsafe_code,
            reason = "this frame built the identifier the zone was named by"
        )]
        unsafe {
            named.release();
        }
        for pattern in ["j", "'abc", "V"] {
            let checked = cldr::validate(pattern).expect_err("a refusal");
            let thrown = ran(
                &mut ctx,
                time::nvs_core_time_parse,
                // A pattern the fold refused prepared nothing, so the call site
                // carries the zero word and the runtime makes the refusal
                // itself — which is what these cases compare.
                &[Value::int(cldr::PREPARED_NONE)],
                &["2026-09-16 10:30:00", pattern],
                &[zone],
            )
            .expect_err("a refusal");
            assert_eq!(
                thrown,
                format!("Core\\Time::parse(): {checked}"),
                "for `{pattern}`"
            );
        }
        for (pattern, subject) in [
            ("yyyy-MM-dd HH:mm:ss", "2026-09-16 10:30:00"),
            ("yyyy-MM-dd'T'HH:mm:ss", "2026-09-16T10:30:00"),
        ] {
            cldr::validate(pattern).expect("a pattern");
            ran(
                &mut ctx,
                time::nvs_core_time_parse,
                // Told what the fold settled, as the regex cases above are:
                // the runtime half is asserted against the *prepared* call and
                // not only against the untold one.
                &[Value::int(cldr::PREPARED_PATTERN)],
                &[subject, pattern],
                &[zone],
            )
            .expect("the runtime reads what the checker prepared");
        }
        #[expect(unsafe_code, reason = "this frame owns the zone the cases shared")]
        unsafe {
            zone.release();
        }
    }
}
