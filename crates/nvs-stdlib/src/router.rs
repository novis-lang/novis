//! `Core\Http\Method` — the closed set of verbs a route is declared under —
//! and `Core\Router`'s link half, which is as much of
//! [ADR 0077](../../../../docs/adr/0077-compile-time-routing.md)'s router as
//! exists today.
//!
//! # Why the enum is here rather than in a module of its own
//!
//! [`crate::registry::ENUMS`] states the rule: one line per enum, declared
//! beside the member that takes it. Three take this one — § 1's
//! `#[Route(method: …)]` payload, § 4's `Core\Router::match`/`methodsFor`, and
//! spec § 15's `Core\Request::method`, which is what answers with one. The
//! first of those is the only one on disk, and it reads the name through
//! `nvs_types::routes::OPTIONS` rather than through this constant, because
//! `nvs-types` interns a roster row by name.
//!
//! Spec § 17's `Core\Http\Client` shares the `Core\Http` prefix and is a
//! different domain ([ADR 0074](../../../../docs/adr/0074-http-defaults-safe-and-finite.md)
//! owns it). When it lands it names *this* enum: a namespace prefix is not a
//! module boundary, and a second `Core\Http\Method` would be two rosters
//! disagreeing about what `Post` is worth.
//!
//! # What the cases are, and what the order buys
//!
//! [ADR 0074](../../../../docs/adr/0074-http-defaults-safe-and-finite.md) § 7
//! names eight verbs and [ADR 0096](../../../../docs/adr/0096-a-route-without-a-declared-access-decision-does-not-compile.md)
//! § 4 names four of them as the ones CSRF enforcement covers. Those four are
//! this enum's contiguous *tail*, the same arrangement — and for the same
//! reason — as [`crate::hash::STRONG`]'s: a rule over a set of cases becomes a
//! bound rather than a match arm nobody remembers to extend.
//!
//! `CONNECT` is deliberately not a case. It establishes a proxy tunnel, so it
//! is neither something a `#[Route]` may be declared under nor something
//! [ADR 0058](../../../../docs/adr/0058-outbound-request-policy.md)'s pinned
//! client sends; a case a program can write and pass nowhere is surface with no
//! meaning behind it, which is [`crate::registry::ENUMS`]' own test for
//! admitting an entry.
//!
//! # Known gaps
//!
//! 1. **The route table is not built**, so nothing yet turns a `#[Route]`
//!    declaration into a row [`CLASS`]'s two members could read: every name
//!    they are given is unknown, and [`no_such_route`] is the whole of the
//!    lookup. `nvs_types::routes`' own gap 1 owns which of ADR 0077's compile
//!    errors are still unreported, and § 4's *literal*-name check is among
//!    them — so `examples/routes.nvs` compiles and then throws, where after the
//!    table lands a bad literal name would not compile at all.
//! 2. **Neither member laundering is real yet.** `url` percent-encodes each
//!    substituted value and prepends the mount prefix
//!    ([ADR 0097](../../../../docs/adr/0097-development-server-and-proxied-origin.md)
//!    § 3), and `urlAbsolute` prepends ADR 0102 § 6's configured origin. Both
//!    are work over a row gap 1 has none of, and there is no mount to read an
//!    origin from in a program run off the command line.
//! 3. **`match` and `methodsFor` are absent.** § 4's other two members answer a
//!    *request*, which lands with the server; `docs/agent/loop-goal.md`
//!    § *Standing decisions* keeps `::match` out of scope on purpose, and
//!    `Core\Router\Match` — the type both answer with — does not exist either.
//! 4. **Nothing converts a request's verb into one of these cases.** There is
//!    no `Core\Request`, so the *parse* — an unrecognized verb answering 501
//!    rather than becoming a case — has no home yet. It belongs beside that
//!    class, which is what makes the closed set safe: a verb outside this
//!    roster never reaches a route table at all.

use nvs_runtime::Fault;

use crate::registry::{CoreClass, CoreEnum, CoreMethod, CoreTy};

/// `Core\Http\Method`'s fully-qualified name, written once so the registry row
/// and every message quoting it cannot drift apart.
pub(crate) const METHOD_NAME: &str = r"Core\Http\Method";

/// ADR 0077 § 1's `Core\Http\Method` — the eight verbs
/// [ADR 0074](../../../../docs/adr/0074-http-defaults-safe-and-finite.md) § 7
/// names, safe ones first so that ADR 0096 § 4's CSRF set is the contiguous
/// tail from `Post` on.
///
/// The integers are each case's own constant, written out rather than
/// auto-incremented, per [`CoreEnum::cases`]. Unlike [`crate::hash::DIGEST`]'s
/// they are not read back out of an argument slot by anything yet — but the
/// tail above is a bound a later CSRF check is meant to take, so reordering
/// this list is a behaviour change rather than a cosmetic one. The test module
/// below is what holds that tail in place.
pub(crate) const METHOD: CoreEnum = CoreEnum {
    name: METHOD_NAME,
    cases: &[
        ("Get", 0),
        ("Head", 1),
        ("Options", 2),
        ("Trace", 3),
        ("Post", 4),
        ("Put", 5),
        ("Patch", 6),
        ("Delete", 7),
    ],
};

/// `Core\Router`'s fully-qualified name, in one place so the registry row and
/// every message quoting it cannot drift apart.
pub(crate) const NAME: &str = r"Core\Router";

/// `$params` — ADR 0077 § 4 writes it `array<string, mixed>`, and that is a
/// spelling the type system has no form for: `nvs_types::ty::Ty::Array` carries
/// one element type, because a Novis array's keys are `int|string` by
/// construction and are not part of its type. So the row declares the half that
/// *is* representable, `array<mixed>`, and the key rule is enforced where it
/// can be — § 4 makes a literal key that names neither a capture nor a declared
/// `#[Query]` parameter a compile error, which is a question about this route's
/// own captures rather than about a type.
const PARAMS: CoreTy = CoreTy::Array(&CoreTy::Mixed);

/// Spec § 15's `Core\Router`, as much of it as ADR 0077 § 4's link half needs.
///
/// `match` and `methodsFor` are deliberately absent — they answer *a request*,
/// which belongs with the server (`docs/agent/loop-goal.md` § *Standing
/// decisions* keeps `::match` out of scope) — so what is here is the two
/// members that build a link, which is what a program does with the table
/// before there is a server to match against it.
pub(crate) const CLASS: CoreClass = CoreClass {
    name: NAME,
    methods: &[
        CoreMethod {
            name: "url",
            params: &[CoreTy::Str, PARAMS],
            defaults: &[],
            return_ty: CoreTy::Str,
            symbol: "nvs_core_router_url",
        },
        CoreMethod {
            name: "urlAbsolute",
            params: &[CoreTy::Str, PARAMS],
            defaults: &[],
            return_ty: CoreTy::Str,
            symbol: "nvs_core_router_url_absolute",
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
        "nvs_core_router_url" => (nvs_core_router_url as *const ()).cast(),
        "nvs_core_router_url_absolute" => (nvs_core_router_url_absolute as *const ()).cast(),
        _ => return None,
    })
}

/// The answer both members give for a name the table does not hold, which is
/// **every** name until gap 2 above closes.
///
/// A throw rather than an abort, and that is the difference from
/// [`crate::program`]: `implementing<T>()` is expanded away in `nvs check`, so
/// reaching its helper is a compiler bug. These two are ordinary runtime
/// members — ADR 0077 § 4 says a *computed* `$name` throws — so the throw is a
/// real answer a program can catch, and it stays the answer for an unknown name
/// after the table lands. What changes then is only which names are unknown.
fn no_such_route(member: &str, args: &[nvs_runtime::Value]) -> Fault {
    let name = args[0].as_text().unwrap_or("<not a string>");
    Fault::thrown(format!(
        "Core\\Router::{member}(): no route is named `{name}`. The compile-time route table is \
         not built yet (ADR 0077 § 5), so no name resolves"
    ))
}

nvs_runtime::nvs_helper! {
    /// `Core\Router::url(string $name, array<mixed> $params): string` — ADR 0077
    /// § 4's launderer for the URL-path sink.
    ///
    /// It percent-encodes each substituted value and prepends the request's
    /// mount prefix, neither of which it can do over a table that does not
    /// exist; see [`no_such_route`] and the module's gap 2.
    fn nvs_core_router_url(_ctx, args: [2]) {
        Err(no_such_route("url", args))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Router::urlAbsolute(string $name, array<mixed> $params): string` —
    /// [`nvs_core_router_url`] with ADR 0102 § 6's configured origin in front.
    ///
    /// The origin is per mount, falling back to `[app] origin`, and is never
    /// derived from `Host` or `X-Forwarded-Host`. Nothing here resolves one:
    /// the route lookup refuses first, and there is no mount to read an origin
    /// from in a program run off the command line at all.
    fn nvs_core_router_url_absolute(_ctx, args: [2]) {
        Err(no_such_route("urlAbsolute", args))
    }
}

#[cfg(test)]
mod tests {
    use super::METHOD;

    /// The tail is what the doc comment promises, asserted rather than
    /// described: an inserted case that pushes `Post` along breaks the bound a
    /// later CSRF check is meant to take, and nothing else would notice. The
    /// bound is written here rather than exported as a constant — no caller
    /// exists to take it yet, and a `pub(crate)` nothing reads is dead code.
    #[test]
    fn the_csrf_set_is_the_contiguous_tail() {
        let unsafe_cases: Vec<&str> = METHOD
            .cases
            .iter()
            .filter(|(_, value)| *value >= 4)
            .map(|(name, _)| *name)
            .collect();
        assert_eq!(unsafe_cases, ["Post", "Put", "Patch", "Delete"]);
    }

    /// Each case's value is its own index, so the tail bound above is also the
    /// position ADR 0074 § 7's roster reads at.
    #[test]
    fn every_case_carries_a_distinct_value_in_declaration_order() {
        for (index, (_, value)) in METHOD.cases.iter().enumerate() {
            assert_eq!(*value, i64::try_from(index).unwrap());
        }
    }
}
