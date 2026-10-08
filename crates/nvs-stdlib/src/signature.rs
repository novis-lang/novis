//! The canonical form every signature in the language is taken over —
//! `rule:core-api/signing-is-over-a-payload`'s "never over assembled text",
//! written once so that the three doors onto it (`Core\Signature`, `Core\Uri`
//! and `Core\Router`) cannot each canonicalize a payload slightly differently.
//!
//! `rule:core-classes/signature` places the class; what belongs here is the
//! wire format itself, why it is a document rather than a string, and what it
//! refuses.
//!
//! # A signature is taken over a document, and the document is built here
//!
//! Every hand-rolled signing helper in every language gets the same thing
//! wrong: it assembles text — `"$id|$until|$path"` — and then two sides of the
//! same protocol disagree about the assembly. A separator that appears in a
//! value, a key written in a different order, an integer rendered `1` on one
//! side and `1.0` on the other, and a signature that verifies over bytes the
//! program never meant. So nothing here is assembled. A payload is walked and
//! written into a **type-tagged, length-prefixed** document: every value
//! carries the tag of its own type, every variable-length body carries its
//! length before it, and no two distinct values encode to the same bytes.
//! `1` and `"1"` differ in their first byte; `["ab", "c"]` and `["a", "bc"]`
//! differ in their lengths; there is no separator to smuggle anything past.
//!
//! **A key is text**, and written as the text the language itself renders it
//! as. An array key is a `string` whatever shape the array is in — a list
//! holds `0` as a position and a hashed array holds it as `"0"`, and
//! `nvs_runtime::NvsArray::degrade`'s own docs are where "without changing a
//! single answer" is written down — so encoding a position as an integer would
//! give one array two signatures, depending only on whether it had ever had a
//! gap in it.
//!
//! **Keys are sorted**, by those octets, at every level. Two
//! programs that build the same map in a different order therefore sign
//! identically, which is the property a signed URL needs — the query string it
//! travels in has no ordering either ([`crate::uri`]'s RFC 3986 § 6.2.2
//! normalization is the one that says so). The consequence is the one thing to
//! know about a round trip: **insertion order is not signed and does not come
//! back.** A verified payload is in canonical order, not in the order the
//! signer wrote it.
//!
//! **The lifetime rides inside the document**, not beside it: [`Until`] is a
//! field of the signed region, so a holder cannot edit an expiry that is
//! covered by the tag, and there is no second parameter for the two sides to
//! keep in step (`rule:core-api/a-lifetime-is-written`).
//!
//! # The domain byte, so one ring cannot be replayed across three doors
//!
//! A program is expected to hand the same key ring to `Core\Signature` and to
//! `$uri->sign` — one rotation vocabulary, one ring. That makes cross-door
//! replay a real question rather than a theoretical one, so every document
//! names the [`Domain`] it was made for, inside the signed region and before
//! anything else. A token minted for a payload does not verify as a URL
//! signature whatever else matches, and the check costs one byte and one
//! comparison.
//!
//! # What a token is on the wire
//!
//! `tag ‖ document`, in unpadded URL-safe base64 (RFC 4648 § 5). The tag is
//! HMAC-SHA256 over the document, whole and untruncated, and it comes **first**
//! so that the split is a fixed offset: [`open`] takes 32 octets and a
//! remainder, with no field to parse before the thing that says the field can
//! be trusted. The alphabet is `A-Za-z0-9-_`, so a token is a legal cookie
//! octet sequence and a legal query-string value at once, and nothing
//! downstream escapes it again.
//!
//! There is no version prefix, no key identifier and no separator outside the
//! signed region: the version is *inside* it, where it cannot be edited, and a
//! key hint would tell an attacker which key of a rotating ring to aim at.
//! Everything a reader needs before it can check the tag is therefore a
//! length, which is the one thing a forger cannot lie about usefully.
//!
//! The ring is [`crate::keyring`]'s, unchanged and uncopied — the same
//! `$keys` a program hands `Core\SignedCookie`. [`mint`] takes the newest key
//! and [`open`] tries the ring in order, stopping at the one that
//! authenticates.
//!
//! # What it refuses, and why refusing is the safe answer
//!
//! An object, a callable and a resource have no canonical form at all — two
//! instances that a program calls equal are two different heaps — so a payload
//! carrying one is a `LogicError` naming the path to it rather than a
//! signature over something arbitrary. A `decimal` **is** signable, because
//! `nvs_runtime::Decimal::reduced` is a canonical form the language already
//! defines: `1.0` and `1.00` sign identically, exactly as they compare equal.
//!
//! Nesting is bounded at [`MAX_DEPTH`]. Both walks here recurse, so the bound
//! is a bound on stack, and a signed payload is a claim set rather than a
//! document — a program with a deep tree to sign signs its serialization as a
//! `string`, which is one value at depth one.
//!
//! # A payload value is text, and both halves of `Core\Signature` agree on it
//!
//! The codec above signs nine kinds of value, and [`nvs_core_signature_verify`]
//! answers `array<tainted string>` — so `Core\Signature`'s own door narrows to
//! the one kind that type can hold, and `sign` declares `array<string>`.
//! `rule:security/verification-does-not-launder` is what forces it:
//! `nvs_types` defines `tainted` over `string` and `bytes` and over nothing
//! else, so there is no `tainted array<mixed>` to answer with, and an
//! `array<mixed>` of verified values would hand a program something that had
//! visibly been verified and invisibly been laundered. [`crate::jwt`]'s *a
//! claim is text* section reached the same place first, from the same two
//! facts, and the two roster entries agree rather than each inventing a rule.
//!
//! The spec writes `array<string, mixed>` for both halves
//! ([docs/spec/01-core-library.md](/docs/spec/01-core-library.md) § 16), and
//! that is the half that yields: a qualifier a program can reach beats a
//! wider payload, per AGENTS.md's ordering. **What it spends** is a signed
//! `int` — a program that wants one writes `string($id)` on the way in and
//! `int($payload["id"])` on the way out, and that second conversion is a
//! checked one, so the value it produces is laundered by the language's own
//! named route rather than by having been signed.
//!
//! The narrowing is `Core\Signature`'s and not the codec's. [`document`] keeps
//! all nine kinds because [`Domain::Uri`] and [`Domain::Route`] sign typed
//! parameters that never come back to a program as a map — `$uri->sign`
//! answers a `Uri` and `verifySignature` answers nothing — so there is no
//! declared element type for them to be honest about.
//!
//! # What it spends
//!
//! Per `sign`: one `Vec` the size of the document, one the size of the token,
//! and the base64 text of it. Per `verify`: the decoded token, and the payload
//! the caller is handed. All of it inside the call and held nowhere between
//! calls (`rule:programs/memory-priority`). The sort is per array level over
//! that level's keys, and the tag is one HMAC per key tried until one
//! authenticates. A document is the payload's own octets plus one tag byte and
//! one length varint per value, plus two bytes of version and domain and one
//! to thirteen of lifetime — so a token, after its 32-octet tag and base64's
//! four thirds, is about `4/3 × (payload + 40)` characters.

use base64::Engine as _;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use nvs_runtime::{Ctx, Decimal, Fault, NvsArray, NvsStr, SlotKey, Tag, ThrownClass, Value};
use subtle::ConstantTimeEq as _;

use crate::keyring::KEY;
use crate::registry::{
    ClassDoc, CoreClass, CoreField, CoreMethod, CoreTy, ErrorDoc, MethodDoc, ParamDoc, Qual,
    ShapeKeyDoc,
};

// ============================================================================
// `Core\Signature` — registration
// ============================================================================

/// The class name, once, for the messages that all name it.
const NAME: &str = r"Core\Signature";

/// `Core\Signature::sign`, spelled the way a refusal names it.
const SIGN: &str = r"Core\Signature::sign";

/// `Core\Signature::verify`, spelled the way a refusal names it.
const VERIFY: &str = r"Core\Signature::verify";

/// One verified payload value, as
/// `rule:security/verification-does-not-launder` requires it back, and the
/// module doc's *a payload value is text* section is the home of why that
/// spelling fixes the element type.
const SIGNED: CoreTy = CoreTy::TaintedStr;

/// The `{keys, until}` every door of
/// `rule:core-api/signing-is-over-a-payload` takes — one arm, because there is
/// nothing here to discriminate on (`rule:core-api/shape-parameter`).
///
/// **A shape and not an options bag**, and `until` is why:
/// `rule:core-api/a-lifetime-is-written` makes the lifetime a *required* key
/// holding a nullable value, and `rule:core-api/shape-rules` R2 makes every
/// member of a bag optional. So a bag could not have said the one thing this
/// parameter exists to say — that `{keys: $ring}` does not compile and
/// `{keys: $ring, until: null}` does, because a permanent signed link should
/// be something a person typed.
///
/// `until` therefore carries `default: None` over a nullable type, which is
/// the one pairing [`crate::registry::Const::NeverWritten`] has nothing to say
/// about: a required field is never omitted, so there is no omission for a
/// written `null` to be confused with.
///
/// Written once and shared, so the three doors cannot come to disagree about
/// what a caller writes, exactly as they already share the codec below.
pub(crate) const SIGNING: &[&[CoreField]] = &[&[
    CoreField {
        name: "keys",
        // The ring, unchanged and uncopied — [`crate::keyring`] is the home of
        // what `$keys` means, and every door over a ring declares this type.
        ty: CoreTy::Array(&KEY),
        default: None,
    },
    CoreField {
        name: "until",
        ty: CoreTy::Nullable(&CoreTy::Instance(crate::time::INSTANT_NAME)),
        default: None,
    },
]];

/// `rule:core-api/shape-flattens-at-the-abi`'s flattening of [`SIGNING`] at
/// `sign`, as ABI slots: the payload, then the shape's two fields in the arm's
/// own order. There is no runtime representation of a shape, so these three
/// are what the helper is handed and this is the only place the numbers are
/// written.
const PAYLOAD_ARG: usize = 0;
/// See [`PAYLOAD_ARG`].
const KEYS_ARG: usize = 1;
/// See [`PAYLOAD_ARG`].
const UNTIL_ARG: usize = 2;

/// `Core\Signature`'s class card — `rule:core-api/reference-card`.
const CARD: ClassDoc = ClassDoc {
    short: "Signs a set of named values with a secret key, so your program can check later that \
            nobody changed them. Signed links and password reset tokens are typical uses. The \
            values are readable by anyone who has the token, so do not sign a secret.",
};

/// `rule:security/protocol-roster`'s fifth and final roster entry, as two rows.
pub(crate) const CLASS: CoreClass = CoreClass {
    name: NAME,
    doc: Some(&CARD),
    methods: &[
        CoreMethod {
            name: "sign",
            names: &["payload", "settings"],
            // The payload's element is marked as [`crate::jwt`]'s claims are,
            // and for that row's reason: the token is base64 and carries no
            // argument's `tainted` into any sink. The mark is on the element
            // because an `array<…>` has no cell of its own, and
            // `CoreTy::classification` reads only a contagious element through
            // the array, so this parameter is unclassified and what admits an
            // argument here is the declared element type and nothing else.
            params: &[
                CoreTy::Array(&CoreTy::Text(Qual::Neutral)),
                CoreTy::Shape(SIGNING),
            ],
            defaults: &[],
            return_ty: CoreTy::Str,
            symbol: "nvs_core_signature_sign",
            doc: Some(&SIGN_DOC),
        },
        CoreMethod {
            name: "verify",
            names: &["token", "keys"],
            // The token is neutral on the way in — it arrives from a URL or a
            // header and a `tainted` one is what this member is written to
            // receive — and the payload is `tainted` on the way out whatever
            // the token was, per `rule:security/verification-does-not-launder`.
            params: &[CoreTy::Text(Qual::Neutral), CoreTy::Array(&KEY)],
            defaults: &[],
            return_ty: CoreTy::Array(&SIGNED),
            symbol: "nvs_core_signature_verify",
            doc: Some(&VERIFY_DOC),
        },
    ],
    instance: &[],
    slots: &[],
    constants: &[],
};

/// `Core\Signature::sign`'s reference card — `rule:core-api/reference-card`.
const SIGN_DOC: MethodDoc = MethodDoc {
    short: "Signs `$payload` with the newest key in `$settings.keys` and returns a token. The \
            token contains the payload, and `Core\\Signature::verify` returns it again. The same \
            payload, keys and `until` always give the same token.",
    params: &[
        ParamDoc {
            name: "payload",
            desc: "The values to sign, each under a name. The order of the names is not signed. \
                   `verify` returns the values sorted by name.",
            shape: &[],
        },
        ParamDoc {
            name: "settings",
            desc: "The keys, and the time when the token stops being valid. You must write both.",
            shape: &[
                ShapeKeyDoc {
                    key: "keys",
                    ty: "array<secret bytes>",
                    desc: "The keys, newest first. `$keys[0]` signs the token. The older keys are \
                           there so that `verify` still accepts tokens made before you added a \
                           new key. A list of one key is `[$key]`.",
                },
                ShapeKeyDoc {
                    key: "until",
                    ty: "?Core\\Time\\Instant",
                    desc: "The time after which `verify` throws an error for this token. The time \
                           is part of the signed data, so nobody can change it. Write `null` for a \
                           token that never expires.",
                },
            ],
        },
    ],
    ret: "The token. It contains only `A-Z`, `a-z`, `0-9`, `-` and `_`, so you can put it in a URL \
          or a cookie without escaping it. It is about 4/3 × (payload + 40) characters long.",
    errors: &[ErrorDoc {
        error: "LogicError",
        desc: "`$settings.keys` is empty, or its first key is not 32 bytes long. \
               `Core\\Crypto::generateKey()` returns a key of the right length.",
    }],
};

/// `Core\Signature::verify`'s reference card — `rule:core-api/reference-card`.
const VERIFY_DOC: MethodDoc = MethodDoc {
    short: "Checks that `$token` was made by `Core\\Signature::sign` with a key in `$keys`, and \
            that it has not expired. It returns the signed payload, or throws an error. Every \
            value in the payload is `tainted` (treated as input from outside), because a \
            signature shows who made the data, not that the data is safe to use.",
    params: &[
        ParamDoc {
            name: "token",
            desc: "The token as your program received it, for example from a URL. A `tainted` \
                   string is allowed here.",
            shape: &[],
        },
        ParamDoc {
            name: "keys",
            desc: "The keys you gave `sign`, newest first. A token made with any key in this list \
                   is accepted. A token made with a key you removed from the list is not.",
            shape: &[],
        },
    ],
    ret: "The signed payload, sorted by name. Every value is a `tainted string`.",
    errors: &[
        ErrorDoc {
            error: "LogicError",
            desc: "`$keys` is empty, or one of its keys is not 32 bytes long.",
        },
        ErrorDoc {
            error: "RuntimeError",
            desc: "`$token` was not made with any key in `$keys`. This includes a changed token, \
                   text that is not a token, and a token for a signed URL. All of these give the \
                   same message, so an attacker learns nothing from it. An expired token also \
                   throws `RuntimeError`, with a different message that gives the time it \
                   expired.",
        },
    ],
};

/// The address of one of *this* module's symbols, or `None` for a symbol that
/// belongs to another domain. See [`crate::symbols`].
pub(crate) fn address(symbol: &str) -> Option<*const u8> {
    Some(match symbol {
        "nvs_core_signature_sign" => (nvs_core_signature_sign as *const ()).cast(),
        "nvs_core_signature_verify" => (nvs_core_signature_verify as *const ()).cast(),
        _ => return None,
    })
}

/// The `string` in slot `slot`.
///
/// # Errors
///
/// A [`Fault::fatal`] naming the member: the slot is a `string` in the row, so
/// another tag is a compiled-code bug rather than anything a program can write.
fn text_of<'a>(args: &'a [Value], slot: usize, member: &str) -> Result<&'a str, Fault> {
    args[slot].as_text().ok_or_else(|| {
        Fault::fatal(format!(
            "{NAME}::{member} expected a `string`, got tag {}",
            args[slot].tag_byte()
        ))
    })
}

/// `at` as the two parts [`Until`] holds.
///
/// `jiff` keeps a timestamp's second and its subsecond of the same sign, so an
/// instant before the epoch carries a negative nanosecond and [`Until::nano`]
/// holds a positive one. The borrow happens here, once, so the lifetime that
/// is signed and the clock it is compared against are normalized the same way
/// — two normalizations is the failure this whole module is written around.
fn until_at(at: jiff::Timestamp) -> Until {
    let (second, nano) = (at.as_second(), at.subsec_nanosecond());
    match nano < 0 {
        true => Until {
            second: second.saturating_sub(1),
            nano: nano.saturating_add(1_000_000_000).unsigned_abs(),
        },
        false => Until {
            second,
            nano: nano.unsigned_abs(),
        },
    }
}

/// The lifetime the `until` field in slot `slot` states, or `None` for the
/// written `null` that is the forever spelling.
///
/// # Errors
///
/// A [`Fault::fatal`] naming the member, for a slot that is neither: the field
/// is `?Core\Time\Instant` in the row, so nothing else reaches here.
pub(crate) fn until_of(args: &[Value], slot: usize, member: &str) -> Result<Option<Until>, Fault> {
    if args[slot].tag() == Some(Tag::Null) {
        return Ok(None);
    }
    Ok(Some(until_at(crate::time::instant_of(args, slot, member)?)))
}

/// Whether every entry of `payload` is a `string`, which is what the row
/// promises and what this class's own `sign` writes.
///
/// A document reaching here has already authenticated, so a value of another
/// kind means a holder of a live key minted one another way — the module doc's
/// *a payload value is text* section is why that is refused rather than
/// answered as an `array<tainted string>` that is not one.
fn all_text(payload: &Value) -> bool {
    let Some(raw) = payload.array_ptr() else {
        return false;
    };
    let array = crate::arr::borrowed(raw);
    let mut from = 0;
    while let Some(live) = array.next_slot(from) {
        from = live + 1;
        if array.value_at(live).and_then(|held| held.tag()) != Some(Tag::Str) {
            return false;
        }
    }
    true
}

/// Releases the payload a refusal is not going to hand to a program.
fn discard(payload: Value) {
    #[expect(
        unsafe_code,
        reason = "this frame owns exactly the reference `open` just produced, and answers a \
                  refusal instead of it"
    )]
    unsafe {
        payload.release();
    }
}

/// The one sentence every failed verification produces before the lifetime has
/// been looked at.
///
/// One function so the call sites cannot drift into several sentences, which
/// is the whole of what makes them indistinguishable
/// (`rule:core-api/one-refusal-except-expiry`).
fn refused() -> Fault {
    Fault::thrown(format!(
        "{NAME}::verify(): $token is not a signature this ring made. Every way of not being one \
         — text that is not base64, a token too short to hold a tag, a tag that authenticates \
         under no key in $keys, a document this runtime did not write, and a token minted for \
         another door — is this one sentence, so a forgery says nothing about which half of it \
         failed."
    ))
}

/// Refuses where the lifetime `until` has passed on `ctx`'s clock, naming the
/// member `who` — spelled `Core\Class::member`, and the parentheses are added
/// here.
///
/// **The one distinguishable refusal, written once for every door.** It is
/// safe to name only because it is reached last: [`open`] has already checked
/// the tag, so nobody but the holder of a genuinely signed token gets this
/// sentence rather than the caller's own one
/// (`rule:core-api/one-refusal-except-expiry`). A second door writing its own
/// wording is how "expired" and "not authentic" would come to be told apart at
/// one door and not at another.
///
/// # Errors
///
/// The `RuntimeError` above, and a [`Fault::fatal`] for a fixed clock outside
/// the range a [`jiff::Timestamp`] holds.
pub(crate) fn judge(ctx: &Ctx, until: Option<Until>, who: &str) -> Result<(), Fault> {
    let Some(until) = until else {
        return Ok(());
    };
    let now = crate::time::wall_clock(ctx).map(until_at).ok_or_else(|| {
        Fault::fatal(format!(
            "{who} found a fixed clock outside the representable range"
        ))
    })?;
    if (now.second, now.nano) >= (until.second, until.nano) {
        return Err(Fault::thrown(format!(
            "{who}(): the signature expired at {}.{:09}, and it is now {}.{:09}. This is the one \
             refusal with its own sentence: it is reached only after the tag has been checked, so \
             nobody but the holder of a real token ever sees it.",
            until.second, until.nano, now.second, now.nano
        )));
    }
    Ok(())
}

/// What [`confirm`] found: a token authentic over the payload the caller
/// already holds, and the lifetime it stated, or nothing.
///
/// Two variants rather than an `Option<Option<Until>>`, because the outer
/// question and the inner one are different questions and a reader should not
/// have to count the layers.
pub(crate) enum Confirmed {
    /// Authentic under some key in the ring, minted for the door that asked,
    /// taken over this very payload, and stating this lifetime — which is
    /// [`judge`]'s to rule on and nobody else's.
    Signed(Option<Until>),
    /// None of those things, folded into one answer for
    /// `rule:core-api/one-refusal-except-expiry`'s reason.
    Refused,
}

/// The lifetime `token` states, if it is a signature this `ring` made over
/// **this** `payload` for this `domain`.
///
/// The door a payload is *derived* from rather than carried in — `$uri->sign`
/// signs the URL, and the URL travels instead of the payload — needs a
/// question [`open`] cannot answer on its own: a token that authenticates
/// proves only that *some* payload was signed, and lifting a live token from
/// one URL onto another is the whole of the attack. So the caller rebuilds the
/// payload from what it holds and this compares the two.
///
/// **The comparison is the codec**, not a walk written beside it: two payloads
/// are the same payload exactly when they write the same document, which is
/// the property the module doc's first section is about. A structural compare
/// would be a second definition of equality, free to drift from the one the
/// signature is taken over. The two documents are public — a URL and a token
/// the holder already has — so the comparison is an ordinary one, and the tag
/// underneath it is still the constant-time one [`open`] takes.
///
/// # Errors
///
/// [`open`]'s, and [`document`]'s for a payload the caller cannot sign — which
/// for a derived payload says something about the caller's own value and
/// nothing about the token.
pub(crate) fn confirm(
    token: &str,
    domain: Domain,
    payload: &Value,
    ring: &NvsArray,
    who: &str,
    root: &str,
) -> Result<Confirmed, Fault> {
    let Some((until, signed)) = open(token, domain, ring, who)? else {
        return Ok(Confirmed::Refused);
    };
    // The lifetime is the one the *token* stated, because it is inside the
    // region the tag covers: rebuilding with any other one would be comparing
    // against a document nobody signed.
    let theirs = document(domain, until, &signed, who, root);
    discard(signed);
    let mine = document(domain, until, payload, who, root)?;
    match theirs? == mine {
        true => Ok(Confirmed::Signed(until)),
        false => Ok(Confirmed::Refused),
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Signature::sign(array<string> $payload, {keys: array<secret bytes>, until: ?Core\Time\Instant}): string`
    /// — the write half of `rule:security/protocol-roster`'s fifth entry,
    /// replacing the `hash_hmac` over an assembled query string that every
    /// signed-URL helper in every framework grows its own slightly different
    /// copy of.
    ///
    /// Thin on purpose: everything that decides what a signature *is* — the
    /// canonical form, the domain byte, the lifetime inside the signed region,
    /// the token's envelope — is [`mint`]'s, so the two doors that land after
    /// this one are the same three lines with another [`Domain`].
    fn nvs_core_signature_sign(_ctx, args: [3]) {
        let ring = crate::keyring::borrow(args, KEYS_ARG, SIGN)?;
        let until = until_of(args, UNTIL_ARG, "sign")?;
        let token = mint(
            Domain::Payload,
            until,
            &args[PAYLOAD_ARG],
            &ring,
            SIGN,
            "$payload",
        )?;
        Ok(Value::str(NvsStr::new(token.as_bytes())))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Signature::verify(string $token, array<secret bytes> $keys): array<tainted string>`
    /// — the read half, and the one place the lifetime that rode inside the
    /// signed bytes is judged.
    ///
    /// The order is load-bearing. The tag is checked before a single field is
    /// read, by [`open`]; the domain is checked there too, so this door cannot
    /// forget it; and the expiry is checked last, which is what makes it safe
    /// to give it a sentence of its own — only the holder of a genuinely
    /// signed token ever reaches it
    /// (`rule:core-api/one-refusal-except-expiry`).
    fn nvs_core_signature_verify(ctx, args: [2]) {
        let token = text_of(args, 0, "verify")?;
        let ring = crate::keyring::borrow(args, 1, VERIFY)?;
        let Some((until, payload)) = open(token, Domain::Payload, &ring, VERIFY)? else {
            return Err(refused());
        };

        if !all_text(&payload) {
            discard(payload);
            return Err(refused());
        }

        if let Err(expired) = judge(ctx, until, VERIFY) {
            discard(payload);
            return Err(expired);
        }

        Ok(payload)
    }
}

/// The document format's own version, first octet of every document and inside
/// the signed region.
///
/// It is *not* a negotiation: a reader accepts this version and no other. What
/// it buys is that a future format change cannot be made to look like the
/// current one by an old token, and that the failure is a refusal rather than
/// a misreading.
const VERSION: u8 = 1;

/// The deepest nesting a payload may carry, counted the way
/// [`crate::json::DEFAULT_MAX_DEPTH`] counts: a scalar payload is depth 1, and
/// `[[1]]` is depth 3.
///
/// Far below JSON's, and deliberately: a document is what
/// [`crate::json`] is for, and a signed payload is a claim set. The number
/// bounds the native stack both walks below recurse on, which is the reason it
/// exists at all.
pub(crate) const MAX_DEPTH: u32 = 32;

/// The tag of each type, on the wire.
///
/// Its own numbering rather than [`Tag`]'s, because these octets are a **wire
/// format** — a token minted today is read back after the runtime's internal
/// discriminants have been renumbered, and nothing stops that renumbering
/// except this module not depending on it.
mod tags {
    pub(super) const NULL: u8 = 0x00;
    pub(super) const FALSE: u8 = 0x01;
    pub(super) const TRUE: u8 = 0x02;
    /// An `int`, as eight big-endian octets of two's complement.
    pub(super) const INT: u8 = 0x03;
    /// A `uint`, as eight big-endian octets.
    pub(super) const UINT: u8 = 0x04;
    /// A `float`, as the eight big-endian octets of its IEEE-754 bits — the
    /// bits and not a rendering, so the round trip is exact and `-0.0` stays
    /// itself.
    pub(super) const FLOAT: u8 = 0x05;
    /// A `decimal`, as the sixteen big-endian octets of
    /// `Decimal::reduced().to_bits()`.
    pub(super) const DECIMAL: u8 = 0x06;
    /// A `string`: a length varint, then that many octets of UTF-8.
    pub(super) const STR: u8 = 0x07;
    /// A `bytes`: a length varint, then that many octets.
    pub(super) const BYTES: u8 = 0x08;
    /// An array: a count varint, then that many key-then-value pairs, the keys
    /// in ascending order of their own encoding.
    pub(super) const ARRAY: u8 = 0x09;
}

/// The lifetime field's two spellings: absent, and present.
mod lifetime {
    pub(super) const FOREVER: u8 = 0x00;
    pub(super) const UNTIL: u8 = 0x01;
}

/// Which door a document was made for — the domain separation the module doc's
/// own section explains.
///
/// The octet is part of the signed region, so a value here is a wire constant:
/// a variant's number is never reused for another door.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub(crate) enum Domain {
    /// `Core\Signature::sign` — a payload map the caller wrote.
    Payload = 1,
    /// `$uri->sign` — one URL in the normalized form `Core\Uri::compareTo`
    /// defines.
    Uri = 2,
    /// `Core\Router::urlSigned` — a route's name and its typed parameters,
    /// which is what survives a remount when a path does not.
    Route = 3,
}

impl Domain {
    /// The octet this domain is written as.
    const fn byte(self) -> u8 {
        self as u8
    }

    /// The domain `byte` names, or `None` for an octet no door writes.
    const fn from_byte(byte: u8) -> Option<Self> {
        match byte {
            1 => Some(Self::Payload),
            2 => Some(Self::Uri),
            3 => Some(Self::Route),
            _ => None,
        }
    }
}

/// The instant a signature stops being valid, in `Core\Time\Instant`'s own two
/// parts.
///
/// Two parts rather than one count because that is what an `Instant` holds
/// (`crate::time`'s `INSTANT` has an epoch-second slot and a subsecond one),
/// so nothing converts on the way in or out and a lifetime that travels in a
/// token is the lifetime that was written. `rule:core-api/a-lifetime-is-written`
/// makes the field required and `null` — [`Option::None`] here — the forever
/// spelling.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Until {
    /// Whole seconds since the Unix epoch, negative before it.
    pub(crate) second: i64,
    /// The subsecond part, `0..1_000_000_000`. A reader refuses anything
    /// larger, so a decoded `Until` is always one an `Instant` can hold.
    pub(crate) nano: u32,
}

/// One nanosecond past the largest [`Until::nano`] a reader accepts.
const NANOS_PER_SECOND: u32 = 1_000_000_000;

/// The signed region for `payload`, under `domain` and expiring at `until`.
///
/// This is the whole of what a signature is taken over, and the whole of what
/// travels in a token: `verify` authenticates these octets and only then reads
/// them back with [`read_document`], so no route exists in which something is
/// parsed before its tag has been checked.
///
/// `who` names the member for a refusal, spelled `Core\Class::member`, and
/// `root` is what the payload is called at the call site — `$payload` — so a
/// message can name the path to the value it refused.
///
/// # Errors
///
/// A `LogicError` for a value with no canonical form — an object, a callable, a
/// resource — and for a payload nested past [`MAX_DEPTH`], both naming the
/// path.
pub(crate) fn document(
    domain: Domain,
    until: Option<Until>,
    payload: &Value,
    who: &str,
    root: &str,
) -> Result<Vec<u8>, Fault> {
    let mut out = vec![VERSION, domain.byte()];
    match until {
        None => out.push(lifetime::FOREVER),
        Some(until) => {
            out.push(lifetime::UNTIL);
            out.extend_from_slice(&until.second.to_be_bytes());
            out.extend_from_slice(&until.nano.to_be_bytes());
        }
    }
    let mut path = String::from(root);
    write_value(&mut out, payload, 1, &mut path, who)?;
    Ok(out)
}

/// The domain, the lifetime and the payload a document holds, or `None` for
/// octets that are not one.
///
/// `None` covers every malformed shape at once — a version this runtime does
/// not write, a truncated body, a length that runs past the end, a key order
/// that is not the canonical one, an octet left over at the end. The caller
/// folds all of it into its own one refusal
/// (`rule:core-api/one-refusal-except-expiry`), because a reader that says
/// *which* part failed is a reader telling a forger which half landed.
///
/// **Strict on purpose.** A document that authenticates but does not re-encode
/// to itself would mean two spellings of one payload, which is the property
/// the whole module exists to deny, so a non-minimal length or an unsorted key
/// is refused here even though the tag has already been checked.
pub(crate) fn read_document(bytes: &[u8]) -> Option<(Domain, Option<Until>, Value)> {
    let mut reader = Reader { bytes, at: 0 };
    if reader.octet()? != VERSION {
        return None;
    }
    let domain = Domain::from_byte(reader.octet()?)?;
    let until = match reader.octet()? {
        lifetime::FOREVER => None,
        lifetime::UNTIL => {
            let second = i64::from_be_bytes(reader.take(8)?.try_into().ok()?);
            let nano = u32::from_be_bytes(reader.take(4)?.try_into().ok()?);
            if nano >= NANOS_PER_SECOND {
                return None;
            }
            Some(Until { second, nano })
        }
        _ => return None,
    };
    let payload = reader.value(1)?;
    if reader.at == reader.bytes.len() {
        return Some((domain, until, payload));
    }
    // An octet past the end is a document that would re-encode to something
    // shorter, so it is not this one. The payload built above owns references
    // that no caller is going to take.
    #[expect(
        unsafe_code,
        reason = "this frame owns exactly the reference `Reader::value` just produced"
    )]
    unsafe {
        payload.release();
    }
    None
}

// ============================================================================
// The token
// ============================================================================

/// The tag's length in octets — HMAC-SHA256's output, whole. A truncated tag
/// would shorten a token by a few characters and spend security to do it.
const TAG_LEN: usize = 32;

/// A token for `payload`, under the newest key in `ring`.
///
/// The door hands its domain, its lifetime and its payload, and gets back the
/// text that travels: **nothing outside this module assembles a token**, which
/// is the same argument [`document`] makes one layer down. `who` names the
/// member for a refusal and `root` is what the payload is called at the call
/// site.
///
/// # Errors
///
/// Everything [`document`] refuses a payload for, and everything
/// [`crate::keyring`] refuses a ring for.
pub(crate) fn mint(
    domain: Domain,
    until: Option<Until>,
    payload: &Value,
    ring: &NvsArray,
    who: &str,
    root: &str,
) -> Result<String, Fault> {
    let signed = document(domain, until, payload, who, root)?;
    let (slot, held) = crate::keyring::newest(ring);
    let key = crate::keyring::key_at(&held, slot, who)?;

    let mut token = Vec::with_capacity(TAG_LEN + signed.len());
    token.extend_from_slice(&crate::hash::hmac_sha256(key, &signed));
    token.extend_from_slice(&signed);
    Ok(URL_SAFE_NO_PAD.encode(&token))
}

/// The lifetime and payload `token` carries, authenticated under some key in
/// `ring` and minted for `domain` — or `None` for a token that is none of
/// those things.
///
/// **One `None` for every way of not being authentic**, per
/// `rule:core-api/one-refusal-except-expiry`: text that is not base64, a token
/// too short to hold a tag, a tag that does not authenticate under any key in
/// the ring, a body that is not a document this runtime writes, and a document
/// minted for another door. Which one it was is exactly what a forger is
/// probing for, so the caller has one refusal to write and no way to write a
/// second by accident.
///
/// The domain is checked here rather than by the caller for the same reason
/// the tag is: a door that has to remember a check is a door that can forget
/// one, and the whole point of the domain octet is that a payload token does
/// not open as a URL signature.
///
/// The expiry is *not* checked here. It is the one distinguishable refusal, so
/// it belongs to the member that can name it, and this answers the [`Until`]
/// it read for that member to judge.
///
/// # Errors
///
/// Only what [`crate::keyring`] refuses a ring for — a program bug, which is
/// not a way of being unauthentic and is raised whatever the token says.
pub(crate) fn open(
    token: &str,
    domain: Domain,
    ring: &NvsArray,
    who: &str,
) -> Result<Option<(Option<Until>, Value)>, Fault> {
    // A token that is not base64 is not a *different* kind of unauthentic, so
    // the ring is still walked and its keys are still checked for shape — the
    // refusal is the caller's one sentence either way.
    let raw = URL_SAFE_NO_PAD.decode(token).ok();
    let split = raw
        .as_deref()
        .filter(|raw| raw.len() >= TAG_LEN)
        .map(|raw| raw.split_at(TAG_LEN));

    for (slot, held) in crate::keyring::entries(ring) {
        let key = crate::keyring::key_at(&held, slot, who)?;
        if let Some((tag, signed)) = split
            && bool::from(crate::hash::hmac_sha256(key, signed).ct_eq(tag))
        {
            return Ok(for_domain(signed, domain));
        }
    }
    Ok(None)
}

/// The lifetime and payload `signed` holds, once its tag has authenticated,
/// and only if it was minted for `domain`.
///
/// A document for another door is dropped here rather than answered, which is
/// the whole of the domain byte's job: the payload it decoded to is released
/// on the way out, because nothing is going to hand it to a program.
fn for_domain(signed: &[u8], domain: Domain) -> Option<(Option<Until>, Value)> {
    let (minted_for, until, payload) = read_document(signed)?;
    if minted_for == domain {
        return Some((until, payload));
    }
    #[expect(
        unsafe_code,
        reason = "this frame owns exactly the reference `read_document` just produced"
    )]
    unsafe {
        payload.release();
    }
    None
}

// ============================================================================
// Writing
// ============================================================================

/// Writes `value` at `depth`, with `path` naming where it sits in the payload.
///
/// # Errors
///
/// The `LogicError`s [`document`] documents.
fn write_value(
    out: &mut Vec<u8>,
    value: &Value,
    depth: u32,
    path: &mut String,
    who: &str,
) -> Result<(), Fault> {
    if depth > MAX_DEPTH {
        return Err(Fault::thrown_as(
            ThrownClass::Logic,
            format!(
                "{who}(): {path} nests deeper than {MAX_DEPTH}, which is as deep as a signed \
                 payload goes — a document belongs in one value, as text or as JSON, rather \
                 than as a tree of its own"
            ),
        ));
    }
    match value.tag() {
        Some(Tag::Null) => out.push(tags::NULL),
        Some(Tag::Bool) => out.push(if value.as_bool().unwrap_or(false) {
            tags::TRUE
        } else {
            tags::FALSE
        }),
        Some(Tag::Int) => {
            out.push(tags::INT);
            out.extend_from_slice(&value.as_int().unwrap_or(0).to_be_bytes());
        }
        Some(Tag::Uint) => {
            out.push(tags::UINT);
            out.extend_from_slice(&value.as_uint().unwrap_or(0).to_be_bytes());
        }
        Some(Tag::Float) => {
            out.push(tags::FLOAT);
            out.extend_from_slice(&value.as_float().unwrap_or(0.0).to_bits().to_be_bytes());
        }
        Some(Tag::Decimal) => {
            let held = value
                .as_decimal()
                .unwrap_or_else(Decimal::zero)
                .reduced()
                .to_bits();
            out.push(tags::DECIMAL);
            out.extend_from_slice(&held.to_be_bytes());
        }
        Some(Tag::Str) => {
            out.push(tags::STR);
            write_run(out, value.as_str_bytes().unwrap_or_default());
        }
        Some(Tag::Bytes) => {
            out.push(tags::BYTES);
            write_run(out, value.as_bytes().unwrap_or_default());
        }
        Some(Tag::Array) => {
            let raw = value.array_ptr().expect("a `Tag::Array` value is an array");
            write_array(out, &crate::arr::borrowed(raw), depth, path, who)?;
        }
        _ => {
            return Err(Fault::thrown_as(
                ThrownClass::Logic,
                format!(
                    "{who}(): {path} is {}, which has no canonical form to sign — a signed \
                     payload holds null, bool, int, uint, float, decimal, string, bytes and \
                     arrays of those, and anything else is signed by writing down what about \
                     it matters",
                    value.tag().map_or("a value of no type", Tag::describe)
                ),
            ));
        }
    }
    Ok(())
}

/// Writes `array`'s entries, keys first and in ascending order of their own
/// encoding.
///
/// The order is the array's *canonical* one and never its insertion order: the
/// module doc's own section is the home of why, and of what a caller gives up.
///
/// # Errors
///
/// The `LogicError`s [`document`] documents, for any entry.
fn write_array(
    out: &mut Vec<u8>,
    array: &NvsArray,
    depth: u32,
    path: &mut String,
    who: &str,
) -> Result<(), Fault> {
    let mut entries: Vec<(Vec<u8>, usize)> = Vec::with_capacity(array.count());
    let mut slot = 0;
    while let Some(live) = array.next_slot(slot) {
        slot = live + 1;
        let key = array.slot_key(live).expect("a live slot has a key");
        entries.push((key.to_str().as_bytes().to_vec(), live));
    }
    // An array holds each key once, so ordering by the key alone is total and
    // the sort has nothing to break ties over.
    entries.sort_unstable_by(|left, right| left.0.cmp(&right.0));

    out.push(tags::ARRAY);
    write_varint(
        out,
        u64::try_from(entries.len()).expect("an array's length fits a `u64`"),
    );
    let root = path.len();
    for (key, live) in entries {
        out.push(tags::STR);
        write_run(out, &key);
        let held = array.value_at(live).expect("a live slot holds a value");
        path.push('[');
        path.push_str(&String::from_utf8_lossy(&key));
        path.push(']');
        write_value(out, &held, depth + 1, path, who)?;
        path.truncate(root);
    }
    Ok(())
}

/// Writes a length varint and then `run`.
fn write_run(out: &mut Vec<u8>, run: &[u8]) {
    write_varint(
        out,
        u64::try_from(run.len()).expect("a run's length fits a `u64`"),
    );
    out.extend_from_slice(run);
}

/// Writes `value` as an unsigned LEB128, which is minimal by construction —
/// [`Reader::varint`] accepts nothing else.
fn write_varint(out: &mut Vec<u8>, mut value: u64) {
    loop {
        let octet = u8::try_from(value & 0x7f).expect("seven bits fit an octet");
        value >>= 7;
        if value == 0 {
            out.push(octet);
            return;
        }
        out.push(octet | 0x80);
    }
}

// ============================================================================
// Reading
// ============================================================================

/// A cursor over a document, answering `None` for every malformed shape.
struct Reader<'a> {
    /// The document.
    bytes: &'a [u8],
    /// How much of it has been read.
    at: usize,
}

impl Reader<'_> {
    /// The next octet.
    fn octet(&mut self) -> Option<u8> {
        Some(self.take(1)?[0])
    }

    /// The next `len` octets.
    fn take(&mut self, len: usize) -> Option<&[u8]> {
        let end = self.at.checked_add(len)?;
        let run = self.bytes.get(self.at..end)?;
        self.at = end;
        Some(run)
    }

    /// The next unsigned LEB128, refusing any spelling but the minimal one.
    ///
    /// A padded varint would be a second encoding of a document that already
    /// has one, which is the property this module denies; refusing it here is
    /// what makes a document that authenticates re-encode to itself.
    fn varint(&mut self) -> Option<u64> {
        let mut value = 0_u64;
        let mut shift = 0_u32;
        loop {
            let octet = self.octet()?;
            if shift >= 64 || (shift == 63 && octet > 1) {
                return None;
            }
            value |= u64::from(octet & 0x7f) << shift;
            if octet & 0x80 == 0 {
                // A final octet of zero past the first is padding, and so is a
                // continuation that carried nothing.
                return (shift == 0 || octet != 0).then_some(value);
            }
            shift += 7;
        }
    }

    /// The next length varint's worth of octets.
    fn run(&mut self) -> Option<&[u8]> {
        let len = usize::try_from(self.varint()?).ok()?;
        self.take(len)
    }

    /// The next value, at `depth`.
    ///
    /// The caller owns whatever references the answer holds; every failure
    /// below releases what it had built before answering `None`.
    fn value(&mut self, depth: u32) -> Option<Value> {
        if depth > MAX_DEPTH {
            return None;
        }
        Some(match self.octet()? {
            tags::NULL => Value::null(),
            tags::FALSE => Value::bool(false),
            tags::TRUE => Value::bool(true),
            tags::INT => Value::int(i64::from_be_bytes(self.take(8)?.try_into().ok()?)),
            tags::UINT => Value::uint(u64::from_be_bytes(self.take(8)?.try_into().ok()?)),
            tags::FLOAT => Value::float(f64::from_bits(u64::from_be_bytes(
                self.take(8)?.try_into().ok()?,
            ))),
            tags::DECIMAL => {
                let bits = u128::from_be_bytes(self.take(16)?.try_into().ok()?);
                let held = Decimal::from_bits(bits)?;
                // A `decimal` is written reduced, so one that is not is a
                // second spelling of a value that already has one.
                if held.to_bits() != held.reduced().to_bits() {
                    return None;
                }
                Value::decimal(held)
            }
            tags::STR => Value::str(NvsStr::new(
                std::str::from_utf8(self.run()?).ok()?.as_bytes(),
            )),
            tags::BYTES => Value::bytes(NvsStr::new(self.run()?)),
            tags::ARRAY => Value::array(self.array(depth)?),
            _ => return None,
        })
    }

    /// The next array's entries, refusing a key order that is not the one
    /// [`write_array`] writes.
    fn array(&mut self, depth: u32) -> Option<NvsArray> {
        let count = usize::try_from(self.varint()?).ok()?;
        let mut array = NvsArray::new();
        let mut previous: Option<Vec<u8>> = None;
        for _ in 0..count {
            let key = self.key()?;
            if previous.is_some_and(|before| before >= key) {
                return None;
            }
            // A failure below drops `array`, whose own `Drop` releases every
            // value already stored in it.
            let held = self.value(depth + 1)?;
            crate::arr::store_at(&mut array, SlotKey::Str(NvsStr::new(&key)), held);
            previous = Some(key);
        }
        Some(array)
    }

    /// The next array key, which is text and whose octets are its order.
    fn key(&mut self) -> Option<Vec<u8>> {
        (self.octet()? == tags::STR)
            .then(|| self.run().map(<[u8]>::to_vec))
            .flatten()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A payload map, built in the order given.
    fn payload(entries: &[(&str, Value)]) -> Value {
        let mut array = NvsArray::new();
        for (key, held) in entries {
            array.set(NvsStr::new(key.as_bytes()), *held);
        }
        Value::array(array)
    }

    /// Releases what a test built, so the crate's allocation gate sees a
    /// balanced run.
    fn dropped(value: Value) {
        #[expect(
            unsafe_code,
            reason = "a test frame owns exactly the reference it built"
        )]
        unsafe {
            value.release();
        }
    }

    /// The document for a payload, under the domain and lifetime a test names.
    fn signed(payload: &Value, until: Option<Until>) -> Vec<u8> {
        document(
            Domain::Payload,
            until,
            payload,
            "Core\\Signature::sign",
            "$payload",
        )
        .expect("a payload of scalars encodes")
    }

    /// The document for a map written inline, which the helper builds, signs
    /// and releases — every reference in `entries` belongs to the map.
    fn signed_map(entries: &[(&str, Value)], until: Option<Until>) -> Vec<u8> {
        let held = payload(entries);
        let wire = signed(&held, until);
        dropped(held);
        wire
    }

    /// Stage 2's first canonicalization check: the order a program writes a
    /// map in is not part of what it signs.
    ///
    /// Asserted over the tag as well as over the document, because the
    /// property that matters to a caller is that the two *sign* alike, and a
    /// document compared to itself would still hold if the tag were taken over
    /// something else.
    #[test]
    fn the_canonical_encoding_sorts_keys_so_two_orderings_sign_identically() {
        let one = payload(&[
            ("user", Value::int(7)),
            ("action", Value::int(2)),
            ("scope", Value::int(9)),
        ]);
        let other = payload(&[
            ("scope", Value::int(9)),
            ("user", Value::int(7)),
            ("action", Value::int(2)),
        ]);

        let left = signed(&one, None);
        let right = signed(&other, None);
        assert_eq!(left, right, "the same pairs in two orders are one document");
        assert_eq!(
            crate::hash::hmac_sha256(&[3_u8; 32], &left),
            crate::hash::hmac_sha256(&[3_u8; 32], &right),
            "so the two sign identically, which is what a query string needs"
        );

        // And the order they come back in is the canonical one, not either
        // one that went in.
        let (_, _, back) = read_document(&left).expect("its own document reads");
        let raw = back.array_ptr().expect("a payload map is an array");
        assert_eq!(
            crate::arr::borrowed(raw).keys(),
            vec![b"action".to_vec(), b"scope".to_vec(), b"user".to_vec()],
            "a verified payload is in canonical order and not in the signer's"
        );

        dropped(back);
        dropped(one);
        dropped(other);
    }

    /// The other half of a canonical form: a value carries its type, so no two
    /// payloads a program can tell apart sign alike.
    ///
    /// Written over the pair every ad-hoc `implode`-and-hash helper conflates
    /// — the number one and the text of it — and over the same pair in key
    /// position, which is the half a value-only encoding still gets wrong.
    #[test]
    fn an_int_one_and_a_string_one_encode_to_different_bytes() {
        let numeric = payload(&[("id", Value::int(1))]);
        let textual = payload(&[("id", Value::str(NvsStr::new(b"1")))]);
        assert_ne!(
            signed(&numeric, None),
            signed(&textual, None),
            "`1` and `\"1\"` are two payloads and sign as two"
        );

        // The same question of a *key* has the opposite answer, and it is the
        // language's rather than this module's: a key is a `string` whichever
        // shape the array is in, so a position and the text of it are one key
        // and sign as one. A list that has degraded to the hash form must not
        // stop verifying, which is what this pins.
        let mut indexed = NvsArray::new();
        indexed.set_index(1, Value::bool(true));
        let indexed = Value::array(indexed);
        let named = payload(&[("1", Value::bool(true))]);
        assert_eq!(
            signed(&indexed, None),
            signed(&named, None),
            "a positional key and the text of its own number are one key"
        );

        // Nor does any other pair of neighbouring types collide, including
        // the two that hold the same octets and the two that hold none.
        let builders: [fn() -> Value; 7] = [
            Value::null,
            || Value::bool(false),
            || Value::int(1),
            || Value::uint(1),
            || Value::float(1.0),
            || Value::str(NvsStr::new(b"")),
            || Value::bytes(NvsStr::new(b"")),
        ];
        let mut seen: Vec<Vec<u8>> = builders
            .iter()
            .map(|build| signed_map(&[("v", build())], None))
            .collect();
        let total = seen.len();
        seen.sort_unstable();
        seen.dedup();
        assert_eq!(seen.len(), total, "each type encodes to bytes of its own");

        dropped(numeric);
        dropped(textual);
        dropped(indexed);
        dropped(named);
    }

    /// Every shape a document takes survives the round trip, including the
    /// two the wire format spends bytes to keep exact.
    #[test]
    fn a_document_reads_back_as_the_payload_that_was_written() {
        let mut nested = NvsArray::new();
        nested.append(Value::float(-0.0));
        nested.append(Value::bytes(NvsStr::new(&[0_u8, 0xff, 0x80])));
        let held = payload(&[
            ("map", Value::array(nested)),
            ("text", Value::str(NvsStr::new("héllo".as_bytes()))),
            ("big", Value::uint(u64::MAX)),
            ("low", Value::int(i64::MIN)),
            ("none", Value::null()),
        ]);
        let until = Until {
            second: 1_800_000_000,
            nano: 123_456_789,
        };

        let wire = signed(&held, Some(until));
        let (domain, read_until, back) = read_document(&wire).expect("its own document reads");
        assert_eq!(domain, Domain::Payload);
        assert_eq!(read_until, Some(until), "the lifetime rides inside");
        assert_eq!(
            signed(&back, Some(until)),
            wire,
            "and what comes back re-encodes to the document it came from"
        );

        dropped(back);
        dropped(held);
    }

    /// A document is refused unless it is *the* encoding of what it holds —
    /// the property that makes "signed" and "canonical" the same word.
    #[test]
    fn a_document_that_would_re_encode_to_something_else_is_refused() {
        let held = payload(&[("a", Value::int(1)), ("b", Value::int(2))]);
        let wire = signed(&held, None);
        assert!(read_document(&wire).is_some(), "the canonical one reads");

        let mut trailing = wire.clone();
        trailing.push(0);
        assert!(read_document(&trailing).is_none(), "an octet past the end");

        let mut short = wire.clone();
        short.pop();
        assert!(read_document(&short).is_none(), "a truncated one");

        let mut version = wire.clone();
        version[0] = VERSION + 1;
        assert!(
            read_document(&version).is_none(),
            "a version we never wrote"
        );

        let mut domain = wire.clone();
        domain[1] = Domain::Uri.byte();
        let (read, _, other) = read_document(&domain).expect("another door's is still a document");
        assert_eq!(read, Domain::Uri, "and the door it names is the one read");
        dropped(other);

        // The two spellings that hold the same payload and are not canonical:
        // the keys the other way round, and a padded length varint.
        assert_eq!(
            signed_map(&[("b", Value::int(2)), ("a", Value::int(1))], None),
            wire,
            "the writer sorts, so the order a caller wrote is already gone"
        );
        // Both entries are the same size — a one-octet key and an `int` — so
        // putting the second first is one rotation past the five octets of
        // version, domain, lifetime, array tag and count.
        const HEADER: usize = 5;
        let mut swapped = wire.clone();
        swapped[HEADER..].rotate_left((wire.len() - HEADER) / 2);
        assert_ne!(swapped, wire, "the rotation moved something");
        assert!(
            read_document(&swapped).is_none(),
            "and a hand-swapped pair is not a document this writer could have written"
        );

        let mut padded = vec![
            VERSION,
            Domain::Payload.byte(),
            lifetime::FOREVER,
            tags::ARRAY,
        ];
        write_varint(&mut padded, 1);
        // The key `"a"`, whose length is written `0x81 0x00` — one, with a
        // continuation octet carrying nothing.
        padded.extend_from_slice(&[tags::STR, 0x81, 0x00, b'a', tags::NULL]);
        assert!(read_document(&padded).is_none(), "a padded length varint");
        let mut minimal = vec![
            VERSION,
            Domain::Payload.byte(),
            lifetime::FOREVER,
            tags::ARRAY,
        ];
        write_varint(&mut minimal, 1);
        minimal.extend_from_slice(&[tags::STR, 0x01, b'a', tags::NULL]);
        let (_, _, one) = read_document(&minimal).expect("and the minimal one reads");
        dropped(one);

        dropped(held);
    }

    /// The refusals: a value with no canonical form, and a payload deeper than
    /// the stack budget both walks share.
    #[test]
    fn a_value_with_no_canonical_form_and_a_payload_too_deep_are_refused() {
        let mut deep = NvsArray::new();
        deep.append(Value::int(1));
        let mut deep = Value::array(deep);
        for _ in 0..MAX_DEPTH {
            let mut outer = NvsArray::new();
            outer.append(deep);
            deep = Value::array(outer);
        }
        let refusal = document(
            Domain::Payload,
            None,
            &deep,
            "Core\\Signature::sign",
            "$payload",
        )
        .expect_err("past the ceiling");
        assert!(
            format!("{refusal:?}").contains("nests deeper"),
            "the depth ceiling names itself: {refusal:?}"
        );

        // And the reader will not build one either, whatever authenticated it.
        let mut wire = vec![VERSION, Domain::Payload.byte(), lifetime::FOREVER];
        for _ in 0..=MAX_DEPTH {
            wire.push(tags::ARRAY);
            write_varint(&mut wire, 1);
            wire.push(tags::INT);
            wire.extend_from_slice(&0_i64.to_be_bytes());
        }
        wire.push(tags::NULL);
        assert!(
            read_document(&wire).is_none(),
            "nor does the reader recurse"
        );

        dropped(deep);
    }

    // ========================================================================
    // The token
    //
    // `Core\Signature`'s two rows are a thin wrapper over [`mint`] and
    // [`open`] — reading the arguments, and turning `open`'s `None` into the
    // one refusal — and they land in the slice after this one. These assert
    // the construction rather than the registry, for the reason
    // [`crate::signed_cookie`]'s own test gives: what can go wrong is the
    // *layering*, and every bit of it is visible here without a compiler in
    // front of it.
    // ========================================================================

    use crate::keyring::tests::{borrowed as ring_borrowed, ring_of};

    /// A token for `payload` under `ring`, with no expiry.
    fn minted(payload: &Value, ring: &Value) -> String {
        minted_at(Domain::Payload, None, payload, ring)
    }

    /// A token for `payload` at `door`, stating `until`, under `ring` — the
    /// two axes a refusal is not allowed to tell apart.
    fn minted_at(door: Domain, until: Option<Until>, payload: &Value, ring: &Value) -> String {
        mint(door, until, payload, &ring_borrowed(ring), SIGN, "$payload")
            .expect("a payload of scalars mints")
    }

    /// What `token` opens as under `ring`, at the payload door.
    fn opened(token: &str, ring: &Value) -> Option<(Option<Until>, Value)> {
        open(
            token,
            Domain::Payload,
            &ring_borrowed(ring),
            r"Core\Signature::verify",
        )
        .expect("a ring of well-formed keys")
    }

    /// Stage 2's round trip: what comes back out of a token is the payload
    /// that went in, in its canonical form, with the lifetime that was
    /// written.
    ///
    /// Compared by re-signing rather than by walking the two maps, because
    /// canonical equality *is* the equality this module promises — two values
    /// that sign alike are the same payload here by definition.
    #[test]
    fn a_payload_round_trips_through_sign_and_verify_unchanged() {
        let keys = ring_of(&[&[1_u8; 32]]);
        let held = payload(&[
            ("id", Value::int(7)),
            ("role", Value::str(NvsStr::new(b"editor"))),
            ("all", Value::bool(true)),
        ]);
        let until = Some(Until {
            second: 1_700_000_000,
            nano: 250,
        });

        let token = mint(
            Domain::Payload,
            until,
            &held,
            &ring_borrowed(&keys),
            SIGN,
            "$payload",
        )
        .expect("a payload of scalars mints");
        let (read, back) = opened(&token, &keys).expect("its own token authenticates");

        assert_eq!(read, until, "the lifetime rides inside the signed bytes");
        assert_eq!(
            signed(&back, until),
            signed(&held, until),
            "and the payload is the one that was signed"
        );

        // One flipped octet of the token is not authentic, so the round trip
        // is authenticity rather than an encoding that happens to reverse.
        let mut tampered = token.clone().into_bytes();
        tampered[TAG_LEN + 4] ^= 0x01;
        assert!(
            opened(
                std::str::from_utf8(&tampered).expect("base64 stays ASCII"),
                &keys
            )
            .is_none(),
            "an altered token authenticates under nothing"
        );

        dropped(back);
        dropped(held);
        dropped(keys);
    }

    /// The token is text every position it travels in accepts as-is, which is
    /// what makes a signed URL a URL and a signed cookie a cookie.
    ///
    /// Asserted over a payload holding the octets that would break each of
    /// those positions — `+`, `/`, `=`, `&` and a space — because an encoder
    /// that leaked its input's shape would still look right on a payload of
    /// letters.
    #[test]
    fn the_token_is_unpadded_url_safe_base64_and_needs_no_further_escaping() {
        let keys = ring_of(&[&[2_u8; 32]]);
        for text in ["", "a", "+/=&? ", "\u{1f512}", &"x".repeat(97)] {
            let held = payload(&[("v", Value::str(NvsStr::new(text.as_bytes())))]);
            let token = minted(&held, &keys);
            assert!(
                token
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_'),
                "{token} carries only the URL-safe alphabet, and no padding"
            );
            dropped(held);
        }
        dropped(keys);
    }

    /// The ring is `Core\SignedCookie`'s, used the same way at this door:
    /// minting takes the newest key and nothing else, and opening walks from
    /// the newest and stops at the key that authenticates.
    ///
    /// The stop is what the retired key at the back makes visible: it is not a
    /// key at all, so a walk that ran past the key that authenticated would
    /// raise its `LogicError` instead of answering.
    #[test]
    fn sign_uses_the_newest_key_alone_and_verify_walks_the_whole_ring_in_order() {
        let newest = [3_u8; 32];
        let older = [4_u8; 32];
        let keys = ring_of(&[&newest, &older]);
        let held = payload(&[("id", Value::int(1))]);

        let token = minted(&held, &keys);
        let raw = URL_SAFE_NO_PAD.decode(&token).expect("its own encoding");
        let (tag, document) = raw.split_at(TAG_LEN);
        assert_eq!(
            tag,
            crate::hash::hmac_sha256(&newest, document),
            "the newest key is the one that signed"
        );
        assert_ne!(
            tag,
            crate::hash::hmac_sha256(&older, document),
            "and no other key in the ring did"
        );

        // A ring whose tail cannot key anything: reaching it is a `LogicError`,
        // so answering at all proves the walk stopped at the first key.
        let broken = ring_of(&[&newest, &[9_u8; 8]]);
        let (_, back) = opened(&token, &broken).expect("the newest key authenticates it");
        dropped(back);
        assert!(
            open(
                "not-a-token",
                Domain::Payload,
                &ring_borrowed(&broken),
                r"Core\Signature::verify"
            )
            .is_err(),
            "while a token that authenticates under nothing does reach the broken key"
        );

        dropped(broken);
        dropped(held);
        dropped(keys);
    }

    /// Rotation, as an operator performs it: a key prepended, and the token
    /// minted before the rotation still verifying while the tokens minted
    /// after it use the new key.
    #[test]
    fn a_token_under_a_rotated_out_key_still_verifies_while_new_tokens_use_the_newest() {
        let retired = [5_u8; 32];
        let fresh = [6_u8; 32];
        let before = ring_of(&[&retired]);
        let after = ring_of(&[&fresh, &retired]);
        let held = payload(&[("id", Value::int(2))]);

        let old_token = minted(&held, &before);
        let (_, back) = opened(&old_token, &after).expect("the retired key is still in the ring");
        dropped(back);

        let new_token = minted(&held, &after);
        assert_ne!(
            old_token, new_token,
            "a token minted after the rotation is signed by the new key"
        );
        assert!(
            opened(&new_token, &before).is_none(),
            "so the ring from before the rotation does not verify it"
        );

        dropped(held);
        dropped(before);
        dropped(after);
    }

    /// The other end of rotation: a key dropped off the tail retires the
    /// tokens it signed, which is the property that makes dropping one a
    /// revocation rather than a tidy-up.
    #[test]
    fn a_token_under_a_key_dropped_past_the_end_of_the_ring_fails() {
        let dropped_key = [7_u8; 32];
        let full = ring_of(&[&[8_u8; 32], &dropped_key]);
        let trimmed = ring_of(&[&[8_u8; 32]]);
        let held = payload(&[("id", Value::int(3))]);

        // Signed by the tail rather than by the head, which is the token an
        // application minted before the head existed.
        let document = signed(&held, None);
        let mut raw = Vec::from(crate::hash::hmac_sha256(&dropped_key, &document));
        raw.extend_from_slice(&document);
        let token = URL_SAFE_NO_PAD.encode(&raw);

        let (_, back) = opened(&token, &full).expect("the key is still in the ring");
        dropped(back);
        assert!(
            opened(&token, &trimmed).is_none(),
            "and dropping it off the tail retires every token it signed"
        );

        dropped(held);
        dropped(full);
        dropped(trimmed);
    }

    /// The domain byte, from the outside: one ring serves all three doors, so
    /// a token minted for a payload must not open as a URL signature however
    /// well it authenticates.
    #[test]
    fn a_token_minted_at_one_door_does_not_open_at_another() {
        let keys = ring_of(&[&[10_u8; 32]]);
        let held = payload(&[("id", Value::int(4))]);
        let token = minted(&held, &keys);

        for door in [Domain::Uri, Domain::Route] {
            assert!(
                open(
                    &token,
                    door,
                    &ring_borrowed(&keys),
                    r"Core\Signature::verify"
                )
                .expect("a ring of well-formed keys")
                .is_none(),
                "a payload token does not open at {door:?}"
            );
        }

        let (_, back) = opened(&token, &keys).expect("while its own door opens it");
        dropped(back);
        dropped(held);
        dropped(keys);
    }

    /// `rule:security/verification-does-not-launder` where it bites: a value
    /// `Core\Signature::verify` answers cannot be written into statement text.
    ///
    /// The refusal itself is `nvs_types`' — `admits_tainted_argument` refuses a
    /// qualified argument at a [`Qual::Sink`] parameter — and this crate cannot
    /// call it, so what is asserted here is the pair of *declarations* that
    /// refusal reads. Both halves are needed and neither is visible from the
    /// other: the payload's element has to carry the qualifier, and every
    /// statement-text position has to be the sink. A row that answered
    /// `CoreTy::Str` would look correct on its own line while quietly
    /// laundering every token a program verifies.
    ///
    /// Asked over the whole registry rather than of `Core\Db\Connection`
    /// alone, because the rule is about the *position*: a class registered
    /// tomorrow with a `$sql` of its own is exactly what this exists to catch.
    #[test]
    fn a_verified_payload_reaching_a_query_text_position_is_refused_as_tainted() {
        let verify = CLASS
            .members()
            .find(|method| method.name == "verify")
            .expect("`Core\\Signature` registers `verify`");
        let CoreTy::Array(element) = verify.return_ty else {
            panic!("`Core\\Signature::verify` answers an array of payload values");
        };
        assert!(
            matches!(element, CoreTy::TaintedStr),
            "`Core\\Signature::verify` answers an `array<{element:?}>`, and a payload value that \
             does not carry the qualifier reaches a sink with nothing to stop it"
        );

        // The other half of the same rule, which is what makes the qualifier
        // worth carrying: a payload this application sealed itself comes back
        // unqualified, so the two roster entries are asserted against each
        // other rather than each against a comment.
        let opened = crate::signed_cookie::CLASS
            .members()
            .find(|method| method.name == "open")
            .expect("`Core\\SignedCookie` registers `open`");
        assert!(
            matches!(opened.return_ty, CoreTy::Str),
            "`Core\\SignedCookie::open` no longer answers a plain `string`, and the asymmetry \
             this rule is made of has gone with it"
        );

        let mut positions = Vec::new();
        for class in crate::registry::CLASSES {
            for method in class.members() {
                if method.names.first() != Some(&"sql") {
                    continue;
                }
                positions.push(format!("{}::{}", class.name, method.name));
                assert_eq!(
                    method.params[0].classification(),
                    Some(Qual::Sink),
                    "{}::{}'s $sql is not a sink, so a verified payload would reach statement \
                     text however it was qualified",
                    class.name,
                    method.name
                );
            }
        }
        assert!(
            positions.len() >= 2,
            "no statement-text position was found, so the assertion above ran over nothing"
        );
    }

    /// A lifetime far from any clock a test could be run under, so that a case
    /// naming the second on either side of it says what it means.
    const UNTIL: Until = Until {
        second: 1_700_000_000,
        nano: 0,
    };

    /// The clause every expiry refusal carries and nothing else does.
    const EXPIRED: &str = "the signature expired at";

    /// The clause the one refusal carries, whichever way the token failed.
    const FORGED: &str = "$token is not a signature this ring made";

    /// `Core\Signature::verify($token, $keys)` on a clock fixed at `second`,
    /// as the payload it answered or the sentence it refused with.
    ///
    /// The whole member through `nvs_runtime::call` rather than [`open`] and
    /// [`judge`] apart, because what the three cases below assert is the
    /// *order* those two are reached in, which neither of them holds on its
    /// own.
    fn verified(second: i64, token: &str, ring: &Value) -> Result<Value, String> {
        let mut ctx = Ctx::buffered();
        ctx.set_fixed_clock(i128::from(second) * i128::from(NANOS_PER_SECOND));
        let carried = Value::str(NvsStr::new(token.as_bytes()));
        let answered = nvs_runtime::call(nvs_core_signature_verify, &mut ctx, &[carried, *ring]);
        let refusal = ctx.take_pending().map(std::borrow::Cow::into_owned);
        dropped(carried);
        match answered {
            Ok(payload) => Ok(payload),
            Err(_) => Err(refusal.expect("a refusal leaves its message on the context")),
        }
    }

    /// Stage 5's one distinguishable refusal, asserted on both sides of the
    /// bound it turns on: the same token under the same ring is a payload one
    /// second before its `until` and the expiry sentence one second after it
    /// (`rule:core-api/one-refusal-except-expiry`).
    ///
    /// Both sides, because a door that never read the lifetime at all would
    /// pass the first half alone, and one that refused every token would pass
    /// the second.
    // covers: Core\Signature::verify
    #[test]
    fn a_token_past_its_until_throws_the_expired_error() {
        let keys = ring_of(&[&[3_u8; 32]]);
        let held = payload(&[("id", Value::str(NvsStr::new(b"7")))]);
        let token = minted_at(Domain::Payload, Some(UNTIL), &held, &keys);

        let live = verified(UNTIL.second - 1, &token, &keys)
            .expect("a token one second inside its own lifetime verifies");
        dropped(live);

        let expired = verified(UNTIL.second + 1, &token, &keys)
            .expect_err("a token one second past its own lifetime does not");
        assert!(
            expired.contains(EXPIRED),
            "an expired token was refused with something other than the expiry sentence: {expired}"
        );
        assert!(
            expired.starts_with(VERIFY),
            "the expiry sentence names some other member: {expired}"
        );
        assert!(
            !expired.contains(FORGED),
            "the expiry refusal also carries the indistinguishable sentence, so the two have \
             stopped being told apart: {expired}"
        );

        dropped(held);
        dropped(keys);
    }

    /// The ordering that makes naming expiry safe: `open` checks the tag
    /// before `judge` reads the lifetime, so a token that is *both* forged and
    /// past its `until` says only that it is not authentic.
    ///
    /// The same token, the same clock and two rings, so neither half is
    /// vacuous — the ring that minted it answers the expiry sentence at that
    /// very second, which is what makes the other ring's silence about the
    /// lifetime a decision rather than an accident. A door that judged first
    /// would hand a forger the fact that some key in the ring once minted a
    /// token expiring then (`rule:core-api/one-refusal-except-expiry`).
    #[test]
    fn a_token_both_forged_and_past_its_until_throws_the_invalid_error_not_the_expired_one() {
        let mine = ring_of(&[&[4_u8; 32]]);
        let theirs = ring_of(&[&[5_u8; 32]]);
        let held = payload(&[("id", Value::str(NvsStr::new(b"7")))]);
        let token = minted_at(Domain::Payload, Some(UNTIL), &held, &theirs);

        let forged = verified(UNTIL.second + 1, &token, &mine)
            .expect_err("a token no key in this ring minted");
        assert!(
            forged.contains(FORGED),
            "a forgery was refused with something other than the one sentence: {forged}"
        );
        assert!(
            !forged.contains(EXPIRED),
            "a forged token was told its lifetime had passed, so the expiry check ran before the \
             tag was authenticated: {forged}"
        );

        let known = verified(UNTIL.second + 1, &token, &theirs)
            .expect_err("its own ring, at the same second, does reach the lifetime");
        assert!(
            known.contains(EXPIRED),
            "the ring that minted the token did not reach the expiry check, so the case above \
             asserts nothing: {known}"
        );
        assert_ne!(
            forged, known,
            "the two refusals are one sentence, so the holder of a real token cannot be told \
             apart from a forger — which is the half of the rule that is not about secrecy"
        );

        dropped(held);
        dropped(theirs);
        dropped(mine);
    }

    /// Every way of not being authentic other than expiry, asserted by
    /// **counting the distinct sentences** rather than by reading each one:
    /// a member that answered plausibly for each way separately still fails
    /// here the moment two of them differ by a word
    /// (`rule:core-api/one-refusal-except-expiry`).
    ///
    /// The list is `refused`'s own — text that is not base64, a token too
    /// short to hold a tag, a tag no key authenticates, a token minted for
    /// another door, and a document whose payload is not what this door
    /// answers. Every one of them is reached with a live lifetime, so nothing
    /// here can be passing because the clock refused it first.
    // covers: Core\Signature::verify
    #[test]
    fn every_other_way_of_not_being_authentic_raises_one_error_with_one_sentence() {
        let keys = ring_of(&[&[6_u8; 32]]);
        let others = ring_of(&[&[7_u8; 32]]);
        let text = payload(&[("id", Value::str(NvsStr::new(b"7")))]);
        let numeric = payload(&[("id", Value::int(7))]);

        let ways = [
            (
                "text that is not base64",
                "not a token, and not base64".to_owned(),
            ),
            // Valid base64 of twelve octets, which is under the tag's own
            // length — there is nothing to authenticate against.
            (
                "a token too short to hold a tag",
                "AAAAAAAAAAAAAAAA".to_owned(),
            ),
            (
                "a tag that authenticates under no key",
                minted(&text, &others),
            ),
            (
                "a token minted for another door",
                minted_at(Domain::Uri, None, &text, &keys),
            ),
            (
                "a document whose payload is not text",
                minted(&numeric, &keys),
            ),
        ];

        let mut sentences = std::collections::BTreeSet::new();
        for (way, token) in &ways {
            match verified(UNTIL.second, token, &keys) {
                Ok(answered) => {
                    dropped(answered);
                    panic!("{way} verified, so it is not a way of failing at all");
                }
                Err(refusal) => {
                    assert!(
                        !refusal.contains(EXPIRED),
                        "{way} was refused for its lifetime rather than for what it is: {refusal}"
                    );
                    sentences.insert(refusal);
                }
            }
        }

        assert_eq!(
            sentences.len(),
            1,
            "the {} ways of not being authentic produced {} different sentences, so a forgery \
             now says which half of the check it failed: {sentences:?}",
            ways.len(),
            sentences.len()
        );
        let one = sentences
            .iter()
            .next()
            .expect("the sweep asked at least one question");
        assert!(
            one.contains(FORGED),
            "the one sentence is no longer `refused`'s: {one}"
        );

        dropped(numeric);
        dropped(text);
        dropped(others);
        dropped(keys);
    }

    /// The write half through the member itself rather than [`mint`]: the
    /// token `sign` returns is the one `verify` opens under the same ring, and
    /// an empty ring is a `LogicError` before any token is written.
    ///
    /// Compared by re-signing, for the round-trip test's reason: two payloads
    /// that write the same document are the same payload here.
    // covers: Core\Signature::sign
    #[test]
    fn sign_returns_a_token_verify_opens_and_an_empty_ring_is_refused() {
        let keys = ring_of(&[&[8_u8; 32]]);
        let held = payload(&[
            ("doc", Value::str(NvsStr::new(b"7"))),
            ("act", Value::str(NvsStr::new(b"download"))),
        ]);

        let mut ctx = Ctx::buffered();
        let token = nvs_runtime::call(
            nvs_core_signature_sign,
            &mut ctx,
            &[held, keys, Value::null()],
        )
        .expect("a ring of one well-formed key signs");
        let text = token.as_text().expect("sign returns a string").to_owned();
        assert!(
            text.bytes()
                .all(|octet| octet.is_ascii_alphanumeric() || octet == b'-' || octet == b'_'),
            "the token is unpadded URL-safe base64: {text}"
        );

        let back = verified(UNTIL.second, &text, &keys).expect("verify opens what sign wrote");
        assert_eq!(
            signed(&back, None),
            signed(&held, None),
            "the payload verify returns is the one sign was given"
        );
        dropped(back);
        dropped(token);

        let empty = ring_of(&[]);
        let refused = nvs_runtime::call(
            nvs_core_signature_sign,
            &mut ctx,
            &[held, empty, Value::null()],
        );
        let sentence = ctx.take_pending().map(std::borrow::Cow::into_owned);
        assert!(
            refused.is_err(),
            "an empty ring has no newest key to sign with"
        );
        assert!(
            sentence.is_some_and(|message| message.contains("$keys is empty")),
            "the refusal names the empty ring"
        );

        dropped(empty);
        dropped(held);
        dropped(keys);
    }
}
