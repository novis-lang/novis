//! `Core\Mail` — `rule:programs/framework-core-half`'s transport
//! half: one member that hands a message to an SMTP endpoint **an operator named**, and no spelling
//! anywhere for one a program chose.
//!
//! `rule:core-api/tier-roster` places the class — Native by
//! test 3, because it waits on the outside world. It is an SMTP client with structured headers,
//! replacing `mail()`. What belongs here is its roster, and the five decisions behind it.
//!
//! # `mail()`'s fourth argument is the whole bug
//!
//! PHP's `mail($to, $subject, $message, $additional_headers)` lets a program concatenate a header
//! block. Every mail header injection in twenty years is that argument: a `\r\n` inside a `$to` or a
//! `$subject` reaching it verbatim, and one more `Bcc:` arriving at the far end. There is no
//! escaping convention that fixes it, because the caller is the one holding the separator.
//!
//! So this class has **no raw header parameter at all**. Every header a program can set is a named
//! parameter of [`send`](CLASS) with a type, and [`compose`](fn@compose) is the only thing that ever writes a
//! `\r\n` — an address is parsed and refused if it is not one
//! (`rule:errors/ambiguous-input-refused`: ambiguous input is refused, never repaired), and a
//! subject is RFC 2047 encoded the moment it holds anything a header
//! line cannot carry, so a control byte becomes *content* rather than structure. That is what
//! structured headers buy, and it is why `$subject` and `$text` accept a `tainted`
//! argument freely: they are data in exactly `rule:security/log-is-not-a-sink`'s sense, and
//! mailing what a user typed is the point.
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
//! and a mail relay lives at `127.0.0.1` or on a container network precisely inside the set that rule
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
//! # Attachments and inline parts are composition, and composition is the program's
//!
//! `rule:programs/framework-core-half` puts this class's line exactly there. Two body parts are here because the
//! transport has to choose a `Content-Type` regardless and a mail with no plain-text alternative is
//! a mail half its readers cannot read; anything richer — templates, files, `multipart/related` —
//! belongs to the program or a package, which composes *into* these arguments.
//!
//! **Declined:** keeping an SMTP session open between messages. Each send opens its own
//! connection and closes it, and no measured program sends enough mail for the handshake to count.
//!
//! # Where each part lives
//!
//! This file has the class row, the reading of `[mail.<name>]` and `send`. [`compose`](mod@compose) writes the
//! message, and [`smtp`] is the session that delivers it.

mod compose;
mod smtp;

use std::cell::Cell;
use std::net::SocketAddr;
use std::time::{Duration, Instant, SystemTime};

use compose::{address_of, compose};
use nvs_runtime::spool::Outcome;
use nvs_runtime::{Ctx, Fault, Tag, ThrownClass, TraceKind, Value};
use nvs_syntax::duration;

use crate::registry::{
    ClassDoc, Const, CoreClass, CoreMethod, CoreOption, CoreTy, ErrorDoc, MethodDoc, ParamDoc,
    Qual, TextOrigin,
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
const DEFAULT_TIMEOUT: Duration = nvs_config::defaults::MAIL_TIMEOUT;

/// The submission port a block that names none is read as meaning.
///
/// 25 rather than 587, because a block that names no port is the unauthenticated
/// local-relay shape and 587 is the *authenticated* submission port. An operator
/// writing the `user` that port exists for writes `port = 587` beside it.
const DEFAULT_PORT: u16 = nvs_config::defaults::MAIL_PORT;

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
/// See [`ENDPOINT`].
const FROM_NAME: usize = 8;
/// See [`ENDPOINT`].
const IN_REPLY_TO: usize = 9;
/// See [`ENDPOINT`].
const REFERENCES: usize = 10;
/// See [`ENDPOINT`].
const UNSUBSCRIBE: usize = 11;
/// See [`ENDPOINT`].
const AUTO_SUBMITTED: usize = 12;

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
    CoreOption {
        name: "fromName",
        ty: CoreTy::Text(Qual::Neutral),
        default: Const::Null,
    },
    CoreOption {
        name: "inReplyTo",
        ty: CoreTy::Text(Qual::Neutral),
        default: Const::Null,
    },
    CoreOption {
        name: "references",
        ty: CoreTy::Array(&CoreTy::Text(Qual::Neutral)),
        default: Const::EmptyArray,
    },
    CoreOption {
        name: "unsubscribe",
        ty: CoreTy::Text(Qual::Neutral),
        default: Const::Null,
    },
    CoreOption {
        name: "autoSubmitted",
        ty: CoreTy::Bool,
        default: Const::Bool(true),
    },
];

/// `Core\Mail`'s one row — `rule:programs/framework-core-half`'s transport half.
///
/// The SMTP class, and `rule:programs/framework-core-half`'s transport half. Registered on its own
/// rather than beside `Core\Http` because it shares nothing with it: the
/// endpoint is an operator-named block and not a program-supplied URL, so
/// `rule:http-server/allow-url-pins-the-address`'s launderer is not in the path at all. [`crate::mail`] owns why
/// `mail()`'s fourth argument has no successor here.
// rank: 880
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
        return_ty: CoreTy::Str,
        text: TextOrigin::Clean(
            "The message id is random hex digits and the domain of the operator's `from`.",
        ),
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
    short: "Sends one message to the SMTP server that `[mail.$endpoint]` names. It returns when \
            that server has accepted the message. Under `nvs test`, it sends nothing. \
            `Core\\Test::sentMail` returns the message.",
    params: &[
        ParamDoc {
            name: "endpoint",
            desc: "The name of the `[mail.<name>]` block in `nvs.toml` to send through. A \
                   value that comes from the request is `tainted`, and a `tainted` value does \
                   not compile here. Write the name in your code.",
            shape: &[],
        },
        ParamDoc {
            name: "to",
            desc: "The recipients, one address per array entry. It is a list because a \
                   comma-separated string can be read in more than one way.",
            shape: &[],
        },
        ParamDoc {
            name: "subject",
            desc: "The subject. If it contains characters that a header line cannot contain, it \
                   is encoded with RFC 2047. So no subject can add a header. A long subject is \
                   split over several header lines, and a mail program joins them again.",
            shape: &[],
        },
        ParamDoc {
            name: "text",
            desc: "The plain-text body. Every message has one.",
            shape: &[],
        },
        ParamDoc {
            name: "cc",
            desc: "More recipients. Their addresses are shown in the message.",
            shape: &[],
        },
        ParamDoc {
            name: "bcc",
            desc: "More recipients. Their addresses are not shown in the message. They are sent \
                   to the server only, and no header contains them.",
            shape: &[],
        },
        ParamDoc {
            name: "replyTo",
            desc: "The address that replies go to, when it is not the sender in the settings.",
            shape: &[],
        },
        ParamDoc {
            name: "html",
            desc: "An HTML version of `$text`. If you give it, the message is \
                   `multipart/alternative` and both versions are sent.",
            shape: &[],
        },
        ParamDoc {
            name: "fromName",
            desc: "The sender name that mail programs show, such as `Example Shop`. It replaces \
                   `from_name` of the block. The sender address is always `from` of the block.",
            shape: &[],
        },
        ParamDoc {
            name: "inReplyTo",
            desc: "The message id of the message this one answers. Mail programs then show both \
                   in one thread. Pass the id that `send` returned, without angle brackets.",
            shape: &[],
        },
        ParamDoc {
            name: "references",
            desc: "The message ids of the earlier messages in the thread, oldest first. Each \
                   one is a message id, as `send` returns it.",
            shape: &[],
        },
        ParamDoc {
            name: "unsubscribe",
            desc: "An `https://` URL that unsubscribes the recipient with one click. Mail \
                   programs show an unsubscribe button and send a POST request to this URL.",
            shape: &[],
        },
        ParamDoc {
            name: "autoSubmitted",
            desc: "Whether the message has the header `Auto-Submitted: auto-generated`. It is \
                   `true` by default, and then automatic replies such as out-of-office messages \
                   are not sent back. Use `false` for a message that a person wrote.",
            shape: &[],
        },
    ],
    ret: "The message id, such as `3f2a…@example.com`, without angle brackets. The mail server \
          accepted the message. Store the id to match a reply or a bounce to the message, or to \
          pass it to `inReplyTo` later.",
    errors: &[
        ErrorDoc {
            error: "RuntimeError",
            desc: "The capability `mail.send` does not allow `$endpoint`. Or no `[mail.<name>]` \
                   block with that name sets `host` or `from`. Or the block sets only one \
                   of `user` and `password`. Or an address is not a valid address. Or \
                   `inReplyTo` or `references` has a value that is not a message id. Or \
                   `unsubscribe` is not an `https://` URL. Each of these is a mistake in the \
                   settings or the call, not a failed send.",
        },
        ErrorDoc {
            error: "IOError",
            desc: "The mail server could not be reached, closed the connection, or rejected a \
                   command. In the last case, the message contains the SMTP reply. It is also \
                   thrown when a block sets a password and the server cannot receive it safely. \
                   That is when the server has no `STARTTLS`, has no `AUTH PLAIN`, or has a \
                   certificate that does not verify. In all these cases, nothing is sent \
                   unencrypted.",
        },
    ],
};

/// The `Core` symbol table's arm for this module — see [`crate::address`].
pub(crate) fn address(symbol: &str) -> Option<nvs_runtime::Entry> {
    Some(nvs_runtime::Entry::borrowed(match symbol {
        "nvs_core_mail_send" => (nvs_core_mail_send as *const ()).cast(),
        _ => return None,
    }))
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
    /// `from_name` — the display name `From:` writes, unless a call's `fromName` replaces it.
    from_name: Option<String>,
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
        from_name: configured(ctx, endpoint, "from_name"),
        timeout,
        credential,
    })
}

/// Where the operator's `host` resolves to.
///
/// **No `pin_host` and no `rule:security/net-address-policy` table**, which the module doc argues in full: this
/// address came out of root-owned configuration, so the authority that would
/// have granted an exception is the one that wrote it, and every ordinary relay
/// sits inside the ranges `rule:security/net-address-policy` denies by default.
///
/// The lookup runs on the blocking pool and waits no later than `until`.
///
/// # Errors
///
/// A catchable `IOError` for a name that resolves to nothing, or whose lookup
/// is still running at `until`.
fn resolve(endpoint: &Endpoint, member: &str, until: Instant) -> Result<SocketAddr, Fault> {
    nvs_runtime::capability::resolve_host(&endpoint.host, member, Some(until))
        .map(|address| SocketAddr::new(address, endpoint.port))
        .map_err(|_| {
            Fault::thrown_as(
                ThrownClass::Io,
                format!(
                    "{member} could not resolve {}, which `[mail]` names as an endpoint",
                    endpoint.host
                ),
            )
        })
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
    texts_of(args, slot, what, member)?
        .iter()
        .map(|text| address_of(text, what, member))
        .collect()
}

/// Every `string` in the `array<string>` at `slot`, in order.
///
/// # Errors
///
/// A [`Fault::fatal`] for an argument or element that is not what the row
/// declared, which the `array<string>` rules out from source.
fn texts_of(args: &[Value], slot: usize, what: &str, member: &str) -> Result<Vec<String>, Fault> {
    let Some(array) = args[slot].array_ptr() else {
        return Err(Fault::fatal(format!(
            "{member} expected {:?} for `{what}`, got tag {}",
            Tag::Array,
            args[slot].tag_byte()
        )));
    };
    let mut found = Vec::new();
    for value in crate::str::Elements::of(array) {
        let text = value.as_text().ok_or_else(|| {
            Fault::fatal(format!(
                "{member} expected a `string` in `{what}`, got tag {}",
                value.tag_byte()
            ))
        })?;
        found.push(text.to_owned());
    }
    Ok(found)
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
    /// string $text, {cc?, bcc?, replyTo?, html?, fromName?, inReplyTo?, references?,
    /// unsubscribe?, autoSubmitted?}): string` — `rule:programs/framework-core-half`'s transport
    /// half, replacing `mail()`. It returns the `Message-ID` it wrote, without angle brackets.
    ///
    /// The order below is the whole security argument, and it is the order the
    /// module doc's second section gives: the grant is asked about `$endpoint`
    /// before any block is read, every address is parsed before a socket is
    /// opened, and the message is composed before anything is sent — so a
    /// refusal costs no connection and reveals no configuration.
    ///
    /// A recorded request files one `mail` event for the call
    /// (`rule:observability/trace-events-carry-a-kind`), with the number of recipients and
    /// whether the send returned, and never an address, the subject or the body.
    fn nvs_core_mail_send(ctx, args: [13]) {
        let member = format!("{NAME}::send");
        let recipients = Cell::new(None);
        crate::site::watched_with(
            ctx,
            Value::null(),
            TraceKind::Mail,
            &member,
            None,
            |ctx| send(ctx, args, &member, &recipients),
            |sent| Outcome {
                recipients: recipients.get(),
                ok: Some(sent.is_some()),
                ..Outcome::default()
            },
        )
    }
}

/// `Core\Mail::send`'s body, which [`nvs_core_mail_send`] files the event around. `recipients`
/// is set to the number of `to`, `cc` and `bcc` addresses once all of them have been parsed.
fn send(
    ctx: &mut Ctx,
    args: &[Value],
    member: &str,
    recipients: &Cell<Option<u64>>,
) -> Result<Value, Fault> {
    let endpoint = text_of(args, ENDPOINT, "endpoint", member)?;
    let to = addresses_of(args, TO, "to", member)?;
    if to.is_empty() {
        return Err(Fault::thrown(format!(
            "{member}: `$to` is empty, and a message with no recipient is not one"
        )));
    }
    let cc = addresses_of(args, CC, "cc", member)?;
    let bcc = addresses_of(args, BCC, "bcc", member)?;
    recipients.set(u64::try_from(to.len() + cc.len() + bcc.len()).ok());
    let reply_to = match optional_of(args, REPLY_TO) {
        Some(text) => Some(address_of(text, "replyTo", member)?),
        None => None,
    };
    let subject = text_of(args, SUBJECT, "subject", member)?;
    let text = text_of(args, TEXT, "text", member)?;
    let html = optional_of(args, HTML);
    let in_reply_to = match optional_of(args, IN_REPLY_TO) {
        Some(id) => Some(compose::message_id_of(id, "inReplyTo", member)?),
        None => None,
    };
    let references = texts_of(args, REFERENCES, "references", member)?
        .iter()
        .map(|id| compose::message_id_of(id, "references", member))
        .collect::<Result<Vec<_>, _>>()?;
    let unsubscribe = match optional_of(args, UNSUBSCRIBE) {
        Some(url) => Some(compose::unsubscribe_of(url, member)?),
        None => None,
    };
    let auto_submitted = args[AUTO_SUBMITTED].as_bool().ok_or_else(|| {
        Fault::fatal(format!(
            "{member} expected a `bool` for `autoSubmitted`, got tag {}",
            args[AUTO_SUBMITTED].tag_byte()
        ))
    })?;

    let configured = endpoint_of(ctx, endpoint, member)?;
    let from_name = optional_of(args, FROM_NAME).or(configured.from_name.as_deref());
    let mut random = [0_u8; 16];
    crate::random::draw(ctx, |rng| rand::Rng::fill_bytes(rng, &mut random));
    let message_id = compose::message_id(random, &configured.from);

    // Under `nvs test`, the message is kept for `Core\Test::sentMail` and no connection is
    // opened, so a test never reaches a real person
    // (`rule:testing/a-test-reads-its-logs-and-mail`). The grant, the block and every address
    // were checked above, as they are in production.
    if let Some(records) = ctx.test_records() {
        records.mail(nvs_runtime::test_records::KeptMail {
            endpoint: endpoint.to_owned(),
            to,
            subject: subject.to_owned(),
            text: text.to_owned(),
            cc,
            bcc,
            reply_to,
            html: html.map(str::to_owned),
        });
        return Ok(Value::text(&message_id));
    }

    let message = compose(
        &compose::Headers {
            from: &configured.from,
            from_name,
            to: &to,
            cc: &cc,
            reply_to: reply_to.as_deref(),
            subject,
            message_id: &message_id,
            in_reply_to: in_reply_to.as_deref(),
            references: &references,
            unsubscribe: unsubscribe.as_deref(),
            auto_submitted,
            now: SystemTime::now(),
        },
        text,
        html,
    );

    // Bcc reaches the envelope and no header, which is the whole of what
    // makes it blind — the header set above never names it.
    let envelope: Vec<String> = to.iter().chain(&cc).chain(&bcc).cloned().collect();

    // The configured `timeout` bounds the whole exchange. The scheduler
    // bounds every park under it by the request's wall deadline, and ends
    // the request there (`rule:errors/on-limit`). The name lookup runs off
    // the core, so it also stops at the request's wall deadline itself.
    let until = Instant::now() + configured.timeout;
    let lookup_until = ctx
        .off_core_deadline()
        .map_or(until, |wall| until.min(wall));
    resolve(&configured, member, lookup_until).and_then(|address| {
        smtp::deliver(address, &configured, until, &envelope, &message, member)
    })?;
    Ok(Value::text(&message_id))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `rule:programs/framework-core-half`'s rule, over the two halves that could break it: the
    /// endpoint is a *name* the grant answers about, and there is no parameter
    /// anywhere on the row through which a program could name a host.
    ///
    /// An acceptance check in a goal file under `docs/goals/`, and it asserts the
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
}
