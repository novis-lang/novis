//! `Core\Uri` — [docs/spec/01-core-library.md](../../../../docs/spec/01-core-library.md)
//! § 12's first table. This module is that table's **percent-encoding half**
//! — `encodeComponent`/`decodeComponent` and
//! `encodeFormValue`/`decodeFormValue`, which replace PHP's
//! `rawurlencode`/`rawurldecode` and `urlencode`/`urldecode` — plus
//! `parseQuery` and `buildQuery`, which are those four applied to a whole
//! query string. The **grammar** half, `parse` and the `Uri` instance, is
//! gap 1.
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
//! is `Uri::isValid`, which is not here yet — see gap 1.
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
//! The **grammar** half of § 12's table — `parse`, `isValid`, `$uri->with`,
//! `$uri->resolve` — is RFC 3986, is therefore a dependency, and lands with
//! the pick that decides it. See gap 1.
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
//! # Known gaps
//!
//! 1. **`Uri::parse`, `Uri::isValid`, `$uri->with` and `$uri->resolve` are not
//!    built**, so the spec table's grammar half is missing and this class has
//!    no instance shape yet. `isValid` is deliberately *not* hand-written
//!    ahead of `parse`: the two answer one question — "is this text a URI" —
//!    and a validator written against one reading of RFC 3986 beside a parser
//!    written against a crate's is the drift [`crate::uuid`] avoids by giving
//!    both members one `read`. Whichever crate `parse` binds decides `isValid`
//!    with it.
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

use mwl_runtime::{Fault, HelperResult, MwlArray, MwlStr, Tag, Value};

use crate::registry::{CoreClass, CoreMethod, CoreTy};

// ============================================================================
// Registration — this class's rows, and where its symbols live
// ============================================================================

/// `Core\Uri`'s fully-qualified name, written once so the registry row and
/// every diagnostic naming the class cannot drift apart.
pub const NAME: &str = r"Core\Uri";

/// `Core\Uri`'s registry rows — spec § 12's first table, percent-encoding
/// half. The grammar half is gap 1.
pub const CLASS: CoreClass = CoreClass {
    name: NAME,
    methods: &[
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
    instance: &[],
    slots: &[],
    constants: &[],
};

/// The address of one of *this* module's symbols, or `None` for a symbol that
/// belongs to another domain. See [`crate::symbols`].
pub(crate) fn address(symbol: &str) -> Option<*const u8> {
    Some(match symbol {
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
fn text_of<'a>(args: &'a [Value], member: &str) -> Result<&'a str, Fault> {
    let bytes = args[0].as_str_bytes().ok_or_else(|| {
        Fault::fatal(format!(
            "Core\\Uri::{member} expected {:?}, got tag {}",
            Tag::Str,
            args[0].tag_byte()
        ))
    })?;
    std::str::from_utf8(bytes).map_err(|_| {
        Fault::fatal(format!(
            "Core\\Uri::{member} received a `string` that is not valid UTF-8, which ADR 0009 \
             guarantees it cannot be"
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
}
