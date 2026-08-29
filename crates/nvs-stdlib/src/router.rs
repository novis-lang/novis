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
//! reason — as `crate::hash`'s private `STRONG`: a rule over a set of cases becomes a
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

use nvs_runtime::{Fault, HelperResult, NvsStr, Tag, Value};

use crate::registry::{CoreClass, CoreEnum, CoreMethod, CoreTy};
use crate::uri::{Form, encode};

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

/// The two symbols ADR 0077 § 4's **folded** link is lowered to, and the wire
/// format they read argument 0 as.
///
/// Neither is a [`CoreMethod`] row, and that is the point: a program calls
/// `Core\Router::url`, and `nvs_ir::lower` redirects the call here whenever
/// `nvs_types::links` resolved its literal name against the compile-time table.
/// So the *member* is one, and which of the two implementations answers is a
/// property of what the compiler could prove — the same arrangement ADR 0057
/// § 4 states for every prepared literal: one implementation, reached at two
/// entry points, never two implementations.
///
/// **Argument 0 is the route's path, already split.** `nvs_types::routes`'
/// `link_pieces` reads § 2's grammar — the only reading of it anywhere — and
/// `nvs_types::UrlPiece::prepared` writes its answer out in this format:
/// pieces separated by [`link::PIECE_SEPARATOR`], each one a tag byte from the four
/// constants below followed by its text. Reading it back is a `split` and a
/// byte test, so no second parser of a path exists to disagree with the first.
pub mod link {
    /// `nvs_core_router_link` — `Core\Router::url` with the lookup already
    /// made.
    pub const SYMBOL: &str = "nvs_core_router_link";
    /// `nvs_core_router_link_absolute` — the same with ADR 0102 § 6's
    /// configured origin in front.
    pub const ABSOLUTE_SYMBOL: &str = "nvs_core_router_link_absolute";
    /// What separates two pieces. `\u{1}` because a path segment cannot hold
    /// one: § 2's capture names are identifiers and its literal segments come
    /// out of a `#[Route]` payload that a control byte would already have made
    /// unusable as a URL.
    pub const PIECE_SEPARATOR: char = '\u{1}';
    /// A literal segment, its leading `/` included — copied out verbatim.
    pub const LITERAL: u8 = b'L';
    /// `{name}`: `/` and the percent-encoded value at `name`.
    pub const REQUIRED: u8 = b'R';
    /// `{name?}`: [`REQUIRED`], or the whole segment dropped.
    pub const OPTIONAL: u8 = b'O';
    /// `{name...}`: [`REQUIRED`] with the value's own `/`s left alone.
    pub const REST: u8 = b'*';

    /// Both symbols, for [`crate::symbols`], which builds the JIT's roster out
    /// of the member rows and so would never reach an implementation no
    /// [`super::CoreMethod`] names — the same reason
    /// [`crate::registry::CONSTRUCTORS`] is chained there.
    pub const SYMBOLS: [&str; 2] = [SYMBOL, ABSOLUTE_SYMBOL];
}

/// The address of one of *this* module's symbols, or `None` for a symbol that
/// belongs to another domain. See [`crate::symbols`].
pub(crate) fn address(symbol: &str) -> Option<*const u8> {
    Some(match symbol {
        "nvs_core_router_url" => (nvs_core_router_url as *const ()).cast(),
        "nvs_core_router_url_absolute" => (nvs_core_router_url_absolute as *const ()).cast(),
        "nvs_core_router_link" => (nvs_core_router_link as *const ()).cast(),
        "nvs_core_router_link_absolute" => (nvs_core_router_link_absolute as *const ()).cast(),
        _ => return None,
    })
}

/// One `$params` value as the text a path segment carries.
///
/// [`nvs_runtime::value_to_string`]'s own rules, plus the release its contract
/// requires: a `Tag::Str` operand comes back as itself with one fresh
/// reference, so the bytes are copied out and the reference dropped here.
fn segment_text(value: Value, member: &str, key: &str) -> Result<String, Fault> {
    let text = nvs_runtime::value_to_string(value).map_err(|_| {
        Fault::thrown(format!(
            "Core\\Router::{member}(): `{key}` holds a value with no text form, so there is \
             nothing a path segment could be built out of it"
        ))
    })?;
    let owned = text
        .as_text()
        .ok_or_else(|| Fault::fatal("`value_to_string` answered something that is not a string"))?
        .to_owned();
    #[expect(
        unsafe_code,
        reason = "`value_to_string` hands back exactly one fresh reference, and \
                  the text has been copied out of it"
    )]
    unsafe {
        text.release();
    }
    Ok(owned)
}

/// The prepared path with every capture substituted — the whole of both link
/// helpers below, since they differ only in what stands in front of it.
///
/// Each substituted value is percent-encoded under
/// [`Form::Component`](crate::uri::Form::Component), which is § 4's launder for
/// the URL-path sink: a value holding a `/` cannot climb out of its segment.
/// A `{name...}` is the one exception, and § 2 is why — that form *is* every
/// remaining segment, so its `/`s are structure rather than content, and each
/// segment between them is encoded on its own.
///
/// **What the path did not take becomes the query string**, which is
/// ADR 0102 § 6's other half: the prepared pieces name every capture, so a
/// `$params` key left over once they have been substituted is by construction
/// not one, and § 6 makes it a query parameter. It is written by
/// [`crate::uri::build`] — `Core\Uri::buildQuery`'s own pass, run over the same
/// array with the captures omitted — so a link's query string is
/// `http_build_query`'s spelling down to its nesting and its `Form::FormValue`
/// escaping, rather than a second convention a reader would have to learn. The
/// walk is O(`$params`) whether or not anything is left over; `$params` is a
/// literal written at the call site and is small.
///
/// The **refusal** half of § 6 — a key that is neither a capture nor a declared
/// `#[Query]` parameter is a compile error — is not here and cannot be: it
/// needs the attribute, and `nvs_types::links`' gap 1 owns why. Until then
/// every leftover key is a query parameter rather than a typo.
fn substitute(template: &str, params: &Value, member: &str) -> Result<String, Fault> {
    let raw = params.array_ptr().ok_or_else(|| {
        Fault::fatal(format!(
            "Core\\Router::{member} expected {:?} for `$params`, got tag {}",
            Tag::Array,
            params.tag_byte()
        ))
    })?;
    let params = crate::arr::borrowed(raw);
    let mut out = String::with_capacity(template.len());
    // Every key the path consumed, in the order the pieces name them — what
    // the query string below is the complement of.
    let mut captures: Vec<&str> = Vec::new();
    for piece in template.split(link::PIECE_SEPARATOR) {
        let (tag, key) = piece.as_bytes().split_first().ok_or_else(|| {
            Fault::fatal("a prepared route link's piece is a tag byte and its text")
        })?;
        let key = std::str::from_utf8(key)
            .map_err(|_| Fault::fatal("a prepared route link is built out of `str`"))?;
        if *tag == link::LITERAL {
            out.push_str(key);
            continue;
        }
        captures.push(key);
        let Some(value) = params.get(key.as_bytes()) else {
            if *tag == link::OPTIONAL {
                continue;
            }
            return Err(Fault::thrown(format!(
                "Core\\Router::{member}(): `$params` holds no `{key}`, which this route's path \
                 captures"
            )));
        };
        let text = segment_text(value, member, key)?;
        out.push('/');
        if *tag == link::REST {
            let encoded: Vec<String> = text
                .split('/')
                .map(|segment| encode(segment.as_bytes(), Form::Component))
                .collect();
            out.push_str(&encoded.join("/"));
        } else {
            out.push_str(&encode(text.as_bytes(), Form::Component));
        }
    }
    let query = crate::uri::build(raw, "Core\\Router", member, &captures)?;
    if !query.is_empty() {
        out.push('?');
        out.push_str(&query);
    }
    Ok(out)
}

nvs_runtime::nvs_helper! {
    /// `Core\Router::url` over a name the compiler resolved — see [`link`].
    ///
    /// The mount prefix ADR 0097 § 3 has this member prepend is empty here and
    /// only here: a program run off the command line is mounted nowhere, and
    /// there is no server yet to be mounted by. The prefix joins in front of
    /// [`substitute`]'s answer when one exists, which is why the substitution
    /// is its own function rather than this body.
    fn nvs_core_router_link(_ctx, args: [2]) {
        let template = link_template(args, "url")?;
        produced(&substitute(template, &args[1], "url")?)
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Router::urlAbsolute` over a name the compiler resolved —
    /// [`nvs_core_router_link`] with ADR 0102 § 6's configured origin in front.
    ///
    /// The origin is per mount, falling back to `[app] origin`, and is never
    /// derived from `Host` or `X-Forwarded-Host` — so this member reads what
    /// was resolved *before* the request ran, out of
    /// [`Ctx::origin`](nvs_runtime::Ctx::origin), and never a value the
    /// request could have influenced. A unit that resolves none throws here
    /// rather than answering with an empty authority in it, which is ADR 0097
    /// § 3's rule at the one place a CLI run can enforce it: § 6 puts the
    /// *boot* error at mount expansion, and there is no mount off the command
    /// line to expand.
    fn nvs_core_router_link_absolute(ctx, args: [2]) {
        let template = link_template(args, "urlAbsolute")?;
        let path = substitute(template, &args[1], "urlAbsolute")?;
        let Some(origin) = ctx.origin() else {
            return Err(Fault::thrown(format!(
                "Core\\Router::urlAbsolute(): no origin is configured for this unit, so `{path}` \
                 has no absolute form. ADR 0102 § 6 refuses to derive one from a request header, \
                 so give `nvs.toml` an `[app] origin`"
            )));
        };
        produced(&format!("{origin}{path}"))
    }
}

/// Argument 0 of a link helper — the prepared template `nvs-ir` emitted.
fn link_template<'a>(args: &'a [Value], member: &str) -> Result<&'a str, Fault> {
    args[0].as_text().ok_or_else(|| {
        Fault::fatal(format!(
            "Core\\Router::{member} expected {:?} for its prepared path, got tag {}",
            Tag::Str,
            args[0].tag_byte()
        ))
    })
}

/// A freshly built `string` result.
fn produced(text: &str) -> HelperResult {
    Ok(Value::str(NvsStr::new(text.as_bytes())))
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
