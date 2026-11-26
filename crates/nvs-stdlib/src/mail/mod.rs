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
//! who sent it. The DKIM signature is the same kind of fact: a block with `dkim_selector` and
//! `dkim_key_file` signs every message it sends, in [`dkim`], and no call site names a key.
//!
//! # TLS is asked for, and where it is asked for it is required and verified
//!
//! `[mail.<name>] tls` is `implicit` (TLS from the first byte, RFC 8314's port 465), `starttls`
//! (the upgrade after the first `EHLO`) or `none`. Unset, it is `implicit` on port 465,
//! `starttls` when the block sets a credential, and `none` otherwise. A credential — `user` and
//! `password`, or `password_file`, the same half arriving as an injected file
//! (`rule:config/a-secret-is-a-file-whose-content-is-the-value`) — beside `tls = "none"` is refused
//! at boot by [`nvs_config::mail::validate`], again by [`endpoint_of`], and a third time where
//! [`smtp`] writes the password. So no path sends it to a plaintext socket.
//!
//! Every TLS session is verified against the name the operator wrote as `host`, and against
//! [`nvs_host::tls`]'s compiled-in anchors or the block's `tls_ca_file`, which replaces them for
//! that block alone. An endpoint that does not offer `STARTTLS` where the block asks for it, or
//! whose certificate does not verify, is refused with nothing sent. `EHLO` is re-issued over the
//! secured stream because the extension list is the *session's*, and an endpoint offering `AUTH`
//! only to a secured client is the ordinary case. The credential goes with `AUTH PLAIN`, or
//! `AUTH LOGIN` where only that is offered or the block's `auth` names it. A block with
//! `auth = "xoauth2"` has no password: it signs `user` in with a token [`oauth`] fetches from the
//! block's `oauth_token_url`, and that token asks for TLS as a password does.
//!
//! **Declined: opportunistic TLS.** It is one of two things. Verified, an internal relay with a
//! certificate the block did not plan for stops working on a setting nobody wrote. Unverified,
//! it is a handshake nobody checked, which [`nvs_host::tls`] has no spelling for and is the false
//! confidence `rule:security/launderers-are-sink-named` refuses to sell. So `starttls` with no
//! credential is the internal-relay case, written out, and verified like any other.
//!
//! # `Core` composes the whole message, and HTML is only ever `Core\Html\Markup`
//!
//! `rule:programs/framework-core-half` gives this class the transport and the message. The rule of
//! the message is `rule:core-classes/mail-message`, and of the block that sends it
//! `rule:core-classes/mail-transport`. Every part a program can send is a typed parameter, so [`compose`](fn@compose) stays the one writer of a
//! `\r\n`. A raw MIME parameter for packages is declined, because it reopens the injection the
//! class closes.
//!
//! `$body` is a `string`, sent as `text/plain`, or a `Core\Html\Markup`, sent as HTML with a
//! plain-text alternative [`parts::read_html`] builds from it. There is no third way to send HTML,
//! so a `tainted` string, which can never become `Markup` unescaped
//! (`rule:core-classes/html-auto-escape`), is always sent as text. Attachments and inline images
//! are [`parts::ATTACHMENT`] values, and [`compose`](fn@compose) nests them `multipart/mixed` >
//! `multipart/related` > `multipart/alternative`, each level only when it is needed.
//!
//! The CSS of an HTML body is always inlined: [`css`] copies each `<style>` rule onto the
//! elements it matches, because most mail programs drop a `<style>` element and none loads a
//! stylesheet. `preview` reads a body through the same [`message_of`] as `send`, so what it
//! returns is what `send` sends.
//!
//! # A block's session is kept for its next send
//!
//! Over TLS, each new connection pays a handshake, two `EHLO`s and `AUTH` before the envelope.
//! [`pool`] keeps a session open per block per core under `[mail.<name>.pool]`, the bounds
//! `[db.<name>.pool]` has, and starts each later message on it with `RSET`. Its module doc says
//! what it spends and when a session is replaced. `pool = false` opens one connection per message.
//!
//! # Where each part lives
//!
//! This file has the class row, the reading of `[mail.<name>]` and `send`. [`compose`](mod@compose) writes the
//! message, [`parts`] the attachments and the plain text, [`css`] the inlining and `preview`'s
//! value, [`smtp`] is the session that delivers it, [`pool`] keeps that session open for the
//! block's next send, and [`oauth`] fetches and keeps an `xoauth2` block's token.

mod capture;
mod compose;
mod css;
mod dkim;
mod oauth;
mod parts;
mod pool;
mod smtp;

pub(crate) use css::PREVIEW;
pub(crate) use parts::{ATTACHMENT, ATTACHMENT_NAME, built as attachment};
#[cfg(test)]
pub(crate) use parts::{
    nvs_core_mail_attachment_bytes, nvs_core_mail_attachment_is_inline,
    nvs_core_mail_attachment_name, nvs_core_mail_attachment_type,
};

use std::cell::Cell;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::time::{Duration, Instant, SystemTime};

use compose::{Mailbox, address_of, compose};
use nvs_config::mail::{Auth, Tls};
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
const BODY: usize = 3;
/// See [`ENDPOINT`].
const CC: usize = 4;
/// See [`ENDPOINT`].
const BCC: usize = 5;
/// See [`ENDPOINT`].
const REPLY_TO: usize = 6;
/// See [`ENDPOINT`].
const TEXT: usize = 7;
/// See [`ENDPOINT`].
const FROM_NAME: usize = 8;
/// See [`ENDPOINT`].
const ATTACHMENTS: usize = 9;
/// See [`ENDPOINT`].
const IN_REPLY_TO: usize = 10;
/// See [`ENDPOINT`].
const REFERENCES: usize = 11;
/// See [`ENDPOINT`].
const UNSUBSCRIBE: usize = 12;
/// See [`ENDPOINT`].
const AUTO_SUBMITTED: usize = 13;
/// See [`ENDPOINT`].
const BOUNCE_TAG: usize = 14;

/// `$body`: a `string`, sent as plain text, or a `Core\Html\Markup`, sent as HTML. A `tainted`
/// string is a string, so it can only ever be sent as text.
const BODY_TY: CoreTy = CoreTy::Union(&[
    CoreTy::Text(Qual::Neutral),
    CoreTy::Instance(crate::html::MARKUP_NAME),
]);

/// [`send`](CLASS)'s trailing shape — every header a program may set that is not
/// required to have a message at all.
///
/// `cc` and `bcc` are arrays of [`RECIPIENT`] and `to` is one as well, because a
/// comma-separated recipient list is the parse `rule:errors/ambiguous-input-refused` refuses: `mail()` splits
/// its `$to` on commas, which makes a comma inside a display name a second
/// recipient. A list has no such reading, and a name is a [`ADDRESS`]'s own
/// argument. A single recipient writes `["a@b.c"]` and pays two characters for it.
const OPTIONS: &[CoreOption] = &[
    CoreOption {
        name: "cc",
        ty: CoreTy::Array(&RECIPIENT),
        default: Const::EmptyArray,
    },
    CoreOption {
        name: "bcc",
        ty: CoreTy::Array(&RECIPIENT),
        default: Const::EmptyArray,
    },
    CoreOption {
        name: "replyTo",
        ty: RECIPIENT,
        default: Const::Null,
    },
    CoreOption {
        name: "text",
        ty: CoreTy::Text(Qual::Neutral),
        default: Const::Null,
    },
    CoreOption {
        name: "fromName",
        ty: CoreTy::Text(Qual::Neutral),
        default: Const::Null,
    },
    CoreOption {
        name: "attachments",
        ty: CoreTy::Array(&CoreTy::Instance(parts::ATTACHMENT_NAME)),
        default: Const::EmptyArray,
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
    CoreOption {
        name: "bounceTag",
        ty: CoreTy::Text(Qual::Neutral),
        default: Const::Null,
    },
];

/// `preview`'s `$body`. The `string` is a sink, unlike `send`'s: `Core\Mail\Preview::text()`
/// returns it as untainted text, so a `tainted` one is refused where it is written.
const PREVIEW_BODY_TY: CoreTy = CoreTy::Union(&[
    CoreTy::Text(Qual::Sink),
    CoreTy::Instance(crate::html::MARKUP_NAME),
]);

/// `preview`'s bag: the two options of `send` that change the body. `text` is a sink for
/// [`PREVIEW_BODY_TY`]'s reason.
const PREVIEW_OPTIONS: &[CoreOption] = &[
    CoreOption {
        name: "text",
        ty: CoreTy::Text(Qual::Sink),
        default: Const::Null,
    },
    CoreOption {
        name: "attachments",
        ty: CoreTy::Array(&CoreTy::Instance(parts::ATTACHMENT_NAME)),
        default: Const::EmptyArray,
    },
];

/// `Core\Mail::preview`'s reference card — `rule:core-api/reference-card`.
const PREVIEW_DOC: MethodDoc = MethodDoc {
    short: "Returns the body of a message exactly as `send` would send it, and sends nothing. \
            You can show it on a page, or check it in a test. It needs no `[mail.<name>]` \
            block.",
    params: &[
        ParamDoc {
            name: "body",
            desc: "The message, as for `send`. In a `Core\\Html\\Markup` body, each CSS rule of a \
                   `<style>` element is copied into the `style` attribute of each element it \
                   matches. A string from the request is `tainted`, which means it may contain \
                   anything a visitor typed. A `tainted` string does not compile here, because \
                   `text()` returns it as text that is not `tainted`.",
            shape: &[],
        },
        ParamDoc {
            name: "text",
            desc: "The plain-text version of an HTML body, as for `send`.",
            shape: &[],
        },
        ParamDoc {
            name: "attachments",
            desc: "The attachments, as for `send`. Each inline image needs its `src()` in the \
                   HTML, and each `cid:` URL in the HTML needs its inline image.",
            shape: &[],
        },
    ],
    ret: "A `Core\\Mail\\Preview` with the HTML and the plain text that `send` would send, and \
          the CSS rules that stay in a `<style>` element.",
    errors: &[ErrorDoc {
        error: "RuntimeError",
        desc: "The body or the attachments have a mistake that `send` throws for. For example, \
               the HTML has a `<link rel=\"stylesheet\">`. A mail cannot load a stylesheet, so \
               put the CSS in a `<style>` element.",
    }],
};

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
    methods: &[
        CoreMethod {
            name: "send",
            names: &["endpoint", "to", "subject", "body"],
            params: &[
                CoreTy::Text(Qual::Sink),
                CoreTy::Array(&RECIPIENT),
                CoreTy::Text(Qual::Neutral),
                BODY_TY,
                CoreTy::Options(OPTIONS),
            ],
            defaults: &[],
            return_ty: CoreTy::Str,
            text: TextOrigin::Clean(
                "The message id is random hex digits and the domain of the operator's `from`.",
            ),
            symbol: "nvs_core_mail_send",
            doc: Some(&SEND_DOC),
        },
        CoreMethod {
            name: "preview",
            names: &["body"],
            params: &[PREVIEW_BODY_TY, CoreTy::Options(PREVIEW_OPTIONS)],
            defaults: &[],
            return_ty: CoreTy::Instance(css::PREVIEW_NAME),
            text: TextOrigin::Unmarked,
            symbol: "nvs_core_mail_preview",
            doc: Some(&PREVIEW_DOC),
        },
    ],
    instance: &[],
    slots: &[],
    constants: &[],
};

/// The name of the class a named recipient is.
pub(crate) const ADDRESS_NAME: &str = r"Core\Mail\Address";

/// One recipient of `send`: a plain address, or a [`ADDRESS`] with a display name.
///
/// A plain string is an address only, and never read for a name: `"Ada <ada@example.com>"` is
/// refused as an address (`rule:errors/ambiguous-input-refused`), because two readers split such
/// a string differently.
const RECIPIENT: CoreTy =
    CoreTy::Union(&[CoreTy::Text(Qual::Neutral), CoreTy::Instance(ADDRESS_NAME)]);

/// Slot 0 of an [`ADDRESS`]: the address as the program wrote it. `send` reads it through
/// [`address_of`] again, so a value rebuilt from bytes is checked too.
const ADDRESS_TEXT: usize = 0;

/// Slot 1 of an [`ADDRESS`]: the display name, any text.
const ADDRESS_DISPLAY: usize = 1;

/// `Core\Mail\Address` — an address with the display name a mail reader shows for it.
///
/// A value class with one constructor and no reader: what it is for is to be passed to `send`.
/// `of` checks the address at once, so a wrong one throws where it was written. The name is
/// any text, `tainted` included, for the same reason a subject is: `compose` encodes it, so it
/// is content and never a header.
// rank: 881
pub(crate) const ADDRESS: CoreClass = CoreClass {
    name: ADDRESS_NAME,
    doc: Some(&ADDRESS_CARD),
    methods: &[CoreMethod {
        name: "of",
        names: &["address", "name"],
        params: &[CoreTy::Text(Qual::Neutral), CoreTy::Text(Qual::Neutral)],
        defaults: &[],
        return_ty: CoreTy::Instance(ADDRESS_NAME),
        text: TextOrigin::Unmarked,
        symbol: "nvs_core_mail_address_of",
        doc: Some(&ADDRESS_OF_DOC),
    }],
    instance: &[],
    slots: &["address", "name"],
    constants: &[],
};

/// `Core\Mail\Address`'s class card — `rule:core-api/reference-card`.
const ADDRESS_CARD: ClassDoc = ClassDoc {
    short: "An email address with a name. A mail program shows the name in place of the address. \
            You can pass it to `Core\\Mail::send` as a recipient, a `cc`, a `bcc` or the `replyTo`.",
};

/// `Core\Mail\Address::of`'s reference card — `rule:core-api/reference-card`.
const ADDRESS_OF_DOC: MethodDoc = MethodDoc {
    short: "Returns an address with a name, such as `\"Ada Lovelace\" <ada@example.com>`.",
    params: &[
        ParamDoc {
            name: "address",
            desc: "The email address, such as `ada@example.com`. A domain with letters that are \
                   not ASCII is sent in its `xn--` form.",
            shape: &[],
        },
        ParamDoc {
            name: "name",
            desc: "The name to show, such as `Ada Lovelace`. It can be any text, including text \
                   a user typed. An empty name sends the address alone.",
            shape: &[],
        },
    ],
    ret: "The address and its name, to pass to `Core\\Mail::send`.",
    errors: &[ErrorDoc {
        error: "RuntimeError",
        desc: "`$address` is not an email address. A string such as `Ada <ada@example.com>` is \
               not an address either: pass the name as `$name`.",
    }],
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
            desc: "The recipients, one per array entry. Each is an address string, or a \
                   `Core\\Mail\\Address` to show a name with the address. A domain that is not \
                   ASCII is sent in its `xn--` form.",
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
            name: "body",
            desc: "The message. In a `Core\\Html\\Markup` body, each CSS rule of a `<style>` \
                   element is copied into the `style` attribute of each element it matches. \
                   Most mail programs ignore a `<style>` element. A `string` is sent as \
                   plain text. A `Core\\Html\\Markup` is sent as HTML, together with a \
                   plain-text version made from it. A `tainted` string, such as text from the \
                   request, is always sent as plain text. So it cannot add HTML to a message. `preview` returns the \
                   result without sending it.",
            shape: &[],
        },
        ParamDoc {
            name: "cc",
            desc: "More recipients, as strings or `Core\\Mail\\Address` values. Their addresses \
                   are shown in the message.",
            shape: &[],
        },
        ParamDoc {
            name: "bcc",
            desc: "More recipients, as strings or `Core\\Mail\\Address` values. Their addresses \
                   are not shown in the message. They are sent to the server only, and no \
                   header contains them.",
            shape: &[],
        },
        ParamDoc {
            name: "replyTo",
            desc: "The address that replies go to, when it is not the sender in the settings. A \
                   string or a `Core\\Mail\\Address`.",
            shape: &[],
        },
        ParamDoc {
            name: "text",
            desc: "The plain-text version of an HTML body, in place of the one made from the \
                   HTML. Links become their text and the URL in parentheses, and list items \
                   start with `- `. With a `string` body, this option throws an error.",
            shape: &[],
        },
        ParamDoc {
            name: "fromName",
            desc: "The sender name that mail programs show, such as `Example Shop`. It replaces \
                   `from_name` of the block. The sender address is always `from` of the block.",
            shape: &[],
        },
        ParamDoc {
            name: "attachments",
            desc: "Files to send with the message, and images to show inside the HTML, each a \
                   `Core\\Mail\\Attachment`. An inline image needs its `src()` in the HTML.",
            shape: &[],
        },
        ParamDoc {
            name: "inReplyTo",
            desc: "The message id of the message this one replies to. Mail programs then show both \
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
        ParamDoc {
            name: "bounceTag",
            desc: "A short tag, such as an order number, that is added to the bounce address. \
                   With `return_path = \"bounces@example.com\"` and the tag `order-42`, a bounce \
                   goes to `bounces+order-42@example.com`. The tag has 1 to 64 letters, digits, \
                   dots, underscores or hyphens.",
            shape: &[],
        },
    ],
    ret: "The message id, such as `3f2a…@example.com`, without angle brackets. The call returns \
          when the mail server has accepted the message, or when a `capture = true` block or \
          `nvs test` has kept it. Store the id to match a reply or a bounce to the message, or to \
          pass it to `inReplyTo` later.",
    errors: &[
        ErrorDoc {
            error: "RuntimeError",
            desc: "The settings or the arguments are wrong, and nothing is sent. The capability \
                   `mail.send` does not allow `$endpoint`, or no `[mail.<name>]` block has that \
                   name. The block sets only one of `user` and `password`. An address is not a \
                   valid address. A value in `inReplyTo` or `references` is not a message id. \
                   `unsubscribe` is not an `https://` URL. `bounceTag` has a character that is \
                   not allowed.",
        },
        ErrorDoc {
            error: "RuntimeError",
            desc: "The body or its attachments are wrong, and nothing is sent. `text` is given \
                   with a `string` body. The HTML uses a `cid:` URL that no inline attachment \
                   has. A `cid:` URL points at an image inside the same mail. An inline \
                   attachment is not used in the HTML. The HTML has a \
                   `<link rel=\"stylesheet\">`. A mail cannot load a stylesheet, so put the CSS \
                   in a `<style>` element.",
        },
        ErrorDoc {
            error: "IOError",
            desc: "The mail server could not be reached, closed the connection, or rejected a \
                   command. In the last case, the message contains the SMTP reply. It is also \
                   thrown when the block needs TLS and the server has no `STARTTLS`, or has a \
                   certificate that does not verify. It is thrown when the server offers neither \
                   `AUTH PLAIN` nor `AUTH LOGIN`, and when the message is larger than the server \
                   accepts. In all these cases, nothing is sent. If the server rejects a \
                   recipient, the message names every rejected address, and nobody gets the \
                   message. An address with letters that are not ASCII before the `@` needs a \
                   server that offers `SMTPUTF8`. With another server, nothing is sent.",
        },
    ],
};

/// The `Core` symbol table's arm for this module — see [`crate::address`].
pub(crate) fn address(symbol: &str) -> Option<nvs_runtime::Entry> {
    Some(nvs_runtime::Entry::borrowed(match symbol {
        "nvs_core_mail_send" => (nvs_core_mail_send as *const ()).cast(),
        "nvs_core_mail_address_of" => (nvs_core_mail_address_of as *const ()).cast(),
        "nvs_core_mail_preview" => (nvs_core_mail_preview as *const ()).cast(),
        _ => return parts::address(symbol).or_else(|| css::address(symbol)),
    }))
}

/// What `[mail.<name>]` said, once every question this member asks of it has an
/// answer.
struct Endpoint {
    /// The block's name, `<name>` in `[mail.<name>]`, which a message about a setting names.
    name: String,
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
    /// `user` and `password`, both or neither — the credential `AUTH` sends,
    /// and the thing whose presence asks for TLS when `tls` is unset.
    credential: Option<(String, String)>,
    /// An `auth = "xoauth2"` block's `user` and `oauth_*` keys. It asks for TLS as a
    /// `credential` does, and a block has at most one of the two.
    oauth: Option<oauth::Grant>,
    /// `tls`, or its default for the port and the credential.
    tls: Tls,
    /// `tls_ca_file`, already made absolute and trust-checked by
    /// [`nvs_config::mail::canonicalize`]. It replaces the compiled-in anchors.
    ca_file: Option<PathBuf>,
    /// `auth`, or `None` for `PLAIN` when offered, else `LOGIN`.
    auth: Option<Auth>,
    /// `return_path`, or `from` — the envelope sender, where bounces go.
    return_path: String,
    /// `capture = true`: the block keeps its mail and opens no connection, and `host` is empty.
    capture: bool,
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
/// `host` or no `from`, for one that sets half a credential, and for a `tls`
/// or `auth` the boot would have refused, so a configuration that skipped
/// [`nvs_config::mail::validate`] still never sends a password in the clear.
fn endpoint_of(ctx: &Ctx, endpoint: &str, member: &str) -> Result<Endpoint, Fault> {
    nvs_runtime::capability::require(
        ctx,
        nvs_config::Cap::MailSend,
        nvs_config::capability::Scope::Name(endpoint),
        member,
    )?;

    // A capture block has no `host`, and the boot refused every key only a connection reads.
    let capture = capture_of(ctx, endpoint);
    let host = match configured(ctx, endpoint, "host") {
        Some(host) => host,
        None if capture => String::new(),
        None => {
            return Err(Fault::thrown(format!(
                "{member}: no `[mail.{endpoint}]` block sets `host`, so there is no endpoint of \
                 that name to send through"
            )));
        }
    };
    let Some(from) = configured(ctx, endpoint, "from") else {
        return Err(Fault::thrown(format!(
            "{member}: `[mail.{endpoint}]` sets no `from`, and the sending identity is the \
             deployment's — there is no parameter for it"
        )));
    };
    let from = address_of(&from, "the configured `from`", member)?;

    let Ok(auth) = nvs_config::mail::auth_from(configured(ctx, endpoint, "auth").as_deref()) else {
        return Err(Fault::thrown(format!(
            "{member}: `[mail.{endpoint}] auth` is not `plain`, `login` or `xoauth2`"
        )));
    };
    let oauth = if auth == Some(Auth::XOAuth2) {
        Some(oauth::grant_of(ctx, endpoint, member)?)
    } else {
        None
    };

    // Both or neither. Half a credential is a block half-written, and reading
    // one as anonymous would submit as nobody to an endpoint the operator
    // plainly meant to log in to — which fails later, further away, and with the
    // endpoint's sentence rather than this one. An `xoauth2` block's `user` is
    // in its grant, and it has no password.
    let credential = match (
        configured(ctx, endpoint, "user"),
        configured(ctx, endpoint, "password"),
    ) {
        _ if oauth.is_some() => None,
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

    let signs_in = credential.is_some() || oauth.is_some();
    let written_tls = configured(ctx, endpoint, "tls");
    let tls = match nvs_config::mail::tls_from(written_tls.as_deref(), port, signs_in) {
        Some(Tls::None) if signs_in => {
            return Err(Fault::thrown(format!(
                "{member}: `[mail.{endpoint}]` sets `tls = \"none\"` and a credential, and a \
                 password is never sent without TLS"
            )));
        }
        Some(tls) => tls,
        None => {
            return Err(Fault::thrown(format!(
                "{member}: `[mail.{endpoint}] tls` is not `implicit`, `starttls` or `none`"
            )));
        }
    };
    let return_path = match configured(ctx, endpoint, "return_path") {
        Some(written) => address_of(&written, "the configured `return_path`", member)?,
        None => from.clone(),
    };

    Ok(Endpoint {
        name: endpoint.to_owned(),
        host,
        port,
        from,
        from_name: configured(ctx, endpoint, "from_name"),
        timeout,
        credential,
        oauth,
        tls,
        ca_file: configured(ctx, endpoint, "tls_ca_file").map(PathBuf::from),
        auth,
        return_path,
        capture,
    })
}

/// Whether `[mail.<endpoint>] capture = true`, read from the typed tree.
fn capture_of(ctx: &Ctx, endpoint: &str) -> bool {
    ctx.config()
        .and_then(|config| config.snapshot().config.mail.get(endpoint))
        .and_then(|block| block.capture)
        == Some(true)
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
) -> Result<Vec<Mailbox>, Fault> {
    let Some(array) = args[slot].array_ptr() else {
        return Err(Fault::fatal(format!(
            "{member} expected {:?} for `{what}`, got tag {}",
            Tag::Array,
            args[slot].tag_byte()
        )));
    };
    crate::str::Elements::of(array)
        .map(|value| mailbox_of(value, what, member))
        .collect()
}

/// The recipient in `value`: a `string`, which is an address only, or a [`ADDRESS`].
///
/// # Errors
///
/// A [`Fault::fatal`] for any other value, which [`RECIPIENT`] rules out from source; a catchable
/// `RuntimeError` for an address [`address_of`] refuses.
fn mailbox_of(value: Value, what: &str, member: &str) -> Result<Mailbox, Fault> {
    if let Some(text) = value.as_text() {
        return Ok(Mailbox {
            address: address_of(text, what, member)?,
            name: None,
        });
    }
    let object = crate::instance::receiver(value, &ADDRESS, what)?;
    let address = crate::instance::slot(object, ADDRESS_TEXT);
    let display = crate::instance::slot(object, ADDRESS_DISPLAY);
    let (Some(address), Some(display)) = (address.as_text(), display.as_text()) else {
        return Err(Fault::fatal(format!(
            "{member} expected two strings in the `{ADDRESS_NAME}` for `{what}`"
        )));
    };
    Ok(Mailbox {
        address: address_of(address, what, member)?,
        name: Some(display.to_owned()),
    })
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

/// What `$body` and the `text` option make: the plain text and the HTML as they are sent.
struct Body {
    /// The plain text: a `string` body, the `text` option, or what [`parts::read_html`] built.
    text: String,
    /// The HTML of a `Core\Html\Markup` body, with its CSS inlined by [`css::inline`].
    html: Option<String>,
}

/// Where a member's arguments have the body, the `text` option and the `attachments` option.
struct BodySlots {
    /// `$body`.
    body: usize,
    /// The `text` option.
    text: usize,
    /// The `attachments` option.
    attachments: usize,
}

/// `send`'s [`BodySlots`].
const SEND_BODY: BodySlots = BodySlots {
    body: BODY,
    text: TEXT,
    attachments: ATTACHMENTS,
};

/// `preview`'s [`BodySlots`].
const PREVIEW_BODY: BodySlots = BodySlots {
    body: 0,
    text: 1,
    attachments: 2,
};

/// The body and the attachments of a message, read and checked. `send` and `preview` both call
/// this, so a preview cannot differ from what is sent.
///
/// # Errors
///
/// [`body_of`]'s, [`attachments_of`]'s and [`parts::check_cids`]'s.
fn message_of(
    args: &[Value],
    slots: &BodySlots,
    member: &str,
) -> Result<(Body, Vec<parts::Part>), Fault> {
    let (body, cids) = body_of(args, slots, member)?;
    let attachments = attachments_of(args, slots.attachments, member)?;
    parts::check_cids(&cids, &attachments, member)?;
    Ok((body, attachments))
}

/// `$body` read: a `string` is the plain text, and a `Core\Html\Markup` is the HTML, its CSS
/// inlined, with the `text` option or the text built from it. The second value is every `cid:`
/// URL the HTML uses, without `cid:`.
///
/// # Errors
///
/// A catchable `RuntimeError` for a `text` option beside a `string` body, which would be a second
/// plain text with nothing to choose between the two, [`css::inline`]'s, and a [`Fault::fatal`]
/// for a value [`BODY_TY`] rules out from source.
fn body_of(args: &[Value], slots: &BodySlots, member: &str) -> Result<(Body, Vec<String>), Fault> {
    let text = optional_of(args, slots.text);
    if let Some(body) = args[slots.body].as_text() {
        if text.is_some() {
            return Err(Fault::thrown(format!(
                "{member}: `text` replaces the plain text made from an HTML body, and this body \
                 is a `string`, which is plain text already. Remove `text`, or pass the HTML as \
                 `Core\\Html\\Markup`."
            )));
        }
        let body = Body {
            text: body.to_owned(),
            html: None,
        };
        return Ok((body, Vec::new()));
    }
    let markup = crate::html::markup_slot(args[slots.body], "body")?;
    let Some(html) = markup.as_text() else {
        return Err(Fault::fatal(format!(
            "{member} expected text in the `Core\\Html\\Markup` for `body`"
        )));
    };
    let read = parts::read_html(html);
    let inlined = css::inline(html, &read, member)?;
    let body = Body {
        text: text.map_or(read.text, str::to_owned),
        html: Some(inlined),
    };
    Ok((body, read.cids))
}

/// Every [`parts::ATTACHMENT`] in the `attachments` option, read back and checked again.
///
/// # Errors
///
/// [`parts::part_of`]'s, and a [`Fault::fatal`] for an argument that is not an array.
fn attachments_of(args: &[Value], slot: usize, member: &str) -> Result<Vec<parts::Part>, Fault> {
    let Some(array) = args[slot].array_ptr() else {
        return Err(Fault::fatal(format!(
            "{member} expected {:?} for `attachments`, got tag {}",
            Tag::Array,
            args[slot].tag_byte()
        )));
    };
    crate::str::Elements::of(array)
        .map(|value| parts::part_of(value, member))
        .collect()
}

nvs_runtime::nvs_helper! {
    /// `Core\Mail::send(string $endpoint, array<string|Core\Mail\Address> $to, string $subject,
    /// string|Core\Html\Markup $body, {cc?, bcc?, replyTo?, text?, fromName?, attachments?,
    /// inReplyTo?, references?, unsubscribe?, autoSubmitted?, bounceTag?}): string` —
    /// `rule:programs/framework-core-half`'s transport and message, replacing `mail()`. It returns
    /// the `Message-ID` it wrote, without angle brackets.
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
    fn nvs_core_mail_send(ctx, args: [15]) {
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

nvs_runtime::nvs_helper! {
    /// `Core\Mail::preview(string|Core\Html\Markup $body, {text?, attachments?}):
    /// Core\Mail\Preview` — the body `send` would send, through the same [`message_of`], with
    /// nothing sent and no block read.
    fn nvs_core_mail_preview(_ctx, args: [3]) {
        let member = format!("{NAME}::preview");
        let (body, _attachments) = message_of(args, &PREVIEW_BODY, &member)?;
        Ok(css::preview(body.html.as_deref(), &body.text))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Mail\Address::of(string $address, string $name): Core\Mail\Address` — a recipient
    /// with a display name. The address is checked here, so a wrong one throws where it was
    /// written, and checked again by `send`.
    fn nvs_core_mail_address_of(_ctx, args: [2]) {
        let member = format!("{ADDRESS_NAME}::of");
        let address = text_of(args, 0, "address", &member)?;
        let display = text_of(args, 1, "name", &member)?;
        address_of(address, "`$address`", &member)?;
        Ok(crate::instance::build(
            &ADDRESS,
            [Value::text(address), Value::text(display)],
        ))
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
    let reply_to = if args[REPLY_TO].tag() == Some(Tag::Null) {
        None
    } else {
        Some(mailbox_of(args[REPLY_TO], "replyTo", member)?)
    };
    let subject = text_of(args, SUBJECT, "subject", member)?;
    let (body, attachments) = message_of(args, &SEND_BODY, member)?;
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

    let bounce_tag = optional_of(args, BOUNCE_TAG);

    let configured = endpoint_of(ctx, endpoint, member)?;
    let sender = smtp::bounce_sender(&configured.return_path, bounce_tag, member)?;
    let from_name = optional_of(args, FROM_NAME).or(configured.from_name.as_deref());
    let mut random = [0_u8; 16];
    crate::random::draw(ctx, |rng| rand::Rng::fill_bytes(rng, &mut random));
    let message_id = compose::message_id(random, &configured.from);

    // Under `nvs test`, and for a `capture = true` block, the message is kept and no connection
    // is opened, so a test never reaches a real person
    // (`rule:testing/a-test-reads-its-logs-and-mail`). The grant, the block and every address
    // were checked above, as they are in production.
    let test_records = ctx.test_records().cloned();
    if test_records.is_some() || configured.capture {
        let addresses = |list: &[Mailbox]| {
            list.iter()
                .map(|each| each.address.clone())
                .collect::<Vec<_>>()
        };
        let kept = nvs_runtime::test_records::KeptMail {
            endpoint: endpoint.to_owned(),
            to: addresses(&to),
            subject: subject.to_owned(),
            text: body.text,
            cc: addresses(&cc),
            bcc: addresses(&bcc),
            reply_to: reply_to.map(|each| each.address),
            html: body.html,
            message_id: message_id.clone(),
            from_name: from_name.map(str::to_owned),
            attachments: attachments
                .into_iter()
                .map(|part| nvs_runtime::test_records::KeptPart {
                    name: part.name,
                    media_type: part.media_type,
                    data: part.data,
                    content_id: part.content_id,
                })
                .collect(),
            in_reply_to,
            references,
            unsubscribe,
            auto_submitted,
            bounce_tag: bounce_tag.map(str::to_owned),
        };
        match test_records {
            Some(records) => records.mail(kept),
            None => capture::spool(ctx, &configured.from, args[BODY], &kept),
        }
        return Ok(Value::text(&message_id));
    }

    let signer = dkim::signer_of(ctx, endpoint, &configured.from, member)?;
    let sent_at = SystemTime::now();
    let message = compose(
        &compose::Headers {
            from: &configured.from,
            from_name,
            to: &to,
            cc: &cc,
            reply_to: reply_to.as_ref(),
            subject,
            message_id: &message_id,
            in_reply_to: in_reply_to.as_deref(),
            references: &references,
            unsubscribe: unsubscribe.as_deref(),
            auto_submitted,
            now: sent_at,
        },
        &compose::Body {
            text: &body.text,
            html: body.html.as_deref(),
            attachments: &attachments,
        },
    );
    // A block with a DKIM key signs the message as `DATA` carries it.
    let message = match &signer {
        Some(signer) => dkim::sign(signer, &message, sent_at, member)?,
        None => message,
    };

    // Bcc reaches the envelope and no header, which is the whole of what
    // makes it blind — the header set above never names it.
    let envelope: Vec<String> = to
        .iter()
        .chain(&cc)
        .chain(&bcc)
        .map(|each| each.address.clone())
        .collect();

    // The configured `timeout` bounds the whole exchange. The scheduler
    // bounds every park under it by the request's wall deadline, and ends
    // the request there (`rule:errors/on-limit`). The name lookup runs off
    // the core, so it also stops at the request's wall deadline itself.
    let now = Instant::now();
    let until = now + configured.timeout;
    let lookup_until = ctx
        .off_core_deadline()
        .map_or(until, |wall| until.min(wall));
    let address = resolve(&configured, member, lookup_until)?;
    let lease = lease_for(ctx, endpoint, until, member)?;
    pool::deliver(
        lease,
        now,
        address,
        &configured,
        &smtp::Envelope {
            sender: &sender,
            recipients: &envelope,
        },
        &message,
        member,
    )?;
    Ok(Value::text(&message_id))
}

/// A slot in `[mail.<name>]`'s pool on this core, under the block's bounds and the configuration
/// generation that declared it. A block the typed tree does not have is read as `pool = false`.
///
/// # Errors
///
/// A catchable `IOError` when the block already has its `max` of sessions on this core and none
/// frees within its `acquire`, or before `until`.
fn lease_for(
    ctx: &mut Ctx,
    endpoint: &str,
    until: Instant,
    member: &str,
) -> Result<nvs_runtime::pool::Lease, Fault> {
    let none = std::collections::BTreeMap::new();
    let snapshot = ctx
        .config()
        .map(|config| std::sync::Arc::clone(config.snapshot()));
    let bounds = snapshot
        .as_ref()
        .and_then(|snapshot| snapshot.config.mail.get(endpoint))
        .and_then(|block| nvs_config::mail::pool_for(endpoint, block, &none).ok())
        .unwrap_or(nvs_config::db::PoolBounds::OFF);
    let key = format!("mail:{endpoint}");
    let ticket = match &snapshot {
        Some(snapshot) => nvs_runtime::pool::Ticket::for_block(snapshot, &key, bounds),
        None => nvs_runtime::pool::Ticket::for_settings(key, bounds),
    };
    let max = bounds.max;
    let full = |waited: &str| {
        Fault::thrown_as(
            ThrownClass::Io,
            format!(
                "{member}: `[mail.{endpoint}]` already has its `max` of {max} connections open on \
                 this core, and {waited}. Raise `[mail.{endpoint}.pool] max`, or send fewer \
                 messages at once."
            ),
        )
    };
    match nvs_runtime::pool::admit(ticket.clone()) {
        Some(lease) => Ok(lease),
        None => crate::db::pool::wait_for_slot(ctx, ticket, &full, Some(until)),
    }
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

    /// `send` returns the `Message-ID` it wrote: 32 hex digits, `@`, and the domain of the
    /// block's `from`, as punycode. Each send has its own id, under `nvs test` too. A named
    /// recipient is kept by its address, and a name written inside an address string throws.
    // covers: Core\Mail::send, Core\Mail\Address::of
    #[test]
    fn send_returns_the_message_id() {
        let mut ctx = Ctx::buffered();
        ctx.set_config(crate::tests::granting(
            "[capabilities.mail]\nsend = [\"site\"]\n\n[mail.site]\nhost = \"mail.example.com\"\n\
             from = \"noreply@bücher.example\"\n",
        ));
        let records = nvs_runtime::test_records::TestRecords::new();
        ctx.set_test_records(std::rc::Rc::clone(&records));

        let named = nvs_runtime::call(
            nvs_core_mail_address_of,
            &mut ctx,
            &[Value::text("ada@example.com"), Value::text("Ada Lovelace")],
        )
        .expect("a valid address");
        let mut to = nvs_runtime::NvsArray::new();
        to.append(named);
        to.append(Value::text("bo@example.com"));
        let empty = || Value::array(nvs_runtime::NvsArray::new());
        let args = [
            Value::text("site"),
            Value::array(to),
            Value::text("Welcome"),
            Value::text("Hello."),
            empty(),
            empty(),
            Value::null(),
            Value::null(),
            Value::null(),
            empty(),
            Value::null(),
            empty(),
            Value::null(),
            Value::bool(true),
            Value::null(),
        ];
        let sent = nvs_runtime::call(nvs_core_mail_send, &mut ctx, &args).expect("a kept send");
        let id = sent.as_text().expect("send returns a string").to_owned();
        let (random, domain) = id.split_once('@').expect("the id is local@domain");
        assert_eq!(
            domain, "xn--bcher-kva.example",
            "the domain of `from`, as punycode"
        );
        assert!(
            random.len() == 32 && random.bytes().all(|byte| byte.is_ascii_hexdigit()),
            "{id}"
        );
        assert!(compose::message_id_of(&id, "inReplyTo", "Core\\Mail::send").is_ok());
        assert_eq!(
            records.sent()[0].to,
            ["ada@example.com", "bo@example.com"],
            "a named recipient is kept by its address"
        );

        let again = nvs_runtime::call(nvs_core_mail_send, &mut ctx, &args).expect("a kept send");
        assert_ne!(
            again.as_text(),
            Some(id.as_str()),
            "each message has its own id"
        );

        let refused = nvs_runtime::call(
            nvs_core_mail_address_of,
            &mut ctx,
            &[Value::text("Ada <ada@example.com>"), Value::text("Ada")],
        );
        assert!(
            refused.is_err(),
            "a name inside the address string is not read"
        );

        #[expect(unsafe_code, reason = "this test owns every value it made")]
        // SAFETY: each value carries the one reference this test made or was given, and none is
        // read after this.
        unsafe {
            for value in args.into_iter().chain([sent, again]) {
                value.release();
            }
        }
    }

    /// A context that grants the block `site` under `nvs test`, and `fs.read` over `read` when it
    /// is given, with the records its sends are kept in.
    fn kept_site(
        read: Option<&std::path::Path>,
    ) -> (Ctx, std::rc::Rc<nvs_runtime::test_records::TestRecords>) {
        let mut ctx = Ctx::buffered();
        let fs = read.map_or_else(String::new, |dir| {
            format!("[capabilities.fs]\nread = ['{}']\n\n", dir.display())
        });
        ctx.set_config(crate::tests::granting(&format!(
            "{fs}[capabilities.mail]\nsend = [\"site\"]\n\n[mail.site]\nhost = \
             \"mail.example.com\"\nfrom = \"noreply@example.com\"\n"
        )));
        let records = nvs_runtime::test_records::TestRecords::new();
        ctx.set_test_records(std::rc::Rc::clone(&records));
        (ctx, records)
    }

    /// `send`'s fifteen arguments to `ada@example.com` through `site`, with `body`, the `text`
    /// option and the `attachments` option. Every other option is omitted.
    fn send_args(body: Value, text: Value, attachments: Vec<Value>) -> [Value; 15] {
        let mut to = nvs_runtime::NvsArray::new();
        to.append(Value::text("ada@example.com"));
        let mut list = nvs_runtime::NvsArray::new();
        for each in attachments {
            list.append(each);
        }
        let empty = || Value::array(nvs_runtime::NvsArray::new());
        [
            Value::text("site"),
            Value::array(to),
            Value::text("Your order"),
            body,
            empty(),
            empty(),
            Value::null(),
            text,
            Value::null(),
            Value::array(list),
            Value::null(),
            empty(),
            Value::null(),
            Value::bool(true),
            Value::null(),
        ]
    }

    /// Frees every value of a test.
    fn release(values: impl IntoIterator<Item = Value>) {
        #[expect(unsafe_code, reason = "each test owns every value it passes here")]
        // SAFETY: each value carries the one reference its test made or was given, and none is
        // read after this.
        unsafe {
            for value in values {
                value.release();
            }
        }
    }

    /// `html` as a `Core\Html\Markup`.
    fn markup(html: &str) -> Value {
        crate::instance::build(&crate::html::MARKUP, [Value::text(html)])
    }

    /// A `Core\Html\Markup` body is sent as HTML, with a plain text built from it.
    // covers: Core\Mail::send
    #[test]
    fn a_markup_body_sends_html_and_generated_text() {
        let (mut ctx, records) = kept_site(None);
        let html =
            "<p>Hello <b>Ada</b>.</p><p><a href=\"https://example.com/o/1\">Your order</a></p>";
        let args = send_args(markup(html), Value::null(), Vec::new());
        let sent = nvs_runtime::call(nvs_core_mail_send, &mut ctx, &args).expect("a kept send");
        {
            let kept = &records.sent()[0];
            assert_eq!(kept.html.as_deref(), Some(html));
            assert_eq!(
                kept.text,
                "Hello Ada.\n\nYour order (https://example.com/o/1)\n"
            );
        }
        release(args.into_iter().chain([sent]));
    }

    /// A `string` body is sent as plain text alone, with no HTML part, and a body that looks like
    /// HTML is still text. With a `string` body, the `text` option throws.
    // covers: Core\Mail::send
    #[test]
    fn a_string_body_sends_plain_text_only() {
        let (mut ctx, records) = kept_site(None);
        let args = send_args(Value::text("<b>Hi</b>"), Value::null(), Vec::new());
        let sent = nvs_runtime::call(nvs_core_mail_send, &mut ctx, &args).expect("a kept send");
        {
            let kept = &records.sent()[0];
            assert_eq!(kept.text, "<b>Hi</b>");
            assert_eq!(kept.html, None);
        }
        let both = send_args(Value::text("Hi"), Value::text("Hi"), Vec::new());
        assert!(nvs_runtime::call(nvs_core_mail_send, &mut ctx, &both).is_err());
        assert_eq!(records.sent().len(), 1, "a refused send keeps nothing");
        release(args.into_iter().chain(both).chain([sent]));
    }

    /// `preview` returns the HTML and the plain text `send` sends, its CSS inlined, and the rules
    /// left in `<style>`. A string body has no HTML, and a stylesheet `<link>` throws in both.
    // covers: Core\Mail::preview, Core\Mail::send, Core\Mail\Preview::html, Core\Mail\Preview::text, Core\Mail\Preview::keptCss, Core\Mail\Preview::unusedCss
    #[test]
    fn preview_returns_what_send_puts_in_the_message() {
        let (mut ctx, records) = kept_site(None);
        let html = "<style>p { color: red } a:hover { color: blue }</style>\
                    <p>Hello <a href=\"https://example.com\">Ada</a>.</p>";
        let args = send_args(markup(html), Value::null(), Vec::new());
        let sent = nvs_runtime::call(nvs_core_mail_send, &mut ctx, &args).expect("a kept send");
        let shown = [
            markup(html),
            Value::null(),
            Value::array(nvs_runtime::NvsArray::new()),
        ];
        let preview =
            nvs_runtime::call(nvs_core_mail_preview, &mut ctx, &shown).expect("a preview");
        let mut read = |helper: usize| {
            nvs_runtime::call(css::HELPERS[helper], &mut ctx, &[preview]).expect("a reader")
        };
        let (html_of, text_of, kept_of) = (read(0), read(1), read(2));
        {
            let kept = &records.sent()[0];
            let inlined = crate::html::markup_slot(html_of, "html").expect("a Markup");
            assert_eq!(inlined.as_text(), kept.html.as_deref());
            assert!(
                kept.html
                    .as_deref()
                    .is_some_and(|sent| sent.contains("color: red"))
            );
            assert_eq!(text_of.as_text(), Some(kept.text.as_str()));
            let rules: Vec<_> = crate::str::Elements::of(kept_of.array_ptr().expect("a list"))
                .map(|rule| rule.as_text().expect("a rule").to_owned())
                .collect();
            assert_eq!(rules, ["a:hover { color: blue; }"]);
        }

        let plain = [
            Value::text("Hi"),
            Value::null(),
            Value::array(nvs_runtime::NvsArray::new()),
        ];
        let text_only =
            nvs_runtime::call(nvs_core_mail_preview, &mut ctx, &plain).expect("a preview");
        let none = nvs_runtime::call(css::HELPERS[0], &mut ctx, &[text_only]).expect("a reader");
        assert_eq!(none.tag(), Some(Tag::Null));

        let linked = "<link rel=\"stylesheet\" href=\"a.css\"><p>Hi</p>";
        let refused = send_args(markup(linked), Value::null(), Vec::new());
        assert!(nvs_runtime::call(nvs_core_mail_send, &mut ctx, &refused).is_err());
        let refused_preview = [
            markup(linked),
            Value::null(),
            Value::array(nvs_runtime::NvsArray::new()),
        ];
        assert!(nvs_runtime::call(nvs_core_mail_preview, &mut ctx, &refused_preview).is_err());
        assert_eq!(records.sent().len(), 1, "a refused send keeps nothing");
        release(
            args.into_iter()
                .chain(shown)
                .chain(plain)
                .chain(refused)
                .chain(refused_preview)
                .chain([sent, preview, html_of, text_of, kept_of, text_only, none]),
        );
    }

    /// The `text` option replaces the plain text built from the HTML.
    // covers: Core\Mail::send
    #[test]
    fn the_text_option_replaces_the_generated_text() {
        let (mut ctx, records) = kept_site(None);
        let args = send_args(markup("<p>Hello</p>"), Value::text("Hi there"), Vec::new());
        let sent = nvs_runtime::call(nvs_core_mail_send, &mut ctx, &args).expect("a kept send");
        assert_eq!(records.sent()[0].text, "Hi there");
        release(args.into_iter().chain([sent]));
    }

    /// An attachment read from a file under `fs.read` and one built from bytes both reach the
    /// message, by name and by content. A file outside the grant throws where it is named.
    // covers: Core\Mail\Attachment::fromFile, Core\Mail\Attachment::fromBytes
    #[cfg_attr(
        miri,
        ignore = "reaches a file through `openat`, which Miri does not emulate"
    )]
    #[test]
    fn an_attachment_from_a_file_and_from_bytes_is_sent() {
        let dir = nvs_repo::scratch("mail-attachment-file");
        let granted = dir.join("granted");
        std::fs::create_dir_all(&granted).expect("a temporary directory the test owns");
        let inside = granted.join("report.csv");
        std::fs::write(&inside, b"day,total\n").expect("the file to attach");
        let outside = dir.join("outside.csv");
        std::fs::write(&outside, b"no\n").expect("a file beside the grant");
        let (mut ctx, _records) = kept_site(Some(&granted));

        let path = Value::text(&inside.to_string_lossy());
        let file = nvs_runtime::call(
            parts::HELPERS[0],
            &mut ctx,
            &[path, Value::null(), Value::text("text/csv")],
        )
        .expect("a granted file");
        let name = Value::text("notes.txt");
        let data = Value::text("hello");
        let bytes = nvs_runtime::call(parts::HELPERS[1], &mut ctx, &[name, data, Value::null()])
            .expect("bytes with a name");
        let stray = Value::text(&outside.to_string_lossy());
        assert!(
            nvs_runtime::call(
                parts::HELPERS[0],
                &mut ctx,
                &[stray, Value::null(), Value::null()],
            )
            .is_err(),
            "a file outside `fs.read` is refused"
        );

        let parts = [
            parts::part_of(file, "send").expect("an attachment"),
            parts::part_of(bytes, "send").expect("an attachment"),
        ];
        assert_eq!(
            parts[0].name, "report.csv",
            "the name is the path's last part"
        );
        assert_eq!(parts[0].media_type, "text/csv");
        assert_eq!(parts[0].data, b"day,total\n");
        let message = compose(
            &compose::Headers {
                from: "noreply@example.com",
                from_name: None,
                to: &[],
                cc: &[],
                reply_to: None,
                subject: "Report",
                message_id: "0123456789abcdef0123456789abcdef@example.com",
                in_reply_to: None,
                references: &[],
                unsubscribe: None,
                auto_submitted: true,
                now: SystemTime::UNIX_EPOCH,
            },
            &compose::Body {
                text: "See attached.",
                html: None,
                attachments: &parts,
            },
        );
        use base64::Engine as _;
        let encoded = base64::engine::general_purpose::STANDARD;
        assert!(message.contains("filename=\"report.csv\""));
        assert!(message.contains("filename=\"notes.txt\""));
        assert!(message.contains(&encoded.encode(b"day,total\n")));
        assert!(message.contains(&encoded.encode(b"hello")));

        release([path, name, data, stray, file, bytes]);
        std::fs::remove_dir_all(&dir).ok();
    }

    /// `inlineFile` reads a granted file when it is called, takes the path's last part as its
    /// name, and its `src()` is `cid:` and the Content-ID the message carries. Two reads of one
    /// file are two images with two Content-IDs.
    // covers: Core\Mail\Attachment::inlineFile, Core\Mail\Attachment::src
    #[cfg_attr(
        miri,
        ignore = "reaches a file through `openat`, which Miri does not emulate"
    )]
    #[test]
    fn an_inline_file_is_read_when_called_and_its_src_is_its_content_id() {
        let dir = nvs_repo::scratch("mail-inline-file");
        let granted = dir.join("granted");
        std::fs::create_dir_all(&granted).expect("a temporary directory the test owns");
        let logo_path = granted.join("logo.png");
        std::fs::write(&logo_path, b"\x89PNG\r\n\x1a\n").expect("the image to attach");
        let (mut ctx, _records) = kept_site(Some(&granted));

        let path = Value::text(&logo_path.to_string_lossy());
        let first = nvs_runtime::call(parts::HELPERS[2], &mut ctx, &[path, Value::null()])
            .expect("a granted file");
        let second = nvs_runtime::call(parts::HELPERS[2], &mut ctx, &[path, Value::null()])
            .expect("a granted file");
        let src = nvs_runtime::call(parts::HELPERS[4], &mut ctx, &[first])
            .expect("an inline image has a src");
        let other = nvs_runtime::call(parts::HELPERS[4], &mut ctx, &[second])
            .expect("an inline image has a src");

        let part = parts::part_of(first, "send").expect("an attachment");
        assert_eq!(part.name, "logo.png", "the name is the path's last part");
        assert_eq!(part.media_type, "image/png", "the type is read from the bytes");
        assert_eq!(part.data, b"\x89PNG\r\n\x1a\n");
        let id = part.content_id.expect("an inline image has a Content-ID");
        assert_eq!(src.as_text(), Some(format!("cid:{id}").as_str()));
        assert_ne!(src.as_text(), other.as_text(), "each read has its own Content-ID");

        std::fs::remove_file(&logo_path).expect("the test's own file");
        assert!(
            parts::part_of(first, "send").is_ok(),
            "the bytes were read when the call ran"
        );

        release([path, first, second, src, other]);
        std::fs::remove_dir_all(&dir).ok();
    }

    /// An inline attachment's `src()` is `cid:` and its Content-ID, and a message whose HTML uses
    /// it is sent. An attachment that is not inline has no `src()`.
    // covers: Core\Mail\Attachment::inlineBytes, Core\Mail\Attachment::src, Core\Mail::send
    #[test]
    fn an_inline_image_src_is_its_content_id() {
        let (mut ctx, records) = kept_site(None);
        let name = Value::text("logo.png");
        let data = Value::text("\u{89}PNG");
        let logo = nvs_runtime::call(parts::HELPERS[3], &mut ctx, &[name, data, Value::null()])
            .expect("an inline image");
        let src = nvs_runtime::call(parts::HELPERS[4], &mut ctx, &[logo])
            .expect("an inline image has a src");
        let src_text = src.as_text().expect("a string").to_owned();
        let id = parts::part_of(logo, "send")
            .expect("an attachment")
            .content_id
            .expect("an inline image has a Content-ID");
        assert_eq!(src_text, format!("cid:{id}"));

        #[expect(
            unsafe_code,
            reason = "the message's array takes one reference to `logo`"
        )]
        // SAFETY: `logo` is live, and the extra reference is released with `args`.
        unsafe {
            logo.retain();
        }
        let args = send_args(
            markup(&format!("<img src=\"{src_text}\" alt=\"Logo\">")),
            Value::null(),
            vec![logo],
        );
        let sent = nvs_runtime::call(nvs_core_mail_send, &mut ctx, &args).expect("a kept send");
        assert_eq!(records.sent()[0].text, "Logo\n");

        let plain = nvs_runtime::call(parts::HELPERS[1], &mut ctx, &[name, data, Value::null()])
            .expect("an attachment");
        assert!(nvs_runtime::call(parts::HELPERS[4], &mut ctx, &[plain]).is_err());
        release(args.into_iter().chain([sent, name, data, logo, src, plain]));
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

    /// A context outside `nvs test` that grants the block `site`, written as `block`, with
    /// `[debug] store` on when `store` is.
    fn granted_site(block: &str, store: bool) -> Ctx {
        let mut ctx = Ctx::buffered();
        let debug = if store {
            "[debug]\nstore = true\n\n"
        } else {
            ""
        };
        ctx.set_config(crate::tests::granting(&format!(
            "{debug}[capabilities.mail]\nsend = [\"site\"]\n\n[mail.site]\n{block}"
        )));
        ctx
    }

    /// Every line this thread's spool writer wrote under `dir`, once it is sealed.
    fn spooled(dir: &std::path::Path) -> (Vec<nvs_runtime::spool::Line>, Vec<u8>) {
        nvs_runtime::spool::seal_now();
        let mut raw = Vec::new();
        for entry in std::fs::read_dir(dir).expect("the spool directory") {
            raw.extend(std::fs::read(entry.expect("an entry").path()).expect("a file"));
        }
        let lines = std::str::from_utf8(&raw)
            .expect("a spool file is UTF-8")
            .lines()
            .map(|line| nvs_runtime::spool::Line::parse(line).expect("a line"))
            .collect();
        (lines, raw)
    }

    /// Makes this thread's spool writer write under `dir`.
    fn spool_into(dir: &std::path::Path) {
        nvs_runtime::spool::install(nvs_runtime::spool::Writer::new(
            dir.to_path_buf(),
            "1-2-0".to_owned(),
            u64::MAX,
        ));
    }

    /// A `capture = true` block with no `host` keeps the message outside a test and returns its
    /// id. It has no address to reach, so a send that tried to connect would throw.
    // covers: Core\Mail::send
    #[test]
    fn a_capture_block_opens_no_connection() {
        let mut ctx = granted_site("capture = true\nfrom = \"noreply@example.com\"\n", false);
        assert!(ctx.test_records().is_none(), "this is not a test");
        let args = send_args(Value::text("Hello."), Value::null(), Vec::new());
        let sent = nvs_runtime::call(nvs_core_mail_send, &mut ctx, &args)
            .expect("a capture block keeps the message");
        let id = sent.as_text().expect("send returns a string").to_owned();
        assert!(id.ends_with("@example.com"), "{id}");
        let again =
            nvs_runtime::call(nvs_core_mail_send, &mut ctx, &args).expect("every message is kept");
        assert_ne!(again.as_text(), Some(id.as_str()));
        release(args.into_iter().chain([sent, again]));
    }

    /// With `[debug] store` on, a captured message is one `mail` line with its HTML as written
    /// and as sent, its plain text, the CSS report and its attachments.
    // covers: Core\Mail::send
    #[cfg_attr(
        miri,
        ignore = "canonicalises a scratch directory, which Miri does not emulate"
    )]
    #[test]
    fn a_captured_message_reaches_the_spool() {
        let dir = nvs_repo::scratch("mail-capture-spool");
        spool_into(&dir);
        let mut ctx = granted_site(
            "capture = true\nfrom = \"noreply@example.com\"\nfrom_name = \"Shop\"\n",
            true,
        );
        let html = "<html><head><style>p { color: red } .gone { color: blue }</style></head>\
                    <body><p>Your order</p></body></html>";
        let report = nvs_runtime::call(
            parts::HELPERS[1],
            &mut ctx,
            &[
                Value::text("report.csv"),
                Value::text("day,total\n"),
                Value::text("text/csv"),
            ],
        )
        .expect("bytes with a name");
        let args = send_args(markup(html), Value::null(), vec![report]);
        let sent = nvs_runtime::call(nvs_core_mail_send, &mut ctx, &args).expect("a kept send");
        let id = sent.as_text().expect("an id").to_owned();
        release(args.into_iter().chain([sent]));

        let (lines, _raw) = spooled(&dir);
        let mails: Vec<_> = lines
            .iter()
            .filter_map(|line| match &line.record {
                nvs_runtime::spool::Record::Mail(mail) => Some(mail),
                _ => None,
            })
            .collect();
        assert_eq!(
            mails.len(),
            1,
            "one `mail` line per kept message: {lines:?}"
        );
        let mail = mails[0];
        assert_eq!(mail.endpoint, "site");
        assert_eq!(mail.message_id, id);
        assert_eq!(mail.from, "noreply@example.com");
        assert_eq!(mail.from_name.as_deref(), Some("Shop"));
        assert_eq!(mail.to, ["ada@example.com"]);
        assert_eq!(mail.subject, "Your order");
        assert_eq!(mail.text, "Your order\n");
        assert_eq!(mail.html_written.as_deref(), Some(html));
        let inlined = mail.html.as_deref().expect("the HTML as sent");
        assert!(inlined.contains("style=\"color: red"), "{inlined}");
        assert_eq!(mail.kept_css, [".gone { color: blue; }"]);
        assert_eq!(mail.unused_css, [".gone { color: blue; }"]);
        assert_eq!(mail.attachments.len(), 1);
        assert_eq!(mail.attachments[0].name, "report.csv");
        assert_eq!(mail.attachments[0].media_type, "text/csv");
        assert_eq!(mail.attachments[0].size, 10);
        assert_eq!(mail.attachments[0].base64, "ZGF5LHRvdGFsCg==");
        assert!(mail.auto_submitted);
        assert!(!mail.elided);
    }

    /// A block that sends writes its `mail` event to the spool and nothing of the message: no
    /// `mail` line, no address, no subject and no byte of the body.
    // covers: Core\Mail::send
    #[cfg_attr(
        miri,
        ignore = "opens a loopback socket and a scratch directory, which Miri does not emulate"
    )]
    #[test]
    fn a_real_send_spools_only_its_event() {
        let dir = nvs_repo::scratch("mail-send-spool");
        spool_into(&dir);
        let (address, server) = smtp::tests::serve(smtp::tests::Fake::default());
        let mut ctx = granted_site(
            &format!(
                "host = \"127.0.0.1\"\nport = {}\nfrom = \"noreply@example.com\"\npool = false\n",
                address.port()
            ),
            true,
        );
        let args = send_args(Value::text("reset-link-7f3a9"), Value::null(), Vec::new());
        let sent = nvs_runtime::call(nvs_core_mail_send, &mut ctx, &args).expect("a sent message");
        release(args.into_iter().chain([sent]));
        let seen = server.join().expect("the server");
        assert!(seen.contains(&"<body>".to_owned()), "{seen:?}");

        let (lines, raw) = spooled(&dir);
        assert_eq!(lines.len(), 1, "one line, the event: {lines:?}");
        let nvs_runtime::spool::Record::Event(event) = &lines[0].record else {
            panic!("a sent message wrote a line that is not its event: {lines:?}");
        };
        assert_eq!(event.event, nvs_runtime::spool::EventKind::Mail);
        assert_eq!(event.outcome.recipients, Some(1));
        assert_eq!(event.outcome.ok, Some(true));
        for secret in [&b"reset-link"[..], b"ada@example.com", b"Your order"] {
            assert!(
                !raw.windows(secret.len()).any(|window| window == secret),
                "{} reached the spool",
                String::from_utf8_lossy(secret)
            );
        }
    }
}
