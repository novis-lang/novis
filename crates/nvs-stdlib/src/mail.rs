//! `Core\Mail` — [ADR 0082](../../../../docs/adr/0082-the-first-party-framework.md) § 2's transport
//! half: one member that hands a message to an SMTP endpoint **an operator named**, and no spelling
//! anywhere for one a program chose.
//!
//! [ADR 0051](../../../../docs/adr/0051-standard-library-tiers.md) § 3 places the class — Native by
//! test 3, because it waits on the outside world — and the spec's § 16 row gives it one sentence,
//! "an SMTP client with structured headers, replacing `mail()`". What belongs here is the roster
//! that sentence does not write, and the four decisions behind it.
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
//! 0095](../../../../docs/adr/0095-ambiguous-input-is-refused-never-repaired.md): ambiguous input is
//! refused, never repaired), and a subject is RFC 2047 encoded the moment it holds anything a header
//! line cannot carry, so a control byte becomes *content* rather than structure. That is what the
//! spec row's "structured headers" buys, and it is why `$subject` and `$text` accept a `tainted`
//! argument freely: they are data in exactly [ADR
//! 0088](../../../../docs/adr/0088-a-sink-is-an-instruction-and-the-default-refuses.md) § 7's sense,
//! and mailing what a user typed is the point.
//!
//! # The endpoint is a name, not a host — ADR 0067 § 3's shape
//!
//! `$endpoint` selects a `[mail.<name>]` block in root-owned `nvs.toml`, granted by name under the
//! `mail.send` capability. There is no host parameter, no port and no credential on the surface, so
//! there is nothing for an attacker-influenced value to redirect: the worst a forged `$endpoint` can
//! name is another block the same operator wrote and the same grant listed. It is a
//! [`Qual::Sink`] for that last inch — a `tainted` block name is a program letting input choose
//! between deployments — and it is the only qualifier this class refuses.
//!
//! ADR 0067 § 3 is where that shape comes from and also why the address is **not** put through
//! [ADR 0058](../../../../docs/adr/0058-outbound-request-policy.md) § 3's denied ranges: an address
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
//! # Known gaps, both recorded rather than worked around
//!
//! **There is no TLS, so there is no `AUTH`.** `nvs-stdlib` has no TLS stack yet — `crate::http`'s
//! transport is plaintext HTTP/1.1 for the same reason — and sending a credential over a cleartext
//! socket is not a thing this class will do quietly. A `[mail.<name>]` block that sets `user` or
//! `password` is therefore **refused at the send**, naming the gap, rather than authenticating in
//! the clear; what works today is the ordinary shape of a local or sidecar relay that accepts
//! unauthenticated submission from its own network. When a TLS-capable stream lands for
//! `Core\Http\Client`, `STARTTLS` and `AUTH PLAIN` are one function each on top of [`Session`], and
//! the refusal below is what gets deleted.
//!
//! **Attachments and inline parts are composition, and composition is § 3's.** ADR 0082 § 2 splits
//! this class at exactly that line. Two body parts are here because the transport has to choose a
//! `Content-Type` regardless and a mail with no plain-text alternative is a mail half its readers
//! cannot read; anything richer — templates, files, `multipart/related` — belongs to the `nvs/web`
//! package, which composes *into* these arguments.

use std::io::{Read as _, Write as _};
use std::net::{SocketAddr, ToSocketAddrs as _};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use nvs_host::net::NvsTcp;
use nvs_runtime::{Ctx, Fault, Tag, ThrownClass, Value};
use nvs_syntax::duration;

use crate::registry::{
    Const, CoreClass, CoreMethod, CoreOption, CoreTy, ErrorDoc, MethodDoc, ParamDoc, Qual,
};

/// The class name, as `registry::CAPABILITIES` and a refusal both spell it.
pub(crate) const NAME: &str = r"Core\Mail";

/// How long the whole exchange may take — connect, banner, every command and the
/// `DATA` body together.
///
/// One bound rather than one per step, for ADR 0074 § 5's reason: a per-step
/// timeout multiplied by the number of steps is the unbounded wait spelled
/// long-hand, and an SMTP conversation has as many steps as there are
/// recipients.
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);

/// The submission port a block that names none is read as meaning.
///
/// 25 rather than 587, because 587 is the *authenticated* submission port and
/// this class cannot authenticate yet — the module doc's first known gap. An
/// operator running a relay on 587 writes `port = 587`.
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
/// comma-separated recipient list is the parse ADR 0095 refuses: `mail()` splits
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

/// `Core\Mail`'s one row — ADR 0082 § 2's transport half.
pub(crate) const CLASS: CoreClass = CoreClass {
    name: NAME,
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

/// `Core\Mail::send`'s reference card — ADR 0117.
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
                   block of that name sets `host` or `from`; or that block sets `user` or \
                   `password`, which cannot be sent without TLS; or an address is not one. Each is \
                   a deployment or a call that was written wrong, not a send that failed.",
        },
        ErrorDoc {
            error: "IOError",
            desc: "The configured endpoint could not be reached, closed the connection, or \
                   refused a command — the last carrying the SMTP reply that said so.",
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
}

/// The directive `[mail.<name>] key`, with an empty value read as absent.
///
/// Absent and blank are one answer for [`crate::cache`]'s reason: `host = ""` is
/// an operator clearing a setting, and reading it as a name would produce a
/// refusal about DNS rather than about the configuration.
fn configured(ctx: &Ctx, endpoint: &str, key: &str) -> Option<String> {
    ctx.config()
        .and_then(|config| config.get(&format!("mail.{endpoint}.{key}")))
        .map(|text| text.trim().to_owned())
        .filter(|text| !text.is_empty())
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
/// `host` or no `from`, and for one that carries a credential this class cannot
/// yet protect.
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

    // The module doc's first known gap, refused rather than honoured: there is
    // no TLS under this socket, so honouring it would put the credential on the
    // wire in the clear.
    if configured(ctx, endpoint, "user").is_some()
        || configured(ctx, endpoint, "password").is_some()
    {
        return Err(Fault::thrown(format!(
            "{member}: `[mail.{endpoint}]` sets a credential, and this transport has no TLS to \
             send one under — use an endpoint that accepts unauthenticated submission from this \
             host until it does"
        )));
    }

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
    })
}

/// Where the operator's `host` resolves to.
///
/// **No `pin_host` and no § 3 table**, which the module doc argues in full: this
/// address came out of root-owned configuration, so the authority that would
/// have granted an exception is the one that wrote it, and every ordinary relay
/// sits inside the ranges ADR 0058 § 3 denies by default.
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
/// `@` are each a refusal. ADR 0095 is why none of them is silently stripped:
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
    /// The socket, which hands the core back rather than blocking it.
    stream: NvsTcp,
    /// Whatever the last read took past the end of the reply it was completing.
    held: Vec<u8>,
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
            stream,
            held: Vec::new(),
        };
        session.expect(220, "the greeting", member)?;
        Ok(session)
    }

    /// One command, and the reply code it was answered with.
    ///
    /// # Errors
    ///
    /// A catchable `IOError` when the write fails, when the connection ends
    /// mid-reply, or when the code is not `want`.
    fn command(&mut self, line: &str, want: i64, what: &str, member: &str) -> Result<(), Fault> {
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

    /// Reads one whole reply and holds it to `want`.
    ///
    /// # Errors
    ///
    /// A catchable `IOError` for a closed connection, a reply that is not
    /// three digits, or a code other than `want` — the last carrying the
    /// server's own sentence, because that is the only part of the failure the
    /// operator did not already write.
    fn expect(&mut self, want: i64, what: &str, member: &str) -> Result<(), Fault> {
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
                if code == want {
                    return Ok(());
                }
                return Err(Fault::thrown_as(
                    ThrownClass::Io,
                    format!("{member}: the endpoint refused {what} — {line}"),
                ));
            }
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

/// The whole exchange, once every argument has been checked and every
/// deployment question answered.
///
/// # Errors
///
/// A catchable `IOError` for anything the endpoint said or did — see
/// [`Session`].
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
    session.command(&format!("EHLO {domain}"), 250, "the greeting", member)?;
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
    /// string $text, {cc?, bcc?, replyTo?, html?}): void` — ADR 0082 § 2's
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

    /// ADR 0082 § 2's rule, over the two halves that could break it: the
    /// endpoint is a *name* the grant answers about, and there is no parameter
    /// anywhere on the row through which a program could name a host.
    ///
    /// The acceptance check `loop-goal.toml` names, and it asserts the
    /// **absence** deliberately: a later slice adding a `host` option would
    /// leave every other test in this file green.
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
    #[test]
    fn a_header_value_cannot_carry_a_line_break_through() {
        assert_eq!(header_value("Your receipt"), "Your receipt");
        let injected = header_value("Hi\r\nBcc: attacker@example.com");
        assert!(
            injected.starts_with("=?UTF-8?B?") && !injected.contains('\r'),
            "an injected header survived encoding: {injected}"
        );
    }

    /// ADR 0095, on the parameter where repair is most tempting: an address
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
