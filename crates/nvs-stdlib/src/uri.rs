//! `Core\Uri` — [docs/spec/01-core-library.md](/docs/spec/01-core-library.md)
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
//! The pair split is not an accident of PHP's history that Novis is copying. A
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
//! the two sets is paid by whoever compares a signature Novis computed against
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
//! ([ADR 0066 § 3a](/docs/decisions/0066.md)).
//! **There is no `Uri::isValid`** — it and `Uri::tryParse($text) != null` are
//! one predicate, and R17 keeps one of them. Which one is not arbitrary: a
//! validator that is a *separate implementation* from the parser is how
//! PHP's `filter_var(FILTER_VALIDATE_URL)` came to accept user-info that
//! `parse_url` read differently (CVE-2024-5458), so the surviving spelling is
//! the one that cannot drift from `parse` because it *is* `parse`.
//!
//! **`$text as ?Uri` does not compile**, and an earlier revision of `rule:expressions/nullable-conversion`
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
//! So the normalization is written once, on `$uri->compareTo($other)`, and
//! `$uri->sign` reaches that same function rather than carrying one of its own
//! — a canonical form used by nothing but a signature is a form no other test
//! constrains, which is where every framework's signed-URL bug in this space
//! has come from (`rule:core-api/signing-is-over-a-payload`). It is the whole
//! of § 6.2.2 and no more:
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
//! **`==` is still identity.** `rule:expressions/object-identity-equality` makes `$a == $b` on two objects ask whether they are the same object
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
//! because every Novis array key is a `string` already.
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
//! because [`NvsArray::append`] already keeps PHP's next-free-integer counter.
//! Past a key of `i64::MAX` there is no next integer, so `a[9223372036854775807]=x&a[]=y`
//! drops its second pair, as `parse_str` does.
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
//! `rule:classes/an-encoder-ends-a-cycle-by-identity` has nothing to carry
//! here, because [`build`]'s walk descends into **arrays and nothing else**:
//! the one shape with reference semantics, and so the one shape that can close
//! a cycle, is never entered. An object reaching a parameter's value is handed
//! whole to [`scalar_text`], which writes a carrier's own text or throws, and
//! an array cannot hold itself — [`nvs_runtime::graph`]'s module doc owns why
//! `$a[] = $a` appends a copy. The descent is therefore finite without a cap,
//! and shared substructure is written once per path that reaches it, which is
//! the same answer that rule gives.
//!
//! # What is not here: no dependency
//!
//! This half binds no outside crate, which is a deliberate exception to
//! [ground-rules.md](/docs/ground-rules.md)'s "an external
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
//! [ground-rules.md](/docs/ground-rules.md) decides that `parse`
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
//! (`rule:http-server/allow-url-pins-the-address`); a
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
//! [AGENTS.md](/AGENTS.md)'s memory rule spent on purpose: a `Uri`
//! holding only its text would re-parse on every accessor call, one holding
//! only its components would recompose on every `toString`, and this pays
//! about twice a URI's length, once, to make both O(1) on the request path.
//! Parsing is a single pass, so a hostile input costs O(n) here as it does
//! everywhere else in this module.
//!
//! # A decoder answers `bytes`
//!
//! Percent-decoding is defined over octets and a client may send any of them,
//! so [`nvs_core_uri_decode_component`] and [`nvs_core_uri_decode_form_value`]
//! answer `rule:types/bytes`'s `bytes` rather than
//! a `string`: `decodeComponent("%FF")` has an answer, and `%ff%fe%fd` is the
//! three octets it spells rather than a refusal. A caller who wants text
//! writes `as string`, which is that ADR § 3's checked row and throws in
//! exactly the place these members used to throw, one line later — so no
//! program is denied an answer it could have used, and the one place this
//! module diverged from PHP, whose strings are byte strings, is closed.
//!
//! The two encoders are untouched. They take text and answer text, and they
//! are this class's two [`Qual::Launder`] rows; a decoder answering octets
//! does not change what an encoder escapes.
//!
//! [`nvs_core_uri_parse_query`] answers those same octets for a **value**. A
//! **name** is the array key the pair is placed under and an array key is a
//! `string`, so a name whose escapes decode outside UTF-8 still refuses —
//! [`text_from`] is that refusal and names the reason at the site. The
//! decoding either half gets is the same [`decode`] either way, which is what
//! the frozen claim "a name and a value decode as octets too" is about.
//!
//! `Core\Request::query` and `Core\Request::post` share [`parse_query`]'s
//! bracket walk and **not** its value type: a served request's parameters are
//! read as text at the door, which is spec § 15's row rather than § 12's, and
//! [`Values`] is the one knob between them.
//!

//! # What these members do with a qualifier
//!
//! `rule:security/unclassified-parameter-refuses-tainted`'s classification, and the judgement that separates this class
//! from [`crate::time`]'s: **a parse here is [`Qual::Contagious`], not
//! [`Qual::Neutral`].** `Core\Time::parse` answers an instant, and an instant
//! is a closed space no byte of the argument survives into. A `Uri` is the
//! opposite shape: `host()`, `path()` and `query()` hand the caller's own text
//! straight back, component for component, which is exactly what *Comparison
//! normalizes* above is written to preserve. So text that arrives tainted
//! leaves tainted, and `resolve`, `with` and `parseQuery` are contagious for
//! the same reason — each answers something built out of its argument's bytes.
//!
//! **The two encoders are the class's whole point, and they are the only
//! [`Qual::Launder`] rows here.** Each names its sink in its own doc comment:
//! [`nvs_core_uri_encode_component`] launders for the URI grammar — the
//! escaping of `/ ? # & =` is what stops a segment climbing out of its
//! position in the path — and [`nvs_core_uri_encode_form_value`] launders for
//! an `application/x-www-form-urlencoded` body, where the escaped `&` and `=`
//! are what stops a value opening a pair of its own. Both are the *whole*
//! escape rather than the unsafe-looking bytes, which is the property the
//! claim rests on.
//!
//! The decoders stay [`Qual::Contagious`], and the asymmetry is deliberate: a
//! laundered value that is decoded again is content once more, so
//! `decodeComponent(encodeComponent($tainted))` is tainted. A decoder that
//! inherited its encoder's mark would launder every string that survived a
//! round trip, which is every string.

use std::mem::ManuallyDrop;

use fluent_uri::component::{Authority, Scheme};
use fluent_uri::pct_enc::EStr;
use fluent_uri::resolve::ResolveError;
use fluent_uri::{ParseErrorKind, Uri, UriRef};
use nvs_runtime::{Fault, HelperResult, NvsArray, NvsStr, Tag, Value};

use crate::registry::{
    ClassDoc, Const, CoreClass, CoreMethod, CoreOption, CoreTy, ErrorDoc, MethodDoc, ParamDoc,
    Qual, ShapeKeyDoc,
};
use crate::signature::{Confirmed, Domain};

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
    doc: Some(&CARD),
    methods: &[
        CoreMethod {
            name: "parse",
            names: &["uri"],
            params: &[CoreTy::Text(Qual::Contagious)],
            defaults: &[],
            return_ty: CoreTy::Instance(NAME),
            symbol: "nvs_core_uri_parse",
            doc: Some(&PARSE_DOC),
        },
        CoreMethod {
            name: "tryParse",
            names: &["uri"],
            params: &[CoreTy::Text(Qual::Contagious)],
            defaults: &[],
            return_ty: CoreTy::Nullable(&CoreTy::Instance(NAME)),
            symbol: "nvs_core_uri_try_parse",
            doc: Some(&TRY_PARSE_DOC),
        },
        CoreMethod {
            name: "encodeComponent",
            names: &["s"],
            params: &[CoreTy::Text(Qual::Launder)],
            defaults: &[],
            return_ty: CoreTy::Str,
            symbol: "nvs_core_uri_encode_component",
            doc: Some(&ENCODE_COMPONENT_DOC),
        },
        CoreMethod {
            name: "decodeComponent",
            names: &["s"],
            params: &[CoreTy::Text(Qual::Contagious)],
            defaults: &[],
            // `bytes`, not `string`: the module docs' *A decoder answers
            // `bytes`* owns why, and spec § 12's row says the same.
            return_ty: CoreTy::Bytes,
            symbol: "nvs_core_uri_decode_component",
            doc: Some(&DECODE_COMPONENT_DOC),
        },
        CoreMethod {
            name: "encodeFormValue",
            names: &["s"],
            params: &[CoreTy::Text(Qual::Launder)],
            defaults: &[],
            return_ty: CoreTy::Str,
            symbol: "nvs_core_uri_encode_form_value",
            doc: Some(&ENCODE_FORM_VALUE_DOC),
        },
        CoreMethod {
            name: "decodeFormValue",
            names: &["s"],
            params: &[CoreTy::Text(Qual::Contagious)],
            defaults: &[],
            // [`nvs_core_uri_decode_component`]'s answer type, for its reason.
            return_ty: CoreTy::Bytes,
            symbol: "nvs_core_uri_decode_form_value",
            doc: Some(&DECODE_FORM_VALUE_DOC),
        },
        CoreMethod {
            name: "parseQuery",
            names: &["query"],
            params: &[CoreTy::Text(Qual::Contagious)],
            defaults: &[],
            return_ty: CoreTy::Array(&CoreTy::Mixed),
            symbol: "nvs_core_uri_parse_query",
            doc: Some(&PARSE_QUERY_DOC),
        },
        CoreMethod {
            name: "buildQuery",
            names: &["parameters"],
            params: &[CoreTy::Array(&CoreTy::Mixed)],
            defaults: &[],
            return_ty: CoreTy::Str,
            symbol: "nvs_core_uri_build_query",
            doc: Some(&BUILD_QUERY_DOC),
        },
    ],
    instance: &[
        CoreMethod {
            name: "scheme",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Nullable(&CoreTy::Str),
            symbol: "nvs_core_uri_scheme",
            doc: Some(&SCHEME_DOC),
        },
        CoreMethod {
            name: "userInfo",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Nullable(&CoreTy::Str),
            symbol: "nvs_core_uri_user_info",
            doc: Some(&USER_INFO_DOC),
        },
        CoreMethod {
            name: "host",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Nullable(&CoreTy::Str),
            symbol: "nvs_core_uri_host",
            doc: Some(&HOST_DOC),
        },
        CoreMethod {
            name: "port",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Nullable(&CoreTy::Int),
            symbol: "nvs_core_uri_port",
            doc: Some(&PORT_DOC),
        },
        CoreMethod {
            name: "path",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Str,
            symbol: "nvs_core_uri_path",
            doc: Some(&PATH_DOC),
        },
        CoreMethod {
            name: "query",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Nullable(&CoreTy::Str),
            symbol: "nvs_core_uri_query",
            doc: Some(&QUERY_DOC),
        },
        CoreMethod {
            name: "fragment",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Nullable(&CoreTy::Str),
            symbol: "nvs_core_uri_fragment",
            doc: Some(&FRAGMENT_DOC),
        },
        CoreMethod {
            name: "toString",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Str,
            symbol: "nvs_core_uri_to_string",
            doc: Some(&TO_STRING_DOC),
        },
        CoreMethod {
            name: "with",
            names: &[],
            params: &[CoreTy::Options(&[
                CoreOption {
                    name: "scheme",
                    ty: CoreTy::Text(Qual::Contagious),
                    default: Const::Null,
                },
                CoreOption {
                    name: "host",
                    ty: CoreTy::Text(Qual::Contagious),
                    default: Const::Null,
                },
                CoreOption {
                    name: "port",
                    ty: CoreTy::Nullable(&CoreTy::Int),
                    default: Const::NeverWritten,
                },
                CoreOption {
                    name: "path",
                    ty: CoreTy::Text(Qual::Contagious),
                    default: Const::Null,
                },
                CoreOption {
                    name: "query",
                    ty: CoreTy::Nullable(&CoreTy::Text(Qual::Contagious)),
                    default: Const::NeverWritten,
                },
                CoreOption {
                    name: "fragment",
                    ty: CoreTy::Nullable(&CoreTy::Text(Qual::Contagious)),
                    default: Const::NeverWritten,
                },
            ])],
            defaults: &[],
            return_ty: CoreTy::Instance(NAME),
            symbol: "nvs_core_uri_with",
            doc: Some(&WITH_DOC),
        },
        CoreMethod {
            name: "queryParameter",
            names: &["name"],
            // `Qual::Neutral`, where every other row on this class that takes
            // text is `Qual::Contagious`: the answer is built out of the
            // *receiver's* query string, and no byte of the name survives into
            // it. The module docs' *What these members do with a qualifier*
            // owns the distinction.
            params: &[CoreTy::Text(Qual::Neutral)],
            defaults: &[],
            return_ty: CoreTy::Mixed,
            symbol: "nvs_core_uri_query_parameter",
            doc: Some(&QUERY_PARAMETER_DOC),
        },
        CoreMethod {
            name: "withQueryParameter",
            names: &["name", "value"],
            // Contagious where the reader beside it is neutral, and for the
            // same reason read the other way: this answer *is* built out of
            // the name's bytes and the value's, since both are written into
            // the query string it recomposes.
            params: &[CoreTy::Text(Qual::Contagious), CoreTy::Mixed],
            defaults: &[],
            return_ty: CoreTy::Instance(NAME),
            symbol: "nvs_core_uri_with_query_parameter",
            doc: Some(&WITH_QUERY_PARAMETER_DOC),
        },
        CoreMethod {
            name: "resolve",
            names: &["reference"],
            params: &[CoreTy::Text(Qual::Contagious)],
            defaults: &[],
            return_ty: CoreTy::Instance(NAME),
            symbol: "nvs_core_uri_resolve",
            doc: Some(&RESOLVE_DOC),
        },
        CoreMethod {
            name: "compareTo",
            names: &["other"],
            params: &[CoreTy::Instance(NAME)],
            defaults: &[],
            return_ty: CoreTy::Int,
            symbol: "nvs_core_uri_compare_to",
            doc: Some(&COMPARE_TO_DOC),
        },
        CoreMethod {
            name: "sign",
            // The shape [`crate::signature`] writes once for all three doors,
            // and nothing beside it: an options bag naming which parameters
            // are covered is where every framework's bypass has lived, so
            // there is no second parameter for one to arrive in
            // (`rule:core-api/signing-is-over-a-payload`).
            // `settings`, not `options`: a shape is a positional parameter a
            // caller writes in full, and `rule:core-api/a-lifetime-is-written`
            // is the whole reason it is not a bag — an omitted `until` does
            // not compile. The spec's § 12 signature column writes the name,
            // which is what `every_registry_rows_names_are_the_specs_signature_column`
            // reads it from.
            names: &["settings"],
            params: &[CoreTy::Shape(crate::signature::SIGNING)],
            defaults: &[],
            return_ty: CoreTy::Instance(NAME),
            symbol: "nvs_core_uri_sign",
            doc: Some(&SIGN_DOC),
        },
        CoreMethod {
            name: "verifySignature",
            // The ring alone. This member takes a *different* thing from
            // `sign` and answers a different thing
            // (`rule:core-api/each-door-takes-a-different-thing`): there is no
            // lifetime to write, because the one that counts rode inside the
            // token, and no payload, because the URL is the payload.
            names: &["keys"],
            params: &[CoreTy::Array(&crate::keyring::KEY)],
            defaults: &[],
            return_ty: CoreTy::Void,
            symbol: "nvs_core_uri_verify_signature",
            doc: Some(&VERIFY_SIGNATURE_DOC),
        },
    ],
    slots: &[
        "text", "scheme", "userInfo", "host", "port", "path", "query", "fragment",
    ],
    constants: &[],
};

/// `Core\Uri::parse`'s reference card — `rule:core-api/reference-card`.
const PARSE_DOC: MethodDoc = MethodDoc {
    short: "Reads the text of an address, such as `https://example.com/cart?id=4`, and returns \
            a `Uri`. Its methods return the parts: `scheme`, `userInfo`, `host`, `port`, `path`, \
            `query` and `fragment`. Each part is returned as it was written. Escapes such as \
            `%20` stay, and upper-case letters stay upper case. PHP's `parse_url` reads the same \
            parts.",
    params: &[ParamDoc {
        name: "uri",
        desc: "The text to read. A relative link, such as `/a?b`, is allowed. Its scheme is \
               `null`.",
        shape: &[],
    }],
    ret: "A `Uri`. Its methods return the parts of the address as they were written.",
    errors: &[ErrorDoc {
        error: "RuntimeError",
        desc: "The text is not an address under RFC 3986 (the standard for addresses). A space, \
               a control character, a non-ASCII character, a `%` without two hex digits and a \
               host in brackets that is not an IPv6 address all cause this. A port larger than \
               `65535` causes it too. The message does not repeat the text, because the text \
               can contain a password.",
    }],
};

/// `Core\Uri::tryParse`'s reference card — `rule:core-api/reference-card`.
const TRY_PARSE_DOC: MethodDoc = MethodDoc {
    short: "`Core\\Uri::parse` with `null` where it throws — the one spelling of \"is this text \
            a URI\", replacing `filter_var` with `FILTER_VALIDATE_URL`; its narrower question, \
            \"is it absolute\", is `tryParse($s)?->scheme() != null`.",
    params: &[ParamDoc {
        name: "uri",
        desc: "The text to read.",
        shape: &[],
    }],
    ret: "The `Uri`, or `null` for text `parse` would throw on — the grammar's refusals and an \
          out-of-range port alike.",
    errors: &[],
};

/// `Core\Uri::encodeComponent`'s reference card — `rule:core-api/reference-card`.
const ENCODE_COMPONENT_DOC: MethodDoc = MethodDoc {
    short: "Escapes `$s` so that it can be one part of a URI, such as a path segment, a fragment \
            or one side of a query pair. A space becomes `%20`. Every byte that is not a letter, \
            a digit or one of `-_.~` becomes a `%` and two hex digits. This includes `/ ? # & =`. \
            PHP's `rawurlencode` does the same.",
    params: &[ParamDoc {
        name: "s",
        desc: "The text to encode.",
        shape: &[],
    }],
    ret: "The escaped text, with upper-case hex digits. A `tainted` argument (text from outside \
          the program, such as a request) gives a plain result. No byte of the result can change \
          the structure of the URI.",
    errors: &[],
};

/// `Core\Uri::decodeComponent`'s reference card — `rule:core-api/reference-card`.
const DECODE_COMPONENT_DOC: MethodDoc = MethodDoc {
    short: "Decodes one escaped part of a URI, such as a path segment or a fragment. Each `%XX` \
            escape becomes the byte it encodes, so `%20` becomes a space. A `+` stays a `+`. \
            `Core\\Uri::encodeComponent` writes the escapes that this function reads. PHP's \
            `rawurldecode` does the same.",
    params: &[ParamDoc {
        name: "s",
        desc: "The text to decode.",
        shape: &[],
    }],
    ret: "The decoded bytes, as `bytes`, because an escape can give any byte. `as string` \
          converts them to text and throws an error when they are not valid UTF-8. A `%` that is \
          not followed by two hex digits is kept, so `100%` returns `100%`.",
    errors: &[],
};

/// `Core\Uri::encodeFormValue`'s reference card — `rule:core-api/reference-card`.
const ENCODE_FORM_VALUE_DOC: MethodDoc = MethodDoc {
    short: "Escapes `$s` so that it can be one value of a form, such as a value in a query \
            string or in the body of a POST request. A space becomes `+`. Every byte that is not \
            a letter, a digit or one of `-_.` becomes a `%` and two hex digits. This includes \
            `~`, `+`, `&` and `=`. PHP's `urlencode` does the same.",
    params: &[ParamDoc {
        name: "s",
        desc: "The text to encode.",
        shape: &[],
    }],
    ret: "The escaped text, with upper-case hex digits. A `tainted` argument (text from outside \
          the program, such as a request) gives a plain result. No byte of the result can start \
          a new pair of the form.",
    errors: &[],
};

/// `Core\Uri::decodeFormValue`'s reference card — `rule:core-api/reference-card`.
const DECODE_FORM_VALUE_DOC: MethodDoc = MethodDoc {
    short: "Decodes one value of a form, such as a value in a query string or in the body of a \
            POST request. A `+` becomes a space. Each `%XX` escape becomes the byte it encodes, \
            so `%2B` becomes a `+` and `%20` becomes a space. `Core\\Uri::encodeFormValue` writes \
            the escapes that this function reads. PHP's `urldecode` does the same.",
    params: &[ParamDoc {
        name: "s",
        desc: "The text to decode.",
        shape: &[],
    }],
    ret: "The decoded bytes, as `bytes`, because an escape can give any byte. `as string` \
          converts them to text and throws an error when they are not valid UTF-8. A `%` that is \
          not followed by two hex digits is kept, so `100%` returns `100%`.",
    errors: &[],
};

/// `Core\Uri::parseQuery`'s reference card — `rule:core-api/reference-card`.
const PARSE_QUERY_DOC: MethodDoc = MethodDoc {
    short: "Reads a query string, such as `q=red+shoes&page=2`, into an array. The text is split \
            into pairs at each `&`, and each pair is split at its first `=`. Names and values \
            are decoded like `decodeFormValue`, so `+` becomes a space. Brackets in a name build \
            nested arrays: `a[]=1&a[]=2` gives a list, and `a[b]=c` gives an array with the key \
            `b`. PHP's `parse_str` reads query strings the same way.",
    params: &[ParamDoc {
        name: "query",
        desc: "The query text, without the `?` at its start.",
        shape: &[],
    }],
    ret: "An array. Each value is `bytes` or another array. A value is `bytes` because an \
          escape can give any byte, and `as string` converts it to text. A pair without `=` has \
          an empty value. A pair with an empty name is left out. When a name appears twice \
          without brackets, the last value is kept.",
    errors: &[ErrorDoc {
        error: "RuntimeError",
        desc: "A name decodes to bytes that are not valid UTF-8. A name is a key of the array, \
               and a key must be text. A value can contain any bytes.",
    }],
};

/// `Core\Uri`'s class card — `rule:core-api/reference-card`.
const CARD: ClassDoc = ClassDoc {
    short: "Reads, builds and encodes URIs and query strings. `parse` reads a URI into its \
            parts, and `with` and `resolve` build a new one. The `encode` and `decode` methods \
            escape and unescape text, and `parseQuery` and `buildQuery` convert between a query \
            string and an array. `sign` and `verifySignature` sign a link and check it.",
};

/// `Core\Uri::buildQuery`'s reference card — `rule:core-api/reference-card`.
const BUILD_QUERY_DOC: MethodDoc = MethodDoc {
    short: "Writes the array `$parameters` as a query string, such as `page=2&sort=name`. It \
            gives the same text as PHP's `http_build_query`, and `Core\\Uri::parseQuery` reads \
            it back to the same array.",
    params: &[ParamDoc {
        name: "parameters",
        desc: "The names and values to write. A value is a scalar or another array. A nested \
               value is written under its whole path, such as `filter[size][0]=M`, with the \
               brackets escaped as `%5B` and `%5D`.",
        shape: &[],
    }],
    ret: "The query string, without a leading `?`. Names and values are escaped as \
          `Core\\Uri::encodeFormValue` escapes them, so a space is `+`. `true` is written as `1` \
          and `false` as `0`. A `null` value writes nothing, and neither does an empty array.",
    errors: &[ErrorDoc {
        error: "RuntimeError",
        desc: "A value is an object or a function, so it has no text form.",
    }],
};

/// `$uri->scheme`'s reference card — `rule:core-api/reference-card`.
const SCHEME_DOC: MethodDoc = MethodDoc {
    short: "Returns the scheme of the address, which is the part before the first `:`. For \
            `https://example.com/` the result is `https`. The scheme is returned as it was \
            written, so `HTTPS` stays upper case.",
    params: &[],
    ret: "The scheme, without the `:`. The result is `null` when the address has no scheme, \
          such as `/about` or `//example.com/`.",
    errors: &[],
};

/// `$uri->userInfo`'s reference card — `rule:core-api/reference-card`.
const USER_INFO_DOC: MethodDoc = MethodDoc {
    short: "Returns the user part of the address, which is the text between `//` and `@`. For \
            `ftp://ann@files.example.com/` the result is `ann`. A password written as \
            `ann:secret` is returned as one text. Escapes such as `%40` stay in it.",
    params: &[],
    ret: "The user part, without the `@`. The result is `null` when the address has no `@` \
          before its host. It is `\"\"` when nothing is written before the `@`.",
    errors: &[],
};

/// `$uri->host`'s reference card — `rule:core-api/reference-card`.
const HOST_DOC: MethodDoc = MethodDoc {
    short: "Returns the host of the address, such as `example.com`. The host is returned as it \
            was written, so upper-case letters stay upper case. An IPv6 address keeps its \
            brackets, such as `[::1]`.",
    params: &[],
    ret: "The host. The result is `null` when the address has no `//` part. It is `\"\"` when \
          the `//` part is empty, as in `file:///tmp`.",
    errors: &[],
};

/// `$uri->port`'s reference card — `rule:core-api/reference-card`.
const PORT_DOC: MethodDoc = MethodDoc {
    short: "Returns the port of the address as a number. For `http://example.com:8080/` the \
            result is `8080`.",
    params: &[],
    ret: "The port, from `0` to `65535`. The result is `null` when the address has no port. It \
          is also `null` when the `:` has no digits after it, as in `http://example.com:/`. A \
          default port, such as `443` for `https`, is never filled in.",
    errors: &[],
};

/// `$uri->path`'s reference card — `rule:core-api/reference-card`.
const PATH_DOC: MethodDoc = MethodDoc {
    short: "Returns the path of the address. For `https://example.com/docs/guide?page=2` the \
            result is `/docs/guide`. The path is returned as it was written, so escapes such as \
            `%20` and parts such as `/../` stay in it.",
    params: &[],
    ret: "The path. It is never `null`. An address with no path, such as `https://example.com`, \
          returns `\"\"`.",
    errors: &[],
};

/// `$uri->query`'s reference card — `rule:core-api/reference-card`.
const QUERY_DOC: MethodDoc = MethodDoc {
    short: "Returns the query of the address, which is the text after the `?`. For \
            `https://example.com/search?q=shoes#top` the result is `q=shoes`. Escapes such as \
            `%20` stay in it, and `Core\\Uri::parseQuery` reads it into an array.",
    params: &[],
    ret: "The query, without the `?`. The result is `null` when the address has no `?`. It is \
          `\"\"` when nothing is written after the `?`.",
    errors: &[],
};

/// `$uri->fragment`'s reference card — `rule:core-api/reference-card`.
const FRAGMENT_DOC: MethodDoc = MethodDoc {
    short: "Returns the fragment of the address, which is the text after the `#`. For \
            `https://example.com/guide#install` the result is `install`. Escapes such as `%20` \
            stay in it, and `Core\\Uri::decodeComponent` decodes them.",
    params: &[],
    ret: "The fragment, without the `#`. The result is `null` when the address has no `#`. It is \
          `\"\"` when nothing is written after the `#`.",
    errors: &[],
};

/// `$uri->toString`'s reference card — `rule:core-api/reference-card`.
const TO_STRING_DOC: MethodDoc = MethodDoc {
    short: "The reference this `Uri` was parsed from, byte for byte — not a recomposition — and \
            what `echo $uri` writes.",
    params: &[],
    ret: "The original text, unchanged.",
    errors: &[],
};

/// `$uri->with`'s reference card — `rule:core-api/reference-card`.
const WITH_DOC: MethodDoc = MethodDoc {
    short: "A fresh `Uri` with the named components replaced and every other one carried over, \
            replacing reassembly by hand. Writing `null` for `port`, `query` or `fragment` \
            removes that component, where leaving the key out carries it over; `userInfo` is \
            not on the bag, so it can neither add nor drop a credential.",
    params: &[
        ParamDoc {
            name: "scheme",
            desc: "The new scheme, without its `:`.",
            shape: &[],
        },
        ParamDoc {
            name: "host",
            desc: "The new host; an IPv6 literal carries its brackets.",
            shape: &[],
        },
        ParamDoc {
            name: "port",
            desc: "The new port, `0`–`65535`, or `null` to remove it.",
            shape: &[],
        },
        ParamDoc {
            name: "path",
            desc: "The new path, already percent-encoded; beside a host it must begin with `/`.",
            shape: &[],
        },
        ParamDoc {
            name: "query",
            desc: "The new query, already encoded and without its `?`, or `null` to remove it; \
                   `\"\"` is an empty query, which is a different thing.",
            shape: &[],
        },
        ParamDoc {
            name: "fragment",
            desc: "The new fragment, already encoded and without its `#`, or `null` to remove \
                   it.",
            shape: &[],
        },
    ],
    ret: "A new `Uri`; the receiver is unchanged. A receiver whose port was written empty \
          (`h:/`) loses that `:` on the way through.",
    errors: &[ErrorDoc {
        error: "RuntimeError",
        desc: "`port` is outside `0`–`65535`; a component makes the result text the RFC 3986 \
               grammar does not admit; or the components do not recompose to a URI that still \
               holds them — a `path` beside a `host` that lacks its leading `/` lands in the \
               host — and the first component that moved is named.",
    }],
};

/// `$uri->queryParameter`'s reference card — `rule:core-api/reference-card`.
const QUERY_PARAMETER_DOC: MethodDoc = MethodDoc {
    short: "Returns one value from the query of the address, by its name. For \
            `/search?q=red+shoes` the result of `queryParameter(\"q\")` is `red shoes`. The query \
            is read the same way as by `Core\\Uri::parseQuery`, and each call reads it again.",
    params: &[ParamDoc {
        name: "name",
        desc: "The name of the parameter, without brackets. For `a[b]=c`, the name is `\"a\"`.",
        shape: &[],
    }],
    ret: "The decoded value as `bytes`. Brackets in the name give an `array<mixed>`. The result \
          is `null` when the name is not in the query, and also when the address has no query. \
          `query()` tells these two apart.",
    errors: &[ErrorDoc {
        error: "RuntimeError",
        desc: "A name in the query has escapes that do not decode to valid UTF-8 text. \
               `Core\\Uri::parseQuery` throws the same error.",
    }],
};

/// `$uri->withQueryParameter`'s reference card — `rule:core-api/reference-card`.
const WITH_QUERY_PARAMETER_DOC: MethodDoc = MethodDoc {
    short: "A fresh `Uri` with one query parameter set, replaced or removed and every other pair \
            carried over — `Core\\Uri::parseQuery`, the edit and `Core\\Uri::buildQuery` in one \
            member instead of three at the call site.",
    params: &[
        ParamDoc {
            name: "name",
            desc: "The parameter's name, decoded and top-level; brackets are written by an array \
                   `value`, never by spelling them into the name.",
            shape: &[],
        },
        ParamDoc {
            name: "value",
            desc: "The new value: a scalar, or an array nested to any depth, which is written \
                   under the bracket convention. `null` removes the parameter.",
            shape: &[],
        },
    ],
    ret: "A new `Uri`; the receiver is unchanged. Removing the last parameter leaves no query at \
          all rather than a bare `?`, and every pair that is carried over is rewritten in \
          `buildQuery`'s spelling rather than the one it arrived in.",
    errors: &[ErrorDoc {
        error: "RuntimeError",
        desc: "A name in the receiver's own query decodes to octets that are not valid UTF-8; \
               `value` is neither a scalar nor a nested array; or the rebuilt reference does not \
               still hold every component it was written out of, which is `with`'s refusal \
               reached through the same recompose-and-reread path.",
    }],
};

/// `$uri->resolve`'s reference card — `rule:core-api/reference-card`.
const RESOLVE_DOC: MethodDoc = MethodDoc {
    short: "Turns a link found on a page into a full address, the way a browser does. The \
            address of the page is the base. For the base `https://example.com/blog/post` and \
            the link `../about`, the result is `https://example.com/about`. The `.` and `..` \
            parts are removed from the path.",
    params: &[ParamDoc {
        name: "reference",
        desc: "The link to follow. It can be a full address, a path such as `/about` or \
               `photo.jpg`, a query such as `?page=2`, or a fragment such as `#top`.",
        shape: &[],
    }],
    ret: "A new `Uri` with a scheme. The fragment of the base is not used. The base itself does \
          not change.",
    errors: &[ErrorDoc {
        error: "RuntimeError",
        desc: "The base has no scheme, such as `/blog/post`. The link has a character that is \
               not allowed in an address, such as a space. Or the base has no `/` after its \
               scheme, such as `mailto:ann@example.com`, so there is no path to add the link to.",
    }],
};

/// `$uri->compareTo`'s reference card — `rule:core-api/reference-card`.
const COMPARE_TO_DOC: MethodDoc = MethodDoc {
    short: "Compares this address with `$other`, to sort addresses or to check if two are the \
            same. Before it compares, it changes a copy of each one. The scheme and the host \
            become lower case. An escape such as `%7E` becomes `~` when that character needs no \
            escape. The `.` and `..` parts are removed from a path that starts with `/`. Neither \
            address itself changes.",
    params: &[ParamDoc {
        name: "other",
        desc: "The `Uri` to compare with.",
        shape: &[],
    }],
    ret: "`0` when both are the same address, `-1` when this address comes first, and `1` when \
          `$other` comes first. The parts are compared in this order: `scheme`, `userInfo`, \
          `host`, `port`, `path`, `query`, `fragment`. A missing part comes before a part that \
          is there. `http://example.com:80/` and `http://example.com/` are different, because \
          the default port of a scheme is not known.",
    errors: &[],
};

/// `$uri->sign`'s reference card — `rule:core-api/reference-card`.
const SIGN_DOC: MethodDoc = MethodDoc {
    short: "Answers the receiver with the reserved `_sig` query parameter set, over a signature \
            taken across everything `compareTo` normalizes — scheme, userInfo, host, port, path \
            and the query's parameters. Appending, removing or editing any parameter invalidates \
            it; reordering them does not, and neither does a fragment.",
    params: &[ParamDoc {
        name: "settings",
        desc: "The key ring and the lifetime, written as one literal because neither has a \
               sensible value this member could choose.",
        shape: &[
            ShapeKeyDoc {
                key: "keys",
                ty: "array<secret bytes>",
                desc: "The key ring, **newest first**: `$keys[0]` signs, and the rest exist so \
                       that a link minted before the last rotation still verifies. The same ring \
                       `Core\\Signature` takes, and a token minted at one door does not verify at \
                       the other.",
            },
            ShapeKeyDoc {
                key: "until",
                ty: "?Core\\Time\\Instant",
                desc: "When the link stops working, inside the signed bytes where a holder cannot \
                       edit it. `null` is the forever spelling, and it has to be written — a \
                       permanent signed URL is a permanent bearer credential, and it ends up in \
                       browser history, `Referer` headers and chat unfurls.",
            },
        ],
    }],
    ret: "A new `Uri`, the receiver with `_sig` set — so it composes with `with` and `toString` \
          like every other member here. A receiver already carrying `_sig` has it replaced rather \
          than nested, and the same URL under the same key and lifetime always mints the same \
          token. The token carries the signed form as well as the tag, so it adds about \
          `4/3 × (URL + 40)` characters. A **relative** reference signs without a scheme, host or \
          port, so its token is valid on any origin: `$uri->scheme()` is what says which you are \
          holding.",
    errors: &[ErrorDoc {
        error: "LogicError",
        desc: "`$settings.keys` is empty, so there is no newest key; or its first entry is not 32 \
               octets long — a `bytes` that was never a key.",
    }],
};

/// `$uri->verifySignature`'s reference card — `rule:core-api/reference-card`.
const VERIFY_SIGNATURE_DOC: MethodDoc = MethodDoc {
    short: "Checks the receiver's `_sig` against every key in `$keys`, and the lifetime that rode \
            inside it. Answers nothing: a signed URL carries no claims to hand back — the claim is \
            the URL the caller already holds — and a `bool` is a value a caller can drop.",
    params: &[ParamDoc {
        name: "keys",
        desc: "The same ring `sign` was given, newest first. A link minted under any key still in \
               the ring verifies; one minted under a key that has been dropped off the end does \
               not.",
        shape: &[],
    }],
    ret: "Nothing. Reaching the next statement is what says the URL is authentic and live.",
    errors: &[
        ErrorDoc {
            error: "LogicError",
            desc: "`$keys` is empty, or one of its entries is not 32 octets long.",
        },
        ErrorDoc {
            error: "RuntimeError",
            desc: "The URL is not one this ring signed — a parameter was added, removed or \
                   edited, the token was altered, it was minted for another door or under a \
                   retired key, there is no `_sig` at all, or there are two of them. Every one of \
                   those is one message. Expiry is the single refusal with a sentence of its own, \
                   because only the holder of a genuinely signed link ever reaches it.",
        },
    ],
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
        "nvs_core_uri_parse" => (nvs_core_uri_parse as *const ()).cast(),
        "nvs_core_uri_try_parse" => (nvs_core_uri_try_parse as *const ()).cast(),
        "nvs_core_uri_scheme" => (nvs_core_uri_scheme as *const ()).cast(),
        "nvs_core_uri_user_info" => (nvs_core_uri_user_info as *const ()).cast(),
        "nvs_core_uri_host" => (nvs_core_uri_host as *const ()).cast(),
        "nvs_core_uri_port" => (nvs_core_uri_port as *const ()).cast(),
        "nvs_core_uri_path" => (nvs_core_uri_path as *const ()).cast(),
        "nvs_core_uri_query" => (nvs_core_uri_query as *const ()).cast(),
        "nvs_core_uri_fragment" => (nvs_core_uri_fragment as *const ()).cast(),
        "nvs_core_uri_to_string" => (nvs_core_uri_to_string as *const ()).cast(),
        "nvs_core_uri_with" => (nvs_core_uri_with as *const ()).cast(),
        "nvs_core_uri_query_parameter" => (nvs_core_uri_query_parameter as *const ()).cast(),
        "nvs_core_uri_with_query_parameter" => {
            (nvs_core_uri_with_query_parameter as *const ()).cast()
        }
        "nvs_core_uri_resolve" => (nvs_core_uri_resolve as *const ()).cast(),
        "nvs_core_uri_compare_to" => (nvs_core_uri_compare_to as *const ()).cast(),
        "nvs_core_uri_sign" => (nvs_core_uri_sign as *const ()).cast(),
        "nvs_core_uri_verify_signature" => (nvs_core_uri_verify_signature as *const ()).cast(),
        "nvs_core_uri_encode_component" => (nvs_core_uri_encode_component as *const ()).cast(),
        "nvs_core_uri_decode_component" => (nvs_core_uri_decode_component as *const ()).cast(),
        "nvs_core_uri_encode_form_value" => (nvs_core_uri_encode_form_value as *const ()).cast(),
        "nvs_core_uri_decode_form_value" => (nvs_core_uri_decode_form_value as *const ()).cast(),
        "nvs_core_uri_parse_query" => (nvs_core_uri_parse_query as *const ()).cast(),
        "nvs_core_uri_build_query" => (nvs_core_uri_build_query as *const ()).cast(),
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
pub(crate) enum Form {
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
pub(crate) fn encode(text: &[u8], form: Form) -> String {
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
/// produces — the UTF-8 question is the caller's, and the module docs'
/// *A decoder answers `bytes`* owns who is left asking it.
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
/// `nvs_types` already checked the declared type, so a slot holding anything
/// else is a runtime-contract violation rather than anything a program can
/// cause — the same treatment [`crate::path`] gives its own arguments.
///
/// That is the only failure. The tag [`Value::as_text`] checks is itself
/// `rule:types/bytes`'s UTF-8 guarantee, so no encoding outcome is left to report.
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
    Ok(Value::str(NvsStr::new(text.as_bytes())))
}

/// `octets` as text, or a throw where they are not UTF-8. `subject` names
/// which octets they were, since [`parse_query`] asks this of a name and, for
/// [`Values::Text`], of a value.
///
/// # Errors
///
/// A [`Fault::thrown`] naming the member, under the class `owner` the program
/// called, and the offset of the first bad byte.
/// The offset is a position in text the caller supplied, so it is safe to name
/// and it is the one fact that makes the throw actionable — the octets
/// themselves are not quoted, since they are by definition not text.
fn text_from(octets: Vec<u8>, owner: &str, member: &str, subject: &str) -> Result<String, Fault> {
    String::from_utf8(octets).map_err(|error| {
        Fault::thrown(format!(
            "{owner}::{member}(): {subject} holds a byte a `string` cannot — byte {} begins a \
             sequence that is not valid UTF-8. Percent-decoding answers octets, so text carrying \
             an escape for a non-UTF-8 byte has no `string` to decode to",
            error.utf8_error().valid_up_to()
        ))
    })
}

/// `octets` as a `bytes` value.
///
/// Total, and that is the point: percent-decoding is defined over octets and a
/// `bytes` holds every one of them, so a decoder has nothing left to refuse.
/// The module docs' *A decoder answers `bytes`* owns the rule.
fn decoded(octets: Vec<u8>) -> HelperResult {
    Ok(Value::bytes(NvsStr::new(&octets)))
}

/// The route capture `name`'s segment text, percent-decoded, or a throw where
/// its octets are not UTF-8.
///
/// Here rather than in [`crate::router`] because this module owns the decoder:
/// `nvs_runtime::routes`' module doc states that rule from the other side, and
/// a second walk beside [`decode`] is the two-that-agree-today shape the
/// tainted laundering rules exist to prevent. [`Form::Component`] and not
/// [`Form::FormValue`], since a capture is a path segment where `+` is a
/// literal plus.
///
/// # Errors
///
/// A [`Fault::thrown`] naming the capture and the offset of the first byte no
/// `string` can hold. Unlike this module's own two decoders, a capture has no
/// `bytes` to answer with — [`crate::router::capture_value`]'s doc owns why —
/// so the refusal is the answer, and the offset is a position in a path the
/// peer supplied, which is the one fact that makes it actionable.
pub(crate) fn decode_capture(text: &str, name: &str) -> Result<String, Fault> {
    String::from_utf8(decode(text, Form::Component)).map_err(|error| {
        Fault::thrown(format!(
            "the route capture `{name}` percent-decodes to a byte a `string` cannot hold — byte \
             {} begins a sequence that is not valid UTF-8. A capture binds as a `tainted string`, \
             so a path segment escaping an octet outside UTF-8 has no capture to become",
            error.utf8_error().valid_up_to()
        ))
    })
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
    authority
        .port_to_u16()
        .map_err(|_| port_out_of_range(member))
}

/// The refusal both ends of the port bound are drawn with.
///
/// It is written once because it is one rule asked from two directions:
/// [`port_of`] reads a run of digits off text and finds it above `65535`,
/// while [`nvs_core_uri_with`] is handed an `int` option that may also be
/// *negative*, which the digit grammar cannot express at all. Left to
/// recompose, a negative port becomes a `-` inside an authority and comes back
/// as "a byte the URI grammar does not admit", which is a true sentence about
/// the wrong thing — the value is out of range, and the bound is 0-65535 at
/// both of its ends.
fn port_out_of_range(member: &str) -> Fault {
    Fault::thrown(format!(
        "Core\\Uri::{member}(): the authority's port is not a TCP port number. RFC 3986 \
         § 3.2.3 admits any run of digits and defines the component as a port, so a value \
         outside 0-65535 has no `int` this member could answer with"
    ))
}

/// Whether `text` is a URI reference `Core\Uri::parse` would answer for, for
/// a caller that wants the refusal and not the object —
/// `rule:expressions/intrinsic-literals`'s fold,
/// which reads a **literal** URI while checking and reports § 3's diagnostic
/// instead of the throw [`nvs_core_uri_parse`] would have made.
///
/// # Errors
///
/// [`read`]'s sentence or [`port_of`]'s, without the member prefix a throw
/// carries. **Both** steps run, for the reason [`nvs_core_uri_try_parse`]'s
/// docs give at length: folding only the grammar one would make this pass
/// silent about one of the two texts `parse` refuses, and a validator that
/// disagrees with its parser is precisely what that member exists to prevent.
/// Nothing is built, so no [`Value`] is allocated in the compiler.
pub fn validate(text: &str) -> Result<(), String> {
    let reference = read(text, "parse").map_err(stated)?;
    if let Some(authority) = reference.authority() {
        port_of(&authority, "parse").map_err(stated)?;
    }
    Ok(())
}

/// The sentence inside a fault [`read`] or [`port_of`] built, with the
/// `Core\Uri::parse(): ` prefix removed — a diagnostic already names the
/// member it points at, and a throw has to.
fn stated(fault: Fault) -> String {
    let Fault::Thrown(_, message) = fault else {
        // Neither step builds any other variant, and a `Fatal` reaching here
        // would be an engine bug rather than something about this literal.
        return "this text is not a URI reference (RFC 3986 § 4.1)".to_owned();
    };
    message
        .strip_prefix(r"Core\Uri::parse(): ")
        .unwrap_or(&message)
        .to_owned()
}

/// A fresh `Core\Uri` holding `reference`'s text and its seven components.
///
/// # Errors
///
/// [`port_of`]'s. It runs **before** the first allocation on purpose: a
/// [`Value`] is not released by falling out of scope, so a refused port
/// halfway through the slot array would strand every `NvsStr` built before it.
fn built(reference: &UriRef<&str>, member: &str) -> HelperResult {
    let authority = reference.authority();
    let port = match authority {
        Some(authority) => port_of(&authority, member)?,
        None => None,
    };
    let text = |held: Option<&str>| {
        held.map_or_else(Value::null, |held| Value::str(NvsStr::new(held.as_bytes())))
    };
    Ok(crate::instance::build(
        &CLASS,
        [
            Value::str(NvsStr::new(reference.as_str().as_bytes())),
            text(reference.scheme().map(Scheme::as_str)),
            text(authority.and_then(|held| held.userinfo()).map(EStr::as_str)),
            // Present-but-empty where `//` was written with nothing after it,
            // which is the distinction `parse_url`'s array cannot hold.
            text(authority.as_ref().map(Authority::host)),
            port.map_or_else(Value::null, |port| Value::int(i64::from(port))),
            Value::str(NvsStr::new(reference.path().as_str().as_bytes())),
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

/// One **non-nullable** option's text, or `None` where the call left it out.
///
/// The three components `with` cannot remove — `scheme`, `host` and `path` —
/// are declared `string` rather than `?string`, so an omitted one arrives as
/// [`Const::Null`] and no written one can be a `null` to collide with it.
/// `rule:core-classes/uri-removable-components` owns which three those are and
/// why each is a different operation wearing a removal's clothes; the other
/// three go through [`removable`] instead.
///
/// # Errors
///
/// A [`Fault::fatal`] where the slot holds something else. `nvs_types` checked
/// the declared type, so that is a runtime-contract violation.
fn written<'a>(value: &'a Value, option: &str) -> Result<Option<&'a str>, Fault> {
    if matches!(value.tag(), Some(Tag::Null)) {
        return Ok(None);
    }
    // Unreachable from source: every option this reads is `CoreTy::Text` with
    // a `Const::Null` default in `CLASS`'s `with` row above, so the slot holds
    // either that null — taken by the branch above — or a string, and anything
    // else is `E0401: expected 'string', found 'mixed'` at the checker.
    value.as_text().map(Some).ok_or_else(|| {
        Fault::fatal(format!(
            "Core\\Uri::with expected a `string` for its `{option}` option, got tag {}",
            value.tag_byte()
        ))
    })
}

/// One **nullable** option's three states: `None` where the call did not
/// mention it, `Some(None)` where it wrote `null`, and `Some(Some(text))`
/// where it wrote a replacement.
///
/// `rule:core-api/omission-is-not-a-written-null` is the mechanism and this is
/// the first member to spend it. An omitted nullable option materializes
/// [`Const::NeverWritten`], which arrives under `Tag::Unset` — a tag no value
/// the type system can spell ever carries — so *leave this alone* and *clear
/// this* are two arguments rather than one. `""` is not a third spelling of
/// either: it is an empty query, which `?` written with nothing after it
/// produces and which [`nvs_core_uri_query`] already reports as distinct from
/// `null`.
///
/// # Errors
///
/// A [`Fault::fatal`] where the slot holds something else, exactly as
/// [`written`] does and for the same reason.
fn removable<'a>(value: &'a Value, option: &str) -> Result<Option<Option<&'a str>>, Fault> {
    match value.tag() {
        Some(Tag::Unset) => Ok(None),
        Some(Tag::Null) => Ok(Some(None)),
        // Unreachable from source, on [`written`]'s judgement: the option is
        // `?string` in `CLASS`'s `with` row above, so the slot holds the
        // marker, a null, or a string, and anything else is `E0401` at the
        // checker.
        _ => value.as_text().map(|text| Some(Some(text))).ok_or_else(|| {
            Fault::fatal(format!(
                "Core\\Uri::with expected a `?string` for its `{option}` option, got tag {}",
                value.tag_byte()
            ))
        }),
    }
}

/// The seven components [`nvs_core_uri_with`] wrote its text out of — what the
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

/// The receiver in `slots`, rebuilt with query parameter `name` set to
/// `value` — or removed, where that is [`None`].
///
/// [`nvs_core_uri_with_query_parameter`]'s whole body below its argument read,
/// lifted out because [`nvs_core_uri_sign`] answers a `Uri` carrying the
/// reserved `_sig` and has to reach the *same* edit. Two spellings of "set a
/// parameter and read the result back" is how the query a program writes and
/// the query a signature is checked against would come to differ.
///
/// Takes over `value`'s reference, exactly as [`NvsArray::set`] does.
///
/// # Errors
///
/// [`parse_query`]'s, for a name in the receiver's own query whose escapes
/// decode to octets that are not UTF-8; [`build`]'s; and [`unmoved`]'s, which
/// neither caller is expected to reach.
fn with_parameter(
    slots: &[Value; 8],
    member: &str,
    name: &str,
    value: Option<Value>,
) -> HelperResult {
    let parsed = held(slots, QUERY_SLOT, member).and_then(|query| match query {
        Some(query) => parse_query(query, "Core\\Uri", member, Values::Octets),
        // No `?` at all, so there is nothing to read and the answer is a
        // URI with one parameter — or, for a removal, the receiver again.
        None => Ok(NvsArray::new()),
    });
    let mut parameters = match parsed {
        Ok(parameters) => parameters,
        Err(refused) => {
            // The caller handed its reference over before this frame could
            // fail, so the refusing path owes the release the array below
            // would have taken — `parse_query` throws for a name in the
            // receiver's own query that decodes to octets no `string` holds.
            if let Some(held) = value {
                discard(held);
            }
            return Err(refused);
        }
    };
    match value {
        None => {
            parameters.unset(name.as_bytes());
        }
        Some(held) => parameters.set(NvsStr::new(name.as_bytes()), held),
    }

    // `build` borrows the pointer it is handed ([`crate::arr::borrowed`]),
    // so the one reference `into_raw` handed over is still this frame's to
    // release — and it is released on the throwing path too, which is why
    // the `?` is below the reconstruction rather than on the call.
    let empty = parameters.is_empty();
    let root = parameters.into_raw();
    let rebuilt = if empty {
        Ok(None)
    } else {
        build(root, "Core\\Uri", member, &[]).map(Some)
    };
    #[expect(
        unsafe_code,
        reason = "`into_raw` handed this frame the one reference the handle \
                  held, and `build` borrowed the pointer rather than taking \
                  it over"
    )]
    unsafe {
        drop(NvsArray::from_raw(root));
    }
    let query = rebuilt?;

    // The receiver's own port, which is an `int` in its slot and text in a
    // recomposition — `nvs_core_uri_with`'s conversion, without its bag,
    // since this edit replaces one component and reads the other six.
    let port = slots[PORT_SLOT].as_int().map(|port| port.to_string());
    let composed = Composed {
        scheme: held(slots, SCHEME_SLOT, member)?,
        user_info: held(slots, USER_INFO_SLOT, member)?,
        host: held(slots, HOST_SLOT, member)?,
        port: port.as_deref(),
        path: held(slots, PATH_SLOT, member)?.unwrap_or(""),
        query: query.as_deref(),
        fragment: held(slots, FRAGMENT_SLOT, member)?,
    };
    let text = recompose(&composed);
    let reference = read(&text, member)?;
    unmoved(&composed, &reference)?;

    built(&reference, member)
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
/// [`nvs_core_uri_resolve`] gives its own receiver.
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
// Signing — the same normal form, reached a second time rather than invented
// ============================================================================

/// `Core\Uri::sign`, spelled the way [`crate::keyring`]'s refusals name it.
const SIGN: &str = r"Core\Uri::sign";

/// `Core\Uri::verifySignature`, spelled the same way.
const VERIFY_SIGNATURE: &str = r"Core\Uri::verifySignature";

/// The one query parameter a signed URL reserves: the token
/// [`nvs_core_uri_sign`] writes and [`nvs_core_uri_verify_signature`] reads,
/// and the one [`crate::router`]'s signing pair writes into the query it
/// builds.
///
/// Reserved rather than configurable. A name a caller chooses is a name the
/// two sides can disagree about, and a signed URL whose parameter name is part
/// of the caller's vocabulary is one an attacker can rename.
///
/// Shared rather than spelled twice, for the reason
/// [`crate::signature`]'s codec is: a URL signed at one door and read at the
/// other would otherwise turn on two constants agreeing.
pub(crate) const SIG_NAME: &str = "_sig";

/// Whether `written` — a query pair's name, still percent-encoded — names the
/// reserved parameter.
///
/// Decoded first, and the bracket base taken, so neither `%5Fsig` nor
/// `_sig[0]` is a way of smuggling a second signature past the count in
/// [`without_signature`]: the base is the key [`with_parameter`] writes, so
/// anything rooted at `_sig` is a pair `sign` would overwrite.
fn is_signature_name(written: &str) -> bool {
    let decoded = decode(written, Form::FormValue);
    let base = path_of(&decoded).map_or(decoded.as_slice(), |(base, _)| base);
    base == SIG_NAME.as_bytes()
}

/// The tokens `query`'s reserved pairs carry, and the query without them.
///
/// Splitting on `&` rather than reading the parsed map back, because the map
/// cannot count: `_sig=a&_sig=b` parses to one entry — the last write wins,
/// as it does everywhere in this class — and "a URL carrying two of them
/// fails" is a rule about the *text* that arrived
/// (`rule:core-api/signing-is-over-a-payload`).
///
/// **The token is taken undecoded, and needs no decoding**: unpadded URL-safe
/// base64 is unreserved from end to end, so § 6.2.2.2 has already restored any
/// escaped spelling of one in [`equivalent`]'s query, and a served request's
/// own query string — which is what `crate::router`'s `signedRoute` hands over,
/// having had no normalization at all — carries those characters themselves. A
/// token written with an escape in it is one that does not authenticate, which
/// is the caller's one refusal rather than a second reading of the text.
pub(crate) fn without_signature(query: &str) -> (Vec<&str>, String) {
    let mut tokens = Vec::new();
    let mut rest = Vec::new();
    for pair in query.split('&') {
        let (name, value) = pair.split_once('=').unwrap_or((pair, ""));
        if is_signature_name(name) {
            tokens.push(value);
        } else {
            rest.push(pair);
        }
    }
    (tokens, rest.join("&"))
}

/// What both signing members take a signature over: [`equivalent`]'s output,
/// with the query as its `parameters`.
///
/// **Not a second normalization.** Every component here is the one
/// [`nvs_core_uri_compare_to`] compares — scheme and host folded, escapes'
/// digits upper-cased, an escaped unreserved character restored, dot segments
/// gone — reached through the same function rather than reproduced, which is
/// the whole of `rule:core-api/signing-is-over-a-payload`: a canonical form
/// used by nothing but the signature is a form no other test constrains.
///
/// Two departures from `compareTo`, both deliberate. The **fragment is absent**
/// — RFC 3986 § 3.5 never sends one to the server, so signing one would mint
/// links that cannot verify. And the query is its **parameters** rather than
/// its text, so writing the same pairs in another order is the same signature,
/// which is the one thing a query string genuinely does not carry; a
/// consequence is that a query with no pairs and no query at all sign alike.
fn payload_of(form: &Equivalent, parameters: NvsArray) -> Value {
    let text = |held: Option<&String>| {
        held.map_or_else(Value::null, |held| Value::str(NvsStr::new(held.as_bytes())))
    };
    let mut payload = NvsArray::new();
    payload.set(NvsStr::new(b"scheme"), text(form.scheme.as_ref()));
    payload.set(NvsStr::new(b"userInfo"), text(form.user_info.as_ref()));
    payload.set(NvsStr::new(b"host"), text(form.host.as_ref()));
    payload.set(
        NvsStr::new(b"port"),
        form.port.map_or_else(Value::null, Value::int),
    );
    payload.set(
        NvsStr::new(b"path"),
        Value::str(NvsStr::new(form.path.as_bytes())),
    );
    payload.set(NvsStr::new(b"query"), Value::array(parameters));
    Value::array(payload)
}

/// Releases a reference this module owns and is not handing to anyone — the
/// payload [`payload_of`] built, or the value a refusing [`with_parameter`]
/// was handed and never stored.
fn discard(value: Value) {
    #[expect(
        unsafe_code,
        reason = "both call sites own exactly the reference they pass"
    )]
    unsafe {
        value.release();
    }
}

/// The one sentence [`nvs_core_uri_verify_signature`] produces for every way
/// of not being a signature this ring made.
///
/// One function so the call sites cannot drift into several sentences, which
/// is the whole of what makes them indistinguishable
/// (`rule:core-api/one-refusal-except-expiry`). A missing `_sig` and a forged
/// one are the same message on purpose: the first is what a caller who strips
/// the parameter produces, and telling it apart is telling an attacker that
/// stripping is the cheaper attack.
fn unsigned() -> Fault {
    Fault::thrown(
        "Core\\Uri::verifySignature(): this URL is not one $keys signed. Every way of not being \
         one — a parameter added, removed or edited, an altered token, a token minted for another \
         door or under a key that has been retired, no `_sig` at all, and two of them — is this \
         one sentence, so a forgery says nothing about which half of it failed."
            .to_owned(),
    )
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
    /// `a[]` — whatever key [`NvsArray::append`] assigns next, which is the
    /// counter PHP calls `nNextFreeElement` and Novis's arrays already keep.
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
/// `a[][x]=1&a[][y]=2` is two arrays rather than one, and answers `None` where
/// `parent`'s next integer key is already taken — a key of `i64::MAX` written
/// earlier in the same query leaves no next one.
///
/// The handle is **borrowed and never dropped**: `parent` owns the only
/// reference to the array it hands back, so writing through this handle finds
/// a refcount of one and mutates in place rather than separating. That is what
/// makes the whole parse O(input) — retaining a second reference would make
/// every descent copy the subtree it descends into.
fn branch(parent: &mut NvsArray, key: Option<&[u8]>) -> Option<ManuallyDrop<NvsArray>> {
    if let Some(key) = key
        && let Some(existing) = parent.get(key).and_then(Value::array_ptr)
    {
        return Some(crate::arr::borrowed(existing));
    }
    let fresh = Value::array(NvsArray::new());
    let address = fresh.array_ptr().expect("just built from an array");
    match key {
        Some(key) => parent.set(NvsStr::new(key), fresh),
        None => {
            if let Err(fresh) = parent.try_append(fresh) {
                discard(fresh);
                return None;
            }
        }
    }
    let child = crate::arr::borrowed(address);
    debug_assert_eq!(
        child.refcount(),
        1,
        "the array just handed to `parent` is owned by it alone"
    );
    Some(child)
}

/// Writes `value` at `base` + `path` inside `out`, building the arrays the
/// path passes through.
///
/// Iterative rather than recursive on purpose: the path's depth is the
/// caller's text, so a recursive descent would let a query string choose this
/// process's stack depth. The arrays it builds are freed through
/// `nvs_runtime::release`'s worklist, which is iterative for the same reason,
/// so nesting is bounded by the input's length and by nothing else — PHP's
/// `max_input_nesting_level` has no equivalent here because it does not need
/// one.
///
/// A pair whose `[]` finds no free integer key is dropped whole, as `parse_str`
/// drops it: the arrays are built from the caller's text, so an occupied next
/// key is input, never a broken invariant.
fn insert(out: &mut NvsArray, base: &[u8], path: &[Index<'_>], value: Value) {
    let Some((last, descents)) = path.split_last() else {
        out.set(NvsStr::new(base), value);
        return;
    };
    let mut current = branch(out, Some(base)).expect("a named key is never refused");
    for index in descents {
        let Some(next) = branch(&mut current, index.key()) else {
            discard(value);
            return;
        };
        current = next;
    }
    match last.key() {
        Some(key) => current.set(NvsStr::new(key), value),
        None => {
            if let Err(value) = current.try_append(value) {
                discard(value);
            }
        }
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
    array: ManuallyDrop<NvsArray>,
    /// The next slot to look at, which [`NvsArray::next_slot`] advances.
    slot: usize,
    /// How many bytes of the running name are this array's own path. Each of
    /// its entries writes its own key after exactly that much.
    prefix: usize,
}

/// One value's text for the right-hand side of a pair.
///
/// `rule:types/conversion`'s conversion rows through `nvs_runtime::value_to_string`, with
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
fn scalar_text(value: Value, owner: &str, member: &str) -> Result<Vec<u8>, Fault> {
    if let Some(set) = value.as_bool() {
        return Ok(if set { b"1".to_vec() } else { b"0".to_vec() });
    }
    // A `bytes` is what `parseQuery` answers for a value now, and this member
    // is that one's inverse — so its octets are written straight into the
    // encoder, which turns every one of them into an ASCII escape. There is no
    // `as string` in the way on purpose: a value that survived the wire once
    // has to survive being written back, and `value_to_string` refuses a
    // `bytes` exactly as `rule:types/conversion` says it should.
    if let Some(bytes) = value.as_bytes() {
        return Ok(bytes.to_vec());
    }
    let text = nvs_runtime::value_to_string(value).map_err(|_| {
        Fault::thrown(format!(
            "{owner}::{member}(): a parameter's value is neither a scalar nor a nested array, \
             so there is no text a query string could write it as"
        ))
    })?;
    // A post-condition of the call above rather than a boundary, and so
    // unreachable from source with no diagnostic to name: every `Ok` arm of
    // `value_to_string` builds a `Value::str` — including `rule:security/capture-answers-the-carrier`'s
    // carrier arm, which hands back the carrier's own checked text slot — so
    // this is a `Tag::Str` or it is the `Err` the `?` above already took.
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
/// **`Core\Router::url` is the second caller**, which is why `owner` names the
/// class rather than being spelled into the message and why `omit` exists at
/// all: `rule:routing/a-leftover-link-key-is-a-query-string` makes a `$params` key that named no capture a query
/// parameter, so the link's query string is exactly this member run over the
/// same array with the captures left out. Written as one pass rather than as a
/// filtered copy of the array, because a copy would be an allocation and a
/// refcount edge per link built. `omit` is matched at the **top level only** —
/// a capture is a whole `$params` key, and `a[b]` is a nested value of the key
/// `a` rather than a name of its own.
///
/// # Errors
///
/// [`scalar_text`]'s, for a value with no text form, and
/// [`crate::regex::grow`]'s `FATAL` for a text past the request's memory limit.
pub(crate) fn build(
    root: *mut nvs_runtime::ArrayHeader,
    owner: &str,
    member: &str,
    omit: &[&str],
) -> Result<String, Fault> {
    let label = format!("{owner}::{member}");
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

        if prefix == 0 && omit.iter().any(|name| name.as_bytes() == key.as_bytes()) {
            continue;
        }

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
        let pair_name = encode(&name, Form::FormValue);
        let pair_value = encode(&scalar_text(value, owner, member)?, Form::FormValue);
        // Every pair repeats its whole path, so the text can be many times the
        // size of the array it came from: the budget is asked before it grows.
        crate::regex::grow(
            &mut out,
            pair_name
                .len()
                .saturating_add(pair_value.len())
                .saturating_add(2),
            &label,
        )?;
        if !out.is_empty() {
            out.push('&');
        }
        out.push_str(&pair_name);
        out.push('=');
        out.push_str(&pair_value);
    }
    Ok(out)
}

// ============================================================================
// The members
// ============================================================================

nvs_runtime::nvs_helper! {
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
    /// that is not there. `rule:core-api/shape-rules` R5's reading of `?T` is the same one.
    fn nvs_core_uri_parse(_ctx, args: [1]) {
        let text = text_of(args, "parse")?;

        built(&read(text, "parse")?, "parse")
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Uri::tryParse(string $uri): ?Uri` —
    /// `rule:expressions/try-parse`: [`nvs_core_uri_parse`] exactly, with `null` where it throws.
    ///
    /// It is `parse` and not a second reader, which is the whole reason R17
    /// allows the question "is this text a URI" one spelling and this is it:
    /// a validator written as separate code from the parser is how
    /// `filter_var(FILTER_VALIDATE_URL)` came to accept user-info that
    /// `parse_url` read differently (CVE-2024-5458). `Core\Uri` declares no
    /// `isValid` for that reason, and the narrower question that one asked —
    /// "is this an **absolute** URI" — is `tryParse($s)?->scheme() != null`.
    ///
    /// The name is the one `try…` `rule:core-api/shape-rules`
    /// R5 admits, because R4's "failure throws, absence is `?T`" leaves a
    /// class no other non-throwing spelling: `as ?T` never targets one.
    ///
    /// Only a *thrown* fault becomes `null`. A `Fault::Fatal` — a wrong
    /// argument tag, an engine invariant — is not a failed parse and
    /// propagates unchanged.
    ///
    /// **Both** of `parse`'s throwing steps are folded, not just the grammar
    /// one: [`read`] refuses text RFC 3986 § 4.1 does not admit, and [`built`]
    /// refuses an authority whose port is outside `0-65535` ([`port_of`] —
    /// § 3.2.3's grammar admits the digits and its prose does not admit the
    /// number). Folding only the first would make `tryParse` throw for one of
    /// the two texts `parse` throws for, which is exactly the parser/validator
    /// divergence the paragraph above says this member exists to prevent.
    fn nvs_core_uri_try_parse(_ctx, args: [1]) {
        let text = text_of(args, "tryParse")?;

        match read(text, "tryParse").and_then(|reference| built(&reference, "tryParse")) {
            Ok(value) => Ok(value),
            Err(Fault::Thrown(..)) => Ok(Value::null()),
            Err(other) => Err(other),
        }
    }
}

nvs_runtime::nvs_helper! {
    /// `$uri->scheme(): ?string` — `null` for a relative reference, and never
    /// case-folded: RFC 3986 § 3.1 makes a scheme case-insensitive to
    /// *compare*, which is a different thing from rewriting what was sent.
    fn nvs_core_uri_scheme(_ctx, args: [1]) {
        component(args, "scheme", SCHEME_SLOT)
    }
}

nvs_runtime::nvs_helper! {
    /// `$uri->userInfo(): ?string` — the whole `user:password` subcomponent as
    /// written, or `null` where no `@` was.
    ///
    /// One reader rather than `parse_url`'s two keys, because RFC 3986
    /// § 3.2.1 deprecates the `user:password` form outright and a member that
    /// split it would be a member that suggested writing one.
    fn nvs_core_uri_user_info(_ctx, args: [1]) {
        component(args, "userInfo", USER_INFO_SLOT)
    }
}

nvs_runtime::nvs_helper! {
    /// `$uri->host(): ?string` — `null` where no authority was written, `""`
    /// where an empty one was (`file:///tmp`), and an IPv6 literal still
    /// inside its brackets, since that is what the host component is.
    fn nvs_core_uri_host(_ctx, args: [1]) {
        component(args, "host", HOST_SLOT)
    }
}

nvs_runtime::nvs_helper! {
    /// `$uri->port(): ?int` — `null` where none was written *and* where an
    /// empty one was, which is [`port_of`]'s one departure from giving back
    /// exactly what came in.
    fn nvs_core_uri_port(_ctx, args: [1]) {
        component(args, "port", PORT_SLOT)
    }
}

nvs_runtime::nvs_helper! {
    /// `$uri->path(): string` — never `null`, because RFC 3986 § 3.3's path is
    /// not optional: a URI with nothing between its authority and its query
    /// has the empty path, and `""` is that path rather than the absence of
    /// one.
    fn nvs_core_uri_path(_ctx, args: [1]) {
        component(args, "path", PATH_SLOT)
    }
}

nvs_runtime::nvs_helper! {
    /// `$uri->query(): ?string` — the raw query, with no `?`, still encoded.
    /// [`nvs_core_uri_parse_query`] is what turns it into an array.
    ///
    /// `null` and `""` are different answers here: `?` written with nothing
    /// after it is an empty query, and no `?` at all is no query.
    fn nvs_core_uri_query(_ctx, args: [1]) {
        component(args, "query", QUERY_SLOT)
    }
}

nvs_runtime::nvs_helper! {
    /// `$uri->fragment(): ?string` — the raw fragment, with no `#`, still
    /// encoded. `null` and `""` differ for [`nvs_core_uri_query`]'s reason.
    fn nvs_core_uri_fragment(_ctx, args: [1]) {
        component(args, "fragment", FRAGMENT_SLOT)
    }
}

nvs_runtime::nvs_helper! {
    /// `$uri->toString(): string` — the reference this `Uri` was parsed from,
    /// byte for byte.
    ///
    /// Not a recomposition of the components: `parse` normalizes nothing, so
    /// there is nothing a round trip could lose, and holding the text is what
    /// buys that guarantee for the price the module docs' *What it spends*
    /// states. Named `toString` for [`crate::uuid`]'s reason, and that name is
    /// load-bearing now: `rule:classes/stringable`'s rendering *is* this member, so
    /// `echo $uri` reaches it too — through the native call the checker
    /// resolves where the operand's type names this class, and through
    /// [`crate::instance`]'s descriptor renderer where it names none.
    fn nvs_core_uri_to_string(_ctx, args: [1]) {
        component(args, "toString", TEXT_SLOT)
    }
}

nvs_runtime::nvs_helper! {
    /// `$uri->with({scheme?, host?, port?, path?, query?, fragment?}): Uri` —
    /// replacing reassembly by hand.
    ///
    /// A fresh `Uri`, not a mutated one: every `Core` class except § 9's
    /// collections is built once and read, and a URI that could change under a
    /// caller who had already validated it is the shape this exists to avoid.
    ///
    /// **Three of the six components can be removed, and `null` is how** —
    /// `port`, `query` and `fragment` are `?T`, so `{fragment: null}` clears
    /// the fragment where `{}` leaves it alone ([`removable`]). The other
    /// three are `string`: `rule:core-classes/uri-removable-components` owns
    /// why each of those is a different operation wearing a removal's clothes,
    /// and [`written`] is what reads them. `userInfo` is not on the bag at all
    /// and is carried over unchanged, so `with` can neither add nor drop a
    /// credential.
    ///
    /// A removal is no shortcut around anything: the result goes back through
    /// [`read`] and then [`unmoved`] exactly as a replacement does, so a
    /// removal that would let a remaining component move into another's
    /// position is the same throw a bad replacement already is.
    ///
    /// An empty port on the receiver has already become "no port" by the time
    /// it reaches a slot ([`port_of`]), so a `with` that does not mention the
    /// port drops the `:` that was written — RFC 3986 § 3.2.3's own
    /// instruction, and the one place a round trip through `with` is not the
    /// identity.
    fn nvs_core_uri_with(_ctx, args: [7]) {
        let receiver = crate::instance::receiver(args[0], &CLASS, "with")?;
        let slots: [Value; 8] =
            std::array::from_fn(|index| crate::instance::slot(receiver, index));
        // The one option declared `?int` rather than `?string`, so it is read
        // here rather than through `removable` — and narrowed to a port here
        // too, at both ends of the bound. A value above `65535` would still
        // recompose and be refused by `port_of` inside `built`, but a
        // *negative* one recomposes to a `-` the authority grammar does not
        // admit at all, so `read` would refuse it a step earlier with a
        // message about a byte. [`port_out_of_range`] is the one rule both
        // ends draw.
        let port = match args[3].tag() {
            // Not mentioned: carry the receiver's own port over.
            Some(Tag::Unset) => slots[PORT_SLOT].as_int(),
            // `{port: null}` — removed, which is the whole of what a written
            // `null` means here (`rule:core-api/a-written-null-removes`).
            Some(Tag::Null) => None,
            // Unreachable from source, on `removable`'s judgement: `port` is
            // `?int`, the two tags above are its other two states, and
            // anything else is `E0401: expected '?int', found 'mixed'` at the
            // checker. The range, which a program *can* get wrong, is
            // `port_out_of_range` below.
            _ => Some(args[3].as_int().ok_or_else(|| {
                Fault::fatal(format!(
                    "Core\\Uri::with expected an `?int` for its `port` option, got tag {}",
                    args[3].tag_byte()
                ))
            })?),
        };
        let port = match port {
            Some(port) => Some(u16::try_from(port).map_err(|_| port_out_of_range("with"))?),
            None => None,
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
            // The two removable text components: an omitted option leaves the
            // receiver's own alone, and a written `null` clears it — which is
            // `removable`'s outer `Option` and `held`'s, in that order, rather
            // than an `or` that cannot tell the two apart.
            query: match removable(&args[5], "query")? {
                Some(replacement) => replacement,
                None => held(&slots, QUERY_SLOT, "with")?,
            },
            fragment: match removable(&args[6], "fragment")? {
                Some(replacement) => replacement,
                None => held(&slots, FRAGMENT_SLOT, "with")?,
            },
        };
        let text = recompose(&composed);
        let reference = read(&text, "with")?;
        unmoved(&composed, &reference)?;

        built(&reference, "with")
    }
}

nvs_runtime::nvs_helper! {
    /// `$uri->queryParameter(string $name): mixed` — one query parameter by
    /// name, without the call site parsing the query string itself.
    ///
    /// `rule:core-classes/uri-removable-components`'s second level, read half.
    /// A URI's components are fixed and few, so [`nvs_core_uri_with`] can name
    /// each of them as a bag key; its parameters are dynamic and many, and a
    /// name chosen at run time is not a key a registry row can declare.
    /// Reaching one without this member means `parseQuery` and an array read
    /// spelled out at every call site, which is where this area's real bugs
    /// come from.
    ///
    /// A composition of the receiver's `query` slot and [`parse_query`] rather
    /// than a walk of its own, so `a[b][]=1` means here exactly what it means
    /// at `Core\Uri::parseQuery` — the same code decides both.
    ///
    /// **The name is a top-level name and the brackets belong to the value.**
    /// Over `a[b]=c`, `queryParameter("a")` answers the nested
    /// `array<mixed>` and `queryParameter("a[b]")` answers `null`, because the
    /// convention places nothing under that name. That is what makes the
    /// writer beside this one able to take an array value and get the bracket
    /// spelling for free.
    ///
    /// `null` answers both a name that is not there and a URI with no query at
    /// all. Those are one question to a caller reading a parameter, and a
    /// program that needs them apart asks [`nvs_core_uri_query`].
    ///
    /// # Errors
    ///
    /// [`parse_query`]'s throw, for a **name** in the receiver's own query
    /// whose escapes decode to octets that are not UTF-8.
    fn nvs_core_uri_query_parameter(_ctx, args: [2]) {
        let receiver = crate::instance::receiver(args[0], &CLASS, "queryParameter")?;
        let slots: [Value; 8] =
            std::array::from_fn(|index| crate::instance::slot(receiver, index));
        // Unreachable from source: parameter 0 is `CoreTy::Text` in `CLASS`'s
        // `queryParameter` row above, so a non-string name is `E0401:
        // expected 'string', found 'mixed'` at the checker. This is
        // `nvs_core_uri_resolve`'s judgement below, for its reason.
        let name = args[1].as_text().ok_or_else(|| {
            Fault::fatal(format!(
                "Core\\Uri::queryParameter expected {:?}, got tag {}",
                Tag::Str,
                args[1].tag_byte()
            ))
        })?;
        let answer = match held(&slots, QUERY_SLOT, "queryParameter")? {
            Some(query) => {
                let parsed = parse_query(query, "Core\\Uri","queryParameter", Values::Octets)?;
                let found = parsed.get(name.as_bytes()).unwrap_or_else(Value::null);
                #[expect(
                    unsafe_code,
                    reason = "`get` borrows from the array this frame built and \
                              is about to drop, so the caller needs a reference \
                              of its own; a `null` carries no payload and the \
                              retain is the no-op `Value::retain` makes of one"
                )]
                unsafe {
                    found.retain();
                }
                found
            }
            // No `?` at all, so there is no pair to find and nothing to parse.
            None => Value::null(),
        };

        Ok(answer)
    }
}

nvs_runtime::nvs_helper! {
    /// `$uri->withQueryParameter(string $name, mixed $value): Uri` — one query
    /// parameter set, replaced or removed, with every other pair carried over.
    ///
    /// `rule:core-classes/uri-removable-components`'s second level, write
    /// half, and [`nvs_core_uri_query_parameter`]'s twin. It adds no mechanism
    /// at all: [`parse_query`] reads the receiver's query, the edit happens on
    /// that array, and [`build`] writes it back, so there is no second
    /// canonicalization of a query string here to drift from the one
    /// `Core\Uri::buildQuery` already is.
    ///
    /// **A written `null` removes the parameter**, which is the whole of what
    /// a `null` means anywhere a `Core` member admits one
    /// (`rule:core-api/a-written-null-removes`). Removing the last one leaves
    /// **no query at all** rather than a bare `?`: a `?` with nothing after it
    /// is an empty query and a different thing from no query
    /// ([`nvs_core_uri_query`]), and a caller who asked to drop the last
    /// parameter asked for the second.
    ///
    /// **The bracket convention is free, and it comes from the value.**
    /// `withQueryParameter("a", ["b" => "c"])` writes `a%5Bb%5D=c` and
    /// [`nvs_core_uri_query_parameter`] reads that back as the same nested
    /// array, so the pair round-trips without either member spelling a
    /// bracket itself. A name that spells one is placed as a **top-level**
    /// key and then written by [`build`], which escapes it into those very
    /// same bytes: `withQueryParameter("a[b]", "c")` and the call above are
    /// one query string, and it reads back nested. That is `buildQuery`'s own
    /// divergence — a query string cannot tell those two keys apart — carried
    /// in rather than a second one this member invented.
    ///
    /// Every pair that is carried over is rewritten in [`build`]'s spelling
    /// rather than the one it was written in, because the edit happens on the
    /// parsed array and the whole query string is recomposed. A query string
    /// has more than one spelling for the same parameters, so this is the same
    /// non-identity `buildQuery(parseQuery($q))` already has.
    ///
    /// The result goes back through [`read`] and [`unmoved`] exactly as
    /// [`nvs_core_uri_with`] does — the query is form-encoded and so cannot
    /// carry a delimiter out of its own component, and checking anyway is what
    /// keeps one recompose-and-reread path rather than two.
    ///
    /// # Errors
    ///
    /// [`parse_query`]'s throw, for a **name** in the receiver's own query
    /// whose escapes decode to octets that are not UTF-8; [`scalar_text`]'s,
    /// for a `value` with no text form; and [`unmoved`]'s, which nothing here
    /// is expected to reach.
    fn nvs_core_uri_with_query_parameter(_ctx, args: [3]) {
        const MEMBER: &str = "withQueryParameter";

        let receiver = crate::instance::receiver(args[0], &CLASS, MEMBER)?;
        let slots: [Value; 8] =
            std::array::from_fn(|index| crate::instance::slot(receiver, index));
        // Unreachable from source: parameter 0 is `CoreTy::Text` in `CLASS`'s
        // `withQueryParameter` row above, so a non-string name is `E0401:
        // expected 'string', found 'mixed'` at the checker.
        let name = args[1].as_text().ok_or_else(|| {
            Fault::fatal(format!(
                "Core\\Uri::withQueryParameter expected {:?}, got tag {}",
                Tag::Str,
                args[1].tag_byte()
            ))
        })?;

        let written = match args[2].tag() {
            Some(Tag::Null) => None,
            _ => {
                #[expect(
                    unsafe_code,
                    reason = "`with_parameter` takes over a reference, as \
                              `NvsArray::set` does, and a helper only borrows \
                              its arguments"
                )]
                unsafe {
                    args[2].retain();
                }
                Some(args[2])
            }
        };
        with_parameter(&slots, MEMBER, name, written)
    }
}

nvs_runtime::nvs_helper! {
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
    fn nvs_core_uri_resolve(_ctx, args: [2]) {
        let receiver = crate::instance::receiver(args[0], &CLASS, "resolve")?;
        let slots: [Value; 8] =
            std::array::from_fn(|index| crate::instance::slot(receiver, index));
        // Unreachable from source: `Core\Uri` declares no `constructor` — the
        // spelling is `E0405: Core\Uri has no member named constructor` — so
        // every instance a program holds came from `built`, whose slot 0 is an
        // unconditional `Value::str` of the whole reference. Slot 0 is the one
        // of the eight that is never null, which is why this is the only
        // `held` call site that unwraps.
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
        // Unreachable from source: parameter 0 is `CoreTy::Str` in `CLASS`'s
        // `resolve` row above, so a non-string reference is `E0401: expected
        // 'string', found 'mixed'` at the checker. What a program can still
        // write is a *string* that is no reference, and that is `read`'s throw
        // on the next line.
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

        // Unreachable from source, and a post-condition rather than a
        // boundary: the text handed to `parse` here is what `fluent-uri`'s own
        // `resolve_against` just produced out of two references it had already
        // parsed, so a failure would be that crate disagreeing with itself
        // rather than anything a call site wrote. `with` earns its
        // recomposition check because it concatenates components a caller
        // chose; this one does not concatenate anything.
        built(&UriRef::parse(resolved.as_str()).map_err(|_| {
            Fault::fatal("Core\\Uri::resolve produced text `fluent-uri` will not read back")
        })?, "resolve")
    }
}

nvs_runtime::nvs_helper! {
    /// `$uri->compareTo(Uri $other): int` — `Comparable`'s member
    /// (`rule:classes/comparable`), over
    /// the two references' RFC 3986 § 6.2.2 normal forms.
    ///
    /// This is the member that answers "are these the same URI", because
    /// `rule:expressions/object-identity-equality` keeps `==` on two objects meaning *the same object* and names
    /// `compareTo($other) == 0` as the spelling for the other question. What
    /// normalizing does and where it stops is the module docs' own section;
    /// nothing here rewrites the receiver, so `$uri->toString()` still answers
    /// with the text that was parsed.
    ///
    /// **Cost:** one `String` per non-empty component of each side, freed
    /// before the member returns. Bounded by the two references' own lengths,
    /// which is why the normal forms are computed per call rather than cached
    /// in an eighth slot every `Uri` would pay for and most would never read.
    fn nvs_core_uri_compare_to(_ctx, args: [2]) {
        let left = equivalent(args, 0, "compareTo")?;
        let right = equivalent(args, 1, "compareTo")?;
        Ok(Value::int(match left.cmp(&right) {
            std::cmp::Ordering::Less => -1,
            std::cmp::Ordering::Equal => 0,
            std::cmp::Ordering::Greater => 1,
        }))
    }
}

nvs_runtime::nvs_helper! {
    /// `$uri->sign({keys: array<secret bytes>, until: ?Core\Time\Instant}): Uri`
    /// — the door onto `rule:core-api/signing-is-over-a-payload` where someone
    /// holding a URL will look, replacing the `hash_hmac` over an assembled
    /// query string that every framework grows its own slightly different copy
    /// of.
    ///
    /// **It signs [`equivalent`]'s output**, which is [`nvs_core_uri_compare_to`]'s
    /// — one function reached twice, not a second normalization. That is the
    /// whole reason these two members sit on this class rather than beside
    /// [`crate::crypto`]: every signed-URL bug in this space comes from a
    /// canonical form used by nothing but the signature, so nothing else
    /// exercises it and no other test constrains it. [`payload_of`] is where
    /// the two departures from `compareTo` are written down.
    ///
    /// The answer is the receiver with [`SIG_NAME`] set, through the same
    /// [`with_parameter`] a program's own `withQueryParameter` reaches, so a
    /// receiver already carrying one has it **replaced** rather than nested —
    /// the token is stripped before the payload is built, so signing twice
    /// signs the same URL twice.
    ///
    /// **Cost:** one document the size of the URL's components, one HMAC, and
    /// the recomposition [`with_parameter`] already pays. All of it inside the
    /// call (`rule:programs/memory-priority`), and a program that signs
    /// nothing pays none of it. What it spends on the *link* is the token
    /// [`crate::signature::mint`] writes, which carries the signed document as
    /// well as the tag: about `4/3 × (URL + 40)` characters, so a signed URL
    /// runs a little over twice the length of the URL it signs. It buys one
    /// wire format for all three doors, rather than a second, shorter one for
    /// the door whose payload happens to be derivable — and a token nothing
    /// outside [`crate::signature`] assembles.
    fn nvs_core_uri_sign(_ctx, args: [3]) {
        const MEMBER: &str = "sign";

        let ring = crate::keyring::borrow(args, 1, SIGN)?;
        let until = crate::signature::until_of(args, 2, MEMBER)?;
        let form = equivalent(args, 0, MEMBER)?;
        let (_, rest) = without_signature(form.query.as_deref().unwrap_or(""));
        let payload = payload_of(&form, parse_query(&rest, "Core\\Uri",MEMBER, Values::Octets)?);

        // `mint` borrows the payload, so this frame still owns the one
        // reference `payload_of` built — and owns it on the refusing path too,
        // which is why the `?` is below the release rather than on the call.
        let minted = crate::signature::mint(Domain::Uri, until, &payload, &ring, SIGN, "$uri");
        discard(payload);
        let token = minted?;

        let receiver = crate::instance::receiver(args[0], &CLASS, MEMBER)?;
        let slots: [Value; 8] =
            std::array::from_fn(|index| crate::instance::slot(receiver, index));
        with_parameter(
            &slots,
            MEMBER,
            SIG_NAME,
            Some(Value::str(NvsStr::new(token.as_bytes()))),
        )
    }
}

nvs_runtime::nvs_helper! {
    /// `$uri->verifySignature(array<secret bytes> $keys): void` — the read
    /// half, and **the reason it answers nothing**: a signed URL carries no
    /// claims to hand back, since the claim is the URL the caller is already
    /// holding, and a `bool` is a value a caller can drop on the floor.
    ///
    /// The order is load-bearing. The token is lifted out of the query and the
    /// payload rebuilt from what is left; [`crate::signature::confirm`] checks
    /// the tag, the door and *this* payload before a field is trusted — a
    /// token that authenticates over some other URL is the whole of the attack
    /// this member is written against — and the expiry is judged last, which
    /// is what makes it safe to give it a sentence of its own
    /// (`rule:core-api/one-refusal-except-expiry`).
    ///
    /// # Errors
    ///
    /// [`unsigned`]'s one sentence, [`crate::signature::judge`]'s expiry, and
    /// [`parse_query`]'s for a parameter name in the receiver's own query
    /// whose escapes decode to octets that are not UTF-8 — which says
    /// something about the URL the caller is holding and nothing about the
    /// token, so it is not a second refusal that sentence has to cover.
    fn nvs_core_uri_verify_signature(ctx, args: [2]) {
        const MEMBER: &str = "verifySignature";

        let ring = crate::keyring::borrow(args, 1, VERIFY_SIGNATURE)?;
        let form = equivalent(args, 0, MEMBER)?;
        let (tokens, rest) = without_signature(form.query.as_deref().unwrap_or(""));
        // Neither none nor two. A URL with two `_sig` parameters is refused
        // rather than checked under either, because picking one is picking
        // which of two answers an attacker gets to try.
        if tokens.len() != 1 {
            return Err(unsigned());
        }
        let payload = payload_of(&form, parse_query(&rest, "Core\\Uri",MEMBER, Values::Octets)?);

        let confirmed = crate::signature::confirm(
            tokens[0],
            Domain::Uri,
            &payload,
            &ring,
            VERIFY_SIGNATURE,
            "$uri",
        );
        discard(payload);
        let Confirmed::Signed(until) = confirmed? else {
            return Err(unsigned());
        };
        crate::signature::judge(ctx, until, VERIFY_SIGNATURE)?;

        Ok(Value::null())
    }
}

nvs_runtime::nvs_helper! {
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
    ///
    /// **It launders** ([`Qual::Launder`]), and the sink it launders for is
    /// the URI grammar itself: a path segment, a query name or value, a
    /// fragment. A `tainted` argument comes back plain because after this
    /// member no byte of it can be read as a delimiter — which is `rule:security/unclassified-parameter-refuses-tainted`
    /// 's whole condition for claiming the mark.
    fn nvs_core_uri_encode_component(_ctx, args: [1]) {
        let text = text_of(args, "encodeComponent")?;

        produced(&encode(text.as_bytes(), Form::Component))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Uri::decodeComponent(string $s): bytes` — replacing PHP's
    /// `rawurldecode`.
    ///
    /// The exact inverse of [`nvs_core_uri_encode_component`] for text that
    /// member produced. A `+` is a literal `+`, which is the whole reason this
    /// is a different member from `decodeFormValue` rather than an option on
    /// one: reading a form value with this decoder turns every space the user
    /// typed into a `+`.
    ///
    /// A malformed escape decodes to itself, and every octet has an answer:
    /// the module docs' *A decoder answers `bytes`* owns both.
    fn nvs_core_uri_decode_component(_ctx, args: [1]) {
        let text = text_of(args, "decodeComponent")?;

        decoded(decode(text, Form::Component))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Uri::encodeFormValue(string $s): string` — replacing PHP's
    /// `urlencode`.
    ///
    /// **For a value in an `application/x-www-form-urlencoded` payload**: a
    /// query-string pair or a POST body. A space becomes `+` and `~` becomes
    /// `%7E`, which are the two bytes this member's set differs from
    /// [`nvs_core_uri_encode_component`]'s on; the module docs own why the
    /// difference is kept rather than collapsed.
    ///
    /// A program building a whole query string reaches for `Uri::buildQuery`
    /// instead, which writes the `=` and the `&` as well. This member
    /// is one side of one pair.
    ///
    /// **It launders** ([`Qual::Launder`]), and the sink it launders for is an
    /// `application/x-www-form-urlencoded` body. Both bytes that structure one
    /// are escaped — the `&` between pairs and the `=` inside a pair — so an
    /// encoded value cannot open a pair of its own.
    fn nvs_core_uri_encode_form_value(_ctx, args: [1]) {
        let text = text_of(args, "encodeFormValue")?;

        produced(&encode(text.as_bytes(), Form::FormValue))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Uri::decodeFormValue(string $s): bytes` — replacing PHP's
    /// `urldecode`.
    ///
    /// The inverse of [`nvs_core_uri_encode_form_value`]: `+` is a space, and
    /// `%2B` is the `+` the user actually typed. Both spellings of a space
    /// therefore read, since `%20` is still an escape — which is what makes
    /// this the right decoder for a query string written by something that
    /// followed RFC 3986 rather than the form encoding.
    ///
    /// A malformed escape decodes to itself, and every octet has an answer:
    /// the module docs' *A decoder answers `bytes`* owns both.
    fn nvs_core_uri_decode_form_value(_ctx, args: [1]) {
        let text = text_of(args, "decodeFormValue")?;

        decoded(decode(text, Form::FormValue))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Uri::parseQuery(string $query): array<mixed>` — replacing PHP's
    /// `parse_str`, which it **returns** rather than populating variables
    /// with.
    ///
    /// Pairs are separated by `&`, each pair by its first `=`, and both halves
    /// are read with [`nvs_core_uri_decode_form_value`]'s decoder — so a `+`
    /// is a space on both sides of the `=`, and a value answers that decoder's
    /// octets. A pair with no `=` at all has empty `bytes` for its value, and
    /// one whose name decodes to nothing is dropped, both as PHP does.
    ///
    /// Names carry the bracket convention in full: `a[]=1&a[]=2` builds a
    /// list, `a[b]=c` builds a map, and the two nest to any depth. A repeated
    /// name without brackets keeps the last value. The module docs own the two
    /// places this diverges from `parse_str` and why.
    ///
    /// # Errors
    ///
    /// [`text_from`]'s throw, for a **name** whose escapes decode to octets
    /// that are not UTF-8 — that name is an array key and an array key is a
    /// `string`. A value has no such refusal. Every value in the answer is a
    /// `bytes` or a nested `array<mixed>`, which is what the spec's
    /// `array<mixed>` says and why it is not `array<bytes>`.
    fn nvs_core_uri_parse_query(_ctx, args: [1]) {
        let query = text_of(args, "parseQuery")?;
        Ok(Value::array(parse_query(query, "Core\\Uri","parseQuery", Values::Octets)?))
    }
}

/// The bracket convention itself, over a raw query string, for whichever member
/// is asking — `owner` and `member` are only the class and the name a refusal
/// quotes, so a `Core\Request` member's throw names `Core\Request`.
///
/// A free function rather than the body of the helper above, because
/// [`crate::request`] reads the same convention off a served request's own query
/// string and spec § 9 requires it to be *the same code*: `Core\Uri::parseQuery`
/// exists in part so that `Core\Request::query` does not answer the question a
/// second time, and two implementations of `a[b][]=1` is exactly the divergence
/// that promise is about.
///
/// # Errors
///
/// [`text_from`]'s throw, for a **name** whose escapes decode to octets that
/// are not UTF-8 — a name is an array key and an array key is a `string`. A
/// value refuses nothing under [`Values::Octets`] and refuses the same octets
/// under [`Values::Text`].
pub(crate) fn parse_query(
    query: &str,
    owner: &str,
    member: &str,
    values: Values,
) -> Result<NvsArray, Fault> {
    let mut out = NvsArray::new();
    for pair in query.split('&') {
        let (written_name, written_value) = pair.split_once('=').unwrap_or((pair, ""));
        let name = text_from(
            decode(written_name, Form::FormValue),
            owner,
            member,
            "the decoded name of a parameter",
        )?;
        if name.is_empty() {
            continue;
        }
        let octets = decode(written_value, Form::FormValue);
        let value = match values {
            Values::Octets => Value::bytes(NvsStr::new(&octets)),
            Values::Text => Value::str(NvsStr::new(
                text_from(octets, owner, member, "the decoded value of a parameter")?.as_bytes(),
            )),
        };
        place(&mut out, name.as_bytes(), value);
    }
    Ok(out)
}

/// What [`parse_query`] makes of a value's decoded octets.
///
/// The bracket walk is shared with [`crate::request`] because spec § 9 requires
/// *the same code* to read `a[b][]=1`; the answer's element type is not shared,
/// and this is the whole of the difference. `Core\Uri::parseQuery` is a § 12
/// member and answers the octets ([`Values::Octets`]); `Core\Request::query`
/// and `Core\Request::post` are § 15 members that read a served request's
/// parameters as text at the door and keep their refusal ([`Values::Text`]).
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Values {
    /// The octets themselves, as a `bytes` — § 12's answer.
    Octets,
    /// The octets as text, refusing the ones no `string` holds — § 15's.
    Text,
}

/// One already-decoded name and value, placed under the bracket convention.
///
/// [`parse_query`]'s tail, lifted out because `Core\Request::post()` reaches
/// the same convention from the other direction: a multipart form field arrives
/// as a name and a value that were never percent-encoded, so it has nothing to
/// decode and everything below the decode to share. Two spellings of the
/// bracket walk is how `a[b]=c` would come to mean one thing in a query string
/// and another in a form.
pub(crate) fn place(out: &mut NvsArray, name: &[u8], value: Value) {
    match path_of(name) {
        Some((base, path)) => insert(out, base, &path, value),
        None => out.set(NvsStr::new(name), value),
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Uri::buildQuery(array<mixed> $parameters): string` — replacing
    /// PHP's `http_build_query`.
    ///
    /// [`nvs_core_uri_parse_query`]'s inverse over the same bracket
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
    fn nvs_core_uri_build_query(_ctx, args: [1]) {
        // Unreachable from source: parameter 0 is `CoreTy::Array(CoreTy::Mixed)`
        // in `CLASS` above, so a non-container argument is `E0401: expected
        // 'array<mixed>', found 'mixed'` at the checker. This is
        // `Core\Arr::count`'s judgement, stated in full at
        // `nvs_core_arr_count`.
        let parameters = args[0].array_ptr().ok_or_else(|| {
            Fault::fatal(format!(
                "Core\\Uri::buildQuery expected {:?}, got tag {}",
                Tag::Array,
                args[0].tag_byte()
            ))
        })?;

        produced(&build(parameters, "Core\\Uri", "buildQuery", &[])?)
    }
}

#[cfg(test)]
mod tests {
    use nvs_runtime::{Ctx, OutputSink, Value, call};

    use super::{Form, encode};

    /// Runs one member through the `rule:errors/propagation` boundary compiled code reaches it
    /// at — [`crate::random`]'s own test helper, for its reasons.
    fn run(
        member: unsafe extern "C" fn(*mut Ctx, *const Value, *mut Value) -> i32,
        subject: &str,
    ) -> Result<String, i32> {
        let mut ctx = Ctx::new(OutputSink::Sink);
        let argument = Value::str(nvs_runtime::NvsStr::new(subject.as_bytes()));
        let answer = call(member, &mut ctx, &[argument]);
        let out = answer.map(|value| {
            let text = String::from_utf8(
                value
                    .as_str_bytes()
                    .expect("every member here answers with a `string`")
                    .to_vec(),
            )
            .expect("`rule:types/bytes` guarantees a `string` is UTF-8");
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

    /// [`run`] for a member that answers `bytes` rather than a `string` —
    /// which is both decoders, since the module docs' *A decoder answers
    /// `bytes`*. Kept separate rather than folded into `run` for the reason
    /// `Value::as_bytes` gives: a caller that means text must not silently
    /// accept octets.
    fn octets(
        member: unsafe extern "C" fn(*mut Ctx, *const Value, *mut Value) -> i32,
        subject: &str,
    ) -> Vec<u8> {
        let mut ctx = Ctx::new(OutputSink::Sink);
        let argument = Value::str(nvs_runtime::NvsStr::new(subject.as_bytes()));
        let answer = call(member, &mut ctx, &[argument]).expect("a decoder refuses nothing");
        let out = answer
            .as_bytes()
            .expect("a decoder answers `bytes`")
            .to_vec();
        #[expect(
            unsafe_code,
            reason = "this frame owns the answer the helper built and the \
                      argument it passed in, and a decoder borrows rather \
                      than consumes"
        )]
        unsafe {
            answer.release();
            argument.release();
        }
        out
    }

    /// The whole of ASCII plus one multi-byte character, round-tripped both
    /// ways: the assertion that catches a byte one encoder escapes and its own
    /// decoder does not restore.
    // covers: Core\Uri::encodeComponent, Core\Uri::decodeComponent, Core\Uri::encodeFormValue, Core\Uri::decodeFormValue
    #[test]
    fn every_byte_round_trips_through_both_encodings() {
        let subject: String = (0..=127_u8).map(char::from).chain(['é', '→']).collect();
        for (encoder, decoder) in [
            (
                super::nvs_core_uri_encode_component
                    as unsafe extern "C" fn(*mut Ctx, *const Value, *mut Value) -> i32,
                super::nvs_core_uri_decode_component
                    as unsafe extern "C" fn(*mut Ctx, *const Value, *mut Value) -> i32,
            ),
            (
                super::nvs_core_uri_encode_form_value,
                super::nvs_core_uri_decode_form_value,
            ),
        ] {
            let encoded = run(encoder, &subject).expect("an encoder never fails");
            assert!(
                encoded.is_ascii(),
                "an encoded answer is ASCII by construction"
            );
            assert_eq!(octets(decoder, &encoded), subject.as_bytes());
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
            // A leaf is `bytes` now, and lossily is the only way to render one
            // on a line: the case that cares which octets they were asserts
            // them directly rather than through here.
            return String::from_utf8_lossy(
                value.as_bytes().expect("a leaf of the answer is a `bytes`"),
            )
            .into_owned();
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
        let argument = Value::str(nvs_runtime::NvsStr::new(query.as_bytes()));
        let answer = call(super::nvs_core_uri_parse_query, &mut ctx, &[argument]);
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
    /// and appends numbered from the highest integer key already used, and an
    /// append after `i64::MAX` dropping its pair.
    // covers: Core\Uri::parseQuery
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
            (
                "a[9223372036854775807]=x&a[]=y&b=1",
                "{a:{9223372036854775807:x},b:1}",
            ),
            (
                "a[9223372036854775807][]=x&a[][]=y",
                "{a:{9223372036854775807:{0:x}}}",
            ),
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
        let argument = Value::str(nvs_runtime::NvsStr::new(query.as_bytes()));
        let parsed = call(super::nvs_core_uri_parse_query, &mut ctx, &[argument]);
        #[expect(unsafe_code, reason = "this frame owns the argument it built")]
        unsafe {
            argument.release();
        }
        let parsed = parsed?;
        let built = call(super::nvs_core_uri_build_query, &mut ctx, &[parsed]);
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
        .expect("`rule:types/bytes` guarantees a `string` is UTF-8");
        #[expect(unsafe_code, reason = "this frame owns the string the helper built")]
        unsafe {
            value.release();
        }
        Ok(text)
    }

    /// What PHP's `http_build_query` writes for the array its own `parse_str`
    /// read from the same query — including the escaped structural brackets
    /// and the indexes written out where the query wrote `[]`.
    // covers: Core\Uri::buildQuery
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

    /// Every pair repeats its whole bracket path, so an array of a few hundred
    /// kilobytes can ask for a text a hundred times its size. The text is
    /// refused before it is built: the request's peak stays near its ceiling
    /// instead of reaching the tens of megabytes the whole text would take.
    // covers: Core\Uri::buildQuery
    #[test]
    fn build_query_asks_the_budget_before_a_text_larger_than_its_input_grows() {
        let mut ctx = Ctx::buffered();
        ctx.set_memory_limit(4 << 20);
        let mut leaves = nvs_runtime::NvsArray::new();
        for _ in 0..20_000 {
            leaves.append(Value::int(1));
        }
        let mut parameters = Value::array(leaves);
        for _ in 0..200 {
            let mut level = nvs_runtime::NvsArray::new();
            level.set(nvs_runtime::NvsStr::new(b"k"), parameters);
            parameters = Value::array(level);
        }

        assert!(call(super::nvs_core_uri_build_query, &mut ctx, &[parameters]).is_err());
        assert!(
            ctx.memory_peak() < 8 << 20,
            "{} bytes at the peak",
            ctx.memory_peak()
        );
        #[expect(unsafe_code, reason = "this frame owns the array it built")]
        unsafe {
            parameters.release();
        }
    }

    /// A name is the array key the pair is placed under and an array key is a
    /// `string`, so a name whose escapes leave UTF-8 still refuses — wherever
    /// in the name it sits. A *value* has no such refusal, which is the
    /// neighbouring case.
    #[test]
    fn a_non_utf8_escape_in_a_name_throws_because_a_key_is_a_string() {
        assert!(parsed("%FF=a").is_err());
        assert!(parsed("a[%FF]=b").is_err());
        assert!(parsed("a=%FF").is_ok());
    }

    /// The claim the stage freezes: a decoder answers the octets themselves,
    /// so the escapes a `string` could never have carried have an answer here
    /// and text is one `as string` away. Asserted as bytes, since rendering
    /// them as text is exactly what this member no longer does.
    // covers: Core\Uri::decodeComponent, Core\Uri::decodeFormValue
    #[test]
    fn decode_component_answers_octets_that_are_not_valid_utf8() {
        assert_eq!(
            octets(super::nvs_core_uri_decode_component, "%ff%fe%fd"),
            [0xff, 0xfe, 0xfd]
        );
        assert_eq!(
            octets(super::nvs_core_uri_decode_component, "a%FFb"),
            [b'a', 0xff, b'b']
        );
        // A truncated sequence, a lone surrogate's encoding and an overlong
        // form: the three shapes `String::from_utf8` refused, all answered.
        assert_eq!(
            octets(super::nvs_core_uri_decode_form_value, "%C3%28"),
            [0xc3, 0x28]
        );
        assert_eq!(
            octets(super::nvs_core_uri_decode_form_value, "%ED%A0%80"),
            [0xed, 0xa0, 0x80]
        );
        assert_eq!(
            octets(super::nvs_core_uri_decode_component, "%C0%AF"),
            [0xc0, 0xaf]
        );
        // Still text where the octets are text, and still one pass: the `+`
        // rule is the only thing the two decoders disagree on.
        assert_eq!(
            octets(super::nvs_core_uri_decode_component, "caf%C3%A9+x"),
            "café+x".as_bytes()
        );
        assert_eq!(
            octets(super::nvs_core_uri_decode_form_value, "caf%C3%A9+x"),
            "café x".as_bytes()
        );
    }

    /// `parseQuery` decodes a name and a value with the same octet decoder, so
    /// a `+` and a `%XX` read alike on both sides of the `=` — and the value
    /// answers those octets even where no `string` could have held them, which
    /// is what the name cannot do, since it is a key.
    #[test]
    fn parse_query_answers_octets_for_a_name_and_for_a_value() {
        // Both halves through the same decoder: `%61` is `a` and `+` is a
        // space, in a name exactly as in a value.
        assert_eq!(parsed("%61+b=%61+c").expect("no throw"), "{a b:a c}");
        // The value's octets, whatever they are. `rendered` reads a leaf
        // lossily, so this asserts the bytes rather than their rendering.
        let query = super::parse_query(
            "n=%ff%fe%fd",
            "Core\\Uri",
            "parseQuery",
            super::Values::Octets,
        )
        .expect("a value refuses nothing");
        let slot = query.next_slot(0).expect("one pair");
        let value = query.value_at(slot).expect("a live slot has a value");
        assert_eq!(
            value.as_bytes().expect("a value is `bytes`"),
            [0xff, 0xfe, 0xfd]
        );
        // The same query read as a served request's parameters keeps § 15's
        // refusal, which is the whole of what `Values` decides.
        assert!(
            super::parse_query("n=%ff%fe%fd", "Core\\Request", "query", super::Values::Text)
                .is_err()
        );
    }

    /// The amendment is the decoders' alone: an encoder still takes text and
    /// still answers text, so `decodeComponent(encodeComponent($s)) as string`
    /// is the round trip it always was.
    // covers: Core\Uri::encodeComponent, Core\Uri::encodeFormValue
    #[test]
    fn the_two_encoders_still_take_text_and_answer_text() {
        for member in [
            super::nvs_core_uri_encode_component,
            super::nvs_core_uri_encode_form_value,
        ] {
            let answer = run(member, "a b/c?d é").expect("an encoder never throws");
            assert!(answer.is_ascii(), "for {answer:?}");
        }
        assert_eq!(
            octets(
                super::nvs_core_uri_decode_component,
                &run(super::nvs_core_uri_encode_component, "a b/c?d é").expect("no throw"),
            ),
            "a b/c?d é".as_bytes()
        );
        assert_eq!(
            octets(
                super::nvs_core_uri_decode_form_value,
                &run(super::nvs_core_uri_encode_form_value, "a b/c?d é").expect("no throw"),
            ),
            "a b/c?d é".as_bytes()
        );
    }

    /// PHP leaves a `%` that does not begin two hex digits exactly as it
    /// stands, and so does this — the module docs own why a decoder at the
    /// edge of a request does not throw over one.
    // covers: Core\Uri::decodeComponent
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
                octets(super::nvs_core_uri_decode_component, subject),
                expected.as_bytes()
            );
        }
    }

    /// The property the module docs promise and the whole reason `url` was not
    /// the pick: what goes in comes back out. Every row here is text the
    /// WHATWG URL Standard would have rewritten — a mixed-case scheme and
    /// host, a port that matches the scheme's default, dot segments, an
    /// escape that did not need escaping — and none of it moves.
    ///
    /// A `.nvst` case pins the components one at a time; this pins that the
    /// *text* is untouched, which is the assertion that fails the day someone
    /// swaps the crate underneath.
    // covers: Core\Uri::parse
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
            let nvs_runtime::Fault::Thrown(_, message) = refused else {
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
        let argument = Value::str(nvs_runtime::NvsStr::new(text.as_bytes()));
        let uri = call(super::nvs_core_uri_parse, &mut ctx, &[argument])
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
        let answer = call(super::nvs_core_uri_compare_to, &mut ctx, &[left, right])
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

    /// `$uri->toString()` — the text an instance kept — taking over the
    /// reference it is handed.
    ///
    /// Separate from [`own_text`] because the `with` cases below hold a `Uri`
    /// a *member* built rather than one `parse` did, and the question they
    /// ask it is the same one.
    fn text_of(uri: Value) -> String {
        let mut ctx = Ctx::new(OutputSink::Sink);
        let answer = call(super::nvs_core_uri_to_string, &mut ctx, &[uri])
            .expect("`toString` reads a slot and never throws");
        let out = String::from_utf8(
            answer
                .as_str_bytes()
                .expect("`toString` answers a `string`")
                .to_vec(),
        )
        .expect("`rule:types/bytes` guarantees a `string` is UTF-8");
        #[expect(
            unsafe_code,
            reason = "this frame owns the instance it was handed and the \
                      reference `toString` handed back"
        )]
        unsafe {
            answer.release();
            uri.release();
        }
        out
    }

    /// `Core\Uri::parse($text)->toString()` — the text the instance kept.
    fn own_text(text: &str) -> String {
        text_of(uri_of(text))
    }

    /// RFC 3986 § 6.2.2's three normalizations, and the three places the
    /// member deliberately stops short of them — the module docs' *Comparison
    /// normalizes* section, asserted rather than described.
    // covers: Core\Uri::compareTo
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

    /// The signature every `nvs_helper!` member symbol has.
    type Reader = unsafe extern "C" fn(*mut Ctx, *const Value, *mut Value) -> i32;

    /// One reader called on a fresh `Core\Uri::parse($text)`, through the
    /// member symbol compiled code calls. `None` is a `null` answer, so it
    /// stays apart from the `Some("")` an empty component answers, and a
    /// port comes back as its digits.
    fn reader(member: Reader, text: &str) -> Option<String> {
        let mut ctx = Ctx::new(OutputSink::Sink);
        let uri = uri_of(text);
        let answer = call(member, &mut ctx, &[uri]).expect("a reader never throws");
        let out = answer.as_int().map(|port| port.to_string()).or_else(|| {
            answer.as_str_bytes().map(|bytes| {
                String::from_utf8(bytes.to_vec())
                    .expect("`rule:types/bytes` guarantees a `string` is UTF-8")
            })
        });
        #[expect(
            unsafe_code,
            reason = "this frame owns the instance and the reference the reader \
                      handed back, and a reader borrows its receiver"
        )]
        unsafe {
            answer.release();
            uri.release();
        }
        out
    }

    /// `host`, `port`, `path` and `fragment` answer each component exactly as
    /// it was written: no case folding, no scheme default for the port, the
    /// escapes and dot segments still in the path. A component that was never
    /// written is `null`, and one written empty is `""`, except the port,
    /// where both are `null`. `path` is never `null`.
    // covers: Core\Uri::host, Core\Uri::port, Core\Uri::path, Core\Uri::fragment
    #[test]
    fn the_four_readers_answer_what_was_written_and_null_only_where_nothing_was() {
        let readers = [
            ("host", super::nvs_core_uri_host as Reader),
            ("port", super::nvs_core_uri_port),
            ("path", super::nvs_core_uri_path),
            ("fragment", super::nvs_core_uri_fragment),
        ];
        // Each row is the text, then what `host`, `port`, `path` and `fragment` answer.
        let rows: [(&str, [Option<&str>; 4]); 9] = [
            (
                "HTTP://Example.COM:0080/a/../b%7E#Top",
                [
                    Some("Example.COM"),
                    Some("80"),
                    Some("/a/../b%7E"),
                    Some("Top"),
                ],
            ),
            (
                "https://example.com",
                [Some("example.com"), None, Some(""), None],
            ),
            ("http://h:/p#", [Some("h"), None, Some("/p"), Some("")]),
            (
                "http://[2001:db8::1]:65535/",
                [Some("[2001:db8::1]"), Some("65535"), Some("/"), None],
            ),
            ("file:///tmp/x", [Some(""), None, Some("/tmp/x"), None]),
            (
                "mailto:someone@example.com",
                [None, None, Some("someone@example.com"), None],
            ),
            (
                "/docs?q=1#a%20b",
                [None, None, Some("/docs"), Some("a%20b")],
            ),
            ("#only", [None, None, Some(""), Some("only")]),
            ("", [None, None, Some(""), None]),
        ];
        for (text, answers) in rows {
            for ((name, member), answer) in readers.iter().zip(answers) {
                assert_eq!(
                    reader(*member, text).as_deref(),
                    answer,
                    "{name} of {text:?}"
                );
            }
        }
    }

    /// The positions in [`omitting`]'s array, which is `with`'s option order
    /// with the receiver taken off the front.
    const SCHEME: usize = 0;
    const HOST: usize = 1;
    const PORT: usize = 2;
    const PATH: usize = 3;
    const QUERY: usize = 4;
    const FRAGMENT: usize = 5;

    /// `with`'s six options, each the value an **omitting** call site
    /// materializes: a null for the three non-nullable text options and the
    /// never-written marker for the three nullable ones, which is the pairing
    /// `rule:core-api/a-nullable-field-omits-as-the-never-written-marker`
    /// holds over every registered row. A case overwrites the one position it
    /// is asking about, so each row below reads as the `{}` a program writes
    /// with a single key added.
    fn omitting() -> [Value; 6] {
        [
            Value::null(),
            Value::null(),
            Value::unset(),
            Value::null(),
            Value::unset(),
            Value::unset(),
        ]
    }

    /// A written `string` option, as the call site's own text.
    fn wrote(text: &str) -> Value {
        Value::str(nvs_runtime::NvsStr::new(text.as_bytes()))
    }

    /// One nullable option in the state a call site put it in: `None` is the
    /// key left out, `Some(None)` a written `null`, and `Some(Some(text))` a
    /// replacement — [`super::removable`]'s own three states, spelled here so
    /// a row can name a state and every call still gets a reference of its
    /// own to release.
    fn state_of(state: Option<Option<&str>>) -> Value {
        match state {
            None => Value::unset(),
            Some(None) => Value::null(),
            Some(Some(text)) => wrote(text),
        }
    }

    /// `Core\Uri::parse($subject)->with({…})`, as the `Uri` the caller then
    /// owns — or the status a refusal returned.
    ///
    /// The options are taken over. `with` borrows its arguments rather than
    /// consuming them, so this frame owes every reference in the array a
    /// release, and doing it here keeps the `unsafe` block out from under
    /// every row of every case below.
    fn with_of(subject: &str, options: [Value; 6]) -> Result<Value, i32> {
        let mut ctx = Ctx::new(OutputSink::Sink);
        let args: Vec<Value> = std::iter::once(uri_of(subject)).chain(options).collect();
        let answer = call(super::nvs_core_uri_with, &mut ctx, &args);
        #[expect(
            unsafe_code,
            reason = "this frame owns the receiver it parsed and every option \
                      it was handed, and `with` borrows rather than consumes"
        )]
        unsafe {
            for argument in args {
                argument.release();
            }
        }
        answer
    }

    /// [`with_of`] with one option written and every other key left out.
    fn one(subject: &str, option: usize, written: Value) -> Result<Value, i32> {
        let mut options = omitting();
        options[option] = written;
        with_of(subject, options)
    }

    /// One nullable reader's answer for a `Uri` this frame hands over: `None`
    /// where the component is absent, `Some(text)` where it is there —
    /// **including** where it is there and empty. Reading it off the tag
    /// rather than off the recomposed text is the point: those are the two
    /// states `rule:core-api/omission-is-not-a-written-null` exists to keep
    /// apart, and a text with a `?` in it is not evidence about either one.
    fn nullable(
        uri: Value,
        member: unsafe extern "C" fn(*mut Ctx, *const Value, *mut Value) -> i32,
    ) -> Option<String> {
        let mut ctx = Ctx::new(OutputSink::Sink);
        let answer = call(member, &mut ctx, &[uri]).expect("a component reader never throws");
        let out = answer.as_str_bytes().map(|bytes| {
            String::from_utf8(bytes.to_vec())
                .expect("`rule:types/bytes` guarantees a `string` is UTF-8")
        });
        #[expect(
            unsafe_code,
            reason = "this frame owns the instance it was handed and the \
                      answer the reader built"
        )]
        unsafe {
            answer.release();
            uri.release();
        }
        out
    }

    /// A call site's two ways of not giving a component a value, on the three
    /// `rule:core-classes/uri-removable-components` gives a removal to. The
    /// identity row is what makes each of the others a *removal*: a member
    /// that rewrote what it was not asked about would pass every removal line
    /// here and fail that one.
    #[test]
    fn a_written_null_removes_a_component_and_an_omitted_key_leaves_it_alone() {
        const SUBJECT: &str = "https://user@example.com:8443/a/b?x=1#top";

        assert_eq!(
            text_of(with_of(SUBJECT, omitting()).expect("no throw")),
            SUBJECT
        );
        for (option, written, expected) in [
            (PORT, Value::null(), "https://user@example.com/a/b?x=1#top"),
            (
                PORT,
                Value::int(80),
                "https://user@example.com:80/a/b?x=1#top",
            ),
            (
                QUERY,
                Value::null(),
                "https://user@example.com:8443/a/b#top",
            ),
            (
                QUERY,
                wrote("y=2"),
                "https://user@example.com:8443/a/b?y=2#top",
            ),
            (
                FRAGMENT,
                Value::null(),
                "https://user@example.com:8443/a/b?x=1",
            ),
            (
                FRAGMENT,
                wrote("end"),
                "https://user@example.com:8443/a/b?x=1#end",
            ),
        ] {
            assert_eq!(
                text_of(one(SUBJECT, option, written).expect("no throw")),
                expected
            );
        }

        // All three at once, because a removal is no shortcut around anything:
        // the result goes back through `read` and `unmoved` exactly as a
        // replacement does.
        let mut options = omitting();
        options[PORT] = Value::null();
        options[QUERY] = Value::null();
        options[FRAGMENT] = Value::null();
        assert_eq!(
            text_of(with_of(SUBJECT, options).expect("no throw")),
            "https://user@example.com/a/b"
        );

        // Removing a component that is not there is the identity too, rather
        // than something a caller has to check for first.
        assert_eq!(
            text_of(one("http://h/p", QUERY, Value::null()).expect("no throw")),
            "http://h/p"
        );
    }

    /// A `?` written with nothing after it is an empty query, an absent query
    /// is a third answer again, and `with` can produce and clear either — the
    /// distinction PHP's own `parse_url` does not report at all, and the one
    /// that collapses the day `""` becomes a removal spelling.
    // covers: Core\Uri::query
    #[test]
    fn an_empty_query_stays_distinct_from_an_absent_one() {
        const SUBJECT: &str = "http://h/p?x=1";

        for (state, text, query) in [
            (None, "http://h/p?x=1", Some("x=1")),
            (Some(None), "http://h/p", None),
            (Some(Some("")), "http://h/p?", Some("")),
            (Some(Some("y=2")), "http://h/p?y=2", Some("y=2")),
        ] {
            assert_eq!(
                text_of(one(SUBJECT, QUERY, state_of(state)).expect("no throw")),
                text,
                "for {state:?}"
            );
            assert_eq!(
                nullable(
                    one(SUBJECT, QUERY, state_of(state)).expect("no throw"),
                    super::nvs_core_uri_query,
                )
                .as_deref(),
                query,
                "for {state:?}"
            );
        }

        // Where an empty query comes from in the first place: one a client
        // wrote, which `parse` keeps and `query` reports as `""` rather than
        // as nothing.
        assert_eq!(
            nullable(uri_of("http://h/p?"), super::nvs_core_uri_query).as_deref(),
            Some("")
        );
        // And removing that one leaves no `?` at all, rather than the bare
        // one a caller editing the text by hand is left holding.
        assert_eq!(
            text_of(one("http://h/p?", QUERY, Value::null()).expect("no throw")),
            "http://h/p"
        );
    }

    /// `rule:core-api/a-written-null-removes`'s second half, swept across the
    /// options rather than asserted on one: `""` is a component that is there
    /// and empty, everywhere, and never an absence. Each row names what `""`
    /// writes beside what the removal of the same component writes, so a
    /// member that grew a `""`-means-remove shortcut fails on the pair even
    /// where either line alone still reads plausibly.
    #[test]
    fn there_is_no_empty_string_means_remove_rule_anywhere_in_with() {
        const SUBJECT: &str = "http://h/p?x=1#top";

        for (option, empty, removed) in [
            (QUERY, "http://h/p?#top", Some("http://h/p#top")),
            (FRAGMENT, "http://h/p?x=1#", Some("http://h/p?x=1")),
            // The other three text options have no removal to offer at all —
            // `rule:core-classes/uri-removable-components` says why for each —
            // so `""` is the empty component and there is no second answer to
            // compare it against. A `null` written into one of them is a
            // *compile* error rather than a removal, which is the checker's
            // half of this rule.
            (HOST, "http:///p?x=1#top", None),
            (PATH, "http://h?x=1#top", None),
        ] {
            assert_eq!(
                text_of(one(SUBJECT, option, wrote("")).expect("no throw")),
                empty
            );
            if let Some(removed) = removed {
                assert_eq!(
                    text_of(one(SUBJECT, option, Value::null()).expect("no throw")),
                    removed
                );
                assert_ne!(empty, removed, "an empty component is not an absent one");
            }
        }

        // The scheme is the one option an empty string cannot even recompose
        // into a reference: `://h/p?x=1#top` is refused by the grammar rather
        // than quietly becoming a relative one.
        assert!(one(SUBJECT, SCHEME, wrote("")).is_err());
    }

    /// `Core\Uri::parse($subject)->withQueryParameter($name, $value)`, as the
    /// `Uri` the caller then owns.
    ///
    /// The value is taken over, for [`with_of`]'s reason: the member borrows
    /// its arguments, so this frame owes every reference in the array a
    /// release and doing it here keeps the `unsafe` out from under each row.
    fn with_parameter_of(subject: &str, name: &str, value: Value) -> Value {
        let mut ctx = Ctx::new(OutputSink::Sink);
        let args = [uri_of(subject), wrote(name), value];
        let answer = call(super::nvs_core_uri_with_query_parameter, &mut ctx, &args)
            .expect("every subject here recomposes into a URI reference");
        #[expect(
            unsafe_code,
            reason = "this frame owns the receiver it parsed, the name it \
                      wrote and the value it was handed, and the member \
                      borrows rather than consumes"
        )]
        unsafe {
            for argument in args {
                argument.release();
            }
        }
        answer
    }

    /// [`with_parameter_of`]'s answer as the text it recomposed to.
    fn with_parameter(subject: &str, name: &str, value: Value) -> String {
        text_of(with_parameter_of(subject, name, value))
    }

    /// `$uri->queryParameter($name)`, as the value the caller then owns, over
    /// a `Uri` this frame hands over.
    fn parameter_of(uri: Value, name: &str) -> Value {
        let mut ctx = Ctx::new(OutputSink::Sink);
        let written = wrote(name);
        let answer = call(
            super::nvs_core_uri_query_parameter,
            &mut ctx,
            &[uri, written],
        )
        .expect("the receiver's own query parses, so the reader does not throw");
        #[expect(
            unsafe_code,
            reason = "this frame owns the instance it was handed and the name \
                      it wrote, and the reader borrows rather than consumes"
        )]
        unsafe {
            uri.release();
            written.release();
        }
        answer
    }

    /// The `bytes` a reader's answer holds, as text — `None` where it answered
    /// a `null` or an array, which are the reader's other two shapes.
    fn octets_of(value: Value) -> Option<String> {
        let out = value.as_bytes().map(<[u8]>::to_vec);
        #[expect(unsafe_code, reason = "this frame owns the answer the reader built")]
        unsafe {
            value.release();
        }
        out.map(|bytes| String::from_utf8(bytes).expect("every case here writes text"))
    }

    /// [`octets_of`] of the value stored under `key` of an array answer —
    /// what the bracket convention nested one level down.
    fn nested(value: Value, key: &[u8]) -> Option<String> {
        let out = value
            .array_ptr()
            .and_then(|root| crate::arr::borrowed(root).get(key))
            .and_then(|held| held.as_bytes().map(<[u8]>::to_vec));
        #[expect(
            unsafe_code,
            reason = "this frame owns the array the reader built, and `get` \
                      borrows from it rather than retaining"
        )]
        unsafe {
            value.release();
        }
        out.map(|bytes| String::from_utf8(bytes).expect("every case here writes text"))
    }

    /// The invariance the member's name states, asserted by sweeping every
    /// pair rather than read off one line: setting any one name rewrites that
    /// pair and leaves the others where they were. A member that rebuilt the
    /// query in an order of its own would pass one row here and fail the next.
    #[test]
    fn with_query_parameter_sets_one_pair_and_leaves_every_other_alone() {
        const THREE: &str = "https://user@example.com:8443/a/b?x=1&y=2&z=3#top";

        for (name, expected) in [
            ("x", "https://user@example.com:8443/a/b?x=9&y=2&z=3#top"),
            ("y", "https://user@example.com:8443/a/b?x=1&y=9&z=3#top"),
            ("z", "https://user@example.com:8443/a/b?x=1&y=2&z=9#top"),
            // A name that is not there is appended, and the other six
            // components are still exactly where they were.
            ("w", "https://user@example.com:8443/a/b?x=1&y=2&z=3&w=9#top"),
        ] {
            assert_eq!(with_parameter(THREE, name, wrote("9")), expected);
        }

        // A URI with no query at all gains one, which is the same call and
        // not a second member.
        assert_eq!(
            with_parameter("https://example.com/a", "x", wrote("1")),
            "https://example.com/a?x=1"
        );

        // Both halves are form-encoded on the way out, so a value carrying
        // every delimiter this class knows opens no pair, no fragment and no
        // component of its own — which is why `unmoved` never fires here.
        assert_eq!(
            with_parameter("https://example.com/a", "q", wrote("a b&c=d#e")),
            "https://example.com/a?q=a+b%26c%3Dd%23e"
        );

        // A fresh `Uri`, not a mutated one: the receiver still reads as it was
        // written once the member has answered out of it.
        let receiver = uri_of(THREE);
        let mut ctx = Ctx::new(OutputSink::Sink);
        let (name, value) = (wrote("x"), wrote("9"));
        let answer = call(
            super::nvs_core_uri_with_query_parameter,
            &mut ctx,
            &[receiver, name, value],
        )
        .expect("no throw");
        #[expect(
            unsafe_code,
            reason = "this frame owns the name and the value it wrote; the \
                      receiver and the answer are handed to `text_of`, which \
                      takes them over"
        )]
        unsafe {
            name.release();
            value.release();
        }
        assert_eq!(
            text_of(answer),
            "https://user@example.com:8443/a/b?x=9&y=2&z=3#top"
        );
        assert_eq!(text_of(receiver), THREE);
    }

    /// A written `null` removes the pair, and removing the **last** one leaves
    /// no query at all rather than the empty one a bare `?` is. The distinction
    /// is read off `query()`'s own tag rather than off a `?` in the recomposed
    /// text, because those are the two states
    /// `rule:core-api/omission-is-not-a-written-null` exists to keep apart and
    /// a text with a `?` in it is not evidence about either.
    #[test]
    fn a_null_value_removes_one_pair_and_the_last_one_leaves_no_query_at_all() {
        assert_eq!(
            with_parameter("https://example.com/a?x=1&y=2", "x", Value::null()),
            "https://example.com/a?y=2"
        );
        assert_eq!(
            with_parameter("https://example.com/a?only=1#top", "only", Value::null()),
            "https://example.com/a#top"
        );
        assert_eq!(
            nullable(
                with_parameter_of("https://example.com/a?only=1", "only", Value::null()),
                super::nvs_core_uri_query,
            ),
            None,
            "the last removal leaves no query, not the empty one a bare `?` is"
        );

        // Removing a name that is not there, and removing from a URI with no
        // query at all, are both the identity rather than something a caller
        // has to check for first.
        assert_eq!(
            with_parameter("https://example.com/a?x=1", "z", Value::null()),
            "https://example.com/a?x=1"
        );
        assert_eq!(
            with_parameter("https://example.com/a#top", "x", Value::null()),
            "https://example.com/a#top"
        );

        // An empty query holds no pairs, so a removal over one takes the `?`
        // away too — the one place this member turns an empty component into
        // an absent one, and it does it by rebuilding rather than by a rule
        // of its own.
        assert_eq!(
            with_parameter("https://example.com/a?", "x", Value::null()),
            "https://example.com/a"
        );
    }

    /// The pair is a round trip and neither half spells a bracket: the writer
    /// is handed an array and the reader answers one, with [`super::build`]
    /// and [`super::parse_query`] the only two places the convention is
    /// written down at all.
    // covers: Core\Uri::queryParameter
    #[test]
    fn a_query_parameter_round_trips_an_array_value_through_the_bracket_convention() {
        let nested_value = || {
            let mut inner = nvs_runtime::NvsArray::new();
            inner.set(
                nvs_runtime::NvsStr::new(b"b"),
                Value::str(nvs_runtime::NvsStr::new(b"c")),
            );
            Value::array(inner)
        };

        assert_eq!(
            with_parameter("https://example.com/a?x=1", "a", nested_value()),
            "https://example.com/a?x=1&a%5Bb%5D=c"
        );
        assert_eq!(
            nested(
                parameter_of(
                    with_parameter_of("https://example.com/a?x=1", "a", nested_value()),
                    "a",
                ),
                b"b",
            ),
            Some("c".to_owned()),
            "the reader answers the array the writer was handed"
        );

        // A scalar under the same name reads back as `bytes`: the reader's
        // answer is `mixed` because the convention's is, not because the
        // member declined to decide.
        assert_eq!(
            octets_of(parameter_of(
                with_parameter_of("https://example.com/a", "a", wrote("c")),
                "a",
            )),
            Some("c".to_owned())
        );

        // A name that spells brackets itself is a top-level key that `build`
        // escapes into the very same bytes an array value writes, so the two
        // calls are one query string and it reads back nested. That is
        // `buildQuery`'s own divergence — a query string cannot tell those
        // two keys apart — and not a second one this pair introduced.
        assert_eq!(
            with_parameter("https://example.com/a", "a[b]", wrote("c")),
            "https://example.com/a?a%5Bb%5D=c"
        );
        assert_eq!(
            octets_of(parameter_of(
                uri_of("https://example.com/a?a%5Bb%5D=c"),
                "a[b]",
            )),
            None
        );
        assert_eq!(
            nested(
                parameter_of(uri_of("https://example.com/a?a%5Bb%5D=c"), "a"),
                b"b",
            ),
            Some("c".to_owned())
        );
    }

    /// `Core\Uri::parse($base)->resolve($reference)->toString()` — the text
    /// answered, or the message the member threw.
    fn resolved(base: &str, reference: &str) -> Result<String, String> {
        let mut ctx = Ctx::new(OutputSink::Sink);
        let args = [uri_of(base), wrote(reference)];
        let answered = call(super::nvs_core_uri_resolve, &mut ctx, &args);
        let refusal = ctx.take_pending().map(std::borrow::Cow::into_owned);
        #[expect(
            unsafe_code,
            reason = "this frame owns the receiver it parsed and the reference \
                      it wrote, and `resolve` borrows rather than consumes"
        )]
        unsafe {
            for arg in args {
                arg.release();
            }
        }
        answered
            .map(text_of)
            .map_err(|_| refusal.expect("a refusal leaves its message in the context"))
    }

    /// RFC 3986 § 5.4.1's normal examples against its own base, the base's
    /// fragment dropped before § 5.3 recomposes, and the two bases § 5.2.1 and
    /// § 5.2.3 leave without an answer — each a throw rather than a guess.
    // covers: Core\Uri::resolve
    #[test]
    fn resolve_runs_the_rfc_normal_examples_and_throws_for_a_base_with_no_answer() {
        const BASE: &str = "http://a/b/c/d;p?q";
        for (reference, expected) in [
            ("g:h", "g:h"),
            ("g", "http://a/b/c/g"),
            ("./g", "http://a/b/c/g"),
            ("g/", "http://a/b/c/g/"),
            ("/g", "http://a/g"),
            ("//g", "http://g"),
            ("?y", "http://a/b/c/d;p?y"),
            ("g?y", "http://a/b/c/g?y"),
            ("#s", "http://a/b/c/d;p?q#s"),
            ("", "http://a/b/c/d;p?q"),
            ("..", "http://a/b/"),
            ("../g", "http://a/b/g"),
            ("../../g", "http://a/g"),
        ] {
            assert_eq!(
                resolved(BASE, reference).as_deref(),
                Ok(expected),
                "RFC 3986 § 5.4.1: {reference:?}"
            );
        }

        assert_eq!(
            resolved("http://a/b?q#f", "").as_deref(),
            Ok("http://a/b?q"),
            "§ 5.1: a base carries no fragment"
        );

        let relative = resolved("/relative/base", "g").expect_err("a relative base has no answer");
        assert!(
            relative.contains("the receiver is a relative reference"),
            "{relative}"
        );
        let opaque = resolved("mailto:a@b", "g").expect_err("an opaque base has no path");
        assert!(opaque.contains("it is opaque"), "{opaque}");
        let unread = resolved(BASE, "a b").expect_err("a space is outside the grammar");
        assert!(
            unread.contains("Core\\Uri::resolve(): this text is not a URI reference"),
            "{unread}"
        );
    }

    /// The scheme comes back in the case it was written in, for a hierarchical
    /// URI and an opaque one alike, and a relative reference — the network-path
    /// one included — has none.
    // covers: Core\Uri::scheme
    #[test]
    fn scheme_is_reported_as_written_and_absent_from_a_relative_reference() {
        for (subject, scheme) in [
            ("https://example.com/", Some("https")),
            ("HTTPS://example.com/", Some("HTTPS")),
            ("mailto:ann@example.com", Some("mailto")),
            ("urn:isbn:0451450523", Some("urn")),
            ("git+ssh://example.com/repo", Some("git+ssh")),
            ("//example.com/a", None),
            ("/a/b", None),
            ("a:b/c", Some("a")),
            ("./a:b", None),
            ("", None),
        ] {
            assert_eq!(
                reader(super::nvs_core_uri_scheme, subject).as_deref(),
                scheme,
                "{subject:?}"
            );
        }
    }

    /// The userinfo is one piece, still escaped: nothing splits it at its `:`,
    /// an `@` written with nothing before it is `""` rather than `null`, and a
    /// `@` outside the authority is not userinfo at all.
    // covers: Core\Uri::userInfo
    #[test]
    fn user_info_is_one_escaped_piece_and_empty_differs_from_absent() {
        for (subject, user_info) in [
            ("https://ann@example.com/", Some("ann")),
            ("https://ann:s3cret@example.com/", Some("ann:s3cret")),
            ("https://ann%40work@example.com/", Some("ann%40work")),
            ("https://@example.com/", Some("")),
            ("https://example.com/", None),
            ("https://example.com/@ann", None),
            ("mailto:ann@example.com", None),
            ("/a/b", None),
        ] {
            assert_eq!(
                reader(super::nvs_core_uri_user_info, subject).as_deref(),
                user_info,
                "{subject:?}"
            );
        }
    }

    /// A key ring of one, as `array<secret bytes>`. A fixed key rather than a
    /// drawn one, because every assertion below compares two tokens and a
    /// signature is deterministic.
    fn ring() -> Value {
        let mut keys = nvs_runtime::NvsArray::new();
        keys.append(Value::bytes(nvs_runtime::NvsStr::new(&[7_u8; 32])));
        Value::array(keys)
    }

    /// Releases what one of these tests built.
    fn dropped(value: Value) {
        #[expect(
            unsafe_code,
            reason = "a test frame owns exactly the reference it built"
        )]
        unsafe {
            value.release();
        }
    }

    /// `Core\Uri::parse($subject)->sign({keys: $keys, until: null})->toString()`
    /// — the signed link, as text, which is the form every assertion below
    /// tampers with.
    fn signed_text(subject: &str, keys: Value) -> String {
        let mut ctx = Ctx::new(OutputSink::Sink);
        let uri = uri_of(subject);
        let signed = call(
            super::nvs_core_uri_sign,
            &mut ctx,
            &[uri, keys, Value::null()],
        )
        .expect("a ring of one 32-octet key signs");
        #[expect(
            unsafe_code,
            reason = "this frame owns the receiver it parsed, and `sign` \
                      borrows rather than consumes"
        )]
        unsafe {
            uri.release();
        }
        text_of(signed)
    }

    /// `Core\Uri::parse($subject)->verifySignature($keys)` — the answer, or the
    /// message it refused with.
    fn verify(subject: &str, keys: Value) -> Result<Value, String> {
        let mut ctx = Ctx::new(OutputSink::Sink);
        let uri = uri_of(subject);
        let answered = call(super::nvs_core_uri_verify_signature, &mut ctx, &[uri, keys]);
        let refusal = ctx.take_pending().map(std::borrow::Cow::into_owned);
        #[expect(
            unsafe_code,
            reason = "this frame owns the receiver it parsed, and \
                      `verifySignature` borrows rather than consumes"
        )]
        unsafe {
            uri.release();
        }
        answered.map_err(|_| refusal.expect("a refusal leaves its message in the context"))
    }

    /// The token the reserved parameter carries, out of a link this suite
    /// signed — stopping at a fragment, which rides after it and is not part
    /// of it.
    fn token_of(link: &str) -> &str {
        link.rsplit("_sig=")
            .next()
            .expect("a signed link carries one")
            .split(['&', '#'])
            .next()
            .expect("a split answers at least once")
    }

    /// The other half of `crate::router`'s
    /// `url_signed_verifies_through_signed_route_after_the_mount_prefix_changes`,
    /// and the whole reason `Core\Router` carries a signing pair of its own.
    ///
    /// A signature taken over a **path** is a signature over where the module
    /// happens to be mounted, and a mount moves: one compiled table serves at
    /// `/ModuleA`, at `/ModuleB` or at `/`
    /// (`rule:http-server/a-mount-table-expands-at-boot`). Nothing here is a
    /// defect — this door signs a URL because a URL is what its caller holds —
    /// but it is the failure a signed *route* does not have, and asserting it
    /// beside the other member is what keeps the pair from looking redundant.
    #[test]
    fn the_same_link_signed_as_a_path_stops_verifying_when_the_mount_moves() {
        let keys = ring();
        let link = signed_text("https://example.com/ModuleA/shop/7", keys);
        assert!(
            verify(&link, keys).is_ok(),
            "the link verifies where it was signed"
        );

        let moved = link.replace("/ModuleA/", "/ModuleB/");
        assert_ne!(moved, link, "the remount is a different path");
        let refusal = verify(&moved, keys)
            .expect_err("and a different path is a different URL, whatever route it reaches");
        assert!(
            refusal.contains("not one $keys signed"),
            "refused as any other edit is: {refusal}"
        );

        dropped(keys);
    }

    /// The round trip, and the property the whole design is for: every
    /// component present is covered, so one more parameter is a different URL.
    ///
    /// This is the bypass every framework in this space has shipped — an
    /// option naming which parameters are signed — asserted as absent rather
    /// than described (`rule:core-api/signing-is-over-a-payload`).
    #[test]
    fn a_signed_uri_round_trips_and_one_appended_query_parameter_invalidates_it() {
        let keys = ring();
        let link = signed_text("https://example.com/report?id=7", keys);
        assert!(
            link.starts_with("https://example.com/report?id=7&_sig="),
            "the answer is the receiver with the reserved parameter set: {link}"
        );

        assert!(
            verify(&link, keys).is_ok(),
            "the link this ring just signed verifies"
        );
        assert!(
            verify(&format!("{link}&admin=1"), keys).is_err(),
            "and appending one parameter is a different URL, whatever it is called"
        );

        dropped(keys);
    }

    /// The other direction of the same rule, which a signature over an
    /// assembled string routinely misses: dropping a parameter is as much a
    /// forgery as adding one.
    #[test]
    fn a_removed_query_parameter_invalidates_it() {
        let keys = ring();
        let link = signed_text("https://example.com/report?id=7&scope=own", keys);
        assert!(link.contains("scope=own&_sig="), "{link}");

        assert!(
            verify(&link.replace("scope=own&", ""), keys).is_err(),
            "a narrowing parameter a holder can drop is the whole attack"
        );
        assert!(
            verify(&link, keys).is_ok(),
            "and the bound's other side: the untouched link still verifies"
        );

        dropped(keys);
    }

    /// The one thing a query string genuinely does not carry.
    ///
    /// A proxy, a redirect or a form may write the same pairs in another
    /// order, so a signature that covered the order would fail on URLs nobody
    /// tampered with — `crates/nvs-stdlib/src/signature.rs` sorts a payload's
    /// keys and [`super::payload_of`] hands it parameters rather than text.
    #[test]
    fn reordering_the_query_still_verifies_because_ordering_is_not_canonical() {
        let keys = ring();
        let link = signed_text("https://example.com/report?id=7&scope=own", keys);
        let token = token_of(&link).to_owned();

        assert!(
            verify(
                &format!("https://example.com/report?scope=own&id=7&_sig={token}"),
                keys
            )
            .is_ok(),
            "the same pairs in the other order are the same URL"
        );
        assert_eq!(
            token_of(&signed_text(
                "https://example.com/report?scope=own&id=7",
                keys
            )),
            token,
            "and signing them in that order mints the same token"
        );

        dropped(keys);
    }

    /// § 6.2.2.1's fold, reached through the same function `compareTo` reaches
    /// it through: an escape rewritten in the other case is not a tampered
    /// URL.
    #[test]
    fn rewriting_an_escapes_hex_digits_in_the_other_case_still_verifies() {
        let keys = ring();
        let link = signed_text("https://example.com/a%2fb?id=7", keys);
        assert!(
            link.contains("%2f"),
            "signing does not rewrite the receiver's own spelling: {link}"
        );

        assert!(
            verify(&link.replace("%2f", "%2F"), keys).is_ok(),
            "the two spellings are one URL, and a second normalization is what \
             would have made them two"
        );

        dropped(keys);
    }

    /// RFC 3986 § 3.5: a fragment is never sent to the server, so signing one
    /// would mint links that cannot verify where it matters.
    #[test]
    fn changing_or_adding_a_fragment_still_verifies_because_a_fragment_is_never_signed() {
        let keys = ring();
        let link = signed_text("https://example.com/a?id=7", keys);
        assert!(
            verify(&format!("{link}#top"), keys).is_ok(),
            "a fragment added after the fact changes nothing the server sees"
        );

        let fragmented = signed_text("https://example.com/a?id=7#one", keys);
        assert!(fragmented.contains("#one"), "{fragmented}");
        assert_eq!(
            token_of(&fragmented),
            token_of(&link),
            "so the receiver's own fragment is not in the payload either"
        );
        assert!(
            verify(&fragmented.replace("#one", "#two"), keys).is_ok(),
            "and editing it is not a forgery"
        );

        dropped(keys);
    }

    /// Agreement, over the pair of members that must not grow two canonical
    /// forms: two references `compareTo` calls one URI sign identically, and
    /// one it orders apart signs apart.
    ///
    /// The shape that catches the failure this whole design is written against
    /// — a normalization written for the signature alone, which no other test
    /// constrains. A `sign` that folded one case less would still round-trip
    /// against itself and pass every other assertion here.
    #[test]
    fn signing_calls_the_same_equivalent_that_compare_to_calls_and_not_a_second_one() {
        let keys = ring();
        let plain = "http://example.com/a/b?id=7";
        let dressed = "HTTP://Example.COM/a/./b?id=7";
        let apart = "http://example.com/a/c?id=7";

        assert_eq!(
            compared(plain, dressed),
            0,
            "§ 6.2.2 folds the scheme, the host and the dot segment"
        );
        assert_eq!(
            token_of(&signed_text(plain, keys)),
            token_of(&signed_text(dressed, keys)),
            "so the two sign identically, because it is one function reached twice"
        );

        assert_ne!(
            compared(plain, apart),
            0,
            "a different path is a different URI"
        );
        assert_ne!(
            token_of(&signed_text(plain, keys)),
            token_of(&signed_text(apart, keys)),
            "and it signs differently"
        );

        dropped(keys);
    }

    /// The reserved parameter excludes itself from its own input, so a link
    /// can be signed again — after a rotation, say — without nesting one token
    /// inside the next payload.
    #[test]
    fn signing_a_uri_that_already_carries_sig_replaces_it_rather_than_nesting() {
        let keys = ring();
        let once = signed_text("https://example.com/a?id=7", keys);
        let twice = signed_text(&once, keys);

        assert_eq!(
            twice.matches("_sig=").count(),
            1,
            "one token, not one wrapped around another: {twice}"
        );
        assert_eq!(
            token_of(&twice),
            token_of(&once),
            "and it is the same token, because the payload is the URL without it"
        );
        assert!(verify(&twice, keys).is_ok());

        dropped(keys);
    }

    /// A URL with two of them is refused rather than checked under one:
    /// picking one is picking which of two answers an attacker gets to try,
    /// and every parser downstream picks a different one.
    #[test]
    fn a_url_carrying_two_sig_parameters_fails_under_either_of_them() {
        let keys = ring();
        let link = signed_text("https://example.com/a?id=7", keys);
        let token = token_of(&link).to_owned();

        for doubled in [
            format!("https://example.com/a?id=7&_sig={token}&_sig={token}"),
            format!("https://example.com/a?id=7&_sig=AAAA&_sig={token}"),
            format!("https://example.com/a?id=7&_sig={token}&_sig=AAAA"),
        ] {
            assert!(
                verify(&doubled, keys).is_err(),
                "two of them is a refusal and not a choice: {doubled}"
            );
        }

        dropped(keys);
    }

    /// The refusal a naive verifier treats as "no signature to check" is the
    /// same sentence a forged one gets (`rule:core-api/one-refusal-except-expiry`).
    #[test]
    fn a_url_with_no_sig_fails_with_the_same_error_a_forged_one_raises() {
        let keys = ring();
        let link = signed_text("https://example.com/a?id=7", keys);

        let missing = verify("https://example.com/a?id=7", keys)
            .expect_err("a URL with no `_sig` is not a signed URL");
        let forged = verify(&format!("{link}AAAA"), keys)
            .expect_err("four more characters of the alphabet still reach the tag check");
        assert_eq!(
            missing, forged,
            "which half failed is exactly what a forger is probing for"
        );

        dropped(keys);
    }

    /// `void`, and a throw: there are no claims to hand back — the claim is
    /// the URL the caller already holds — and a `bool` is a value a caller can
    /// drop on the floor.
    #[test]
    fn verify_signature_answers_nothing_and_throws_rather_than_returning_a_bool() {
        let keys = ring();
        let link = signed_text("https://example.com/a?id=7", keys);

        let answer = verify(&link, keys).expect("the link this ring signed verifies");
        assert_eq!(
            answer.tag(),
            Some(nvs_runtime::Tag::Null),
            "the row answers `void`, which is not a `bool` to test"
        );
        assert!(
            verify("https://example.com/a?id=7", keys).is_err(),
            "and a URL that does not verify throws rather than answering `false`"
        );

        dropped(keys);
    }
}
