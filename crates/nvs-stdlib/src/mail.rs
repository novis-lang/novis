//! `Core\Mail` — `rule:programs/framework-core-half`'s transport
//! half: one member that hands a message to an SMTP endpoint **an operator named**, and no spelling
//! anywhere for one a program chose.
//!
//! `rule:core-api/tier-roster` places the class — Native by
//! test 3, because it waits on the outside world — and the spec's § 16 row gives it one sentence,
//! "an SMTP client with structured headers, replacing `mail()`". What belongs here is the roster
//! that sentence does not write, and the five decisions behind it.
//!
//! # `mail()`'s fourth argument is the whole bug
//!
//! PHP's `mail($to, $subject, $message, $additional_headers)` lets a program concatenate a header
//! block. Every mail header injection in twenty years is that argument: a `\r\n` inside a `$to` or a
//! `$subject` reaching it verbatim, and one more `Bcc:` arriving at the far end. There is no
//! escaping convention that fixes it, because the caller is the one holding the separator.
//!
//! So this class has **no raw header parameter at all**. Every header a program can set is a named
//! parameter of [`send`](CLASS) with a type, and [`compose`] is the only thing that ever writes a
//! `\r\n` — an address is parsed and refused if it is not one ([ADR
//! 0095](/docs/decisions/0095.md): ambiguous input is
//! refused, never repaired), and a subject is RFC 2047 encoded the moment it holds anything a header
//! line cannot carry, so a control byte becomes *content* rather than structure. That is what the
//! spec row's "structured headers" buys, and it is why `$subject` and `$text` accept a `tainted`
//! argument freely: they are data in exactly [ADR
//! 0088](/docs/decisions/0088.md) § 7's sense,
//! and mailing what a user typed is the point.
//!
//! # The endpoint is a name, not a host — `rule:core-classes/db-capabilities`'s shape
//!
//! `$endpoint` selects a `[mail.<name>]` block in root-owned `nvs.toml`, granted by name under the
//! `mail.send` capability. There is no host parameter, no port and no credential on the surface, so
//! there is nothing for an attacker-influenced value to redirect: the worst a forged `$endpoint` can
//! name is another block the same operator wrote and the same grant listed. It is a
//! [`Qual::Sink`] for that last inch — a `tainted` block name is a program letting input choose
//! between deployments — and it is the only qualifier this class refuses.
//!
//! `rule:core-classes/db-capabilities` is where that shape comes from and also why the address is **not** put through
//! `rule:security/net-address-policy`'s denied ranges: an address
//! an operator wrote into root-owned configuration carries the same authority as the grant itself,
//! and a mail relay lives at `127.0.0.1` or on a container network precisely inside the set § 3
//! denies. `Core\Http`'s launderer pins because *its* host came from the program;
//! [`resolve`] does not, because this one could not have.
//!
//! # Envelope identity is the deployment's, and the program does not get a say
//!
//! There is no `from` parameter. The envelope sender and the `From:` header are `[mail.<name>]
//! from`, and nothing at a call site can override them. SPF, DKIM and DMARC alignment are facts
//! about the domain the operator runs, not about the request being served, and a program that could
//! choose its own `From:` is a program that can forge a colleague's. `replyTo` is the option that
//! covers the real need — answer to the user — and it changes where a reply goes without claiming
//! who sent it.
//!
//! # A credential asks for TLS, and asking for it is what makes it required
//!
//! A `[mail.<name>]` block that sets `user` and `password` — or `password_file`, the same half of
//! the credential arriving as an injected file (`rule:config/a-secret-is-a-file-whose-content-is-the-value`) — is sent through `STARTTLS` and
//! authenticated with `AUTH PLAIN`, and there is no path on which the credential reaches a
//! plaintext socket: the upgrade is issued after the first `EHLO`, `EHLO` is re-issued over the
//! secured stream because the extension list is the *session's* and an endpoint offering `AUTH`
//! only to a secured client is the ordinary case, and an endpoint that advertises neither — or
//! whose certificate does not verify against [`nvs_host::tls`]'s compiled-in anchors — is refused
//! with nothing sent.
//!
//! A block with **no** credential stays in the clear, and that is a decision rather than an
//! omission. The alternative is opportunistic TLS, which is one of two things: verified, and then
//! the sidecar relay presenting an internal certificate — the case [`nvs_host::tls`] names as the
//! operator's and has no `nvs.toml` key for yet — stops working with no remedy in the file that
//! would hold one; or unverified, which that module has no spelling for and will not grow one,
//! because a handshake nobody checked is exactly the false confidence
//! `rule:security/launderers-are-sink-named` refuses to sell.
//! So TLS here is *asked for*, by configuring the credential that cannot travel without it, and
//! where it is asked for it is required and verified. Encryption without authentication has no key
//! today; it belongs beside the anchor bundle that module already names as unlanded, and the two
//! are one configuration slice.
//!
//! # Attachments and inline parts are composition, and composition is § 3's
//!
//! `rule:programs/framework-core-half` splits this class at exactly that line. Two body parts are here because the
//! transport has to choose a `Content-Type` regardless and a mail with no plain-text alternative is
//! a mail half its readers cannot read; anything richer — templates, files, `multipart/related` —
//! belongs to the `nvs/web` package, which composes *into* these arguments.

use std::io::{Read, Write};
use std::net::{SocketAddr, ToSocketAddrs as _};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use nvs_host::net::NvsTcp;
use nvs_host::tls::NvsTls;
use nvs_runtime::{Ctx, Fault, Tag, ThrownClass, Value};
use nvs_syntax::duration;

use crate::registry::{
    ClassDoc, Const, CoreClass, CoreMethod, CoreOption, CoreTy, ErrorDoc, MethodDoc, ParamDoc, Qual,
};

/// The class name, as `registry::CAPABILITIES` and a refusal both spell it.
pub(crate) const NAME: &str = r"Core\Mail";

/// How long the whole exchange may take — connect, banner, every command and the
/// `DATA` body together.
///
/// One bound rather than one per step, for `rule:http-server/no-spelling-for-an-unbounded-wait`'s reason: a per-step
/// timeout multiplied by the number of steps is the unbounded wait spelled
/// long-hand, and an SMTP conversation has as many steps as there are
/// recipients.
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);

/// The submission port a block that names none is read as meaning.
///
/// 25 rather than 587, because a block that names no port is the unauthenticated
/// local-relay shape and 587 is the *authenticated* submission port. An operator
/// writing the `user` that port exists for writes `port = 587` beside it.
const DEFAULT_PORT: u16 = 25;

/// The ABI slot each parameter and each flattened option lands in — the bag
/// expands to one argument per option, in declaration order, after the four
/// positionals.
const ENDPOINT: usize = 0;
/// See [`ENDPOINT`].
const TO: usize = 1;
/// See [`ENDPOINT`].
const SUBJECT: usize = 2;
/// See [`ENDPOINT`].
const TEXT: usize = 3;
/// See [`ENDPOINT`].
const CC: usize = 4;
/// See [`ENDPOINT`].
const BCC: usize = 5;
/// See [`ENDPOINT`].
const REPLY_TO: usize = 6;
/// See [`ENDPOINT`].
const HTML: usize = 7;

/// [`send`](CLASS)'s trailing shape — the four headers a program may set that
/// are not required to have a message at all.
///
/// `cc` and `bcc` are `array<string>` and `to` is one as well, because a
/// comma-separated recipient list is the parse `rule:errors/ambiguous-input-refused` refuses: `mail()` splits
/// its `$to` on commas, which makes a comma inside a display name a second
/// recipient. A list has no such reading. A single recipient writes `["a@b.c"]`
/// and pays two characters for it.
const OPTIONS: &[CoreOption] = &[
    CoreOption {
        name: "cc",
        ty: CoreTy::Array(&CoreTy::Text(Qual::Neutral)),
        default: Const::EmptyArray,
    },
    CoreOption {
        name: "bcc",
        ty: CoreTy::Array(&CoreTy::Text(Qual::Neutral)),
        default: Const::EmptyArray,
    },
    CoreOption {
        name: "replyTo",
        ty: CoreTy::Text(Qual::Neutral),
        default: Const::Null,
    },
    CoreOption {
        name: "html",
        ty: CoreTy::Text(Qual::Neutral),
        default: Const::Null,
    },
];

/// `Core\Mail`'s one row — `rule:programs/framework-core-half`'s transport half.
pub(crate) const CLASS: CoreClass = CoreClass {
    name: NAME,
    doc: Some(&CARD),
    methods: &[CoreMethod {
        name: "send",
        names: &["endpoint", "to", "subject", "text"],
        params: &[
            CoreTy::Text(Qual::Sink),
            CoreTy::Array(&CoreTy::Text(Qual::Neutral)),
            CoreTy::Text(Qual::Neutral),
            CoreTy::Text(Qual::Neutral),
            CoreTy::Options(OPTIONS),
        ],
        defaults: &[],
        return_ty: CoreTy::Void,
        symbol: "nvs_core_mail_send",
        doc: Some(&SEND_DOC),
    }],
    instance: &[],
    slots: &[],
    constants: &[],
};

/// `Core\Mail`'s class card — `rule:core-api/reference-card`.
const CARD: ClassDoc = ClassDoc {
    short: "Sends email. `send` gives one message to the mail server that a `[mail.<name>]` block \
            of `nvs.toml` names. The program writes the block's name. The host, the port, the \
            password and the sender address stay in the block.",
};

/// `Core\Mail::send`'s reference card — `rule:core-api/reference-card`.
const SEND_DOC: MethodDoc = MethodDoc {
    short: "Hands one message to the SMTP endpoint `[mail.$endpoint]` names, and returns once that \
            endpoint has accepted it.",
    params: &[
        ParamDoc {
            name: "endpoint",
            desc: "Which `[mail.<name>]` block in `nvs.toml` to send through. Refuses a `tainted` \
                   argument: it selects a deployment, so it is written at the call site and never \
                   read from input.",
            shape: &[],
        },
        ParamDoc {
            name: "to",
            desc: "The recipients, one address per entry. A list rather than a comma-separated \
                   string, which has no unambiguous reading.",
            shape: &[],
        },
        ParamDoc {
            name: "subject",
            desc: "The subject. RFC 2047 encoded where it holds anything a header line cannot \
                   carry, so no value of it can add a header.",
            shape: &[],
        },
        ParamDoc {
            name: "text",
            desc: "The plain-text body, which every message has.",
            shape: &[],
        },
        ParamDoc {
            name: "cc",
            desc: "Further recipients, named in the message.",
            shape: &[],
        },
        ParamDoc {
            name: "bcc",
            desc: "Further recipients, not named in the message — they reach the envelope and no \
                   header.",
            shape: &[],
        },
        ParamDoc {
            name: "replyTo",
            desc: "Where a reply should go, when that is not the configured sender.",
            shape: &[],
        },
        ParamDoc {
            name: "html",
            desc: "An HTML alternative to `$text`. Given one, the message is \
                   `multipart/alternative` and both parts are sent.",
            shape: &[],
        },
    ],
    ret: "Nothing. The endpoint accepted the message; delivery past it is the endpoint's.",
    errors: &[
        ErrorDoc {
            error: "RuntimeError",
            desc: "The capability `mail.send` does not grant `$endpoint`; or no `[mail.<name>]` \
                   block of that name sets `host` or `from`; or that block sets one of `user` and \
                   `password` without the other; or an address is not one. Each is a deployment or \
                   a call that was written wrong, not a send that failed.",
        },
        ErrorDoc {
            error: "IOError",
            desc: "The configured endpoint could not be reached, closed the connection, or \
                   refused a command — the last carrying the SMTP reply that said so. Also where \
                   a block configures a credential and its endpoint cannot carry one: no \
                   `STARTTLS`, no `AUTH PLAIN` over it, or a certificate that does not verify. \
                   Nothing is sent in the clear on any of those paths.",
        },
    ],
};

/// The `Core` symbol table's arm for this module — see [`crate::address`].
pub(crate) fn address(symbol: &str) -> Option<*const u8> {
    Some(match symbol {
        "nvs_core_mail_send" => (nvs_core_mail_send as *const ()).cast(),
        _ => return None,
    })
}

/// What `[mail.<name>]` said, once every question this member asks of it has an
/// answer.
struct Endpoint {
    /// `host`, as written — resolved but never re-read, and never pinned.
    host: String,
    /// `port`, or [`DEFAULT_PORT`].
    port: u16,
    /// `from` — the envelope sender and the `From:` header, both.
    from: String,
    /// `timeout`, or [`DEFAULT_TIMEOUT`].
    timeout: Duration,
    /// `user` and `password`, both or neither — the credential `AUTH PLAIN`
    /// sends, and the thing whose presence asks for `STARTTLS` at all.
    credential: Option<(String, String)>,
}

/// The directive `[mail.<name>] key`, with an empty value read as absent.
///
/// Absent and blank are one answer for [`crate::cache`]'s reason: `host = ""` is
/// an operator clearing a setting, and reading it as a name would produce a
/// refusal about DNS rather than about the configuration.
fn configured(ctx: &Ctx, endpoint: &str, key: &str) -> Option<String> {
    ctx.config()
        .and_then(|config| config.get(&format!("mail.{endpoint}.{key}")))
        .and_then(present)
}

/// Blank read as absent, and anything else answered **verbatim**.
///
/// The two halves are separate rules and this reader used to conflate them. A
/// blank value is a cleared setting, which is the paragraph above. A *non-blank*
/// value is the operator's, byte for byte: `password = "hunter2 "` is a
/// credential that ends in a space, `rule:config/a-secret-is-a-file-whose-content-is-the-value` is explicit that such a value
/// is kept rather than trimmed, and the boot already says `W1007` about it. This
/// function trimmed the value it returned, so the one endpoint that could have
/// used that password submitted a different one — an authentication failure at
/// the far end, from a file that plainly held the right bytes. `rule:errors/ambiguous-input-refused` is the
/// general form: input is read or refused, never repaired.
fn present(text: String) -> Option<String> {
    if text.trim().is_empty() {
        None
    } else {
        Some(text)
    }
}

/// The block `endpoint` names, once `mail.send` has been shown to grant it.
///
/// The order is the point, and it is [`nvs_runtime::capability::open_read`]'s:
/// the grant is asked **first**, so a program with no grant learns nothing about
/// which blocks exist. A name outside the grant and a name with no block behind
/// it are two different messages, and only the second is reachable by a program
/// the operator already trusted with that name.
///
/// # Errors
///
/// A catchable `RuntimeError` for an ungranted name, for a block that sets no
/// `host` or no `from`, and for one that sets half a credential.
fn endpoint_of(ctx: &Ctx, endpoint: &str, member: &str) -> Result<Endpoint, Fault> {
    nvs_runtime::capability::require(
        ctx,
        nvs_config::Cap::MailSend,
        nvs_config::capability::Scope::Name(endpoint),
        member,
    )?;

    let Some(host) = configured(ctx, endpoint, "host") else {
        return Err(Fault::thrown(format!(
            "{member}: no `[mail.{endpoint}]` block sets `host`, so there is no endpoint of that \
             name to send through"
        )));
    };
    let Some(from) = configured(ctx, endpoint, "from") else {
        return Err(Fault::thrown(format!(
            "{member}: `[mail.{endpoint}]` sets no `from`, and the sending identity is the \
             deployment's — there is no parameter for it"
        )));
    };
    address_of(&from, "the configured `from`", member)?;

    // Both or neither. Half a credential is a block half-written, and reading
    // one as anonymous would submit as nobody to an endpoint the operator
    // plainly meant to log in to — which fails later, further away, and with the
    // endpoint's sentence rather than this one.
    let credential = match (
        configured(ctx, endpoint, "user"),
        configured(ctx, endpoint, "password"),
    ) {
        (Some(user), Some(password)) => Some((user, password)),
        (None, None) => None,
        (user, _) => {
            let (written, missing) = if user.is_some() {
                ("user", "password")
            } else {
                ("password", "user")
            };
            return Err(Fault::thrown(format!(
                "{member}: `[mail.{endpoint}]` sets `{written}` and no `{missing}`, and half a \
                 credential is not one"
            )));
        }
    };

    let port = configured(ctx, endpoint, "port")
        .and_then(|text| text.parse::<u16>().ok())
        .filter(|port| *port != 0)
        .unwrap_or(DEFAULT_PORT);
    let timeout = configured(ctx, endpoint, "timeout")
        .and_then(|text| duration::parse(&text).ok())
        .map(|nanos| Duration::from_nanos(nanos.unsigned_abs()))
        .filter(|bound| !bound.is_zero())
        .unwrap_or(DEFAULT_TIMEOUT);

    Ok(Endpoint {
        host,
        port,
        from,
        timeout,
        credential,
    })
}

/// Where the operator's `host` resolves to.
///
/// **No `pin_host` and no § 3 table**, which the module doc argues in full: this
/// address came out of root-owned configuration, so the authority that would
/// have granted an exception is the one that wrote it, and every ordinary relay
/// sits inside the ranges `rule:security/net-address-policy` denies by default.
///
/// # Errors
///
/// A catchable `IOError` for a name that resolves to nothing.
fn resolve(endpoint: &Endpoint, member: &str) -> Result<SocketAddr, Fault> {
    let bare = endpoint
        .host
        .strip_prefix('[')
        .and_then(|held| held.strip_suffix(']'))
        .unwrap_or(&endpoint.host);
    if let Ok(literal) = bare.parse::<std::net::IpAddr>() {
        return Ok(SocketAddr::new(literal, endpoint.port));
    }
    (endpoint.host.as_str(), endpoint.port)
        .to_socket_addrs()
        .ok()
        .and_then(|mut found| found.next())
        .ok_or_else(|| {
            Fault::thrown_as(
                ThrownClass::Io,
                format!(
                    "{member} could not resolve {}, which `[mail]` names as an endpoint",
                    endpoint.host
                ),
            )
        })
}

/// `text` as an addr-spec, or a refusal naming `what` it was.
///
/// The check is deliberately narrower than RFC 5321's grammar and deliberately
/// not a repair. What it establishes is the one property the rest of this module
/// relies on — that the value can be written inside `<...>` on a command line
/// and inside a header without changing either one's structure — so a byte that
/// would end a line, a byte that would end the address, and the absence of an
/// `@` are each a refusal. `rule:errors/ambiguous-input-refused` is why none of them is silently stripped:
/// an address a program did not mean is a message going somewhere it did not
/// mean, and the caller is the only one who can say which.
///
/// # Errors
///
/// A catchable `RuntimeError` naming the value and what was wrong with it.
fn address_of(text: &str, what: &str, member: &str) -> Result<String, Fault> {
    let refuse = |why: &str| {
        Err(Fault::thrown(format!(
            "{member}: {what} is not an address — {why}"
        )))
    };
    if text.is_empty() {
        return refuse("it is empty");
    }
    if text
        .bytes()
        .any(|byte| byte < 0x20 || byte == 0x7f || byte == b'<' || byte == b'>' || byte == b',')
    {
        return refuse(
            "it holds a byte an address cannot carry (a control byte, an angle bracket or a comma)",
        );
    }
    if !text.is_ascii() {
        return refuse(
            "it is not ASCII, and internationalized addresses need the SMTPUTF8 extension this \
             transport does not negotiate",
        );
    }
    match text.split_once('@') {
        Some((local, domain)) if !local.is_empty() && domain.contains('.') => Ok(text.to_owned()),
        _ => refuse("it is not `local@domain`"),
    }
}

/// Every address in the `array<string>` at `slot`, each through
/// [`address_of`].
///
/// # Errors
///
/// A [`Fault::fatal`] for an argument or element that is not what the row
/// declared, which the `array<string>` rules out from source; a catchable
/// `RuntimeError` for an element that is not an address.
fn addresses_of(
    args: &[Value],
    slot: usize,
    what: &str,
    member: &str,
) -> Result<Vec<String>, Fault> {
    let Some(array) = args[slot].array_ptr() else {
        return Err(Fault::fatal(format!(
            "{member} expected {:?} for `{what}`, got tag {}",
            Tag::Array,
            args[slot].tag_byte()
        )));
    };
    let mut found = Vec::new();
    let mut from = 0_usize;
    loop {
        #[expect(
            unsafe_code,
            reason = "a Tag::Array argument owns a reference to a live allocation, \
                      so it is live for the length of this call, and `from` only \
                      ever advances past a slot this same cursor reported"
        )]
        let (slot_at, value) = unsafe {
            let slot_at = nvs_runtime::nvs_array_next_slot(array, from);
            let Ok(slot_at) = usize::try_from(slot_at) else {
                break;
            };
            let mut value = Value::null();
            nvs_runtime::nvs_array_value_at(array, slot_at, &raw mut value);
            (slot_at, value)
        };
        from = slot_at + 1;

        let text = value.as_text().ok_or_else(|| {
            Fault::fatal(format!(
                "{member} expected a `string` in `{what}`, got tag {}",
                value.tag_byte()
            ))
        })?;
        found.push(address_of(text, what, member)?);
    }
    Ok(found)
}

/// `text` as a header value: itself when a header line can carry it, and its
/// RFC 2047 `=?UTF-8?B?...?=` encoding when it cannot.
///
/// This is the function the module doc's first section is about. A value that
/// reaches here holding a `\r\n` does not get it stripped and does not get it
/// through — it comes out base64, which is one header token by construction, so
/// the injected line arrives as characters a reader sees rather than as
/// structure a parser obeys.
fn header_value(text: &str) -> String {
    let plain = text.is_ascii() && text.bytes().all(|byte| byte >= 0x20 && byte != 0x7f);
    if plain {
        text.to_owned()
    } else {
        format!("=?UTF-8?B?{}?=", STANDARD.encode(text.as_bytes()))
    }
}

/// `bytes` as base64, wrapped at 76 columns with CRLF, which is the transfer
/// encoding both body parts use.
///
/// Base64 for every body rather than quoted-printable for some of them: it is
/// one path instead of two, it cannot produce a line that is a lone `.` — so
/// there is no dot-stuffing rule for a later reader to get wrong — and it holds
/// UTF-8 without a second question about which bytes needed escaping.
fn body_encoded(bytes: &[u8]) -> String {
    let raw = STANDARD.encode(bytes);
    let mut out = String::with_capacity(raw.len() + raw.len() / 76 * 2 + 2);
    for (index, chunk) in raw.as_bytes().chunks(76).enumerate() {
        if index > 0 {
            out.push_str("\r\n");
        }
        out.push_str(std::str::from_utf8(chunk).unwrap_or_default());
    }
    out.push_str("\r\n");
    out
}

/// `now` as RFC 5322 writes a date, in UTC.
///
/// Written here rather than reached for through [`crate::time`]: what a header
/// needs is one fixed English spelling with a `+0000` offset, which is not the
/// locale-shaped, zone-shaped question that module answers, and a `DateTime`
/// threaded through this module for one line would be the larger coupling.
fn rfc5322_date(now: SystemTime) -> String {
    let secs =
        i64::try_from(now.duration_since(UNIX_EPOCH).unwrap_or_default().as_secs()).unwrap_or(0);
    let days = secs.div_euclid(86_400);
    let rest = secs.rem_euclid(86_400);
    let (hour, minute, second) = (rest / 3_600, (rest % 3_600) / 60, rest % 60);

    // Howard Hinnant's civil_from_days, which is the shortest correct spelling
    // of a proleptic Gregorian conversion and has no table to get wrong.
    let shifted = days + 719_468;
    let era = shifted.div_euclid(146_097);
    let doe = shifted.rem_euclid(146_097);
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let year = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = if month <= 2 { year + 1 } else { year };

    const WEEKDAYS: [&str; 7] = ["Thu", "Fri", "Sat", "Sun", "Mon", "Tue", "Wed"];
    const MONTHS: [&str; 12] = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];
    let weekday = WEEKDAYS[usize::try_from(days.rem_euclid(7)).unwrap_or(0)];
    let month_name = MONTHS[usize::try_from(month - 1).unwrap_or(0)];

    format!("{weekday}, {day:02} {month_name} {year} {hour:02}:{minute:02}:{second:02} +0000")
}

/// The message `DATA` carries — every header this class writes, then the body.
///
/// The **only** place a `\r\n` is produced. Every value reaching it has been
/// through [`header_value`] or [`address_of`], so there is no path from an
/// argument to a line break here, which is the property the module doc's first
/// section names.
fn compose(
    from: &str,
    to: &[String],
    cc: &[String],
    subject: &str,
    text: &str,
    html: Option<&str>,
    now: SystemTime,
) -> String {
    let mut out = String::new();
    out.push_str(&format!("Date: {}\r\n", rfc5322_date(now)));
    out.push_str(&format!("From: <{from}>\r\n"));
    out.push_str(&format!("To: {}\r\n", to.join(", ")));
    if !cc.is_empty() {
        out.push_str(&format!("Cc: {}\r\n", cc.join(", ")));
    }
    out.push_str(&format!("Subject: {}\r\n", header_value(subject)));
    out.push_str("MIME-Version: 1.0\r\n");

    match html {
        None => {
            out.push_str("Content-Type: text/plain; charset=UTF-8\r\n");
            out.push_str("Content-Transfer-Encoding: base64\r\n\r\n");
            out.push_str(&body_encoded(text.as_bytes()));
        }
        Some(html) => {
            // A fixed boundary is safe because neither part is ever transmitted
            // raw: both are base64, whose alphabet cannot contain it.
            const BOUNDARY: &str = "nvs-alternative-boundary";
            out.push_str(&format!(
                "Content-Type: multipart/alternative; boundary=\"{BOUNDARY}\"\r\n\r\n"
            ));
            out.push_str(&format!("--{BOUNDARY}\r\n"));
            out.push_str("Content-Type: text/plain; charset=UTF-8\r\n");
            out.push_str("Content-Transfer-Encoding: base64\r\n\r\n");
            out.push_str(&body_encoded(text.as_bytes()));
            out.push_str(&format!("\r\n--{BOUNDARY}\r\n"));
            out.push_str("Content-Type: text/html; charset=UTF-8\r\n");
            out.push_str("Content-Transfer-Encoding: base64\r\n\r\n");
            out.push_str(&body_encoded(html.as_bytes()));
            out.push_str(&format!("\r\n--{BOUNDARY}--\r\n"));
        }
    }
    out
}

/// One SMTP conversation over [`nvs_host`]'s parking stream.
///
/// Held as a struct for the read buffer alone: a reply is line-oriented and a
/// multiline one is only finished when a line's fourth byte is a space, so a
/// reader that did not keep what it over-read would lose the head of the next
/// reply.
struct Session {
    /// The socket, or the TLS session `STARTTLS` replaced it with — either way
    /// it hands the core back rather than blocking it.
    stream: Wire,
    /// Whatever the last read took past the end of the reply it was completing.
    held: Vec<u8>,
}

/// What a [`Session`] is talking over.
///
/// An enum rather than a boxed trait object because there are two variants and
/// there will not be a third, and because the upgrade *consumes* the socket:
/// [`NvsTls::over`] takes an [`NvsTcp`] by value, which is what leaves nobody
/// holding a plaintext stream to an endpoint that has been secured.
///
/// `Secured` is about a kilobyte wider than `Plain` — `rustls`'s session state
/// is held inline — and that is the trade AGENTS.md's priority ordering asks
/// for, spent deliberately: there is one `Wire` per in-flight `send`, so the
/// cost is O(in-flight) rather than O(messages sent), and boxing it would buy an
/// allocation and an indirection on every record read to save a kilobyte
/// priority 5 says not to chase.
#[allow(clippy::large_enum_variant)]
enum Wire {
    /// Before `STARTTLS`, and for the whole of a session with no credential.
    Plain(NvsTcp),
    /// After it. Every wait inside is still [`nvs_host::net`]'s park, which
    /// [`nvs_host::tls`]'s module doc is the home of.
    Secured(NvsTls),
}

impl Read for Wire {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match self {
            Self::Plain(stream) => stream.read(buf),
            Self::Secured(stream) => stream.read(buf),
        }
    }
}

impl Write for Wire {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        match self {
            Self::Plain(stream) => stream.write(buf),
            Self::Secured(stream) => stream.write(buf),
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        match self {
            Self::Plain(stream) => stream.flush(),
            Self::Secured(stream) => stream.flush(),
        }
    }
}

impl Session {
    /// The connected session, with the server's banner already accepted.
    ///
    /// # Errors
    ///
    /// A catchable `IOError` for a socket that will not open and for a banner
    /// that is not a 220.
    fn open(address: SocketAddr, timeout: Duration, member: &str) -> Result<Self, Fault> {
        let stream = NvsTcp::connect_timeout(address, timeout).map_err(|why| {
            Fault::thrown_as(
                ThrownClass::Io,
                format!("{member} could not reach {address}: {why}"),
            )
        })?;
        let mut session = Self {
            stream: Wire::Plain(stream),
            held: Vec::new(),
        };
        session.expect(220, "the greeting", member)?;
        Ok(session)
    }

    /// The same conversation over TLS: `STARTTLS`, a handshake verified against
    /// `name`, and the session that answers on the other side of it.
    ///
    /// Taken by value, which is the property rather than a style choice — the
    /// plaintext session is gone by the time this returns, so no later step can
    /// reach for it.
    ///
    /// # Errors
    ///
    /// A catchable `IOError` when the endpoint refuses `STARTTLS`, when it sends
    /// anything between agreeing to it and the handshake, and when the handshake
    /// does not complete — including a certificate that does not verify against
    /// [`nvs_host::tls`]'s compiled-in anchors.
    fn secure(mut self, name: &str, member: &str) -> Result<Self, Fault> {
        self.command("STARTTLS", 220, "the upgrade", member)?;
        // The whole reason `held` is inspected anywhere. A man in the middle
        // who writes commands *after* the endpoint's 220 and before the
        // handshake has them buffered here, and a session that carried the
        // buffer across would replay them as though the secured peer had sent
        // them. The endpoint owes silence between the two, so anything at all is
        // refused rather than discarded — discarding is the same defence with
        // the evidence thrown away.
        if !self.held.is_empty() {
            return Err(Fault::thrown_as(
                ThrownClass::Io,
                format!(
                    "{member}: the endpoint sent {} more byte(s) after agreeing to `STARTTLS`, \
                     which is how a plaintext command is smuggled into a secured session",
                    self.held.len()
                ),
            ));
        }
        let Wire::Plain(stream) = self.stream else {
            return Err(Fault::thrown_as(
                ThrownClass::Io,
                format!("{member}: the session is already secured"),
            ));
        };
        let secured = NvsTls::over(stream, name).map_err(|why| {
            Fault::thrown_as(
                ThrownClass::Io,
                format!("{member}: the TLS handshake with `{name}` did not complete — {why}"),
            )
        })?;
        Ok(Self {
            stream: Wire::Secured(secured),
            held: Vec::new(),
        })
    }

    /// One command, and the lines of the reply it was answered with.
    ///
    /// `line` never reaches a diagnostic — `what` names the step instead —
    /// because one of the commands this sends is `AUTH PLAIN`.
    ///
    /// # Errors
    ///
    /// A catchable `IOError` when the write fails, when the connection ends
    /// mid-reply, or when the code is not `want`.
    fn command(
        &mut self,
        line: &str,
        want: i64,
        what: &str,
        member: &str,
    ) -> Result<Vec<String>, Fault> {
        self.stream
            .write_all(format!("{line}\r\n").as_bytes())
            .and_then(|()| self.stream.flush())
            .map_err(|why| {
                Fault::thrown_as(
                    ThrownClass::Io,
                    format!("{member} could not send {what}: {why}"),
                )
            })?;
        self.expect(want, what, member)
    }

    /// Reads one whole reply, holds it to `want`, and answers with its lines —
    /// which for `EHLO` are the extension list [`advertised`] reads.
    ///
    /// # Errors
    ///
    /// A catchable `IOError` for a closed connection, a reply that is not
    /// three digits, or a code other than `want` — the last carrying the
    /// server's own sentence, because that is the only part of the failure the
    /// operator did not already write.
    fn expect(&mut self, want: i64, what: &str, member: &str) -> Result<Vec<String>, Fault> {
        let mut reply = Vec::new();
        loop {
            let line = self.line(what, member)?;
            let code = line
                .get(..3)
                .and_then(|digits| digits.parse::<i64>().ok())
                .ok_or_else(|| {
                    Fault::thrown_as(
                        ThrownClass::Io,
                        format!(
                            "{member}: the endpoint answered {what} with `{line}`, which is \
                                 not an SMTP reply"
                        ),
                    )
                })?;
            // A continuation line writes `250-`; the last one writes `250 `.
            if line.as_bytes().get(3) != Some(&b'-') {
                if code != want {
                    return Err(Fault::thrown_as(
                        ThrownClass::Io,
                        format!("{member}: the endpoint refused {what} — {line}"),
                    ));
                }
                reply.push(line);
                return Ok(reply);
            }
            reply.push(line);
        }
    }

    /// The next CRLF-terminated line, without its terminator.
    ///
    /// # Errors
    ///
    /// A catchable `IOError` when the connection closes before one arrives.
    fn line(&mut self, what: &str, member: &str) -> Result<String, Fault> {
        loop {
            if let Some(end) = self.held.iter().position(|byte| *byte == b'\n') {
                let mut line: Vec<u8> = self.held.drain(..=end).collect();
                while line
                    .last()
                    .is_some_and(|byte| *byte == b'\n' || *byte == b'\r')
                {
                    line.pop();
                }
                return Ok(String::from_utf8_lossy(&line).into_owned());
            }
            let mut chunk = [0_u8; 512];
            let read = self.stream.read(&mut chunk).map_err(|why| {
                Fault::thrown_as(
                    ThrownClass::Io,
                    format!("{member} could not read the reply to {what}: {why}"),
                )
            })?;
            if read == 0 {
                return Err(Fault::thrown_as(
                    ThrownClass::Io,
                    format!("{member}: the endpoint closed the connection during {what}"),
                ));
            }
            self.held.extend_from_slice(&chunk[..read]);
        }
    }
}

/// The parameters `keyword` was advertised with in an `EHLO` reply, or `None`
/// where the endpoint did not advertise it at all.
///
/// The reply's first line is the endpoint's own greeting rather than a keyword,
/// so it is skipped; every other line is `250-KEYWORD PARAM PARAM`, and RFC 5321
/// § 2.4 makes the keyword case-insensitive.
fn advertised<'a>(reply: &'a [String], keyword: &str) -> Option<&'a str> {
    reply.iter().skip(1).find_map(|line| {
        let rest = line.get(4..)?;
        let (word, params) = rest.split_once(' ').unwrap_or((rest, ""));
        word.eq_ignore_ascii_case(keyword).then_some(params)
    })
}

/// RFC 4616's `PLAIN` message, base64'd: an empty authorization identity — so
/// the endpoint uses the authentication one — then the user and the password,
/// NUL-separated.
fn plain(user: &str, password: &str) -> String {
    STANDARD.encode(format!("\0{user}\0{password}"))
}

/// The whole exchange, once every argument has been checked and every
/// deployment question answered.
///
/// # Errors
///
/// A catchable `IOError` for anything the endpoint said or did — see
/// [`Session`] — and for an endpoint that cannot carry a configured credential:
/// the module doc's § *A credential asks for TLS* is why that is a refusal here
/// rather than a send in the clear.
fn deliver(
    address: SocketAddr,
    endpoint: &Endpoint,
    envelope: &[String],
    message: &str,
    member: &str,
) -> Result<(), Fault> {
    let mut session = Session::open(address, endpoint.timeout, member)?;
    // The domain the operator's own `from` names, so nothing about the host
    // serving the request is announced to the relay.
    let domain = endpoint
        .from
        .split_once('@')
        .map_or("localhost", |(_, d)| d);
    let greeting = format!("EHLO {domain}");
    let extensions = session.command(&greeting, 250, "the greeting", member)?;

    if let Some((user, password)) = &endpoint.credential {
        if advertised(&extensions, "STARTTLS").is_none() {
            return Err(Fault::thrown_as(
                ThrownClass::Io,
                format!(
                    "{member}: `[mail]` configures a credential for {} and that endpoint offers \
                     no `STARTTLS`, so there is nothing to send one under — nothing was sent",
                    endpoint.host
                ),
            ));
        }
        // The certificate is checked against the name the *operator* wrote, the
        // same rule `crate::http::transport` states: the address was derived
        // from that name and answers for nothing on its own.
        session = session.secure(&endpoint.host, member)?;
        // A second `EHLO`, because the extension list belongs to the session and
        // this is a new one — an endpoint that offers `AUTH` only once the
        // stream is secured is the ordinary case, not an unusual one.
        let secured = session.command(&greeting, 250, "the greeting", member)?;
        let offers_plain = advertised(&secured, "AUTH").is_some_and(|mechanisms| {
            mechanisms
                .split_ascii_whitespace()
                .any(|mechanism| mechanism.eq_ignore_ascii_case("PLAIN"))
        });
        if !offers_plain {
            return Err(Fault::thrown_as(
                ThrownClass::Io,
                format!(
                    "{member}: {} offers no `AUTH PLAIN` over TLS, and this transport has no \
                     second mechanism — nothing was sent",
                    endpoint.host
                ),
            ));
        }
        session.command(
            &format!("AUTH PLAIN {}", plain(user, password)),
            235,
            "the credential",
            member,
        )?;
    }

    session.command(
        &format!("MAIL FROM:<{}>", endpoint.from),
        250,
        "the envelope sender",
        member,
    )?;
    for recipient in envelope {
        session.command(
            &format!("RCPT TO:<{recipient}>"),
            250,
            "a recipient",
            member,
        )?;
    }
    session.command("DATA", 354, "the body", member)?;
    session
        .stream
        .write_all(message.as_bytes())
        .and_then(|()| session.stream.write_all(b"\r\n.\r\n"))
        .and_then(|()| session.stream.flush())
        .map_err(|why| {
            Fault::thrown_as(
                ThrownClass::Io,
                format!("{member} could not send the message: {why}"),
            )
        })?;
    session.expect(250, "the message", member)?;
    // A QUIT the endpoint never answers is not a message that failed to send:
    // it was accepted at the 250 above, so the reply here is not asked about.
    let _ = session.stream.write_all(b"QUIT\r\n");
    Ok(())
}

/// The `string` in `slot`.
///
/// # Errors
///
/// A [`Fault::fatal`], for [`crate::http`]'s `text_of` reason: the row declares
/// the slot, so another tag is compiled-code damage rather than anything a
/// program can write.
fn text_of<'a>(args: &'a [Value], slot: usize, what: &str, member: &str) -> Result<&'a str, Fault> {
    args[slot].as_text().ok_or_else(|| {
        Fault::fatal(format!(
            "{member} expected a `string` for `{what}`, got tag {}",
            args[slot].tag_byte()
        ))
    })
}

/// The `string` in `slot`, or `None` where the option was omitted.
fn optional_of(args: &[Value], slot: usize) -> Option<&str> {
    args[slot].as_text()
}

nvs_runtime::nvs_helper! {
    /// `Core\Mail::send(string $endpoint, array<string> $to, string $subject,
    /// string $text, {cc?, bcc?, replyTo?, html?}): void` — `rule:programs/framework-core-half`'s
    /// transport half, replacing `mail()`.
    ///
    /// The order below is the whole security argument, and it is the order the
    /// module doc's second section gives: the grant is asked about `$endpoint`
    /// before any block is read, every address is parsed before a socket is
    /// opened, and the message is composed before anything is sent — so a
    /// refusal costs no connection and reveals no configuration.
    fn nvs_core_mail_send(ctx, args: [8]) {
        let member = format!("{NAME}::send");

        let endpoint = text_of(args, ENDPOINT, "endpoint", &member)?;
        let to = addresses_of(args, TO, "to", &member)?;
        if to.is_empty() {
            return Err(Fault::thrown(format!(
                "{member}: `$to` is empty, and a message with no recipient is not one"
            )));
        }
        let cc = addresses_of(args, CC, "cc", &member)?;
        let bcc = addresses_of(args, BCC, "bcc", &member)?;
        let reply_to = match optional_of(args, REPLY_TO) {
            Some(text) => Some(address_of(text, "replyTo", &member)?),
            None => None,
        };
        let subject = text_of(args, SUBJECT, "subject", &member)?;
        let text = text_of(args, TEXT, "text", &member)?;
        let html = optional_of(args, HTML);

        let configured = endpoint_of(ctx, endpoint, &member)?;

        let mut message = compose(
            &configured.from,
            &to,
            &cc,
            subject,
            text,
            html,
            SystemTime::now(),
        );
        if let Some(reply_to) = &reply_to {
            message.insert_str(0, &format!("Reply-To: <{reply_to}>\r\n"));
        }

        // Bcc reaches the envelope and no header, which is the whole of what
        // makes it blind — the header set above never names it.
        let envelope: Vec<String> = to.iter().chain(&cc).chain(&bcc).cloned().collect();

        let address = resolve(&configured, &member)?;
        deliver(address, &configured, &envelope, &message, &member)?;
        Ok(Value::null())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The extension reader, over the three things an `EHLO` reply can do to
    /// it: name the endpoint on its first line, spell a keyword in whatever
    /// case it likes, and carry parameters after it.
    ///
    /// The greeting line matters because it is the one line that is *not* a
    /// keyword: a reader that did not skip it would read an endpoint calling
    /// itself `starttls.example.com` as an offer of `STARTTLS`, and offer to
    /// upgrade a connection nothing would answer.
    #[test]
    fn the_ehlo_reply_is_read_as_keywords_after_its_first_line() {
        let reply: Vec<String> = [
            "250-starttls.example.com at your service",
            "250-SIZE 35882577",
            "250-StartTls",
            "250 AUTH LOGIN PLAIN XOAUTH2",
        ]
        .iter()
        .map(|line| (*line).to_owned())
        .collect();

        // Case-insensitive, per RFC 5321 § 2.4, and found on a continuation
        // line as readily as on the last one.
        assert_eq!(advertised(&reply, "STARTTLS"), Some(""));
        assert_eq!(advertised(&reply, "size"), Some("35882577"));
        assert_eq!(advertised(&reply, "AUTH"), Some("LOGIN PLAIN XOAUTH2"));

        // And the greeting is not a keyword, however much it looks like one.
        let greeting_only = vec![reply[0].clone()];
        assert_eq!(advertised(&greeting_only, "STARTTLS"), None);

        // A keyword nobody offered is absent rather than empty.
        assert_eq!(advertised(&reply, "DSN"), None);
    }

    /// RFC 4616's framing, which is three fields and not two: the empty
    /// authorization identity in front is what makes the endpoint authorize as
    /// whoever authenticated, and a message missing it is refused by every
    /// server rather than read as a shorter one.
    #[test]
    fn auth_plain_sends_an_empty_authorization_identity_first() {
        let message = plain("postmaster@example.com", "hunter2");
        let decoded = STANDARD.decode(&message).expect("`plain` emits base64");

        assert_eq!(
            decoded, b"\0postmaster@example.com\0hunter2",
            "the PLAIN message is NUL-separated with an empty first field"
        );
        // The credential is never on the wire in any other spelling, so the
        // encoded form is the only thing a reply can quote back.
        assert!(!message.contains("hunter2"));
    }

    /// `rule:programs/framework-core-half`'s rule, over the two halves that could break it: the
    /// endpoint is a *name* the grant answers about, and there is no parameter
    /// anywhere on the row through which a program could name a host.
    ///
    /// An acceptance check in the goal records under `data/goals/`, and it asserts the
    /// **absence** deliberately: a later slice adding a `host` option would
    /// leave every other test in this file green.
    // covers: Core\Mail::send
    #[test]
    fn mail_sends_against_an_operator_named_endpoint_and_no_other() {
        let row = CLASS.methods[0];
        assert_eq!(row.name, "send");

        // Nothing on the surface names where the mail goes out through.
        let named: Vec<&str> = row
            .names
            .iter()
            .copied()
            .chain(OPTIONS.iter().map(|option| option.name))
            .collect();
        for forbidden in ["host", "port", "server", "url", "relay", "smtp", "from"] {
            assert!(
                !named.contains(&forbidden),
                "`{forbidden}` is on Core\\Mail::send's surface, so the endpoint is no longer the \
                 operator's alone"
            );
        }

        // The one thing that selects a deployment refuses a tainted argument.
        assert!(
            matches!(row.params[ENDPOINT], CoreTy::Text(Qual::Sink)),
            "the endpoint name must be a sink: it chooses between operator blocks"
        );

        // And it is granted by name, under `mail.send` and nothing wider.
        let capability = crate::registry::CAPABILITIES
            .iter()
            .find(|(class, member, _)| *class == NAME && *member == "send")
            .expect("Core\\Mail::send has a capability row");
        assert_eq!(capability.2, Some(nvs_config::Cap::MailSend));
    }

    /// The module doc's first section, as the property it claims: no value of
    /// an argument can add a header, because the encoder answers a header line
    /// with one token whenever the value could not be one.
    // covers: Core\Mail::send
    #[test]
    fn a_header_value_cannot_carry_a_line_break_through() {
        assert_eq!(header_value("Your receipt"), "Your receipt");
        let injected = header_value("Hi\r\nBcc: attacker@example.com");
        assert!(
            injected.starts_with("=?UTF-8?B?") && !injected.contains('\r'),
            "an injected header survived encoding: {injected}"
        );
    }

    /// `rule:errors/ambiguous-input-refused` on the reader every directive comes through: a blank value is a
    /// cleared setting and reads as absent, and a value that is not blank is
    /// answered byte for byte.
    ///
    /// The second half is the one with a bug behind it. A credential is exactly
    /// as the operator wrote it (`rule:config/a-secret-is-a-file-whose-content-is-the-value`), so a password with an edge space
    /// is a password with an edge space; this reader trimmed it, and the only
    /// place that showed was a rejected `AUTH PLAIN` against a file that held
    /// the right bytes.
    #[test]
    fn a_configured_value_is_read_or_absent_and_never_repaired() {
        assert_eq!(present(String::new()), None);
        assert_eq!(present("   ".to_string()), None);
        assert_eq!(present("\t\n".to_string()), None);

        for verbatim in [" hunter2 ", "hunter2 ", " hunter2", "hun ter2", "hunter2"] {
            assert_eq!(
                present(verbatim.to_string()).as_deref(),
                Some(verbatim),
                "a non-blank value is the operator's, byte for byte",
            );
        }
    }

    /// `rule:errors/ambiguous-input-refused`, on the parameter where repair is most tempting: an address
    /// with a line break in it is refused rather than trimmed.
    #[test]
    fn an_address_is_refused_rather_than_repaired() {
        let member = "Core\\Mail::send";
        assert!(address_of("user@example.com", "to", member).is_ok());
        for bad in [
            "user@example.com\r\nRCPT TO:<victim@example.com>",
            "a@b.c, d@e.f",
            "<user@example.com>",
            "userexample.com",
            "",
        ] {
            assert!(
                address_of(bad, "to", member).is_err(),
                "`{bad}` was accepted as an address"
            );
        }
    }

    /// The one date this module writes, against a value with a known answer.
    #[test]
    fn the_date_header_is_rfc_5322() {
        let epoch = UNIX_EPOCH + Duration::from_secs(1_756_684_800);
        assert_eq!(rfc5322_date(epoch), "Mon, 01 Sep 2025 00:00:00 +0000");
        assert_eq!(rfc5322_date(UNIX_EPOCH), "Thu, 01 Jan 1970 00:00:00 +0000");
    }

    /// A `bcc` recipient reaches the envelope and no header — asserted over the
    /// composed message, because that is the only place the difference shows.
    // covers: Core\Mail::send
    #[test]
    fn a_bcc_recipient_is_in_no_header() {
        let message = compose(
            "noreply@example.com",
            &["to@example.com".to_owned()],
            &["cc@example.com".to_owned()],
            "Hello",
            "body",
            None,
            UNIX_EPOCH,
        );
        assert!(message.contains("To: to@example.com"));
        assert!(message.contains("Cc: cc@example.com"));
        assert!(!message.contains("bcc@example.com"));
    }
}
