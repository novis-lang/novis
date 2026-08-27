//! `Core\Uri` — [docs/spec/01-core-library.md](../../../../docs/spec/01-core-library.md)
//! § 12's first table, which has two halves. The **percent-encoding half** is
//! `encodeComponent`/`decodeComponent` and
//! `encodeFormValue`/`decodeFormValue`, which replace PHP's
//! `rawurlencode`/`rawurldecode` and `urlencode`/`urldecode`, plus
//! `parseQuery` and `buildQuery`, which are those four applied to a whole
//! query string. The **grammar half** is `parse` and the `Uri` instance it
//! answers with, plus `$uri->with` and `$uri->resolve` on it — RFC 3986, read
//! by `fluent-uri`, argued four sections below.
//!
//! # Two encodings, because PHP has two and the wire has two
//!
//! The pair split is not an accident of PHP's history that MWL is copying. A
//! percent-encoded *URI component* (RFC 3986 § 2.1) and an
//! `application/x-www-form-urlencoded` *form value* (WHATWG URL § 5) are
//! genuinely different encodings, and they disagree on one byte that matters:
//! a space is `%20` in a path segment and `+` in a form body. A program that
//! uses one where the other belongs produces a literal `+` inside a filename,
//! or a space where a `+` was typed — silently, and only for inputs nobody
//! tested with.
//!
//! So the members are named for **where the answer goes**, not for which C
//! function they came from: `encodeComponent` for anything that is part of a
//! URI, `encodeFormValue` for a value in a query string or a form body. That
//! is the whole of R10's argument-order-and-naming rule applied to the pair —
//! `rawurlencode` and `urlencode` differ by a prefix that says nothing about
//! which is which.
//!
//! # The two byte sets are PHP's, exactly
//!
//! | | left as itself | space | everything else |
//! |---|---|---|---|
//! | `encodeComponent` | `A-Z a-z 0-9 - _ . ~` | `%20` | `%XX`, upper-case hex |
//! | `encodeFormValue` | `A-Z a-z 0-9 - _ .` | `+` | `%XX`, upper-case hex |
//!
//! The one surprise in that table is that `~` is percent-encoded by
//! `encodeFormValue` and not by `encodeComponent`. It is not a typo and it is
//! not simplification worth taking: RFC 3986 § 2.3 added `~` to the unreserved
//! set, `application/x-www-form-urlencoded` is still defined against RFC 1738's
//! older set, and PHP's two functions each follow their own specification.
//! AGENTS.md ranks PHP-compatible observable behaviour (priority 2) above
//! simplicity of the implementation (priority 4), and the cost of collapsing
//! the two sets is paid by whoever compares a signature MWL computed against
//! one PHP computed — an HMAC over a form body differs by one byte and nothing
//! says why. Both spellings decode identically, so nothing is lost by matching.
//!
//! Hex digits are emitted **upper case**, which is PHP's choice and RFC 3986
//! § 2.1's recommendation. Both cases are read on the way back in.
//!
//! # Decoding is total, and a malformed escape is text
//!
//! `%` followed by anything that is not two hex digits — including a `%` at
//! the very end of the string — is left exactly as it stands rather than
//! throwing or dropping. That is PHP's behaviour for both functions, and it is
//! the right one for the position: a decoder sits at the edge of a request,
//! where a byte a client mistyped should not be able to abort a handler that
//! was going to reject the value anyway. The program still sees the `%`, so
//! nothing is silently swallowed.
//!
//! Neither decoder is a *validator*. `decodeComponent` will happily decode text
//! that could never have appeared in a URI; asking whether something is a URI
//! is `Uri::tryParse($text)`, which is `parse` with `null` where it throws
//! ([ADR 0066 § 3a](../../../../docs/adr/0066-nullable-conversion-operator.md)).
//! **There is no `Uri::isValid`** — it and `Uri::tryParse($text) != null` are
//! one predicate, and R17 keeps one of them. Which one is not arbitrary: a
//! validator that is a *separate implementation* from the parser is how
//! PHP's `filter_var(FILTER_VALIDATE_URL)` came to accept user-info that
//! `parse_url` read differently (CVE-2024-5458), so the surviving spelling is
//! the one that cannot drift from `parse` because it *is* `parse`.
//!
//! **`$text as ?Uri` does not compile**, and an earlier revision of ADR 0066
//! said it did. Its § 3 withdrew that closed two-class "parse roster": `as?`
//! spells a *downcast* in every language a reader arrives from, so spelling a
//! parse that way inverted the one intuition the syntax carried — and it never
//! removed the second spelling it was justified by removing, since
//! `Uri::parse` and `$s as ?Uri` both existed. `as` now targets no class at
//! all, and `tryParse` is R5's one admitted `try…`.
//!
//! # Comparison normalizes; `parse` still reports
//!
//! Those two rules look like they disagree and do not, because they happen at
//! different moments. `parse` hands back every component exactly as it was
//! written — that is what makes an allowlist check honest, since the program
//! compares against text a client actually sent. But `http://Example.COM/a%7Eb`
//! and `http://example.com/a~b` are one URI by RFC 3986 § 6.2.2, and a program
//! asking whether two references name the same resource is asking a question
//! about *equivalence*, not about spelling.
//!
//! So the normalization lives on `$uri->compareTo($other)` and nowhere else.
//! It is the whole of § 6.2.2 and no more:
//!
//! - **§ 6.2.2.1, case.** The scheme and the host fold to lower case; every
//!   other component keeps its own. Every `%XX` escape's hex digits fold to
//!   upper case, everywhere.
//! - **§ 6.2.2.2, percent-encoding.** An escape that spells an *unreserved*
//!   character (§ 2.3: `A-Z a-z 0-9 - . _ ~`) becomes that character. An
//!   escape that spells anything else is left escaped, which is what keeps
//!   `/a%2Fb` from ever comparing equal to `/a/b`.
//! - **§ 6.2.2.3, dot segments.** [`remove_dot_segments`] runs over an
//!   **absolute** path only. § 6.2.2.3's own wording is about dot segments
//!   "in non-relative paths"; a relative reference's `..` is meaningful until
//!   something resolves it, so folding it here would make `../a` and `a` one
//!   URI when they are two.
//!
//! It stops short of § 6.2.3's *scheme-based* normalization, deliberately:
//! `http://h:80/` and `http://h/` stay two URIs here. Knowing that `80` is
//! `http`'s default port is knowledge about a scheme, and a comparison that
//! carried a table of them would answer differently as the table grew — while
//! a caller who wants that reading can write it in one `with` call.
//!
//! **`==` is still identity.** [ADR 0090](../../../../docs/adr/0090-one-equality-operator-and-disjoint-types-do-not-compile.md)
//! § 4 makes `$a == $b` on two objects ask whether they are the same object
//! and closes the door on a per-class equality hook, so `compareTo` is the
//! named spelling that ADR itself points at for content equality — the same
//! answer [`crate::time`] gives for an `Instant`. Two references are the same
//! URI when `$a->compareTo($b) == 0`.
//!
//! The order the member defines is component-lexicographic in the class's own
//! slot order (scheme, userInfo, host, port, path, query, fragment), with an
//! absent component sorting before a present one — `null` host before `""`
//! host, which is the one distinction `parse_url`'s array could not hold. RFC
//! 3986 defines no ordering at all, so any total order agreeing with its
//! equivalence would do; this one sorts a list of URIs into the groups a
//! reader expects.
//!
//! # The bracket convention, read and written
//!
//! `a[]=1&a[]=2` builds a list, `a[b]=c` builds a map, and the two nest to any
//! depth. That is not a URL-specification feature — RFC 3986 says a query is
//! opaque text — but it is how every PHP form posts, so the spec adopts it in
//! full, and `Core\Request::query` will answer the same shape from this same
//! code at M8 rather than deciding the question twice.
//!
//! The shape is what makes the return type `array<mixed>`: a value is a
//! `string` **or** a nested `array<mixed>`, and no more precise element type
//! exists to write. `Core\Arr::keys` over the answer is still `array<string>`,
//! because every MWL array key is a `string` already.
//!
//! Two things PHP does here are deliberately **not** reproduced, and both are
//! substitutions rather than structure:
//!
//! - **A key is never rewritten.** `parse_str` turns `.` and ` ` into `_` in
//!   the part of a name before the first bracket, because it was built to
//!   create *variables* and a PHP variable cannot hold either character. This
//!   member returns an array, so nothing constrains a key at all, and
//!   `user.name=x` keeps the key the client actually sent. Rewriting it would
//!   silently merge two distinct parameters — `a.b` and `a_b` — which is worse
//!   than the compatibility it buys.
//! - **A malformed name is one literal key, not a repaired one.** A name is a
//!   path only when it is a non-empty base followed by complete `[…]` groups
//!   and nothing else; `a[b=c` and `a[b]c=d` are not, so each whole name
//!   becomes a single key. PHP instead patches the text — `a[b` becomes the
//!   key `a_b`, and `a[b]c` quietly drops the `c`. Keeping the name preserves
//!   what arrived, which is the only honest answer for input no correct client
//!   produces.
//!
//! Everything else matches, including the parts that look like accidents and
//! are load-bearing for real forms: `a=1&a=2` keeps the last value, `a=1&a[]=2`
//! replaces the scalar with a list and `a[]=1&a=2` replaces the list with the
//! scalar, and `a[]=1&a[3]=x&a[]=y` numbers its appends 0, 3, 4 — the last
//! because [`MwlArray::append`] already keeps PHP's next-free-integer counter.
//!
//! `buildQuery` writes the same convention back, matching `http_build_query`
//! down to its escaping: a nested value goes under its whole path, and the
//! structural brackets are form-encoded like every other byte, so `b[0]=2`
//! goes out as `b%5B0%5D=2`. It writes a list's **indexes** rather than empty
//! brackets, which is what makes `buildQuery(parseQuery($q))` parse back to
//! the same array — `b[]=` would renumber from zero and lose `a[3]`'s key.
//! Equal *text* is not on offer and never was: one set of parameters has many
//! spellings, and this is the one every reader accepts.
//!
//! # What is not here: no dependency
//!
//! This half binds no outside crate, which is a deliberate exception to
//! [ground-rules.md](../../../../docs/adr/ground-rules.md)'s "an external
//! specification is a dependency rather than a hand-written parser". The rule
//! is about *grammars* — RFC 8259's, RFC 3986's — where a hand-written reader
//! accumulates divergences no test finds. There is no grammar here: the whole
//! specification is the two rows of the table above, and any crate that could
//! be bound would still need both byte sets written out beside it, because
//! neither is that crate's default. Binding one would add a dependency to
//! remove nothing.
//!
//! # The grammar half: RFC 3986, and which of the two specifications it is
//!
//! The percent-encoding half above is the exception; the grammar half is the
//! rule. RFC 3986 is a grammar with an external specification, so
//! [ground-rules.md](../../../../docs/adr/ground-rules.md) decides that `parse`
//! binds a crate rather than growing a hand-written scanner. What that rule
//! does *not* decide is **which** specification, because there are two and
//! they are not a strict and a lax reading of one thing.
//!
//! `url` implements the **WHATWG URL Standard** — what a browser does with
//! text typed into an address bar — and it rewrites its input on the way
//! through: it lower-cases the host, punycodes a non-ASCII one, drops a port
//! that matches the scheme's default, removes `.` and `..` from the path, and
//! turns `\` into `/` for the schemes it calls special. It also has no way to
//! hold a relative reference at all without a base. `Uri::parse` replaces
//! `parse_url` and answers "**what are the components of this text**", so
//! every one of those rewrites would be an answer about a URI the caller never
//! sent — and a program comparing `$uri->host()` against an allowlist would be
//! comparing against something a client did not write. `fluent-uri` implements
//! RFC 3986 itself: a URI *reference*, every component borrowed as written,
//! normalization only where it is asked for. The workspace manifest's own
//! comment on the dependency owns the rest of the argument.
//!
//! So: **`parse` reports, it does not normalize.** Scheme and host keep their
//! case, a default port stays written, dot segments stay in the path, and
//! nothing is percent-decoded — `decodeComponent` is one call away for a
//! caller that wants text, and dot-segment removal happens in `$uri->resolve`,
//! which is the one place RFC 3986 § 5.2.4 asks for it.
//!
//! # Decision: normalization happens at the comparison, not at the parse
//!
//! The rule above says what `parse` does not do; this says where the
//! normalizing does happen. `==` over two `Uri` values applies RFC 3986
//! § 6.2.2's syntax-based normalization — scheme and host lower-cased, a
//! default port dropped, unreserved percent-escapes decoded, dot segments
//! removed — and then compares **component by component, never as strings**.
//! The two rules are the same rule seen from both ends: § 3.1 makes a scheme
//! case-insensitive *to compare*, which is a different thing from rewriting
//! what was sent, so the case survives the parse and stops mattering at the
//! comparison.
//!
//! Every program wants "is this the same URL". Defining it once here is what
//! stops each of them from writing a weaker version, and a weaker version is
//! what an SSRF allowlist bypass is made of. PHP shipped `ext/uri` in 8.5 and
//! took four CVEs in seven months; one of them, CVE-2026-44928, is `EqualsUri`
//! answering that two unequal URIs are equal.
//!
//! Two guards pin it, because a hand-written case list is what every one of
//! those four CVEs got past: a property test that `parse` → serialize →
//! `parse` is stable, and a differential corpus against the PHP 8.5 oracle
//! this repository already keeps for correctness. A `fuzz/fuzz_targets` entry
//! sits beside `lex.rs` and `parse.rs` for the same reason.
//!
//! # What `parse` takes, and what it does not ask
//!
//! `parse` takes a **URI reference** — RFC 3986 § 4.1's `URI / relative-ref` —
//! because a request line carries one and `$uri->resolve` is defined over one.
//! `Uri::parse("/a?b#c")` therefore answers a `Uri` whose `scheme()` is
//! `null`, and it throws only on text the grammar refuses: a space, a control
//! byte, a bare `%`, a `<`, and every non-ASCII byte, which is an IRI's
//! business (RFC 3987) and not this member's.
//!
//! **"Is this an absolute URI" is a second question, and it has no member.**
//! It is `parse` succeeding *and* a scheme being present —
//! `Uri::tryParse($s)?->scheme() != null` — which is what
//! `filter_var(…, FILTER_VALIDATE_URL)` is actually asked. The answer differs
//! from PHP's in both directions and deliberately: PHP accepts a space in a
//! path and this refuses it, PHP refuses a URI whose host is empty and this
//! accepts `file:///tmp`. And it **launders nothing** — whether a URL may be
//! *fetched* is `Core\Http::allowUrl` at § 16
//! ([ADR 0058](../../../../docs/adr/0058-outbound-request-policy.md)); a
//! `true` here says only that the text is a URI.
//!
//! An **empty authority is not a missing one.** `parse("file:///tmp")` answers
//! `host()` of `""` and `parse("/tmp")` answers `host()` of `null`, which is
//! the difference between `//` having been written and not. That distinction
//! is the one place this shape is richer than `parse_url`'s array, which
//! cannot express it, and it is what makes `toString` give back the text that
//! went in.
//!
//! # What it spends
//!
//! One `string` per call, at most three bytes of output per input byte, freed
//! with the request that produced it. Nothing is retained between calls and no
//! table grows with traffic. Both encoders and both decoders are a single pass
//! with no backtracking, so a hostile input costs O(n) and cannot be made to
//! cost more.
//!
//! `parseQuery` spends one array per bracket level the query actually writes,
//! plus one `string` per name and per value, all reachable from the one array
//! it answers and freed with it. It is a single pass too: each descent borrows
//! the child array the parent already owns rather than retaining a second
//! reference to it, so no subtree is ever copied and depth costs no stack —
//! see [`branch`] and [`insert`]. `buildQuery` spends the string it answers
//! plus one [`Level`] per open bracket level, and walks with an explicit stack
//! for the same reason: its argument is often a `parseQuery` answer, whose
//! depth came off the wire.
//!
//! A `Uri` spends one object of eight slots — [`crate::instance`] owns what
//! that costs — holding the whole text plus one `string` per component that
//! was written, each a borrowed substring of the text at parse time and each
//! allocated exactly once. The text is kept **as well as** the components
//! rather than instead of them, which is
//! [AGENTS.md](../../../../AGENTS.md)'s memory rule spent on purpose: a `Uri`
//! holding only its text would re-parse on every accessor call, one holding
//! only its components would recompose on every `toString`, and this pays
//! about twice a URI's length, once, to make both O(1) on the request path.
//! Parsing is a single pass, so a hostile input costs O(n) here as it does
//! everywhere else in this module.
//!
//! # Known gaps
//!
//! 1. **`$uri->with` replaces a component and cannot remove one**, so there is
//!    no spelling for "this URI without its fragment". [`written`] owns the
//!    mechanism — an omitted option and a written `null` would arrive as the
//!    same `Tag::Null`, so the option types are `string` rather than `?string`
//!    and a clearing spelling would have to overload a real value. The fix is
//!    an options bag that can tell the two apart, not an `""`-means-remove
//!    rule: `""` is already an empty query, which `?` written with nothing
//!    after it produces and which `query()` reports as distinct from `null`.
//! 2. **A decoder answers `string`, so it throws on bytes that are not valid
//!    UTF-8** — `decodeComponent("%FF")` throws rather than answering. The
//!    honest signature is `: bytes`, since percent-decoding is defined over
//!    octets and a client can send any of them; the throw is exactly what
//!    [ADR 0009](../../../../docs/adr/0009-string-and-bytes.md) § 3's checked
//!    `bytes as string` row would do one line later, so no program is denied
//!    an answer it could have used. `parseQuery` throws on the same octets for
//!    the same reason, for a name as well as for a value. The runtime half of
//!    this is no longer missing — `mwl_runtime::Tag` has its `Bytes` row now —
//!    so what remains is a spec question: § 12's table writes `: string` for
//!    both decoders, and changing it is a spec slice rather than a runtime
//!    one. This is the one place this module diverges from PHP, whose strings
//!    are byte strings.

use std::mem::ManuallyDrop;

use fluent_uri::component::{Authority, Scheme};
use fluent_uri::pct_enc::EStr;
use fluent_uri::resolve::ResolveError;
use fluent_uri::{ParseErrorKind, Uri, UriRef};
use mwl_runtime::{Fault, HelperResult, MwlArray, MwlStr, Tag, Value};

use crate::registry::{Const, CoreClass, CoreMethod, CoreOption, CoreTy};

// ============================================================================
// Registration — this class's rows, and where its symbols live
// ============================================================================

/// `Core\Uri`'s fully-qualified name, written once so the registry row and
/// every diagnostic naming the class cannot drift apart.
pub const NAME: &str = r"Core\Uri";

/// `Core\Uri`'s registry rows — the whole of spec § 12's first table.
///
/// One class with both halves, because the spec writes `parse(string $uri):
/// Uri`: the static members are namespaced functions and the instance members
/// read the seven components [`read`] found. [`crate::regex`] splits its
/// instance out into `Core\Regex\Match` for the opposite reason — a match is
/// not a regex — and nothing here is a second thing.
pub const CLASS: CoreClass = CoreClass {
    name: NAME,
    methods: &[
        CoreMethod {
            name: "parse",
            params: &[CoreTy::Str],
            defaults: &[],
            return_ty: CoreTy::Instance(NAME),
            symbol: "mwl_core_uri_parse",
        },
        CoreMethod {
            name: "tryParse",
            params: &[CoreTy::Str],
            defaults: &[],
            return_ty: CoreTy::Nullable(&CoreTy::Instance(NAME)),
            symbol: "mwl_core_uri_try_parse",
        },
        CoreMethod {
            name: "encodeComponent",
            params: &[CoreTy::Str],
            defaults: &[],
            return_ty: CoreTy::Str,
            symbol: "mwl_core_uri_encode_component",
        },
        CoreMethod {
            name: "decodeComponent",
            params: &[CoreTy::Str],
            defaults: &[],
            return_ty: CoreTy::Str,
            symbol: "mwl_core_uri_decode_component",
        },
        CoreMethod {
            name: "encodeFormValue",
            params: &[CoreTy::Str],
            defaults: &[],
            return_ty: CoreTy::Str,
            symbol: "mwl_core_uri_encode_form_value",
        },
        CoreMethod {
            name: "decodeFormValue",
            params: &[CoreTy::Str],
            defaults: &[],
            return_ty: CoreTy::Str,
            symbol: "mwl_core_uri_decode_form_value",
        },
        CoreMethod {
            name: "parseQuery",
            params: &[CoreTy::Str],
            defaults: &[],
            return_ty: CoreTy::Array(&CoreTy::Mixed),
            symbol: "mwl_core_uri_parse_query",
        },
        CoreMethod {
            name: "buildQuery",
            params: &[CoreTy::Array(&CoreTy::Mixed)],
            defaults: &[],
            return_ty: CoreTy::Str,
            symbol: "mwl_core_uri_build_query",
        },
    ],
    instance: &[
        CoreMethod {
            name: "scheme",
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Nullable(&CoreTy::Str),
            symbol: "mwl_core_uri_scheme",
        },
        CoreMethod {
            name: "userInfo",
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Nullable(&CoreTy::Str),
            symbol: "mwl_core_uri_user_info",
        },
        CoreMethod {
            name: "host",
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Nullable(&CoreTy::Str),
            symbol: "mwl_core_uri_host",
        },
        CoreMethod {
            name: "port",
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Nullable(&CoreTy::Int),
            symbol: "mwl_core_uri_port",
        },
        CoreMethod {
            name: "path",
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Str,
            symbol: "mwl_core_uri_path",
        },
        CoreMethod {
            name: "query",
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Nullable(&CoreTy::Str),
            symbol: "mwl_core_uri_query",
        },
        CoreMethod {
            name: "fragment",
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Nullable(&CoreTy::Str),
            symbol: "mwl_core_uri_fragment",
        },
        CoreMethod {
            name: "toString",
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Str,
            symbol: "mwl_core_uri_to_string",
        },
        CoreMethod {
            name: "with",
            params: &[CoreTy::Options(&[
                CoreOption {
                    name: "scheme",
                    ty: CoreTy::Str,
                    default: Const::Null,
                },
                CoreOption {
                    name: "host",
                    ty: CoreTy::Str,
                    default: Const::Null,
                },
                CoreOption {
                    name: "port",
                    ty: CoreTy::Int,
                    default: Const::Null,
                },
                CoreOption {
                    name: "path",
                    ty: CoreTy::Str,
                    default: Const::Null,
                },
                CoreOption {
                    name: "query",
                    ty: CoreTy::Str,
                    default: Const::Null,
                },
                CoreOption {
                    name: "fragment",
                    ty: CoreTy::Str,
                    default: Const::Null,
                },
            ])],
            defaults: &[],
            return_ty: CoreTy::Instance(NAME),
            symbol: "mwl_core_uri_with",
        },
        CoreMethod {
            name: "resolve",
            params: &[CoreTy::Str],
            defaults: &[],
            return_ty: CoreTy::Instance(NAME),
            symbol: "mwl_core_uri_resolve",
        },
        CoreMethod {
            name: "compareTo",
            params: &[CoreTy::Instance(NAME)],
            defaults: &[],
            return_ty: CoreTy::Int,
            symbol: "mwl_core_uri_compare_to",
        },
    ],
    slots: &[
        "text", "scheme", "userInfo", "host", "port", "path", "query", "fragment",
    ],
    constants: &[],
};

/// [`CLASS`]'s slots, by index. `TEXT_SLOT` holds the whole reference and the
/// seven after it hold the components of it, which is the trade the module
/// docs' *What it spends* states.
const TEXT_SLOT: usize = 0;
/// See [`TEXT_SLOT`].
const SCHEME_SLOT: usize = 1;
/// See [`TEXT_SLOT`].
const USER_INFO_SLOT: usize = 2;
/// See [`TEXT_SLOT`].
const HOST_SLOT: usize = 3;
/// See [`TEXT_SLOT`].
const PORT_SLOT: usize = 4;
/// See [`TEXT_SLOT`].
const PATH_SLOT: usize = 5;
/// See [`TEXT_SLOT`].
const QUERY_SLOT: usize = 6;
/// See [`TEXT_SLOT`].
const FRAGMENT_SLOT: usize = 7;

/// The address of one of *this* module's symbols, or `None` for a symbol that
/// belongs to another domain. See [`crate::symbols`].
pub(crate) fn address(symbol: &str) -> Option<*const u8> {
    Some(match symbol {
        "mwl_core_uri_parse" => (mwl_core_uri_parse as *const ()).cast(),
        "mwl_core_uri_try_parse" => (mwl_core_uri_try_parse as *const ()).cast(),
        "mwl_core_uri_scheme" => (mwl_core_uri_scheme as *const ()).cast(),
        "mwl_core_uri_user_info" => (mwl_core_uri_user_info as *const ()).cast(),
        "mwl_core_uri_host" => (mwl_core_uri_host as *const ()).cast(),
        "mwl_core_uri_port" => (mwl_core_uri_port as *const ()).cast(),
        "mwl_core_uri_path" => (mwl_core_uri_path as *const ()).cast(),
        "mwl_core_uri_query" => (mwl_core_uri_query as *const ()).cast(),
        "mwl_core_uri_fragment" => (mwl_core_uri_fragment as *const ()).cast(),
        "mwl_core_uri_to_string" => (mwl_core_uri_to_string as *const ()).cast(),
        "mwl_core_uri_with" => (mwl_core_uri_with as *const ()).cast(),
        "mwl_core_uri_resolve" => (mwl_core_uri_resolve as *const ()).cast(),
        "mwl_core_uri_compare_to" => (mwl_core_uri_compare_to as *const ()).cast(),
        "mwl_core_uri_encode_component" => (mwl_core_uri_encode_component as *const ()).cast(),
        "mwl_core_uri_decode_component" => (mwl_core_uri_decode_component as *const ()).cast(),
        "mwl_core_uri_encode_form_value" => (mwl_core_uri_encode_form_value as *const ()).cast(),
        "mwl_core_uri_decode_form_value" => (mwl_core_uri_decode_form_value as *const ()).cast(),
        "mwl_core_uri_parse_query" => (mwl_core_uri_parse_query as *const ()).cast(),
        "mwl_core_uri_build_query" => (mwl_core_uri_build_query as *const ()).cast(),
        _ => return None,
    })
}

// ============================================================================
// The two encodings — one implementation each, selected by a flag
// ============================================================================

/// Which of the module docs' two rows a call is running.
///
/// A parameter rather than four separate loops, because the two rows differ in
/// exactly two decisions and duplicating the pass would make it possible for
/// them to drift in the twenty they share.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Form {
    /// RFC 3986 § 2.1: `~` is unreserved and a space is `%20`.
    Component,
    /// `application/x-www-form-urlencoded`: `~` is encoded and a space is `+`.
    FormValue,
}

impl Form {
    /// Whether `byte` is written as itself rather than percent-encoded.
    ///
    /// The module docs' table, in code: both sets are the ASCII alphanumerics
    /// plus `-_.`, and `~` joins them only for a URI component.
    const fn unreserved(self, byte: u8) -> bool {
        byte.is_ascii_alphanumeric()
            || matches!(byte, b'-' | b'_' | b'.')
            || (matches!(self, Self::Component) && byte == b'~')
    }
}

/// The upper-case hex digits a percent-escape is written with — PHP's case and
/// RFC 3986 § 2.1's recommendation.
const HEX: [u8; 16] = *b"0123456789ABCDEF";

/// `text` percent-encoded under `form`.
///
/// Capacity is the input's length rather than three times it: the common
/// subject is mostly unreserved, so reserving for the worst case would triple
/// the allocation of every call to pay for the rare one that needs it.
fn encode(text: &[u8], form: Form) -> String {
    let mut out = String::with_capacity(text.len());
    for &byte in text {
        match byte {
            b' ' if form == Form::FormValue => out.push('+'),
            _ if form.unreserved(byte) => out.push(char::from(byte)),
            _ => {
                out.push('%');
                out.push(char::from(HEX[usize::from(byte >> 4)]));
                out.push(char::from(HEX[usize::from(byte & 0x0f)]));
            }
        }
    }
    out
}

/// The two hex digits at `at`, as the byte they spell, or `None` where either
/// is missing or is not a hex digit.
///
/// Either case reads, which is RFC 3986 § 6.2.2.1's normalization rule seen
/// from the reading side: `%2f` and `%2F` are one octet, so refusing the
/// lower-case spelling would reject text every other decoder accepts.
fn escaped(bytes: &[u8], at: usize) -> Option<u8> {
    let digit = |offset: usize| -> Option<u8> {
        let value = char::from(*bytes.get(at + offset)?).to_digit(16)?;
        u8::try_from(value).ok()
    };
    Some((digit(0)? << 4) | digit(1)?)
}

/// `text` percent-decoded under `form`, as the octets it spells.
///
/// Answers bytes rather than a `String` because that is what percent-decoding
/// produces — the UTF-8 question is the caller's, and gap 2 owns why it is
/// asked at all.
fn decode(text: &str, form: Form) -> Vec<u8> {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut at = 0;
    while at < bytes.len() {
        match bytes[at] {
            b'+' if form == Form::FormValue => {
                out.push(b' ');
                at += 1;
            }
            // A malformed escape is text, not an error — see the module docs.
            b'%' => match escaped(bytes, at + 1) {
                Some(byte) => {
                    out.push(byte);
                    at += 3;
                }
                None => {
                    out.push(b'%');
                    at += 1;
                }
            },
            byte => {
                out.push(byte);
                at += 1;
            }
        }
    }
    out
}

// ============================================================================
// The boundary — arguments in, one `string` out
// ============================================================================

/// The `string` in argument slot 0.
///
/// # Errors
///
/// A [`Fault::fatal`] naming the member. Compiled code wrote the tag and
/// `mwl_types` already checked the declared type, so a slot holding anything
/// else is a runtime-contract violation rather than anything a program can
/// cause — the same treatment [`crate::path`] gives its own arguments.
///
/// That is the only failure. The tag [`Value::as_text`] checks is itself
/// ADR 0009's UTF-8 guarantee, so no encoding outcome is left to report.
fn text_of<'a>(args: &'a [Value], member: &str) -> Result<&'a str, Fault> {
    args[0].as_text().ok_or_else(|| {
        Fault::fatal(format!(
            "Core\\Uri::{member} expected {:?}, got tag {}",
            Tag::Str,
            args[0].tag_byte()
        ))
    })
}

/// `text` as an owned `string` value.
fn produced(text: &str) -> HelperResult {
    Ok(Value::str(MwlStr::new(text.as_bytes())))
}

/// `octets` as text, or a throw where they are not UTF-8. `subject` names
/// which octets they were, since `parseQuery` decodes two kinds.
///
/// # Errors
///
/// A [`Fault::thrown`] naming the member and the offset of the first bad byte.
/// This is gap 2: percent-decoding answers octets, `string` is UTF-8, and the
/// throw is ADR 0009 § 3's checked `bytes as string` row reached one member
/// early. The offset is a position in text the caller supplied, so it is safe
/// to name and it is the one fact that makes the throw actionable — the octets
/// themselves are not quoted, since they are by definition not text.
fn text_from(octets: Vec<u8>, member: &str, subject: &str) -> Result<String, Fault> {
    String::from_utf8(octets).map_err(|error| {
        Fault::thrown(format!(
            "Core\\Uri::{member}(): {subject} holds a byte a `string` cannot — byte {} begins a \
             sequence that is not valid UTF-8. Percent-decoding answers octets, so text carrying \
             an escape for a non-UTF-8 byte has no `string` to decode to",
            error.utf8_error().valid_up_to()
        ))
    })
}

/// `octets` as a `string` value, or a throw where they are not UTF-8.
///
/// # Errors
///
/// [`text_from`]'s, which owns why this throws at all.
fn decoded(octets: Vec<u8>, member: &str) -> HelperResult {
    produced(&text_from(octets, member, "the decoded octets")?)
}

// ============================================================================
// The grammar — RFC 3986 through `fluent-uri`, and the instance it fills
// ============================================================================

/// `text` read as an RFC 3986 URI reference, with nothing normalized.
///
/// # Errors
///
/// A [`Fault::thrown`] naming the member, what the grammar refused and where.
/// This is the one place in this module that rejects its input rather than
/// reading it as literally as it can, and the module docs own why: `parse`
/// answers a *structure*, and text the grammar does not admit has none to
/// answer with.
///
/// The message gives the byte offset and **never quotes the text back**. Text
/// that failed to parse here is attacker-supplied by definition, and a URI's
/// userinfo component is where credentials get written, so a message that
/// echoed it would put them in whatever log the throw reaches.
fn read<'a>(text: &'a str, member: &str) -> Result<UriRef<&'a str>, Fault> {
    UriRef::parse(text).map_err(|error| {
        let refused = match error.kind() {
            ParseErrorKind::InvalidPctEncodedOctet => "a `%` that does not begin a `%XX` escape",
            ParseErrorKind::InvalidIpv6Addr => "a bracketed host that is not an IPv6 address",
            // Every byte outside the grammar arrives here: a space, a control
            // byte, a `<`, a `"`, and every non-ASCII byte, which is RFC 3987's
            // business and not this member's.
            ParseErrorKind::UnexpectedChar => "a byte the URI grammar does not admit",
        };
        Fault::thrown(format!(
            "Core\\Uri::{member}(): this text is not a URI reference (RFC 3986 § 4.1) — {refused} \
             at byte {}. The text itself is not quoted back, since a URI's userinfo component is \
             where credentials are written",
            error.index()
        ))
    })
}

/// `authority`'s port, narrowed to what a port is.
///
/// # Errors
///
/// A [`Fault::thrown`] where the digits do not fit a [`u16`]. RFC 3986
/// § 3.2.3's *grammar* is `*DIGIT` while its *prose* defines the component as
/// a TCP port number, so `//h:99999/` is text the grammar admits and a port
/// nothing can dial. Storing it would answer with a number no caller could
/// use and truncating it would answer with a different URI's port, so the
/// member refuses instead — the same line [`crate::validate`] draws, drawn
/// where the caller can read it.
///
/// An **empty** port is `None` rather than an error, which is § 3.2.3's own
/// instruction to a producer. `toString` still gives the `:` back, because it
/// answers with the text that was parsed rather than with a recomposition.
fn port_of(authority: &Authority<'_>, member: &str) -> Result<Option<u16>, Fault> {
    authority.port_to_u16().map_err(|_| {
        Fault::thrown(format!(
            "Core\\Uri::{member}(): the authority's port is not a TCP port number. RFC 3986 \
             § 3.2.3 admits any run of digits and defines the component as a port, so a value \
             outside 0-65535 has no `int` this member could answer with"
        ))
    })
}

/// A fresh `Core\Uri` holding `reference`'s text and its seven components.
///
/// # Errors
///
/// [`port_of`]'s. It runs **before** the first allocation on purpose: a
/// [`Value`] is not released by falling out of scope, so a refused port
/// halfway through the slot array would strand every `MwlStr` built before it.
fn built(reference: &UriRef<&str>, member: &str) -> HelperResult {
    let authority = reference.authority();
    let port = match authority {
        Some(authority) => port_of(&authority, member)?,
        None => None,
    };
    let text = |held: Option<&str>| {
        held.map_or_else(Value::null, |held| Value::str(MwlStr::new(held.as_bytes())))
    };
    Ok(crate::instance::build(
        &CLASS,
        [
            Value::str(MwlStr::new(reference.as_str().as_bytes())),
            text(reference.scheme().map(Scheme::as_str)),
            text(authority.and_then(|held| held.userinfo()).map(EStr::as_str)),
            // Present-but-empty where `//` was written with nothing after it,
            // which is the distinction `parse_url`'s array cannot hold.
            text(authority.as_ref().map(Authority::host)),
            port.map_or_else(Value::null, |port| Value::int(i64::from(port))),
            Value::str(MwlStr::new(reference.path().as_str().as_bytes())),
            text(reference.query().map(EStr::as_str)),
            text(reference.fragment().map(EStr::as_str)),
        ],
    ))
}

/// One of the receiver's slots, handed back with a reference of its own.
///
/// Every reader on this class is this call with a different slot: [`read`]
/// did the work once, and an accessor is a field read.
fn component(args: &[Value], member: &str, index: usize) -> HelperResult {
    let receiver = crate::instance::receiver(args[0], &CLASS, member)?;
    let held = crate::instance::slot(receiver, index);
    #[expect(
        unsafe_code,
        reason = "the receiver's slot owns the reference this borrowed read \
                  returned, so the caller needs one of its own"
    )]
    unsafe {
        held.retain();
    }
    Ok(held)
}

/// The bytes one of a receiver's `string`-or-`null` slots holds, borrowed from
/// `slots` rather than from the object, which is what keeps the read a
/// [`crate::instance::slot`] borrow and not a retain.
///
/// # Errors
///
/// A [`Fault::fatal`] where the slot holds neither. Both halves of that
/// invariant are in this module — [`built`] fills every slot — so a mismatch
/// is a paste error here rather than anything a program can cause.
fn held<'a>(slots: &'a [Value], index: usize, member: &str) -> Result<Option<&'a str>, Fault> {
    let value = &slots[index];
    if matches!(value.tag(), Some(Tag::Null)) {
        return Ok(None);
    }
    value.as_text().map(Some).ok_or_else(|| {
        Fault::fatal(format!(
            "Core\\Uri::{member} found tag {} in slot {index}",
            value.tag_byte()
        ))
    })
}

/// One written option's text, or `None` where the call left the option out.
///
/// An omitted option arrives as [`Const::Null`] and a **written** one cannot
/// be `null`, because each option's declared type is `string` rather than
/// `?string` — which is what makes "not given" a state the helper can tell
/// apart from every value a call site could write. It is also why `with`
/// replaces and never removes: with no second null to spend, a clearing
/// spelling would have to overload a legitimate value, and `""` is already an
/// empty query rather than the absence of one.
///
/// # Errors
///
/// A [`Fault::fatal`] where the slot holds something else. `mwl_types` checked
/// the declared type, so that is a runtime-contract violation.
fn written<'a>(value: &'a Value, option: &str) -> Result<Option<&'a str>, Fault> {
    if matches!(value.tag(), Some(Tag::Null)) {
        return Ok(None);
    }
    value.as_text().map(Some).ok_or_else(|| {
        Fault::fatal(format!(
            "Core\\Uri::with expected a `string` for its `{option}` option, got tag {}",
            value.tag_byte()
        ))
    })
}

/// The seven components [`mwl_core_uri_with`] wrote its text out of — what the
/// result must still parse back to.
///
/// A recomposition is a concatenation, so the *grammar* is still
/// `fluent-uri`'s: [`recompose`] joins, [`read`] decides. What a concatenation
/// can still do is move a component across a delimiter — `with({path: "a"})`
/// on a URI with an authority would write `//hosta`, whose host is `hosta` and
/// whose path is empty — and re-parsing alone would not notice, because that
/// text is a perfectly good URI. So the check is this struct compared against
/// what came back, one component at a time, rather than three hand-written
/// rules about where a `/` has to be: it catches the cases nobody enumerated.
#[derive(Debug)]
struct Composed<'a> {
    scheme: Option<&'a str>,
    user_info: Option<&'a str>,
    host: Option<&'a str>,
    port: Option<&'a str>,
    path: &'a str,
    query: Option<&'a str>,
    fragment: Option<&'a str>,
}

/// RFC 3986 § 5.3's recomposition of `composed`, with no delimiter written for
/// a component that is not there.
fn recompose(composed: &Composed<'_>) -> String {
    let mut text = String::new();
    if let Some(scheme) = composed.scheme {
        text.push_str(scheme);
        text.push(':');
    }
    if let Some(host) = composed.host {
        text.push_str("//");
        if let Some(user_info) = composed.user_info {
            text.push_str(user_info);
            text.push('@');
        }
        text.push_str(host);
        if let Some(port) = composed.port {
            text.push(':');
            text.push_str(port);
        }
    }
    text.push_str(composed.path);
    if let Some(query) = composed.query {
        text.push('?');
        text.push_str(query);
    }
    if let Some(fragment) = composed.fragment {
        text.push('#');
        text.push_str(fragment);
    }
    text
}

/// That `reference` still holds every component [`recompose`] put into it.
///
/// # Errors
///
/// A [`Fault::thrown`] naming the **first** component that moved. That name is
/// the whole value of the check: `with({path: "a"})` on a URI with an
/// authority fails saying `host`, which is where the caller's missing `/`
/// actually landed, rather than saying the result is not a URI when it is.
fn unmoved(composed: &Composed<'_>, reference: &UriRef<&str>) -> Result<(), Fault> {
    let authority = reference.authority();
    let moved = if composed.scheme != reference.scheme().map(Scheme::as_str) {
        "scheme"
    } else if composed.user_info != authority.and_then(|held| held.userinfo()).map(EStr::as_str) {
        "userInfo"
    } else if composed.host != authority.as_ref().map(Authority::host) {
        "host"
    } else if composed.port != authority.and_then(|held| held.port()).map(EStr::as_str) {
        "port"
    } else if composed.path != reference.path().as_str() {
        "path"
    } else if composed.query != reference.query().map(EStr::as_str) {
        "query"
    } else if composed.fragment != reference.fragment().map(EStr::as_str) {
        "fragment"
    } else {
        return Ok(());
    };
    Err(Fault::thrown(format!(
        "Core\\Uri::with(): the components given do not recompose to a URI that still holds them \
         — the `{moved}` of the result is not the one asked for. A component that has to carry a \
         delimiter must carry it: a `path` beside a `host` begins with `/`, and a `scheme` is not \
         written into one"
    )))
}

// ============================================================================
// Equivalence — RFC 3986 § 6.2.2, run at the comparison and never at the parse
// ============================================================================

/// Whether `byte` is RFC 3986 § 2.3's *unreserved* — the set § 6.2.2.2
/// restores from its escaped spelling.
///
/// Deliberately not [`Form::unreserved`]: that one answers for an *encoder*
/// and carries the `~` disagreement between a URI component and a form value,
/// which is a question about producing text. This one answers § 2.3's set and
/// nothing else, because § 6.2.2.2 names that set by reference. Collapsing the
/// two would tie a comparison's meaning to which encoder ran last.
const fn unreserved(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~')
}

/// `text` under RFC 3986 § 6.2.2.1 and § 6.2.2.2 — every surviving escape
/// written `%XX` in upper case, every escape spelling an [`unreserved`]
/// character replaced by that character, and the ASCII letters folded to lower
/// case where `fold` says this component is case-insensitive.
///
/// `fold` is § 6.2.2.1's first half, applied per component rather than to the
/// whole reference: the scheme and the host are case-insensitive and nothing
/// else is, so a path's `A` stays an `A`.
///
/// A malformed escape is left as the text [`decode`] would also leave — a
/// comparison is not the place to start refusing what `parse` admitted, and
/// two references that both wrote the same stray `%` are still the same
/// reference.
fn normalized(text: &str, fold: bool) -> String {
    let bytes = text.as_bytes();
    let mut out = String::with_capacity(bytes.len());
    let mut at = 0;
    while at < bytes.len() {
        match bytes[at] {
            b'%' => match escaped(bytes, at + 1) {
                Some(byte) if unreserved(byte) => {
                    out.push(char::from(fold_ascii(byte, fold)));
                    at += 3;
                }
                Some(byte) => {
                    out.push('%');
                    out.push(char::from(HEX[usize::from(byte >> 4)]));
                    out.push(char::from(HEX[usize::from(byte & 0x0f)]));
                    at += 3;
                }
                None => {
                    out.push('%');
                    at += 1;
                }
            },
            byte => {
                out.push(char::from(fold_ascii(byte, fold)));
                at += 1;
            }
        }
    }
    out
}

/// `byte` lower-cased where `fold`, and itself otherwise.
///
/// ASCII only, and that is § 6.2.2.1's own scope: a scheme is ASCII by grammar
/// and a host that is not is punycode, which is ASCII too. Folding by Unicode
/// rules here would make the answer depend on a case table rather than on the
/// RFC.
const fn fold_ascii(byte: u8, fold: bool) -> u8 {
    if fold {
        byte.to_ascii_lowercase()
    } else {
        byte
    }
}

/// RFC 3986 § 5.2.4's `remove_dot_segments` over `path`, which is what
/// § 6.2.2.3 asks a normalizer to apply.
///
/// Runs on an **absolute** path only — one beginning with `/`. § 6.2.2.3's own
/// wording is about dot segments "in non-relative paths", and the restriction
/// matters: a relative reference's leading `..` is meaningful until something
/// resolves it, so folding it away here would answer that `../a` and `a` are
/// one URI when they are two different references to two different resources.
///
/// A list of surviving segments rather than the RFC's literal two-buffer
/// transcription, which rescans its output string to find the last segment
/// written: the input is a request path, so a quadratic pass over it would be
/// a cost the caller chooses. The two produce the same answer, including on
/// the two cases a segment list makes easy to get wrong — an **empty interior
/// segment is kept** (`/a//b` is not `/a/b`), and a **trailing** `.` or `..`
/// leaves the path ending in `/`, which is § 5.2.4 steps 2C and 2D appending
/// an empty segment.
fn remove_dot_segments(path: &str) -> String {
    if !path.starts_with('/') {
        return path.to_owned();
    }
    // Non-empty for any absolute path: `"/"` splits to `["", ""]`.
    let segments: Vec<&str> = path.split('/').skip(1).collect();
    let last = segments.len() - 1;
    let mut kept: Vec<&str> = Vec::with_capacity(segments.len());
    for (index, segment) in segments.iter().enumerate() {
        match *segment {
            "." => {
                if index == last {
                    kept.push("");
                }
            }
            ".." => {
                kept.pop();
                if index == last {
                    kept.push("");
                }
            }
            other => kept.push(other),
        }
    }
    format!("/{}", kept.join("/"))
}

/// One `Uri`'s seven components under [`normalized`], in the class's own slot
/// order — which is also the order [`Ord`] compares them in.
///
/// Derived rather than hand-written: a lexicographic walk of the fields in
/// declaration order is exactly the order the module docs state, and a derive
/// cannot forget a field the way a hand-written chain of `then_with` can. An
/// absent component sorts before a present one, which is [`Option`]'s own
/// derived order and the reading the module docs give.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
struct Equivalent {
    scheme: Option<String>,
    user_info: Option<String>,
    host: Option<String>,
    port: Option<i64>,
    path: String,
    query: Option<String>,
    fragment: Option<String>,
}

/// The `Core\Uri` in argument slot `at`, read into its [`Equivalent`] form.
///
/// # Errors
///
/// [`crate::instance::receiver`]'s where the slot does not hold one, and
/// [`held`]'s where a component slot holds neither a `string` nor `null`.
///
/// Every slot read here is [`crate::instance::slot`]'s **borrow**, so nothing
/// is retained and nothing needs releasing on either edge — the same treatment
/// [`mwl_core_uri_resolve`] gives its own receiver.
fn equivalent(args: &[Value], at: usize, member: &str) -> Result<Equivalent, Fault> {
    let object = crate::instance::receiver(args[at], &CLASS, member)?;
    let slots: [Value; 8] = std::array::from_fn(|index| crate::instance::slot(object, index));
    let component = |index: usize, fold: bool| -> Result<Option<String>, Fault> {
        Ok(held(&slots, index, member)?.map(|text| normalized(text, fold)))
    };
    let port = match slots[PORT_SLOT].tag() {
        Some(Tag::Null) => None,
        _ => Some(slots[PORT_SLOT].as_int().ok_or_else(|| {
            Fault::fatal(format!(
                "Core\\Uri::{member} found tag {} in the `port` slot",
                slots[PORT_SLOT].tag_byte()
            ))
        })?),
    };
    let path = component(PATH_SLOT, false)?
        .ok_or_else(|| Fault::fatal(format!("Core\\Uri::{member} found a null `path` slot")))?;
    Ok(Equivalent {
        scheme: component(SCHEME_SLOT, true)?,
        user_info: component(USER_INFO_SLOT, false)?,
        host: component(HOST_SLOT, true)?,
        port,
        path: remove_dot_segments(&path),
        query: component(QUERY_SLOT, false)?,
        fragment: component(FRAGMENT_SLOT, false)?,
    })
}

// ============================================================================
// The bracket convention — one pass, no recursion
// ============================================================================

/// One step of a parameter name's bracket path.
///
/// The two spellings a name can write between one pair of brackets, and the
/// only two: `a[k]` names a key and `a[]` asks for the next one.
enum Index<'a> {
    /// `a[k]` — the key written between the brackets, never empty, since
    /// empty brackets are [`Self::Next`].
    At(&'a [u8]),
    /// `a[]` — whatever key [`MwlArray::append`] assigns next, which is the
    /// counter PHP calls `nNextFreeElement` and MWL's arrays already keep.
    Next,
}

impl Index<'_> {
    /// The key this step names, or `None` for `[]`.
    const fn key(&self) -> Option<&[u8]> {
        match self {
            Self::At(key) => Some(key),
            Self::Next => None,
        }
    }
}

/// `name` split into the key it opens with and the bracket path that follows,
/// or `None` where `name` is not a bracket path at all.
///
/// A path is a **non-empty base followed by zero or more complete `[…]` groups
/// and nothing else**. Anything short of that — an unclosed `[`, text after
/// the last `]`, a name that opens with `[` — answers `None`, and the caller
/// takes the whole name as one literal key. The module docs own why that is
/// the rule rather than PHP's character substitutions.
fn path_of(name: &[u8]) -> Option<(&[u8], Vec<Index<'_>>)> {
    let open = match name.iter().position(|&byte| byte == b'[') {
        None => return Some((name, Vec::new())),
        Some(0) => return None,
        Some(at) => at,
    };
    let mut path = Vec::new();
    let mut rest = &name[open..];
    while let Some(&byte) = rest.first() {
        if byte != b'[' {
            return None;
        }
        let close = rest.iter().position(|&byte| byte == b']')?;
        path.push(if close == 1 {
            Index::Next
        } else {
            Index::At(&rest[1..close])
        });
        rest = &rest[close + 1..];
    }
    Some((&name[..open], path))
}

/// The array `key` names inside `parent`, displacing whatever was there when
/// it is not an array already — which is `a=1&a[]=2` answering `{a: ["2"]}`,
/// exactly as PHP does. `None` always builds a fresh one and appends it, since
/// `a[][x]=1&a[][y]=2` is two arrays rather than one.
///
/// The handle is **borrowed and never dropped**: `parent` owns the only
/// reference to the array it hands back, so writing through this handle finds
/// a refcount of one and mutates in place rather than separating. That is what
/// makes the whole parse O(input) — retaining a second reference would make
/// every descent copy the subtree it descends into.
fn branch(parent: &mut MwlArray, key: Option<&[u8]>) -> ManuallyDrop<MwlArray> {
    if let Some(key) = key
        && let Some(existing) = parent.get(key).and_then(Value::array_ptr)
    {
        return crate::arr::borrowed(existing);
    }
    let fresh = Value::array(MwlArray::new());
    let address = fresh.array_ptr().expect("just built from an array");
    match key {
        Some(key) => parent.set(MwlStr::new(key), fresh),
        None => parent.append(fresh),
    }
    let child = crate::arr::borrowed(address);
    debug_assert_eq!(
        child.refcount(),
        1,
        "the array just handed to `parent` is owned by it alone"
    );
    child
}

/// Writes `value` at `base` + `path` inside `out`, building the arrays the
/// path passes through.
///
/// Iterative rather than recursive on purpose: the path's depth is the
/// caller's text, so a recursive descent would let a query string choose this
/// process's stack depth. The arrays it builds are freed through
/// `mwl_runtime::release`'s worklist, which is iterative for the same reason,
/// so nesting is bounded by the input's length and by nothing else — PHP's
/// `max_input_nesting_level` has no equivalent here because it does not need
/// one.
fn insert(out: &mut MwlArray, base: &[u8], path: &[Index<'_>], value: Value) {
    let Some((last, descents)) = path.split_last() else {
        out.set(MwlStr::new(base), value);
        return;
    };
    let mut current = branch(out, Some(base));
    for index in descents {
        let next = branch(&mut current, index.key());
        current = next;
    }
    match last.key() {
        Some(key) => current.set(MwlStr::new(key), value),
        None => current.append(value),
    }
}

/// One array [`build`] is walking, and how much of the running name belongs to
/// the path that reached it.
///
/// A stack of these rather than a recursive walk, for [`insert`]'s reason: the
/// depth is the caller's data, and a `parseQuery` answer is caller's data that
/// arrived over the wire.
struct Level {
    /// The entries, borrowed — `build` only reads, and the argument owns them.
    array: ManuallyDrop<MwlArray>,
    /// The next slot to look at, which [`MwlArray::next_slot`] advances.
    slot: usize,
    /// How many bytes of the running name are this array's own path. Each of
    /// its entries writes its own key after exactly that much.
    prefix: usize,
}

/// One value's text for the right-hand side of a pair.
///
/// ADR 0007 § 2's conversion rows through `mwl_runtime::value_to_string`, with
/// one deliberate exception: `false` writes `0` rather than the empty string
/// that `false as string` answers. `http_build_query` makes the same exception,
/// and it is the right one here — the wire has no booleans, an empty value is
/// how a form spells *absent*, and every reader of a query string treats `0`
/// and `1` as the pair. The exception is scoped to this member, so the
/// language's own conversion is untouched.
///
/// # Errors
///
/// A [`Fault::thrown`] where the value is one `string` has no conversion from
/// — an object or a closure. `null` never reaches here: [`build`] drops the
/// pair instead, which is `http_build_query`'s behaviour and the only one that
/// round-trips, since a query string cannot spell an absent value.
fn scalar_text(value: Value, member: &str) -> Result<Vec<u8>, Fault> {
    if let Some(set) = value.as_bool() {
        return Ok(if set { b"1".to_vec() } else { b"0".to_vec() });
    }
    let text = mwl_runtime::value_to_string(value).map_err(|_| {
        Fault::thrown(format!(
            "Core\\Uri::{member}(): a parameter's value is neither a scalar nor a nested array, \
             so there is no text a query string could write it as"
        ))
    })?;
    let bytes = text
        .as_str_bytes()
        .ok_or_else(|| Fault::fatal("`value_to_string` answered something that is not a string"))?
        .to_vec();
    #[expect(
        unsafe_code,
        reason = "`value_to_string` hands back exactly one fresh reference, and \
                  the bytes have been copied out of it"
    )]
    unsafe {
        text.release();
    }
    Ok(bytes)
}

/// `root` written as a query string: depth-first in entry order, every name
/// and every value form-encoded.
///
/// The name a nested value is written under is the whole path — `a[b][c]` —
/// with its structural brackets encoded like any other byte, which is what
/// `http_build_query` writes and what [`path_of`] reads back. A list is
/// therefore written with its indexes (`b[0]=`, not `b[]=`), so the round trip
/// preserves the keys rather than renumbering them.
///
/// # Errors
///
/// [`scalar_text`]'s, for a value with no text form.
fn build(root: *mut mwl_runtime::ArrayHeader, member: &str) -> Result<String, Fault> {
    let mut out = String::new();
    let mut name: Vec<u8> = Vec::new();
    let mut stack = vec![Level {
        array: crate::arr::borrowed(root),
        slot: 0,
        prefix: 0,
    }];
    while let Some(level) = stack.last_mut() {
        let Some(live) = level.array.next_slot(level.slot) else {
            stack.pop();
            continue;
        };
        level.slot = live + 1;
        let prefix = level.prefix;
        let key = level.array.key_at(live).expect("a live slot has a key");
        let value = level.array.value_at(live).expect("a live slot has a value");

        name.truncate(prefix);
        if prefix == 0 {
            name.extend_from_slice(key.as_bytes());
        } else {
            name.push(b'[');
            name.extend_from_slice(key.as_bytes());
            name.push(b']');
        }

        if let Some(nested) = value.array_ptr() {
            stack.push(Level {
                array: crate::arr::borrowed(nested),
                slot: 0,
                prefix: name.len(),
            });
            continue;
        }
        // A `null` is dropped rather than written empty — see `scalar_text`.
        if value.tag() == Some(Tag::Null) {
            continue;
        }
        if !out.is_empty() {
            out.push('&');
        }
        out.push_str(&encode(&name, Form::FormValue));
        out.push('=');
        out.push_str(&encode(&scalar_text(value, member)?, Form::FormValue));
    }
    Ok(out)
}

// ============================================================================
// The members
// ============================================================================

mwl_runtime::mwl_helper! {
    /// `Core\Uri::parse(string $uri): Uri` — replacing PHP's `parse_url`.
    ///
    /// Takes a URI *reference*, reports rather than normalizes, and throws on
    /// text RFC 3986 refuses. The module docs own all three, and every
    /// component comes back exactly as written — still percent-encoded, still
    /// in the case it arrived in.
    ///
    /// Where `parse_url` answers an array with a key missing for every absent
    /// component, this answers an object whose readers are `?string`, so
    /// "absent" is a value the type system knows about rather than an index
    /// that is not there. ADR 0063 R5's reading of `?T` is the same one.
    fn mwl_core_uri_parse(_ctx, args: [1]) {
        let text = text_of(args, "parse")?;

        built(&read(text, "parse")?, "parse")
    }
}

mwl_runtime::mwl_helper! {
    /// `Core\Uri::tryParse(string $uri): ?Uri` —
    /// [ADR 0066](../../../../docs/adr/0066-nullable-conversion-operator.md)
    /// § 3a: [`mwl_core_uri_parse`] exactly, with `null` where it throws.
    ///
    /// It is `parse` and not a second reader, which is the whole reason R17
    /// allows the question "is this text a URI" one spelling and this is it:
    /// a validator written as separate code from the parser is how
    /// `filter_var(FILTER_VALIDATE_URL)` came to accept user-info that
    /// `parse_url` read differently (CVE-2024-5458). `Core\Uri` declares no
    /// `isValid` for that reason, and the narrower question that one asked —
    /// "is this an **absolute** URI" — is `tryParse($s)?->scheme() != null`.
    ///
    /// The name is the one `try…` [ADR 0063](../../../../docs/adr/0063-core-api-conventions.md)
    /// R5 admits, because R4's "failure throws, absence is `?T`" leaves a
    /// class no other non-throwing spelling: `as ?T` never targets one.
    ///
    /// Only a *thrown* fault becomes `null`. A `Fault::Fatal` — a wrong
    /// argument tag, an engine invariant — is not a failed parse and
    /// propagates unchanged.
    fn mwl_core_uri_try_parse(_ctx, args: [1]) {
        let text = text_of(args, "tryParse")?;

        match read(text, "tryParse") {
            Ok(reference) => built(&reference, "tryParse"),
            Err(Fault::Thrown(..)) => Ok(Value::null()),
            Err(other) => Err(other),
        }
    }
}

mwl_runtime::mwl_helper! {
    /// `$uri->scheme(): ?string` — `null` for a relative reference, and never
    /// case-folded: RFC 3986 § 3.1 makes a scheme case-insensitive to
    /// *compare*, which is a different thing from rewriting what was sent.
    fn mwl_core_uri_scheme(_ctx, args: [1]) {
        component(args, "scheme", SCHEME_SLOT)
    }
}

mwl_runtime::mwl_helper! {
    /// `$uri->userInfo(): ?string` — the whole `user:password` subcomponent as
    /// written, or `null` where no `@` was.
    ///
    /// One reader rather than `parse_url`'s two keys, because RFC 3986
    /// § 3.2.1 deprecates the `user:password` form outright and a member that
    /// split it would be a member that suggested writing one.
    fn mwl_core_uri_user_info(_ctx, args: [1]) {
        component(args, "userInfo", USER_INFO_SLOT)
    }
}

mwl_runtime::mwl_helper! {
    /// `$uri->host(): ?string` — `null` where no authority was written, `""`
    /// where an empty one was (`file:///tmp`), and an IPv6 literal still
    /// inside its brackets, since that is what the host component is.
    fn mwl_core_uri_host(_ctx, args: [1]) {
        component(args, "host", HOST_SLOT)
    }
}

mwl_runtime::mwl_helper! {
    /// `$uri->port(): ?int` — `null` where none was written *and* where an
    /// empty one was, which is [`port_of`]'s one departure from giving back
    /// exactly what came in.
    fn mwl_core_uri_port(_ctx, args: [1]) {
        component(args, "port", PORT_SLOT)
    }
}

mwl_runtime::mwl_helper! {
    /// `$uri->path(): string` — never `null`, because RFC 3986 § 3.3's path is
    /// not optional: a URI with nothing between its authority and its query
    /// has the empty path, and `""` is that path rather than the absence of
    /// one.
    fn mwl_core_uri_path(_ctx, args: [1]) {
        component(args, "path", PATH_SLOT)
    }
}

mwl_runtime::mwl_helper! {
    /// `$uri->query(): ?string` — the raw query, with no `?`, still encoded.
    /// [`mwl_core_uri_parse_query`] is what turns it into an array.
    ///
    /// `null` and `""` are different answers here: `?` written with nothing
    /// after it is an empty query, and no `?` at all is no query.
    fn mwl_core_uri_query(_ctx, args: [1]) {
        component(args, "query", QUERY_SLOT)
    }
}

mwl_runtime::mwl_helper! {
    /// `$uri->fragment(): ?string` — the raw fragment, with no `#`, still
    /// encoded. `null` and `""` differ for [`mwl_core_uri_query`]'s reason.
    fn mwl_core_uri_fragment(_ctx, args: [1]) {
        component(args, "fragment", FRAGMENT_SLOT)
    }
}

mwl_runtime::mwl_helper! {
    /// `$uri->toString(): string` — the reference this `Uri` was parsed from,
    /// byte for byte.
    ///
    /// Not a recomposition of the components: `parse` normalizes nothing, so
    /// there is nothing a round trip could lose, and holding the text is what
    /// buys that guarantee for the price the module docs' *What it spends*
    /// states. Named `toString` for [`crate::uuid`]'s reason, and that name is
    /// load-bearing now: ADR 0028 § 1's rendering *is* this member, so
    /// `echo $uri` reaches it too — through the native call the checker
    /// resolves where the operand's type names this class, and through
    /// [`crate::instance`]'s descriptor renderer where it names none.
    fn mwl_core_uri_to_string(_ctx, args: [1]) {
        component(args, "toString", TEXT_SLOT)
    }
}

mwl_runtime::mwl_helper! {
    /// `$uri->with({scheme?, host?, port?, path?, query?, fragment?}): Uri` —
    /// replacing reassembly by hand.
    ///
    /// A fresh `Uri`, not a mutated one: every `Core` class except § 9's
    /// collections is built once and read, and a URI that could change under a
    /// caller who had already validated it is the shape this exists to avoid.
    ///
    /// **It replaces and never removes** — [`written`] owns why, and the one
    /// component that is not on the bag at all, `userInfo`, is carried over
    /// unchanged, so `with` can neither add nor drop a credential. The result
    /// goes back through [`read`] and then [`unmoved`], which is what makes a
    /// bad option a throw rather than a `Uri` describing somewhere else.
    ///
    /// An empty port on the receiver has already become "no port" by the time
    /// it reaches a slot ([`port_of`]), so a `with` that does not mention the
    /// port drops the `:` that was written — RFC 3986 § 3.2.3's own
    /// instruction, and the one place a round trip through `with` is not the
    /// identity.
    fn mwl_core_uri_with(_ctx, args: [7]) {
        let receiver = crate::instance::receiver(args[0], &CLASS, "with")?;
        let slots: [Value; 8] =
            std::array::from_fn(|index| crate::instance::slot(receiver, index));
        // The one option declared `int` rather than `string`, so it is read
        // here rather than through `written`. An out-of-range one still
        // recomposes and still parses; `port_of` inside `built` is what
        // refuses it, with the message that names why.
        let port = if matches!(args[3].tag(), Some(Tag::Null)) {
            slots[PORT_SLOT].as_int()
        } else {
            Some(args[3].as_int().ok_or_else(|| {
                Fault::fatal(format!(
                    "Core\\Uri::with expected an `int` for its `port` option, got tag {}",
                    args[3].tag_byte()
                ))
            })?)
        };
        let port = port.map(|port| port.to_string());
        let composed = Composed {
            scheme: written(&args[1], "scheme")?.or(held(&slots, SCHEME_SLOT, "with")?),
            user_info: held(&slots, USER_INFO_SLOT, "with")?,
            host: written(&args[2], "host")?.or(held(&slots, HOST_SLOT, "with")?),
            port: port.as_deref(),
            path: written(&args[4], "path")?
                .or(held(&slots, PATH_SLOT, "with")?)
                .unwrap_or(""),
            query: written(&args[5], "query")?.or(held(&slots, QUERY_SLOT, "with")?),
            fragment: written(&args[6], "fragment")?.or(held(&slots, FRAGMENT_SLOT, "with")?),
        };
        let text = recompose(&composed);
        let reference = read(&text, "with")?;
        unmoved(&composed, &reference)?;

        built(&reference, "with")
    }
}

mwl_runtime::mwl_helper! {
    /// `$uri->resolve(string $reference): Uri` — RFC 3986 § 5's reference
    /// resolution, which PHP has no function for at all.
    ///
    /// The receiver is the **base** and must therefore be an absolute URI
    /// (§ 5.2.1), so resolving against a relative one throws rather than
    /// guessing. Its fragment is dropped first, which is not a normalization:
    /// § 5.1 defines a base URI as one without a fragment, and every
    /// implementation that "supports" a base with one is doing this silently.
    ///
    /// This is the one member here that rewrites a path — § 5.2.4's
    /// dot-segment removal — and it is the place the RFC asks for it. An
    /// **opaque** base, one with a rootless path and no authority
    /// (`mailto:a@b`), has no path to merge a relative reference into and
    /// throws saying so.
    fn mwl_core_uri_resolve(_ctx, args: [2]) {
        let receiver = crate::instance::receiver(args[0], &CLASS, "resolve")?;
        let slots: [Value; 8] =
            std::array::from_fn(|index| crate::instance::slot(receiver, index));
        let base_text = held(&slots, TEXT_SLOT, "resolve")?
            .ok_or_else(|| Fault::fatal("Core\\Uri::resolve found a null `text` slot"))?;
        let base = Uri::parse(base_text).map_err(|_| {
            Fault::thrown(
                "Core\\Uri::resolve(): the receiver is a relative reference, and RFC 3986 \
                 § 5.2.1 resolves against an absolute URI. Give this `Uri` a scheme first, \
                 or resolve against one that has one"
                    .to_owned(),
            )
        })?;
        let text = args[1].as_text().ok_or_else(|| {
            Fault::fatal(format!(
                "Core\\Uri::resolve expected {:?}, got tag {}",
                Tag::Str,
                args[1].tag_byte()
            ))
        })?;
        let reference = read(text, "resolve")?;
        let resolved = reference
            .resolve_against(&base.strip_fragment())
            .map_err(|error| {
                let refused = match error {
                    ResolveError::InvalidReferenceAgainstOpaqueBase => {
                        "the base has no authority and a rootless path, so it is opaque and there \
                         is no path for a relative reference to be merged into"
                    }
                    // `resolve_against` allows path underflow, and the base's
                    // fragment is stripped above, so neither of the other two
                    // is reachable from here.
                    _ => "RFC 3986 § 5.2 does not define a result for this pair",
                };
                Fault::thrown(format!("Core\\Uri::resolve(): {refused}"))
            })?;

        built(&UriRef::parse(resolved.as_str()).map_err(|_| {
            Fault::fatal("Core\\Uri::resolve produced text `fluent-uri` will not read back")
        })?, "resolve")
    }
}

mwl_runtime::mwl_helper! {
    /// `$uri->compareTo(Uri $other): int` — `Comparable`'s member
    /// ([ADR 0013](../../../../docs/adr/0013-comparable-interface.md)), over
    /// the two references' RFC 3986 § 6.2.2 normal forms.
    ///
    /// This is the member that answers "are these the same URI", because
    /// [ADR 0090](../../../../docs/adr/0090-one-equality-operator-and-disjoint-types-do-not-compile.md)
    /// § 4 keeps `==` on two objects meaning *the same object* and names
    /// `compareTo($other) == 0` as the spelling for the other question. What
    /// normalizing does and where it stops is the module docs' own section;
    /// nothing here rewrites the receiver, so `$uri->toString()` still answers
    /// with the text that was parsed.
    ///
    /// **Cost:** one `String` per non-empty component of each side, freed
    /// before the member returns. Bounded by the two references' own lengths,
    /// which is why the normal forms are computed per call rather than cached
    /// in an eighth slot every `Uri` would pay for and most would never read.
    fn mwl_core_uri_compare_to(_ctx, args: [2]) {
        let left = equivalent(args, 0, "compareTo")?;
        let right = equivalent(args, 1, "compareTo")?;
        Ok(Value::int(match left.cmp(&right) {
            std::cmp::Ordering::Less => -1,
            std::cmp::Ordering::Equal => 0,
            std::cmp::Ordering::Greater => 1,
        }))
    }
}

mwl_runtime::mwl_helper! {
    /// `Core\Uri::encodeComponent(string $s): string` — replacing PHP's
    /// `rawurlencode`.
    ///
    /// **For a piece of a URI**: a path segment, a fragment, or one side of a
    /// query pair being assembled by hand. A space becomes `%20`, because that
    /// is what a space is inside a URI — a `+` there is a literal `+`.
    ///
    /// Every byte outside RFC 3986 § 2.3's unreserved set is escaped,
    /// including the reserved delimiters `/ ? # & =`. That is what makes the
    /// answer safe to *interpolate*: a segment holding a `/` cannot climb out
    /// of its position in the path, and a value holding an `&` cannot open a
    /// second query pair. Escaping only the unsafe-looking bytes is how the
    /// injection this member exists to prevent gets back in.
    fn mwl_core_uri_encode_component(_ctx, args: [1]) {
        let text = text_of(args, "encodeComponent")?;

        produced(&encode(text.as_bytes(), Form::Component))
    }
}

mwl_runtime::mwl_helper! {
    /// `Core\Uri::decodeComponent(string $s): string` — replacing PHP's
    /// `rawurldecode`.
    ///
    /// The exact inverse of [`mwl_core_uri_encode_component`] for text that
    /// member produced. A `+` is a literal `+`, which is the whole reason this
    /// is a different member from `decodeFormValue` rather than an option on
    /// one: reading a form value with this decoder turns every space the user
    /// typed into a `+`.
    ///
    /// A malformed escape decodes to itself and non-UTF-8 octets throw — the
    /// module docs and gap 2 own both.
    fn mwl_core_uri_decode_component(_ctx, args: [1]) {
        let text = text_of(args, "decodeComponent")?;

        decoded(decode(text, Form::Component), "decodeComponent")
    }
}

mwl_runtime::mwl_helper! {
    /// `Core\Uri::encodeFormValue(string $s): string` — replacing PHP's
    /// `urlencode`.
    ///
    /// **For a value in an `application/x-www-form-urlencoded` payload**: a
    /// query-string pair or a POST body. A space becomes `+` and `~` becomes
    /// `%7E`, which are the two bytes this member's set differs from
    /// [`mwl_core_uri_encode_component`]'s on; the module docs own why the
    /// difference is kept rather than collapsed.
    ///
    /// A program building a whole query string reaches for `Uri::buildQuery`
    /// instead (gap 3), which writes the `=` and the `&` as well. This member
    /// is one side of one pair.
    fn mwl_core_uri_encode_form_value(_ctx, args: [1]) {
        let text = text_of(args, "encodeFormValue")?;

        produced(&encode(text.as_bytes(), Form::FormValue))
    }
}

mwl_runtime::mwl_helper! {
    /// `Core\Uri::decodeFormValue(string $s): string` — replacing PHP's
    /// `urldecode`.
    ///
    /// The inverse of [`mwl_core_uri_encode_form_value`]: `+` is a space, and
    /// `%2B` is the `+` the user actually typed. Both spellings of a space
    /// therefore read, since `%20` is still an escape — which is what makes
    /// this the right decoder for a query string written by something that
    /// followed RFC 3986 rather than the form encoding.
    ///
    /// A malformed escape decodes to itself and non-UTF-8 octets throw — the
    /// module docs and gap 2 own both.
    fn mwl_core_uri_decode_form_value(_ctx, args: [1]) {
        let text = text_of(args, "decodeFormValue")?;

        decoded(decode(text, Form::FormValue), "decodeFormValue")
    }
}

mwl_runtime::mwl_helper! {
    /// `Core\Uri::parseQuery(string $query): array<mixed>` — replacing PHP's
    /// `parse_str`, which it **returns** rather than populating variables
    /// with.
    ///
    /// Pairs are separated by `&`, each pair by its first `=`, and both halves
    /// are read with [`mwl_core_uri_decode_form_value`]'s decoder — so a `+`
    /// is a space on both sides of the `=`. A pair with no `=` at all has the
    /// empty string for its value, and one whose name decodes to nothing is
    /// dropped, both as PHP does.
    ///
    /// Names carry the bracket convention in full: `a[]=1&a[]=2` builds a
    /// list, `a[b]=c` builds a map, and the two nest to any depth. A repeated
    /// name without brackets keeps the last value. The module docs own the two
    /// places this diverges from `parse_str` and why.
    ///
    /// # Errors
    ///
    /// [`text_from`]'s throw, for a name or a value whose escapes decode to
    /// octets that are not UTF-8. Every value in the answer is a `string` or a
    /// nested `array<mixed>`, which is what the spec's `array<mixed>` says and
    /// why it is not `array<string>`.
    fn mwl_core_uri_parse_query(_ctx, args: [1]) {
        let query = text_of(args, "parseQuery")?;

        let mut out = MwlArray::new();
        for pair in query.split('&') {
            let (written_name, written_value) = pair.split_once('=').unwrap_or((pair, ""));
            let name = text_from(
                decode(written_name, Form::FormValue),
                "parseQuery",
                "the decoded name of a query parameter",
            )?;
            if name.is_empty() {
                continue;
            }
            let value = text_from(
                decode(written_value, Form::FormValue),
                "parseQuery",
                "the decoded value of a query parameter",
            )?;
            let value = Value::str(MwlStr::new(value.as_bytes()));
            match path_of(name.as_bytes()) {
                Some((base, path)) => insert(&mut out, base, &path, value),
                None => out.set(MwlStr::new(name.as_bytes()), value),
            }
        }

        Ok(Value::array(out))
    }
}

mwl_runtime::mwl_helper! {
    /// `Core\Uri::buildQuery(array<mixed> $parameters): string` — replacing
    /// PHP's `http_build_query`.
    ///
    /// [`mwl_core_uri_parse_query`]'s inverse over the same bracket
    /// convention, so `buildQuery(parseQuery($q))` answers a query string that
    /// parses back to the same array. It is not `$q` byte for byte, and cannot
    /// be: a query string has more than one spelling for the same parameters,
    /// and this member writes the one every reader accepts — pairs joined by
    /// `&`, both halves form-encoded, and a nested value under its whole
    /// bracket path with the indexes written out.
    ///
    /// A `null` value drops its pair entirely, since a query string cannot
    /// spell an absent value; [`scalar_text`] owns that and the one other
    /// place this differs from `as string`.
    ///
    /// # Errors
    ///
    /// [`scalar_text`]'s throw, for a value that is neither a scalar nor a
    /// nested array.
    fn mwl_core_uri_build_query(_ctx, args: [1]) {
        let parameters = args[0].array_ptr().ok_or_else(|| {
            Fault::fatal(format!(
                "Core\\Uri::buildQuery expected {:?}, got tag {}",
                Tag::Array,
                args[0].tag_byte()
            ))
        })?;

        produced(&build(parameters, "buildQuery")?)
    }
}

#[cfg(test)]
mod tests {
    use mwl_runtime::{Ctx, OutputSink, Value, call};

    use super::{Form, encode};

    /// Runs one member through the ADR 0002 boundary compiled code reaches it
    /// at — [`crate::random`]'s own test helper, for its reasons.
    fn run(
        member: unsafe extern "C" fn(*mut Ctx, *const Value, *mut Value) -> i32,
        subject: &str,
    ) -> Result<String, i32> {
        let mut ctx = Ctx::new(OutputSink::Sink);
        let argument = Value::str(mwl_runtime::MwlStr::new(subject.as_bytes()));
        let answer = call(member, &mut ctx, &[argument]);
        let out = answer.map(|value| {
            let text = String::from_utf8(
                value
                    .as_str_bytes()
                    .expect("every member here answers with a `string`")
                    .to_vec(),
            )
            .expect("ADR 0009 guarantees a `string` is UTF-8");
            #[expect(unsafe_code, reason = "this frame owns the reference the helper built")]
            unsafe {
                value.release();
            }
            text
        });
        #[expect(
            unsafe_code,
            reason = "this frame owns the argument it built, and every member \
                      here borrows rather than consumes"
        )]
        unsafe {
            argument.release();
        }
        out
    }

    /// The whole of ASCII plus one multi-byte character, round-tripped both
    /// ways: the assertion that catches a byte one encoder escapes and its own
    /// decoder does not restore.
    #[test]
    fn every_byte_round_trips_through_both_encodings() {
        let subject: String = (0..=127_u8).map(char::from).chain(['é', '→']).collect();
        for (encoder, decoder) in [
            (
                super::mwl_core_uri_encode_component
                    as unsafe extern "C" fn(*mut Ctx, *const Value, *mut Value) -> i32,
                super::mwl_core_uri_decode_component
                    as unsafe extern "C" fn(*mut Ctx, *const Value, *mut Value) -> i32,
            ),
            (
                super::mwl_core_uri_encode_form_value,
                super::mwl_core_uri_decode_form_value,
            ),
        ] {
            let encoded = run(encoder, &subject).expect("an encoder never fails");
            assert!(
                encoded.is_ascii(),
                "an encoded answer is ASCII by construction"
            );
            assert_eq!(
                run(decoder, &encoded).expect("its own output decodes"),
                subject
            );
        }
    }

    /// The two sets differ on exactly two bytes — the module docs' table,
    /// asserted rather than described, so widening either set fails here.
    #[test]
    fn the_two_encodings_differ_on_exactly_space_and_tilde() {
        let differing: Vec<u8> = (0..=127_u8)
            .filter(|&byte| {
                encode(char::from(byte).to_string().as_bytes(), Form::Component)
                    != encode(char::from(byte).to_string().as_bytes(), Form::FormValue)
            })
            .collect();
        assert_eq!(differing, [b' ', b'~']);
    }

    /// One entry of a `parseQuery` answer, rendered `key:value` with a nested
    /// array in braces — enough to compare a whole shape against PHP's own
    /// output on one line, and nothing a query string can write is ambiguous
    /// in it that the cases below rely on.
    fn rendered(value: Value) -> String {
        let Some(address) = value.array_ptr() else {
            return String::from_utf8(
                value
                    .as_str_bytes()
                    .expect("a leaf of the answer is a `string`")
                    .to_vec(),
            )
            .expect("ADR 0009 guarantees a `string` is UTF-8");
        };
        let array = crate::arr::borrowed(address);
        let mut out = String::from("{");
        let mut slot = 0;
        while let Some(live) = array.next_slot(slot) {
            if out.len() > 1 {
                out.push(',');
            }
            let key = array.key_at(live).expect("a live slot has a key");
            out.push_str(&String::from_utf8_lossy(key.as_bytes()));
            out.push(':');
            out.push_str(&rendered(
                array.value_at(live).expect("a live slot has a value"),
            ));
            slot = live + 1;
        }
        out.push('}');
        out
    }

    /// `Core\Uri::parseQuery(query)`, rendered by [`rendered`].
    fn parsed(query: &str) -> Result<String, i32> {
        let mut ctx = Ctx::new(OutputSink::Sink);
        let argument = Value::str(mwl_runtime::MwlStr::new(query.as_bytes()));
        let answer = call(super::mwl_core_uri_parse_query, &mut ctx, &[argument]);
        let out = answer.map(|value| {
            let text = rendered(value);
            #[expect(unsafe_code, reason = "this frame owns the array the helper built")]
            unsafe {
                value.release();
            }
            text
        });
        #[expect(unsafe_code, reason = "this frame owns the argument it built")]
        unsafe {
            argument.release();
        }
        out
    }

    /// The bracket convention, row for row against what PHP 8.5's `parse_str`
    /// answers for the same query — including the parts that look like
    /// accidents: last-value-wins, a scalar and a list replacing each other,
    /// and appends numbered from the highest integer key already used.
    #[test]
    fn the_bracket_convention_answers_what_parse_str_answers() {
        for (query, expected) in [
            ("", "{}"),
            ("=v", "{}"),
            ("&&a=1", "{a:1}"),
            ("a", "{a:}"),
            ("a[]", "{a:{0:}}"),
            ("a=1&b[]=2&b[]=3&c[k]=v", "{a:1,b:{0:2,1:3},c:{k:v}}"),
            ("a=1&a=2", "{a:2}"),
            ("a[b][c]=d", "{a:{b:{c:d}}}"),
            ("a[1]=x&a[0]=y", "{a:{1:x,0:y}}"),
            ("a[]=1&a[b]=2", "{a:{0:1,b:2}}"),
            ("a=1&a[]=2", "{a:{0:2}}"),
            ("a=1&a[b]=2", "{a:{b:2}}"),
            ("a[]=1&a=2", "{a:2}"),
            ("a%5Bb%5D=c", "{a:{b:c}}"),
            ("a[b.c]=1", "{a:{b.c:1}}"),
            ("a[][]=1&a[][]=2", "{a:{0:{0:1},1:{0:2}}}"),
            ("a[][x]=1&a[][y]=2", "{a:{0:{x:1},1:{y:2}}}"),
            ("a[]=1&a[3]=x&a[]=y", "{a:{0:1,3:x,4:y}}"),
            ("x[0]=a&x[]=b", "{x:{0:a,1:b}}"),
            ("a[0][x]=1&a[]=2", "{a:{0:{x:1},1:2}}"),
            ("a[07]=x&a[]=y", "{a:{07:x,0:y}}"),
        ] {
            assert_eq!(parsed(query).expect("no throw"), expected, "for {query:?}");
        }
    }

    /// Both halves of a pair are read with the form decoder, so a `+` is a
    /// space on the name's side too — and no character in either is rewritten,
    /// which is where PHP's variable-registering heritage is left behind. The
    /// module docs own each of these divergences.
    #[test]
    fn a_key_is_never_rewritten_and_a_malformed_name_stays_whole() {
        for (query, expected) in [
            ("+a+=+b+", "{ a : b }"),
            ("a.b=1", "{a.b:1}"),
            ("a b=1", "{a b:1}"),
            ("a[ ]=1", "{a:{ :1}}"),
            ("a[b=c", "{a[b:c}"),
            ("a[b]c=d", "{a[b]c:d}"),
            ("[]=1", "{[]:1}"),
        ] {
            assert_eq!(parsed(query).expect("no throw"), expected, "for {query:?}");
        }
    }

    /// `Core\Uri::buildQuery(Core\Uri::parseQuery(query))` — the round trip
    /// both members are specified against, run through the same boundary.
    fn rebuilt(query: &str) -> Result<String, i32> {
        let mut ctx = Ctx::new(OutputSink::Sink);
        let argument = Value::str(mwl_runtime::MwlStr::new(query.as_bytes()));
        let parsed = call(super::mwl_core_uri_parse_query, &mut ctx, &[argument]);
        #[expect(unsafe_code, reason = "this frame owns the argument it built")]
        unsafe {
            argument.release();
        }
        let parsed = parsed?;
        let built = call(super::mwl_core_uri_build_query, &mut ctx, &[parsed]);
        #[expect(unsafe_code, reason = "this frame owns the array `parseQuery` built")]
        unsafe {
            parsed.release();
        }
        let value = built?;
        let text = String::from_utf8(
            value
                .as_str_bytes()
                .expect("`buildQuery` answers a `string`")
                .to_vec(),
        )
        .expect("ADR 0009 guarantees a `string` is UTF-8");
        #[expect(unsafe_code, reason = "this frame owns the string the helper built")]
        unsafe {
            value.release();
        }
        Ok(text)
    }

    /// What PHP's `http_build_query` writes for the array its own `parse_str`
    /// read from the same query — including the escaped structural brackets
    /// and the indexes written out where the query wrote `[]`.
    #[test]
    fn build_query_writes_what_http_build_query_writes() {
        for (query, expected) in [
            ("", ""),
            ("a=", "a="),
            ("a b=x y", "a+b=x+y"),
            (
                "a=1&b[]=2&b[]=3&c[k]=v",
                "a=1&b%5B0%5D=2&b%5B1%5D=3&c%5Bk%5D=v",
            ),
            (
                "a[b][c]=d&a[b][e][]=f",
                "a%5Bb%5D%5Bc%5D=d&a%5Bb%5D%5Be%5D%5B0%5D=f",
            ),
            ("a[]=1&a[3]=x&a[]=y", "a%5B0%5D=1&a%5B3%5D=x&a%5B4%5D=y"),
        ] {
            assert_eq!(rebuilt(query).expect("no throw"), expected, "for {query:?}");
        }
    }

    /// The property the pair actually promises: the *text* a round trip
    /// answers is not the input's, but it is its own — so the parameters
    /// survive any number of trips. Writing `[]` instead of the indexes would
    /// fail here on the third row by renumbering `a[3]`.
    #[test]
    fn the_round_trip_reaches_a_fixed_point_in_one_step() {
        for query in [
            "a=1&b[]=2&b[]=3&c[k]=v",
            "a[b][c]=d&a[b][e][]=f",
            "a[]=1&a[3]=x&a[]=y",
            "a=1&a[]=2",
            "a b=x y&c~d=e+f",
            "a[b=c",
        ] {
            let once = rebuilt(query).expect("no throw");
            assert_eq!(rebuilt(&once).expect("no throw"), once, "for {query:?}");
        }
    }

    /// A name's escapes are decoded to octets exactly as a value's are, so
    /// either side can carry bytes no `string` holds — gap 2, on both.
    #[test]
    fn a_non_utf8_escape_in_either_half_of_a_pair_throws() {
        assert!(parsed("a=%FF").is_err());
        assert!(parsed("%FF=a").is_err());
        assert!(parsed("a[%FF]=b").is_err());
    }

    /// A decoded octet outside UTF-8 has no `string` to land in, so the member
    /// throws rather than substituting — gap 2, and ADR 0009 § 3's rule.
    #[test]
    fn a_non_utf8_octet_throws_rather_than_being_replaced() {
        assert!(run(super::mwl_core_uri_decode_component, "a%FFb").is_err());
        assert!(run(super::mwl_core_uri_decode_form_value, "%C3%28").is_err());
    }

    /// PHP leaves a `%` that does not begin two hex digits exactly as it
    /// stands, and so does this — the module docs own why a decoder at the
    /// edge of a request does not throw over one.
    #[test]
    fn a_malformed_escape_decodes_to_itself() {
        for (subject, expected) in [
            ("%zz", "%zz"),
            ("%4", "%4"),
            ("100%", "100%"),
            ("%%41", "%A"),
            ("%2f", "/"),
        ] {
            assert_eq!(
                run(super::mwl_core_uri_decode_component, subject).expect("no throw"),
                expected
            );
        }
    }

    /// The property the module docs promise and the whole reason `url` was not
    /// the pick: what goes in comes back out. Every row here is text the
    /// WHATWG URL Standard would have rewritten — a mixed-case scheme and
    /// host, a port that matches the scheme's default, dot segments, an
    /// escape that did not need escaping — and none of it moves.
    ///
    /// A `.mwlt` case pins the components one at a time; this pins that the
    /// *text* is untouched, which is the assertion that fails the day someone
    /// swaps the crate underneath.
    #[test]
    fn nothing_is_normalized_on_the_way_through() {
        for subject in [
            "HTTP://Example.COM:80/a/../b",
            "https://example.com/%7Euser/",
            "http://example.com",
            "file:///tmp/x",
            "//host/path",
            "/relative?a=1#f",
            "a/b:c",
            "?just-a-query",
            "#just-a-fragment",
            "",
        ] {
            let reference = super::read(subject, "parse").expect("a URI reference");
            assert_eq!(reference.as_str(), subject);
        }
    }

    /// RFC 3986's grammar, refused byte by byte — the line this member draws,
    /// stated where a caller can read it. A space and a `<` are the two PHP's
    /// `parse_url` waves through, and a non-ASCII byte is RFC 3987's business
    /// rather than this member's.
    #[test]
    fn the_grammar_refuses_what_rfc_3986_refuses() {
        for subject in [
            "http://example.com/a b",
            "http://example.com/a<b",
            "http://example.com/a\u{7f}b",
            "http://example.com/a\nb",
            "http://example.com/%zz",
            "http://example.com/%4",
            "http://example.com/ünicode",
            "http://[::g]/",
        ] {
            assert!(
                super::read(subject, "parse").is_err(),
                "{subject:?} is not a URI reference"
            );
        }
    }

    /// RFC 3986 § 3.2.3's grammar is `*DIGIT` and its prose is "a TCP port
    /// number", and [`super::port_of`] is where the two are reconciled: an
    /// empty port is absent, a `u16` is itself, and anything above one is
    /// refused rather than truncated.
    #[test]
    fn a_port_is_a_tcp_port_or_it_is_refused() {
        let port = |subject: &str| {
            let reference = super::read(subject, "parse").expect("a URI reference");
            let authority = reference.authority().expect("an authority");
            super::port_of(&authority, "parse")
        };
        assert_eq!(port("//h:8443/").expect("in range"), Some(8443));
        assert_eq!(port("//h:65535/").expect("in range"), Some(65535));
        assert_eq!(port("//h:/").expect("empty is absent"), None);
        assert_eq!(port("//h/").expect("absent is absent"), None);
        assert!(port("//h:65536/").is_err());
        assert!(port("//h:99999999999999999999/").is_err());
    }

    /// RFC 3986 § 5.3's guarantee, which is what makes `$uri->with({})` the
    /// identity: recomposing a reference out of the components it was parsed
    /// into gives the reference back. Only an empty port moves, and
    /// [`super::port_of`] owns why.
    #[test]
    fn recomposing_a_parsed_reference_gives_it_back() {
        for subject in [
            "https://user@example.com:8443/a/b?x=1#top",
            "HTTP://Example.COM:80/a/../b",
            "file:///tmp/x",
            "http://[::1]:8080/x",
            "//host/path",
            "/relative?a=1#f",
            "mailto:someone@example.test",
            "?just-a-query",
            "#just-a-fragment",
            "",
        ] {
            let reference = super::read(subject, "parse").expect("a URI reference");
            let authority = reference.authority();
            let composed = super::Composed {
                scheme: reference.scheme().map(super::Scheme::as_str),
                user_info: authority
                    .and_then(|held| held.userinfo())
                    .map(super::EStr::as_str),
                host: authority.as_ref().map(super::Authority::host),
                port: authority
                    .and_then(|held| held.port())
                    .map(super::EStr::as_str),
                path: reference.path().as_str(),
                query: reference.query().map(super::EStr::as_str),
                fragment: reference.fragment().map(super::EStr::as_str),
            };
            assert_eq!(super::recompose(&composed), subject);
        }
    }

    /// The check `with` does that re-parsing alone would not: each row here
    /// recomposes to text that parses perfectly well and is a *different* URI
    /// than the one asked for, because a component crossed a delimiter. The
    /// name in the throw is the component it landed in, which is where the
    /// caller's missing `/` actually went.
    #[test]
    fn a_component_that_crosses_a_delimiter_is_refused_by_name() {
        let bare = super::Composed {
            scheme: None,
            user_info: None,
            host: None,
            port: None,
            path: "",
            query: None,
            fragment: None,
        };
        let rows = [
            // A path beside a host that does not begin with `/` joins the host.
            (
                super::Composed {
                    scheme: Some("https"),
                    host: Some("h"),
                    path: "a",
                    ..bare
                },
                "host",
            ),
            // A first path segment holding a `:`, with no scheme in front of
            // it, becomes the scheme.
            (
                super::Composed {
                    path: "a:b",
                    ..bare
                },
                "scheme",
            ),
            // A rootless path opening with `//` and no authority becomes one.
            (
                super::Composed {
                    scheme: Some("x"),
                    path: "//h/p",
                    ..bare
                },
                "host",
            ),
        ];
        for (composed, moved) in rows {
            let text = super::recompose(&composed);
            let reference = super::read(&text, "with").expect("the recomposition still parses");
            let refused = super::unmoved(&composed, &reference).expect_err("the component moved");
            let mwl_runtime::Fault::Thrown(_, message) = refused else {
                panic!("`with` throws rather than faulting");
            };
            assert!(
                message.contains(&format!("`{moved}`")),
                "{text:?} should name `{moved}`, said {message}"
            );
        }
    }

    /// One reference per row, each written the way a client might: the corpus
    /// both round-trip halves below run over.
    const CORPUS: &[&str] = &[
        "http://example.com/",
        "https://user:pw@example.com:8443/a/b?c=d#e",
        "HTTP://Example.COM/a%7Eb",
        "file:///tmp/x",
        "mailto:someone@example.com",
        "urn:isbn:0451450523",
        "http://[2001:db8::1]:8080/",
        "http://example.com/a//b/./c/../d/",
        "//example.com/protocol-relative",
        "/absolute/path?q",
        "relative/path",
        "?query-only",
        "#fragment-only",
        "",
    ];

    /// `Core\Uri::parse($text)`, as the instance value the caller then owns.
    fn uri_of(text: &str) -> Value {
        let mut ctx = Ctx::new(OutputSink::Sink);
        let argument = Value::str(mwl_runtime::MwlStr::new(text.as_bytes()));
        let uri = call(super::mwl_core_uri_parse, &mut ctx, &[argument])
            .expect("every subject here is a URI reference");
        #[expect(unsafe_code, reason = "this frame owns the argument it built")]
        unsafe {
            argument.release();
        }
        uri
    }

    /// `Core\Uri::parse($left)->compareTo(Core\Uri::parse($right))`.
    fn compared(left: &str, right: &str) -> i64 {
        let mut ctx = Ctx::new(OutputSink::Sink);
        let (left, right) = (uri_of(left), uri_of(right));
        let answer = call(super::mwl_core_uri_compare_to, &mut ctx, &[left, right])
            .expect("comparing two `Uri`s never throws")
            .as_int()
            .expect("`compareTo` answers an `int`");
        #[expect(
            unsafe_code,
            reason = "this frame owns both instances, and `compareTo` borrows \
                      rather than consumes"
        )]
        unsafe {
            left.release();
            right.release();
        }
        answer
    }

    /// `Core\Uri::parse($text)->toString()` — the text the instance kept.
    fn own_text(text: &str) -> String {
        let mut ctx = Ctx::new(OutputSink::Sink);
        let uri = uri_of(text);
        let answer = call(super::mwl_core_uri_to_string, &mut ctx, &[uri])
            .expect("`toString` reads a slot and never throws");
        let out = String::from_utf8(
            answer
                .as_str_bytes()
                .expect("`toString` answers a `string`")
                .to_vec(),
        )
        .expect("ADR 0009 guarantees a `string` is UTF-8");
        #[expect(
            unsafe_code,
            reason = "this frame owns the instance and the reference `toString` \
                      handed back"
        )]
        unsafe {
            answer.release();
            uri.release();
        }
        out
    }

    /// RFC 3986 § 6.2.2's three normalizations, and the three places the
    /// member deliberately stops short of them — the module docs' *Comparison
    /// normalizes* section, asserted rather than described.
    #[test]
    fn two_uris_compare_by_normalized_components() {
        // One URI, two spellings, per § 6.2.2.1 (case), § 6.2.2.2
        // (percent-encoding) and § 6.2.2.3 (dot segments).
        for (left, right) in [
            ("HTTP://example.com/a", "http://example.com/a"),
            ("http://Example.COM/a", "http://example.com/a"),
            ("http://example.com/a%7Eb", "http://example.com/a~b"),
            ("http://example.com/a%2fb", "http://example.com/a%2Fb"),
            ("http://u%73er@example.com/", "http://user@example.com/"),
            ("http://example.com/x/./y", "http://example.com/x/y"),
            ("http://example.com/x/../y", "http://example.com/y"),
            ("http://example.com/x/y/..", "http://example.com/x/"),
            ("http://example.com/..", "http://example.com/"),
        ] {
            assert_eq!(
                compared(left, right),
                0,
                "{left:?} and {right:?} are one URI under RFC 3986 § 6.2.2"
            );
        }

        // And the lines it does not cross. Each of these is a *different* URI
        // here, and the module docs say why for each.
        for (left, right, because) in [
            (
                "http://example.com:80/",
                "http://example.com/",
                "a default port is § 6.2.3, which needs a table of schemes",
            ),
            (
                "http://example.com",
                "http://example.com/",
                "an empty path is § 6.2.3 for the same reason",
            ),
            (
                "http://example.com/a%2Fb",
                "http://example.com/a/b",
                "an escape spelling a reserved octet stays escaped",
            ),
            (
                "http://example.com/A",
                "http://example.com/a",
                "only the scheme and the host are case-insensitive",
            ),
            (
                "http://example.com//a",
                "http://example.com/a",
                "an empty interior segment is a segment",
            ),
            (
                "../a",
                "a",
                "a relative reference's `..` is meaningful until it is resolved",
            ),
        ] {
            assert_ne!(
                compared(left, right),
                0,
                "{left:?} and {right:?} are two URIs — {because}"
            );
        }

        // The order is total and component-lexicographic, with an absent
        // component sorting before a present one.
        assert_eq!(compared("http://a/", "http://b/"), -1);
        assert_eq!(compared("http://b/", "http://a/"), 1);
        assert_eq!(compared("/a", "http:/a"), -1);
        for subject in CORPUS {
            assert_eq!(compared(subject, subject), 0, "{subject:?} equals itself");
        }
    }

    /// Two round trips, because `parse` promises two different things. Its
    /// *text* comes back byte for byte, which is the module docs' "reports
    /// rather than normalizes"; and RFC 3986 § 5.3's recomposition of the
    /// components it found parses back to an equivalent reference, which is
    /// what makes the seven readers a faithful decomposition rather than seven
    /// plausible substrings.
    #[test]
    fn a_parsed_uri_round_trips_through_its_own_text() {
        for subject in CORPUS {
            assert_eq!(
                &own_text(subject),
                subject,
                "`toString` answers the text that was parsed"
            );
            assert_eq!(
                compared(subject, &own_text(subject)),
                0,
                "re-parsing {subject:?}'s own text is the same URI"
            );

            let reference = super::read(subject, "compareTo").expect("the corpus parses");
            let authority = reference.authority();
            let composed = super::Composed {
                scheme: reference.scheme().map(super::Scheme::as_str),
                user_info: authority
                    .and_then(|held| held.userinfo())
                    .map(super::EStr::as_str),
                host: authority.as_ref().map(super::Authority::host),
                port: authority
                    .and_then(|held| held.port())
                    .map(super::EStr::as_str),
                path: reference.path().as_str(),
                query: reference.query().map(super::EStr::as_str),
                fragment: reference.fragment().map(super::EStr::as_str),
            };
            let recomposed = super::recompose(&composed);
            assert_eq!(
                compared(subject, &recomposed),
                0,
                "{subject:?} recomposes to {recomposed:?}, which is a different URI"
            );
        }
    }
}
