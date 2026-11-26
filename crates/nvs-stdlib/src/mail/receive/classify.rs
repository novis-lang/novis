//! What a received message is about: its kind, the `Message-ID` it answers, a delivery report's
//! recipients and the bounce tag (`rule:core-classes/mail-receive`).
//!
//! **Only what a message says about itself is read.** A bounce is a delivery report (RFC 3464),
//! a complaint a feedback report (RFC 5965), and an auto-reply a message with one of the headers
//! in [`AUTO_REPLY`] or an `Auto-Submitted` that is not `no` (RFC 3834). A bounce that is not a
//! delivery report is [`Kind::Other`]: guessing from a subject or a sender is declined
//! (`rule:errors/ambiguous-input-refused`), and the large senders all write RFC 3464.
//!
//! **Bounds.** A report is read up to [`MAX_RECIPIENTS`] recipients and [`super::MAX_HEADERS`]
//! lines per group, a folded line counted as one, and one field's value up to
//! [`MAX_FIELD_BYTES`]. So the work is linear in the report part, which is within the message,
//! and what is kept per recipient is bounded.

use mail_parser::{MessageParser, MessagePart, MessageRef, PartFlags, PartKind};
use nvs_runtime::{Fault, NvsArray, Value};

use crate::registry::{
    CaseDoc, ClassDoc, CoreClass, CoreEnum, CoreMethod, CoreTy, EnumDoc, MethodDoc, TextOrigin,
};

/// The most recipients read from one delivery report. A recipient past it is not read.
pub(crate) const MAX_RECIPIENTS: usize = 1_000;

/// The most bytes kept of one field of a delivery report, its folded lines included: 4 KiB,
/// four times RFC 5322's line limit. The rest of a longer `Diagnostic-Code` is not read.
pub(crate) const MAX_FIELD_BYTES: usize = 4 << 10;

/// `Core\Mail\Message::Kind`'s name. `rule:enums/an-enum-lives-in-the-class-that-uses-it` puts
/// it in `Core\Mail\Message`, the one class that returns it.
pub(crate) const KIND_NAME: &str = r"Core\Mail\Message::Kind";

/// `Core\Mail\ReportRecipient::Action`'s name, in the one class that returns it.
pub(crate) const ACTION_NAME: &str = r"Core\Mail\ReportRecipient::Action";

/// `Core\Mail\Report`'s name.
pub(crate) const REPORT_NAME: &str = r"Core\Mail\Report";

/// `Core\Mail\ReportRecipient`'s name.
pub(crate) const RECIPIENT_NAME: &str = r"Core\Mail\ReportRecipient";

/// [`Kind`] as a `Core` enum. The values are [`Kind`]'s discriminants.
pub(crate) const KIND: CoreEnum = CoreEnum {
    name: KIND_NAME,
    cases: &[
        ("Bounce", Kind::Bounce as i64),
        ("Delay", Kind::Delay as i64),
        ("Complaint", Kind::Complaint as i64),
        ("AutoReply", Kind::AutoReply as i64),
        ("Reply", Kind::Reply as i64),
        ("Other", Kind::Other as i64),
    ],
    doc: Some(&KIND_DOC),
};

/// [`KIND`]'s card — `rule:core-api/reference-card`.
const KIND_DOC: EnumDoc = EnumDoc {
    short: "What a received message is, as `Core\\Mail\\Message::kind()` returns it.",
    cases: &[
        CaseDoc {
            name: "Bounce",
            desc: "A delivery report with at least one recipient whose delivery failed.",
        },
        CaseDoc {
            name: "Delay",
            desc: "A delivery report with no failed recipient and at least one delayed \
                   recipient. The server still tries to deliver the message.",
        },
        CaseDoc {
            name: "Complaint",
            desc: "A feedback report: a recipient marked the message as spam.",
        },
        CaseDoc {
            name: "AutoReply",
            desc: "A message that a program wrote in answer, such as an out-of-office reply.",
        },
        CaseDoc {
            name: "Reply",
            desc: "A message with an `In-Reply-To` header.",
        },
        CaseDoc {
            name: "Other",
            desc: "Any other message.",
        },
    ],
};

/// [`Action`] as a `Core` enum. The values are [`Action`]'s discriminants.
pub(crate) const ACTION: CoreEnum = CoreEnum {
    name: ACTION_NAME,
    cases: &[
        ("Failed", Action::Failed as i64),
        ("Delayed", Action::Delayed as i64),
        ("Delivered", Action::Delivered as i64),
        ("Relayed", Action::Relayed as i64),
        ("Expanded", Action::Expanded as i64),
    ],
    doc: Some(&ACTION_DOC),
};

/// [`ACTION`]'s card — `rule:core-api/reference-card`.
const ACTION_DOC: EnumDoc = EnumDoc {
    short: "What the server did with the message for one recipient of a delivery report.",
    cases: &[
        CaseDoc {
            name: "Failed",
            desc: "The message could not be delivered.",
        },
        CaseDoc {
            name: "Delayed",
            desc: "The message is late. The server still tries to deliver it.",
        },
        CaseDoc {
            name: "Delivered",
            desc: "The server put the message in the mailbox.",
        },
        CaseDoc {
            name: "Relayed",
            desc: "The message was passed to another server, which sends no report.",
        },
        CaseDoc {
            name: "Expanded",
            desc: "The message was sent on to the members of a mailing list.",
        },
    ],
};

/// `Core\Mail\Report` — the recipients of one delivery report.
// rank: 885
pub(crate) const REPORT: CoreClass = CoreClass {
    name: REPORT_NAME,
    doc: Some(&REPORT_CARD),
    methods: &[],
    instance: &[CoreMethod {
        name: "recipients",
        names: &[],
        params: &[],
        defaults: &[],
        return_ty: RECIPIENTS,
        text: TextOrigin::Unmarked,
        symbol: "nvs_core_mail_report_recipients",
        doc: Some(&RECIPIENTS_DOC),
    }],
    slots: &["recipients"],
    constants: &[],
};

/// `recipients()`'s type.
const RECIPIENTS: CoreTy = CoreTy::Array(&CoreTy::Instance(RECIPIENT_NAME));

/// [`REPORT`]'s slot types, for `crate::instance`'s table.
pub(crate) const REPORT_SLOT_TYPES: &[CoreTy] = &[RECIPIENTS];

/// [`REPORT`]'s card.
const REPORT_CARD: ClassDoc = ClassDoc {
    short: "The delivery report in a bounce or a delay. `Core\\Mail\\Message::report()` \
            returns it.",
};

/// `recipients`' card.
const RECIPIENTS_DOC: MethodDoc = MethodDoc {
    short: "Returns each recipient the report is about, in the order of the report.",
    params: &[],
    ret: "The recipients. At most 1,000 are read.",
    errors: &[],
};

/// The slots of [`RECIPIENT`], in order.
const RECIPIENT_SLOTS: &[&str] = &["address", "action", "status", "permanent", "diagnostic"];

/// One [`RECIPIENT`] reader with no parameter.
const fn recipient_reader(
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

/// `Core\Mail\ReportRecipient` — one recipient of a delivery report.
// rank: 886
pub(crate) const RECIPIENT: CoreClass = CoreClass {
    name: RECIPIENT_NAME,
    doc: Some(&RECIPIENT_CARD),
    methods: &[],
    instance: &[
        recipient_reader(
            "address",
            CoreTy::Str,
            "nvs_core_mail_report_recipient_address",
            &ADDRESS_DOC,
        ),
        recipient_reader(
            "action",
            CoreTy::Enum(ACTION_NAME),
            "nvs_core_mail_report_recipient_action",
            &ACTION_MEMBER_DOC,
        ),
        recipient_reader(
            "status",
            CoreTy::Str,
            "nvs_core_mail_report_recipient_status",
            &STATUS_DOC,
        ),
        recipient_reader(
            "isPermanent",
            CoreTy::Bool,
            "nvs_core_mail_report_recipient_is_permanent",
            &PERMANENT_DOC,
        ),
        recipient_reader(
            "diagnostic",
            CoreTy::Nullable(&CoreTy::Str),
            "nvs_core_mail_report_recipient_diagnostic",
            &DIAGNOSTIC_DOC,
        ),
    ],
    slots: RECIPIENT_SLOTS,
    constants: &[],
};

/// [`RECIPIENT`]'s slot types, for `crate::instance`'s table: [`RECIPIENT_SLOTS`]' order.
pub(crate) const RECIPIENT_SLOT_TYPES: &[CoreTy] = &[
    CoreTy::Str,
    CoreTy::Enum(ACTION_NAME),
    CoreTy::Str,
    CoreTy::Bool,
    CoreTy::Nullable(&CoreTy::Str),
];

/// [`RECIPIENT`]'s card.
const RECIPIENT_CARD: ClassDoc = ClassDoc {
    short: "One recipient of a delivery report: the address, what the server did, and the \
            status code. Every text it returns is `tainted`, because the server that sent the \
            report wrote it.",
};

/// `address`' card.
const ADDRESS_DOC: MethodDoc = MethodDoc {
    short: "Returns the address that the server tried to deliver to. It comes from the \
            `Final-Recipient` line of the report. The type `rfc822;` and the angle brackets are \
            removed.",
    params: &[],
    ret: "The address, such as `user@example.com`. It is tainted, because the server that sent \
          the report wrote it.",
    errors: &[],
};

/// `action`'s card.
const ACTION_MEMBER_DOC: MethodDoc = MethodDoc {
    short: "Returns what the server did for this recipient.",
    params: &[],
    ret: "A `Core\\Mail\\ReportRecipient::Action` case.",
    errors: &[],
};

/// `status`' card.
const STATUS_DOC: MethodDoc = MethodDoc {
    short: "Returns the status code, such as `5.1.1` for an address that does not exist.",
    params: &[],
    ret: "The status code from the `Status` line of the report. The code is three numbers \
          joined by dots, such as `5.1.1`. The result is an empty string when the report has no \
          such code. It is tainted, because the server that sent the report wrote it.",
    errors: &[],
};

/// `isPermanent`'s card.
const PERMANENT_DOC: MethodDoc = MethodDoc {
    short: "Returns `true` when the status code starts with `5`. The server will not deliver \
            to this address if you send again. A code that starts with `4` is temporary.",
    params: &[],
    ret: "`true` for a permanent failure.",
    errors: &[],
};

/// `diagnostic`'s card.
const DIAGNOSTIC_DOC: MethodDoc = MethodDoc {
    short: "Returns the server's own message, such as `550 5.1.1 User unknown`. It comes from \
            the `Diagnostic-Code` line of the report.",
    params: &[],
    ret: "The message, or `null` when the report gives none. It is tainted, because the server \
          that sent the report wrote it.",
    errors: &[],
};

/// `recipients` as a [`REPORT`] instance.
pub(crate) fn report_value(recipients: &[Recipient]) -> Value {
    let mut list = NvsArray::new();
    for recipient in recipients {
        list.append(crate::instance::build(
            &RECIPIENT,
            [
                Value::text(&recipient.address),
                Value::enum_int(recipient.action as i64),
                Value::text(&recipient.status),
                Value::bool(recipient.is_permanent()),
                recipient
                    .diagnostic
                    .as_deref()
                    .map_or_else(Value::null, Value::text),
            ],
        ));
    }
    crate::instance::build(&REPORT, [Value::array(list)])
}

/// One reader's body: the slot `name` of `class`.
fn read(args: &[Value], class: &'static CoreClass, name: &str) -> Result<Value, Fault> {
    let index = class
        .slots
        .iter()
        .position(|slot| *slot == name)
        .ok_or_else(|| Fault::fatal(format!("{} has no slot `{name}`", class.name)))?;
    crate::instance::read_slot(args, class, index, name)
}

/// Declares one slot reader per `(helper, class, slot)`.
macro_rules! readers {
    ($($helper:ident => $class:ident . $slot:literal,)*) => {
        $(
            nvs_runtime::nvs_helper! {
                #[doc = concat!("`->", $slot, "()`: the slot of the same name.")]
                fn $helper(_ctx, args: [1]) {
                    read(args, &$class, $slot)
                }
            }
        )*
    };
}

readers! {
    nvs_core_mail_report_recipients => REPORT."recipients",
    nvs_core_mail_report_recipient_address => RECIPIENT."address",
    nvs_core_mail_report_recipient_action => RECIPIENT."action",
    nvs_core_mail_report_recipient_status => RECIPIENT."status",
    nvs_core_mail_report_recipient_is_permanent => RECIPIENT."permanent",
    nvs_core_mail_report_recipient_diagnostic => RECIPIENT."diagnostic",
}

/// This module's arm of the `Core` symbol table — see [`crate::address`].
pub(super) fn address(symbol: &str) -> Option<nvs_runtime::Entry> {
    let helper: nvs_runtime::NvsFn = match symbol {
        "nvs_core_mail_report_recipients" => nvs_core_mail_report_recipients,
        "nvs_core_mail_report_recipient_address" => nvs_core_mail_report_recipient_address,
        "nvs_core_mail_report_recipient_action" => nvs_core_mail_report_recipient_action,
        "nvs_core_mail_report_recipient_status" => nvs_core_mail_report_recipient_status,
        "nvs_core_mail_report_recipient_is_permanent" => {
            nvs_core_mail_report_recipient_is_permanent
        }
        "nvs_core_mail_report_recipient_diagnostic" => nvs_core_mail_report_recipient_diagnostic,
        _ => return None,
    };
    Some(nvs_runtime::Entry::borrowed((helper as *const ()).cast()))
}

#[cfg(test)]
mod tests;

/// What a message is, in the order [`classified`] checks: the first match is the kind.
///
/// The discriminants are `Core\Mail\Message::Kind`'s case values, so a case is appended and never
/// inserted.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Kind {
    /// A delivery report with at least one `failed` recipient.
    Bounce = 0,
    /// A delivery report with no `failed` recipient and at least one `delayed` one.
    Delay = 1,
    /// A feedback report: a recipient marked the message as spam.
    Complaint = 2,
    /// A message a program or a server wrote in answer, such as an out-of-office reply.
    AutoReply = 3,
    /// A message with `In-Reply-To`.
    Reply = 4,
    /// None of the above.
    #[default]
    Other = 5,
}

/// A delivery report recipient's `Action` (RFC 3464 § 2.3.3). The discriminants are
/// `Core\Mail\ReportRecipient::Action`'s case values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Action {
    /// The message could not be delivered.
    Failed = 0,
    /// Delivery is late, and the server still tries.
    Delayed = 1,
    /// The message was delivered.
    Delivered = 2,
    /// The message was passed to a server that writes no report.
    Relayed = 3,
    /// The message was delivered to a list, which sends it on.
    Expanded = 4,
}

impl Action {
    /// The action an `Action` field names, or `None` for a value RFC 3464 does not define.
    fn of(written: &str) -> Option<Self> {
        let word = written.split([';', '(']).next().unwrap_or_default().trim();
        [
            ("failed", Self::Failed),
            ("delayed", Self::Delayed),
            ("delivered", Self::Delivered),
            ("relayed", Self::Relayed),
            ("expanded", Self::Expanded),
        ]
        .into_iter()
        .find(|(name, _)| word.eq_ignore_ascii_case(name))
        .map(|(_, action)| action)
    }
}

/// One recipient of a delivery report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Recipient {
    /// The `Final-Recipient` address, its address type removed.
    pub address: String,
    /// The `Action`.
    pub action: Action,
    /// The `Status`, such as `5.1.1`, or empty when the report gives no RFC 3463 code.
    pub status: String,
    /// The `Diagnostic-Code` text, its type removed.
    pub diagnostic: Option<String>,
}

impl Recipient {
    /// Whether the status is of class 5: the server will never deliver it.
    pub(crate) fn is_permanent(&self) -> bool {
        self.status.starts_with("5.")
    }
}

/// Whether `word` is an RFC 3463 status code: a class of `2`, `4` or `5`, then a subject and a
/// detail of one to three digits each, joined by dots.
fn is_status_code(word: &str) -> bool {
    let number =
        |text: &str| (1..=3).contains(&text.len()) && text.bytes().all(|b| b.is_ascii_digit());
    let mut pieces = word.split('.');
    matches!(pieces.next(), Some("2" | "4" | "5"))
        && pieces.next().is_some_and(number)
        && pieces.next().is_some_and(number)
        && pieces.next().is_none()
}

/// Adds `more` to `value`, cut at a character so the value is at most [`MAX_FIELD_BYTES`].
fn push_bounded(value: &mut String, more: &str) {
    let room = MAX_FIELD_BYTES.saturating_sub(value.len());
    let mut end = more.len().min(room);
    while !more.is_char_boundary(end) {
        end -= 1;
    }
    value.push_str(&more[..end]);
}

/// What [`classified`] read.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(crate) struct Classified {
    /// The kind.
    pub kind: Kind,
    /// The `Message-ID` this message is about, without angle brackets.
    pub answers: Option<String>,
    /// A delivery report's recipients, `None` for a message that is not a delivery report.
    pub report: Option<Vec<Recipient>>,
    /// The `+tag` of the first recipient address that has a valid one.
    pub bounce_tag: Option<String>,
}

/// The headers common auto-responders write, as a lowercased name and the value it needs: any
/// value for `None`, or one of its comma- or space-separated words for `Some`. A row is added only
/// with a sample message that needs it.
pub(crate) const AUTO_REPLY: &[(&str, Option<&str>)] = &[
    ("x-autoreply", None),
    ("x-autorespond", None),
    ("precedence", Some("auto_reply")),
    ("x-auto-response-suppress", Some("oof")),
];

/// `root` classified. `headers` are its first header lines, lowercased names and decoded values,
/// as [`super::parse`] read them, and `to` its `To` addresses.
pub(crate) fn classified(
    root: &MessageRef<'_>,
    headers: &[(String, String)],
    in_reply_to: Option<&str>,
    references: &[String],
    to: &[super::Mailbox],
) -> Classified {
    let report_type = root
        .content_type()
        .filter(|kind| {
            kind.ctype().eq_ignore_ascii_case("multipart")
                && kind
                    .subtype()
                    .is_some_and(|sub| sub.eq_ignore_ascii_case("report"))
        })
        .and_then(|kind| kind.attribute("report-type"))
        .map(str::to_ascii_lowercase);
    let delivery = report_type.as_deref() == Some("delivery-status");
    let feedback = report_type.as_deref() == Some("feedback-report");
    let report = delivery.then(|| report_of(root));
    let has = |action: Action| {
        report
            .as_ref()
            .is_some_and(|list| list.iter().any(|recipient| recipient.action == action))
    };
    let kind = if has(Action::Failed) {
        Kind::Bounce
    } else if has(Action::Delayed) {
        Kind::Delay
    } else if feedback {
        Kind::Complaint
    } else if is_auto_reply(headers) {
        Kind::AutoReply
    } else if in_reply_to.is_some() {
        Kind::Reply
    } else {
        Kind::Other
    };
    let original = (delivery || feedback).then(|| original_id(root)).flatten();
    let answers = original
        .or_else(|| in_reply_to.map(str::to_owned))
        .or_else(|| references.last().cloned());
    Classified {
        kind,
        answers,
        report,
        bounce_tag: bounce_tag(headers, to),
    }
}

/// Whether `headers` mark an automatic answer: `Auto-Submitted` other than `no`, or a row of
/// [`AUTO_REPLY`].
pub(crate) fn is_auto_reply(headers: &[(String, String)]) -> bool {
    headers.iter().any(|(name, value)| {
        if name == "auto-submitted" {
            let word = value.split([';', '(']).next().unwrap_or_default().trim();
            return !word.is_empty() && !word.eq_ignore_ascii_case("no");
        }
        AUTO_REPLY.iter().any(|(row, needs)| {
            name == row
                && needs.is_none_or(|needed| {
                    value
                        .split([',', ' ', '\t'])
                        .any(|word| word.trim().eq_ignore_ascii_case(needed))
                })
        })
    })
}

/// Whether `part` is within the parser's bounds.
fn within(part: &MessagePart<'_>) -> bool {
    !part.flags().contains(PartFlags::LIMIT_REACHED)
}

/// The recipients of `root`'s first delivery-status part, or none when it has no such part.
fn report_of(root: &MessageRef<'_>) -> Vec<Recipient> {
    root.parts()
        .filter(within)
        .find(|part| {
            part.is_content_type("message", "delivery-status")
                || part.is_content_type("message", "global-delivery-status")
        })
        .map(|part| recipients(&part.decoded()))
        .unwrap_or_default()
}

/// The `Message-ID` of the original message a report returns: its `message/rfc822` part, or the
/// headers of its `text/rfc822-headers` part.
fn original_id(root: &MessageRef<'_>) -> Option<String> {
    root.parts().filter(within).find_map(|part| {
        if let PartKind::Message(nested) = part.kind() {
            return nested
                .message_id()
                .filter(|id| !id.is_empty())
                .map(str::to_owned);
        }
        let headers_only = part.is_content_type("text", "rfc822-headers")
            || part.is_content_type("message", "global-headers");
        if !headers_only {
            return None;
        }
        let bytes = part.decoded();
        let parsed = MessageParser::new()
            .max_depth(super::MAX_DEPTH)
            .max_parts(super::MAX_PARTS)
            .parse(&bytes)?;
        parsed
            .root()
            .message_id()
            .filter(|id| !id.is_empty())
            .map(str::to_owned)
    })
}

/// A `message/delivery-status` body read into its recipients: each group of fields, separated
/// by a blank line, that has a `Final-Recipient` with an address and an `Action` RFC 3464
/// defines. The first group is the report's own fields, and it is read like the others: it has
/// no `Final-Recipient`, so it gives no recipient.
pub(crate) fn recipients(body: &[u8]) -> Vec<Recipient> {
    let text = String::from_utf8_lossy(body);
    let mut groups: Vec<Vec<(String, String)>> = vec![Vec::new()];
    let mut lines = 0_usize;
    for line in text.lines() {
        if groups.len() > MAX_RECIPIENTS + 1 {
            break;
        }
        let group = groups.last_mut().expect("there is always a group");
        if line.trim().is_empty() {
            if !group.is_empty() {
                groups.push(Vec::new());
                lines = 0;
            }
            continue;
        }
        if lines >= super::MAX_HEADERS {
            continue;
        }
        lines += 1;
        if line.starts_with([' ', '\t']) {
            if let Some((_, value)) = group.last_mut() {
                push_bounded(value, " ");
                push_bounded(value, line.trim());
            }
        } else if let Some((name, written)) = line.split_once(':') {
            let mut value = String::new();
            push_bounded(&mut value, written.trim());
            group.push((name.trim().to_ascii_lowercase(), value));
        }
    }
    groups
        .iter()
        .filter_map(|group| {
            let field = |name: &str| {
                group
                    .iter()
                    .find(|(written, _)| written == name)
                    .map(|(_, value)| value.as_str())
            };
            let written = typed_value(field("final-recipient")?);
            let address = if written.contains('<') {
                bare_address(written)
            } else {
                written.split_whitespace().next().unwrap_or_default()
            };
            if !super::is_address(address) {
                return None;
            }
            Some(Recipient {
                address: address.to_owned(),
                action: Action::of(field("action")?)?,
                status: field("status")
                    .and_then(|status| status.split_whitespace().next())
                    .filter(|word| is_status_code(word))
                    .unwrap_or_default()
                    .to_owned(),
                diagnostic: field("diagnostic-code")
                    .map(typed_value)
                    .filter(|text| !text.is_empty())
                    .map(str::to_owned),
            })
        })
        .take(MAX_RECIPIENTS)
        .collect()
}

/// A typed field's value, its `type;` removed: `rfc822; a@example.com` is `a@example.com`.
fn typed_value(written: &str) -> &str {
    written
        .split_once(';')
        .map_or(written, |(_, value)| value)
        .trim()
}

/// The `+tag` of the first `Delivered-To`, else `X-Original-To`, else `To` address that has one
/// which fits `send`'s bounce tag rule.
pub(crate) fn bounce_tag(headers: &[(String, String)], to: &[super::Mailbox]) -> Option<String> {
    let written = |name: &'static str| {
        headers
            .iter()
            .filter(move |(written, _)| written == name)
            .map(|(_, value)| bare_address(value))
    };
    written("delivered-to")
        .chain(written("x-original-to"))
        .chain(to.iter().map(|mailbox| mailbox.address.as_str()))
        .find_map(|address| {
            let (local, _) = address.rsplit_once('@')?;
            let (_, tag) = local.split_once('+')?;
            super::super::smtp::is_bounce_tag(tag).then(|| tag.to_owned())
        })
}

/// The address in a header value: inside its angle brackets when it has them.
fn bare_address(value: &str) -> &str {
    match (value.find('<'), value.rfind('>')) {
        (Some(open), Some(close)) if open < close => value[open + 1..close].trim(),
        _ => value.trim(),
    }
}
