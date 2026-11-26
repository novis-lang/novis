//! `Core\Mail\Message` — a received message, read once into its headers, its two bodies and its
//! attachments (`rule:core-classes/mail-receive`).
//!
//! [`parse`] reads the bytes with `mail-parser` and copies what a program can ask for into the
//! instance's slots, so the parse runs once per message and every reader is a slot read. The
//! program sees Novis types only: text, `bytes`, `Core\Mail\Address`, `Core\Time\DateTime` and
//! `Core\Mail\Attachment`.
//!
//! **It never throws on a malformed message.** A program that reads a mailbox must reach every
//! message in it, so a missing header is `null` or an empty array, an undecodable part is
//! skipped, and bytes with no header at all are an empty message whose `raw()` is those bytes.
//!
//! **Every text it returns is `tainted`**, because somebody outside the program wrote all of it.
//! `html()` is a `tainted` string and never `Core\Html\Markup`, so a page shows it only through
//! `Core\Html::sanitize`.
//!
//! **A received file name is repaired, not refused.** [`file_name`] keeps the last path segment
//! and removes the control characters, as `Core\Zip` judges an entry name by its segments. A
//! name with nothing left is `null`. Refusing would leave the program unable to read the other
//! parts of a message it did not write.
//!
//! **Three bounds, each counted before the parse descends:** [`MAX_DEPTH`] levels of nesting
//! and [`MAX_PARTS`] parts, which `mail-parser` itself stops at, and [`MAX_HEADERS`] header
//! lines per part, which this module reads no further than. A part past a bound is skipped, and
//! the message keeps what came before it.
//!
//! **What it spends** (`rule:programs/memory-priority`): the raw bytes, kept for `raw()`, and the
//! decoded headers, bodies and attachments beside them, so about twice the message, charged to
//! the request and freed with the instance's last reference. Nothing runs for a program that
//! never calls `parse`.

use mail_parser::{HeaderForm, MessageParser, MessagePart, PartFlags, PartKind, PartRole};
use nvs_runtime::{Fault, NvsArray, NvsStr, Value};

use crate::registry::{
    ClassDoc, CoreClass, CoreMethod, CoreTy, MethodDoc, ParamDoc, Qual, TextOrigin,
};

/// The name of the class.
pub(crate) const MESSAGE_NAME: &str = r"Core\Mail\Message";

/// The most containers a part is nested in, the message itself included.
pub(crate) const MAX_DEPTH: usize = 16;

/// The most parts a message is read into, every nested message's included.
pub(crate) const MAX_PARTS: usize = 1_000;

/// The most header lines read from one part. A line past it is not read.
pub(crate) const MAX_HEADERS: usize = 1_000;

/// The slots, in order. [`parse`] fills each, and each reader returns one.
const SLOTS: &[&str] = &[
    "messageId",
    "from",
    "to",
    "cc",
    "replyTo",
    "subject",
    "date",
    "headerNames",
    "headerValues",
    "inReplyTo",
    "references",
    "text",
    "html",
    "attachments",
    "raw",
    "kind",
    "answers",
    "report",
    "bounceTag",
    "uid",
];

/// The index of the slot `name` in [`SLOTS`].
const fn slot_of(name: &str) -> usize {
    let mut at = 0;
    while at < SLOTS.len() {
        if const_eq(SLOTS[at], name) {
            return at;
        }
        at += 1;
    }
    panic!("no such slot");
}

/// `a == b`, in a `const fn`.
const fn const_eq(a: &str, b: &str) -> bool {
    let (a, b) = (a.as_bytes(), b.as_bytes());
    if a.len() != b.len() {
        return false;
    }
    let mut at = 0;
    while at < a.len() {
        if a[at] != b[at] {
            return false;
        }
        at += 1;
    }
    true
}

/// The slot `header` reads the lowercased names from.
const HEADER_NAMES: usize = slot_of("headerNames");
/// The slot `header` reads the decoded values from, one for each name.
const HEADER_VALUES: usize = slot_of("headerValues");

/// What `parse` reads: the raw message as `bytes` or a `string`, `tainted` included.
const RAW: CoreTy = CoreTy::Union(&[CoreTy::Text(Qual::Neutral), CoreTy::Blob(Qual::Neutral)]);

/// An address list reader's type.
const ADDRESSES: CoreTy = CoreTy::Array(&CoreTy::Instance(super::ADDRESS_NAME));

/// One reader: no parameter, its slot, and text that is always `tainted`.
const fn reader(
    name: &'static str,
    return_ty: CoreTy,
    symbol: &'static str,
    doc: &'static MethodDoc,
) -> CoreMethod {
    CoreMethod {
        name,
        names: &[],
        params: &[],
        defaults: &[],
        return_ty,
        text: TextOrigin::Source.on(&return_ty),
        symbol,
        doc: Some(doc),
    }
}

/// `Core\Mail\Message` — one received message.
// rank: 884
pub(crate) const MESSAGE: CoreClass = CoreClass {
    name: MESSAGE_NAME,
    doc: Some(&CARD),
    methods: &[CoreMethod {
        name: "parse",
        names: &["raw"],
        params: &[RAW],
        defaults: &[],
        return_ty: CoreTy::Instance(MESSAGE_NAME),
        text: TextOrigin::Unmarked,
        symbol: "nvs_core_mail_message_parse",
        doc: Some(&PARSE_DOC),
    }],
    instance: &[
        reader(
            "messageId",
            CoreTy::Nullable(&CoreTy::Str),
            "nvs_core_mail_message_message_id",
            &MESSAGE_ID_DOC,
        ),
        reader(
            "from",
            CoreTy::Nullable(&CoreTy::Instance(super::ADDRESS_NAME)),
            "nvs_core_mail_message_from",
            &FROM_DOC,
        ),
        reader("to", ADDRESSES, "nvs_core_mail_message_to", &TO_DOC),
        reader("cc", ADDRESSES, "nvs_core_mail_message_cc", &CC_DOC),
        reader(
            "replyTo",
            ADDRESSES,
            "nvs_core_mail_message_reply_to",
            &REPLY_TO_DOC,
        ),
        reader(
            "subject",
            CoreTy::Str,
            "nvs_core_mail_message_subject",
            &SUBJECT_DOC,
        ),
        reader(
            "date",
            CoreTy::Nullable(&CoreTy::Instance(crate::time::DATETIME_NAME)),
            "nvs_core_mail_message_date",
            &DATE_DOC,
        ),
        CoreMethod {
            name: "header",
            names: &["name"],
            params: &[CoreTy::Text(Qual::Neutral)],
            defaults: &[],
            return_ty: CoreTy::Array(&CoreTy::Str),
            text: TextOrigin::Source,
            symbol: "nvs_core_mail_message_header",
            doc: Some(&HEADER_DOC),
        },
        reader(
            "inReplyTo",
            CoreTy::Nullable(&CoreTy::Str),
            "nvs_core_mail_message_in_reply_to",
            &IN_REPLY_TO_DOC,
        ),
        reader(
            "references",
            CoreTy::Array(&CoreTy::Str),
            "nvs_core_mail_message_references",
            &REFERENCES_DOC,
        ),
        reader(
            "text",
            CoreTy::Nullable(&CoreTy::Str),
            "nvs_core_mail_message_text",
            &TEXT_DOC,
        ),
        reader(
            "html",
            CoreTy::Nullable(&CoreTy::Str),
            "nvs_core_mail_message_html",
            &HTML_DOC,
        ),
        reader(
            "attachments",
            CoreTy::Array(&CoreTy::Instance(super::ATTACHMENT_NAME)),
            "nvs_core_mail_message_attachments",
            &ATTACHMENTS_DOC,
        ),
        reader("raw", CoreTy::Bytes, "nvs_core_mail_message_raw", &RAW_DOC),
        reader(
            "kind",
            CoreTy::Enum(classify::KIND_NAME),
            "nvs_core_mail_message_kind",
            &KIND_DOC,
        ),
        reader(
            "answers",
            CoreTy::Nullable(&CoreTy::Str),
            "nvs_core_mail_message_answers",
            &ANSWERS_DOC,
        ),
        reader(
            "report",
            CoreTy::Nullable(&CoreTy::Instance(classify::REPORT_NAME)),
            "nvs_core_mail_message_report",
            &REPORT_DOC,
        ),
        reader(
            "bounceTag",
            CoreTy::Nullable(&CoreTy::Str),
            "nvs_core_mail_message_bounce_tag",
            &BOUNCE_TAG_DOC,
        ),
        reader(
            "uid",
            CoreTy::Nullable(&CoreTy::Int),
            "nvs_core_mail_message_uid",
            &UID_DOC,
        ),
    ],
    slots: SLOTS,
    constants: &[],
};

/// The slot types, for `crate::instance`'s table: [`SLOTS`]' order.
pub(crate) const SLOT_TYPES: &[CoreTy] = &[
    CoreTy::Nullable(&CoreTy::Str),
    CoreTy::Nullable(&CoreTy::Instance(super::ADDRESS_NAME)),
    ADDRESSES,
    ADDRESSES,
    ADDRESSES,
    CoreTy::Str,
    CoreTy::Nullable(&CoreTy::Instance(crate::time::DATETIME_NAME)),
    CoreTy::Array(&CoreTy::Str),
    CoreTy::Array(&CoreTy::Str),
    CoreTy::Nullable(&CoreTy::Str),
    CoreTy::Array(&CoreTy::Str),
    CoreTy::Nullable(&CoreTy::Str),
    CoreTy::Nullable(&CoreTy::Str),
    CoreTy::Array(&CoreTy::Instance(super::ATTACHMENT_NAME)),
    CoreTy::Bytes,
    CoreTy::Enum(classify::KIND_NAME),
    CoreTy::Nullable(&CoreTy::Str),
    CoreTy::Nullable(&CoreTy::Instance(classify::REPORT_NAME)),
    CoreTy::Nullable(&CoreTy::Str),
    CoreTy::Nullable(&CoreTy::Int),
];

/// The class card — `rule:core-api/reference-card`.
const CARD: ClassDoc = ClassDoc {
    short: "A message that your program received, such as a reply or a bounce. \
            `Core\\Mail\\Message::parse` reads it from its raw bytes. Every text it returns is \
            `tainted`, because the sender wrote it.",
};

/// `parse`'s card.
const PARSE_DOC: MethodDoc = MethodDoc {
    short: "Reads a raw message, such as the content of an `.eml` file. A broken message is \
            not an error. A header that is missing is `null` or an empty array. A part (one \
            piece of a message, such as the text, the HTML or a file) that cannot be read is \
            skipped.",
    params: &[ParamDoc {
        name: "raw",
        desc: "The whole message, headers and body, as `bytes` or a `string`.",
        shape: &[],
    }],
    ret: "The message. Novis reads the first 16 levels of nesting and the first 1,000 parts, \
          and skips the rest. Only the first 1,000 header lines of a part are read.",
    errors: &[],
};

/// `messageId`'s card.
const MESSAGE_ID_DOC: MethodDoc = MethodDoc {
    short: "Returns the `Message-ID` header without its angle brackets, such as \
            `1f2e@example.com`.",
    params: &[],
    ret: "The message id, or `null` when the message has none.",
    errors: &[],
};

/// `from`'s card.
const FROM_DOC: MethodDoc = MethodDoc {
    short: "Returns the first address of the `From` header. The sender writes this header, so \
            it does not prove who sent the message.",
    params: &[],
    ret: "The address and its name, or `null` when the message has no `From` address.",
    errors: &[],
};

/// `to`'s card.
const TO_DOC: MethodDoc = MethodDoc {
    short: "Returns the addresses of the `To` header.",
    params: &[],
    ret: "One address for each recipient, in order. The array is empty when there is none.",
    errors: &[],
};

/// `cc`'s card.
const CC_DOC: MethodDoc = MethodDoc {
    short: "Returns the addresses of the `Cc` header.",
    params: &[],
    ret: "One address for each recipient, in order. The array is empty when there is none.",
    errors: &[],
};

/// `replyTo`'s card.
const REPLY_TO_DOC: MethodDoc = MethodDoc {
    short: "Returns the addresses of the `Reply-To` header. A reply goes to these addresses.",
    params: &[],
    ret: "The addresses, in order. The array is empty when the message has no `Reply-To`.",
    errors: &[],
};

/// `subject`'s card.
const SUBJECT_DOC: MethodDoc = MethodDoc {
    short: "Returns the subject. An encoded subject, such as `=?UTF-8?B?…?=`, is decoded.",
    params: &[],
    ret: "The subject, or an empty string when the message has none.",
    errors: &[],
};

/// `date`'s card.
const DATE_DOC: MethodDoc = MethodDoc {
    short: "Returns the `Date` header, in the time zone the sender wrote.",
    params: &[],
    ret: "The date, or `null` when the message has no date or the date cannot be read.",
    errors: &[],
};

/// `header`'s card.
const HEADER_DOC: MethodDoc = MethodDoc {
    short: "Returns every value of the header `$name`, decoded, in the order of the message.",
    params: &[ParamDoc {
        name: "name",
        desc: "The header name, such as `List-Id`. Upper and lower case are the same.",
        shape: &[],
    }],
    ret: "One string for each line of that header. The array is empty when there is none.",
    errors: &[],
};

/// `inReplyTo`'s card.
const IN_REPLY_TO_DOC: MethodDoc = MethodDoc {
    short: "Returns the message id in the `In-Reply-To` header, without its angle brackets. \
            A reply has this header. It is the id of the message that was answered.",
    params: &[],
    ret: "The message id, or `null` when the message has no `In-Reply-To`.",
    errors: &[],
};

/// `references`' card.
const REFERENCES_DOC: MethodDoc = MethodDoc {
    short: "Returns the message ids in the `References` header, without their angle brackets.",
    params: &[],
    ret: "The ids, oldest first. The array is empty when the message has none.",
    errors: &[],
};

/// `text`'s card.
const TEXT_DOC: MethodDoc = MethodDoc {
    short: "Returns the plain text body of the message.",
    params: &[],
    ret: "The text, or `null` when the message has no `text/plain` part.",
    errors: &[],
};

/// `html`'s card.
const HTML_DOC: MethodDoc = MethodDoc {
    short: "Returns the HTML body of the message, as a `tainted` string. To show it on a page, \
            pass it to `Core\\Html::sanitize` first.",
    params: &[],
    ret: "The HTML, or `null` when the message has no HTML part.",
    errors: &[],
};

/// `attachments`' card.
const ATTACHMENTS_DOC: MethodDoc = MethodDoc {
    short: "Returns the files of the message and the images shown inside its HTML.",
    params: &[],
    ret: "One attachment for each file part, in order. A part is one piece of a message, such \
          as the text, the HTML or a file. The array is empty when there is none.",
    errors: &[],
};

/// `raw`'s card.
const RAW_DOC: MethodDoc = MethodDoc {
    short: "Returns the message exactly as `parse` read it.",
    params: &[],
    ret: "The raw bytes, as `tainted bytes`, because the sender wrote them.",
    errors: &[],
};

/// `kind`'s card.
const KIND_DOC: MethodDoc = MethodDoc {
    short: "Returns what the message is: a bounce, a delay, a complaint, an auto-reply, a reply \
            or something else. Only a delivery report is a bounce or a delay.",
    params: &[],
    ret: "A `Core\\Mail\\Message::Kind` case.",
    errors: &[],
};

/// `answers`' card.
const ANSWERS_DOC: MethodDoc = MethodDoc {
    short: "Returns the `Message-ID` of the mail this message is about, without angle brackets. \
            For a bounce, a delay or a complaint, it is the id of the returned message. \
            Otherwise it is `In-Reply-To`, then the last id in `References`.",
    params: &[],
    ret: "The id, or `null` when the message names none.",
    errors: &[],
};

/// `report`'s card.
const REPORT_DOC: MethodDoc = MethodDoc {
    short: "Returns the delivery report of a bounce or a delay, with the status of each \
            recipient.",
    params: &[],
    ret: "The report, or `null` when the message is not a delivery report.",
    errors: &[],
};

/// `bounceTag`'s card.
const BOUNCE_TAG_DOC: MethodDoc = MethodDoc {
    short: "Returns the tag after `+` in the address this message was sent to, such as \
            `order-42` for `bounces+order-42@example.com`. It is the tag that the `bounceTag` \
            option of `Core\\Mail::send` wrote. The address is read from `Delivered-To`, then \
            `X-Original-To`, then `To`.",
    params: &[],
    ret: "The tag, or `null` when no address has a valid tag.",
    errors: &[],
};

/// `uid`'s card.
const UID_DOC: MethodDoc = MethodDoc {
    short: "Returns the number of the message in its mailbox folder. `Core\\Imap\\Folder::fetch` \
            sets it. Pass it to the other methods of the folder.",
    params: &[],
    ret: "The UID, or `null` for a message that `parse` read.",
    errors: &[],
};

/// What [`parse`] read: owned Rust data, so the tests can check it without a value, and
/// [`built`] makes the instance from it.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(crate) struct Parsed {
    /// The `Message-ID`, without angle brackets.
    pub message_id: Option<String>,
    /// The first `From` address.
    pub from: Option<Mailbox>,
    /// The `To` addresses.
    pub to: Vec<Mailbox>,
    /// The `Cc` addresses.
    pub cc: Vec<Mailbox>,
    /// The `Reply-To` addresses.
    pub reply_to: Vec<Mailbox>,
    /// The decoded subject, empty when there is none.
    pub subject: String,
    /// The `Date`: the civil time and its offset east of UTC in seconds.
    pub date: Option<(mail_parser::DateTime, i32)>,
    /// The root part's header lines, each a lowercased name and its decoded value.
    pub headers: Vec<(String, String)>,
    /// The first `In-Reply-To` id.
    pub in_reply_to: Option<String>,
    /// The `References` ids.
    pub references: Vec<String>,
    /// The first `text/plain` body part.
    pub text: Option<String>,
    /// The first `text/html` body part.
    pub html: Option<String>,
    /// The attachments and the inline parts.
    pub attachments: Vec<Received>,
    /// The kind, the ID it answers, the delivery report and the bounce tag.
    pub classified: classify::Classified,
}

/// One address of a received header.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Mailbox {
    /// The address, as the header wrote it.
    pub address: String,
    /// The display name, empty when there is none.
    pub name: String,
}

/// One received attachment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Received {
    /// The file name, through [`file_name`].
    pub name: Option<String>,
    /// The media type, lowercased, or `application/octet-stream`.
    pub media_type: String,
    /// The decoded bytes.
    pub data: Vec<u8>,
    /// The Content-ID, without angle brackets.
    pub content_id: Option<String>,
}

/// `raw` read into a [`Parsed`]. It never fails: bytes the parser cannot read at all are an
/// empty message.
pub(crate) fn parse(raw: &[u8]) -> Parsed {
    let parser = MessageParser::new()
        .max_depth(MAX_DEPTH)
        .max_parts(MAX_PARTS);
    let Some(message) = parser.parse(raw) else {
        return Parsed::default();
    };
    let root = message.root();
    let fields: Vec<mail_parser::Header<'_>> = root.headers().iter().take(MAX_HEADERS).collect();
    let field = |name: &str| {
        fields
            .iter()
            .find(|field| field.raw_name().eq_ignore_ascii_case(name))
            .map(mail_parser::Header::value)
    };
    let addresses = |name: &str| {
        field(name)
            .and_then(|value| value.as_address())
            .map(|list| {
                list.mailboxes()
                    .filter_map(|mailbox| {
                        let address = mailbox.address().filter(|a| !a.is_empty())?;
                        Some(Mailbox {
                            address: address.to_owned(),
                            name: mailbox.name().unwrap_or_default().to_owned(),
                        })
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default()
    };
    let ids = |name: &str| {
        field(name)
            .and_then(|value| value.as_text_list())
            .map(|list| {
                list.iter()
                    .filter(|id| !id.is_empty())
                    .map(str::to_owned)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default()
    };
    let date = field("date")
        .and_then(|value| value.as_datetime())
        .map(|at| {
            let east = i32::from(at.tz_hour) * 3600 + i32::from(at.tz_minute) * 60;
            let offset = if at.tz_before_gmt { -east } else { east };
            (at, offset)
        });
    let headers: Vec<(String, String)> = fields
        .iter()
        .map(|field| {
            let parsed = field.parse_as(HeaderForm::Text);
            let value = parsed.value().as_text().unwrap_or_default().to_owned();
            (field.raw_name().to_ascii_lowercase(), value)
        })
        .collect();
    let within = |part: &MessagePart<'_>| !part.flags().contains(PartFlags::LIMIT_REACHED);
    let text = root
        .text_body()
        .filter(within)
        .find(|part| matches!(part.kind(), PartKind::Text))
        .and_then(|part| part.text().map(|text| text.into_owned()));
    let html = root
        .html_body()
        .filter(within)
        .find(|part| matches!(part.kind(), PartKind::Html))
        .and_then(|part| part.text().map(|html| html.into_owned()));
    let attachments = root
        .parts()
        .filter(within)
        .filter(|part| part.is_attachment() || matches!(part.role(), PartRole::MediaBody))
        .map(|part| received(&part))
        .collect();
    let to = addresses("to");
    let in_reply_to = ids("in-reply-to").into_iter().next();
    let references = ids("references");
    let classified =
        classify::classified(&root, &headers, in_reply_to.as_deref(), &references, &to);
    Parsed {
        message_id: field("message-id")
            .and_then(|value| value.as_text())
            .filter(|id| !id.is_empty())
            .map(str::to_owned),
        from: addresses("from").into_iter().next(),
        to,
        cc: addresses("cc"),
        reply_to: addresses("reply-to"),
        subject: field("subject")
            .and_then(|value| value.as_text())
            .unwrap_or_default()
            .to_owned(),
        date,
        headers,
        in_reply_to,
        references,
        text,
        html,
        attachments,
        classified,
    }
}

/// One attachment part as a [`Received`].
fn received(part: &MessagePart<'_>) -> Received {
    let media_type = part
        .content_type()
        .and_then(|kind| {
            let written = format!("{}/{}", kind.ctype(), kind.subtype()?);
            super::parts::type_of(&written, "").ok()
        })
        .unwrap_or_else(|| "application/octet-stream".to_owned());
    let content_id = part
        .content_id()
        .map(|id| id.trim().trim_start_matches('<').trim_end_matches('>'))
        .filter(|id| !id.is_empty() && !id.chars().any(char::is_control))
        .map(str::to_owned);
    Received {
        name: part.attachment_name().and_then(file_name),
        media_type,
        data: part.decoded().into_owned(),
        content_id,
    }
}

/// A received file name, repaired: its last path segment with every control character
/// removed, or `None` when nothing is left or what is left is `.` or `..`.
///
/// `/` and `\` both end a segment on every platform, as `Core\Path`'s one grammar reads them, so
/// a name cannot reach another folder on any system it is saved to.
pub(crate) fn file_name(written: &str) -> Option<String> {
    let last = written.rsplit(['/', '\\']).next().unwrap_or_default();
    let name: String = last.chars().filter(|c| !c.is_control()).collect();
    let name = name.trim();
    match name {
        "" | "." | ".." => None,
        name => Some(name.to_owned()),
    }
}

/// `parsed` and `raw` as the instance, with the mailbox `uid` it was fetched under, if any.
fn built(parsed: Parsed, raw: &[u8], uid: Option<u32>) -> Value {
    let text = |text: Option<String>| text.as_deref().map_or_else(Value::null, Value::text);
    let list = |items: &[String]| {
        let mut out = NvsArray::new();
        for item in items {
            out.append(Value::text(item));
        }
        Value::array(out)
    };
    let mailbox = |mailbox: &Mailbox| {
        crate::instance::build(
            &super::ADDRESS,
            [Value::text(&mailbox.address), Value::text(&mailbox.name)],
        )
    };
    let mailboxes = |items: &[Mailbox]| {
        let mut out = NvsArray::new();
        for item in items {
            out.append(mailbox(item));
        }
        Value::array(out)
    };
    let date = parsed
        .date
        .and_then(|(at, offset)| {
            crate::time::datetime_at(
                &crate::time::Civil {
                    year: i32::from(at.year),
                    month: at.month,
                    day: at.day,
                    hour: at.hour,
                    minute: at.minute,
                    second: at.second,
                    nanosecond: 0,
                },
                offset,
            )
        })
        .unwrap_or_else(Value::null);
    let (names, values): (Vec<String>, Vec<String>) = parsed.headers.into_iter().unzip();
    let mut attachments = NvsArray::new();
    for part in &parsed.attachments {
        attachments.append(super::parts::received(
            part.name.as_deref(),
            &part.media_type,
            &part.data,
            part.content_id.as_deref(),
        ));
    }
    crate::instance::build(
        &MESSAGE,
        [
            text(parsed.message_id),
            parsed.from.as_ref().map_or_else(Value::null, mailbox),
            mailboxes(&parsed.to),
            mailboxes(&parsed.cc),
            mailboxes(&parsed.reply_to),
            Value::text(&parsed.subject),
            date,
            list(&names),
            list(&values),
            text(parsed.in_reply_to),
            list(&parsed.references),
            text(parsed.text),
            text(parsed.html),
            Value::array(attachments),
            Value::bytes(NvsStr::new(raw)),
            Value::enum_int(parsed.classified.kind as i64),
            text(parsed.classified.answers),
            parsed
                .classified
                .report
                .as_deref()
                .map_or_else(Value::null, classify::report_value),
            text(parsed.classified.bounce_tag),
            uid.map_or_else(Value::null, |uid| Value::int(i64::from(uid))),
        ],
    )
}

nvs_runtime::nvs_helper! {
    /// `Core\Mail\Message::parse(string|bytes $raw): Core\Mail\Message`.
    fn nvs_core_mail_message_parse(_ctx, args: [1]) {
        let raw = args[0].as_bytes().or_else(|| args[0].as_str_bytes()).ok_or_else(|| {
            Fault::fatal(format!(
                "{MESSAGE_NAME}::parse expected a `string` or `bytes` for `$raw`, got tag {}",
                args[0].tag_byte()
            ))
        })?;
        Ok(built(parse(raw), raw, None))
    }
}

/// One reader's body: the slot named `name`.
fn read(args: &[Value], name: &str) -> Result<Value, Fault> {
    let index = SLOTS
        .iter()
        .position(|slot| *slot == name)
        .ok_or_else(|| Fault::fatal(format!("{MESSAGE_NAME} has no slot `{name}`")))?;
    crate::instance::read_slot(args, &MESSAGE, index, name)
}

/// Declares one slot reader per `(helper, slot)` pair.
macro_rules! readers {
    ($($helper:ident => $slot:literal,)*) => {
        $(
            nvs_runtime::nvs_helper! {
                #[doc = concat!("`$message->", $slot, "()`: the slot of the same name.")]
                fn $helper(_ctx, args: [1]) {
                    read(args, $slot)
                }
            }
        )*
    };
}

readers! {
    nvs_core_mail_message_message_id => "messageId",
    nvs_core_mail_message_from => "from",
    nvs_core_mail_message_to => "to",
    nvs_core_mail_message_cc => "cc",
    nvs_core_mail_message_reply_to => "replyTo",
    nvs_core_mail_message_subject => "subject",
    nvs_core_mail_message_date => "date",
    nvs_core_mail_message_in_reply_to => "inReplyTo",
    nvs_core_mail_message_references => "references",
    nvs_core_mail_message_text => "text",
    nvs_core_mail_message_html => "html",
    nvs_core_mail_message_attachments => "attachments",
    nvs_core_mail_message_raw => "raw",
    nvs_core_mail_message_kind => "kind",
    nvs_core_mail_message_answers => "answers",
    nvs_core_mail_message_report => "report",
    nvs_core_mail_message_bounce_tag => "bounceTag",
    nvs_core_mail_message_uid => "uid",
}

nvs_runtime::nvs_helper! {
    /// `$message->header(string $name): array<string>` — every value of that header, in order.
    fn nvs_core_mail_message_header(_ctx, args: [2]) {
        let member = format!("{MESSAGE_NAME}::header");
        let object = crate::instance::receiver(args[0], &MESSAGE, &member)?;
        let name = args[1].as_text().ok_or_else(|| {
            Fault::fatal(format!("{member} expected a `string` for `$name`"))
        })?;
        let names = crate::instance::slot(object, HEADER_NAMES);
        let values = crate::instance::slot(object, HEADER_VALUES);
        let (Some(names), Some(values)) = (names.as_array(), values.as_array()) else {
            return Err(Fault::fatal(format!("{member} expected its two header lists")));
        };
        let mut out = NvsArray::new();
        for index in 0..i64::try_from(names.count()).unwrap_or(i64::MAX) {
            let matches = names
                .get_index(index)
                .and_then(|written| written.as_text().map(|w| w.eq_ignore_ascii_case(name)))
                .unwrap_or(false);
            if let (true, Some(value)) = (matches, values.get_index(index)) {
                out.append(Value::text(value.as_text().unwrap_or_default()));
            }
        }
        Ok(Value::array(out))
    }
}

/// This module's arm of the `Core` symbol table — see [`crate::address`].
pub(super) fn address(symbol: &str) -> Option<nvs_runtime::Entry> {
    let helper: nvs_runtime::NvsFn = match symbol {
        "nvs_core_mail_message_parse" => nvs_core_mail_message_parse,
        "nvs_core_mail_message_message_id" => nvs_core_mail_message_message_id,
        "nvs_core_mail_message_from" => nvs_core_mail_message_from,
        "nvs_core_mail_message_to" => nvs_core_mail_message_to,
        "nvs_core_mail_message_cc" => nvs_core_mail_message_cc,
        "nvs_core_mail_message_reply_to" => nvs_core_mail_message_reply_to,
        "nvs_core_mail_message_subject" => nvs_core_mail_message_subject,
        "nvs_core_mail_message_date" => nvs_core_mail_message_date,
        "nvs_core_mail_message_header" => nvs_core_mail_message_header,
        "nvs_core_mail_message_in_reply_to" => nvs_core_mail_message_in_reply_to,
        "nvs_core_mail_message_references" => nvs_core_mail_message_references,
        "nvs_core_mail_message_text" => nvs_core_mail_message_text,
        "nvs_core_mail_message_html" => nvs_core_mail_message_html,
        "nvs_core_mail_message_attachments" => nvs_core_mail_message_attachments,
        "nvs_core_mail_message_raw" => nvs_core_mail_message_raw,
        "nvs_core_mail_message_kind" => nvs_core_mail_message_kind,
        "nvs_core_mail_message_answers" => nvs_core_mail_message_answers,
        "nvs_core_mail_message_report" => nvs_core_mail_message_report,
        "nvs_core_mail_message_bounce_tag" => nvs_core_mail_message_bounce_tag,
        "nvs_core_mail_message_uid" => nvs_core_mail_message_uid,
        _ => return classify::address(symbol),
    };
    Some(nvs_runtime::Entry::borrowed((helper as *const ()).cast()))
}

/// `raw` parsed into a new instance, with one reference the caller owns: what `parse` returns.
pub(crate) fn message_of(raw: &[u8]) -> Value {
    built(parse(raw), raw, None)
}

/// [`message_of`] for a message read from a mailbox folder, whose `uid()` is `uid`.
pub(crate) fn fetched_message(raw: &[u8], uid: u32) -> Value {
    built(parse(raw), raw, Some(uid))
}

pub(crate) mod classify;

#[cfg(test)]
mod tests;
