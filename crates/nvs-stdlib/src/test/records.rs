//! `Core\Test::logs` and `Core\Test::sentMail`, with `Core\Test\LogRecord` and
//! `Core\Test\SentMail`: a test reads the log records written and the mail sent by
//! the test, its requests and its runs (`rule:testing/a-test-reads-its-logs-and-mail`).
//!
//! The records are kept in `nvs_runtime::test_records`, as owned text, because a
//! request's values may not cross into the test's isolate. This module builds the
//! test's own instances from that text each time the test reads them.
//!
//! **What it spends:** per call, one instance for each record or message kept, freed
//! with the list it returns.

use nvs_runtime::{Fault, NvsArray, Value};

use crate::registry::{ClassDoc, CoreClass, CoreMethod, CoreTy, MethodDoc};

/// `Core\Test\LogRecord`'s fully-qualified name.
pub(crate) const LOG_RECORD_NAME: &str = r"Core\Test\LogRecord";

/// `Core\Test\SentMail`'s fully-qualified name.
pub(crate) const SENT_MAIL_NAME: &str = r"Core\Test\SentMail";

/// [`LOG_RECORD`]'s slots: the level's name, the message, and the fields as the
/// JSON text a `[log]` line writes for them, or empty for none.
const LEVEL_SLOT: usize = 0;
/// See [`LEVEL_SLOT`].
const MESSAGE_SLOT: usize = 1;
/// See [`LEVEL_SLOT`].
const FIELDS_SLOT: usize = 2;

/// One reader: a member with no parameter.
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
        symbol,
        doc: Some(doc),
    }
}

/// One log record a test caused. A `Core`-owned instance beside `Core\Test`
/// for the reason `Core\Test\Response` is one.
// rank: 476
pub(crate) const LOG_RECORD: CoreClass = CoreClass {
    name: LOG_RECORD_NAME,
    doc: Some(&LOG_RECORD_CARD),
    methods: &[],
    instance: &[
        reader(
            "level",
            CoreTy::Str,
            "nvs_core_test_log_record_level",
            &LEVEL_DOC,
        ),
        reader(
            "message",
            CoreTy::Str,
            "nvs_core_test_log_record_message",
            &MESSAGE_DOC,
        ),
        reader(
            "fields",
            CoreTy::Array(&CoreTy::Mixed),
            "nvs_core_test_log_record_fields",
            &FIELDS_DOC,
        ),
    ],
    slots: &["level", "message", "fields"],
    constants: &[],
};

/// [`SENT_MAIL`]'s slots, in the order of its readers.
const SENT_MAIL_SLOTS: &[&str] = &[
    "endpoint", "to", "subject", "text", "cc", "bcc", "replyTo", "html",
];

/// One message `Core\Mail::send` was given in a test. Its readers are the
/// parameters of `Core\Mail::send`, read back.
// rank: 477
pub(crate) const SENT_MAIL: CoreClass = CoreClass {
    name: SENT_MAIL_NAME,
    doc: Some(&SENT_MAIL_CARD),
    methods: &[],
    instance: &[
        reader(
            "endpoint",
            CoreTy::Str,
            "nvs_core_test_sent_mail_endpoint",
            &ENDPOINT_DOC,
        ),
        reader(
            "to",
            CoreTy::Array(&CoreTy::Str),
            "nvs_core_test_sent_mail_to",
            &TO_DOC,
        ),
        reader(
            "subject",
            CoreTy::Str,
            "nvs_core_test_sent_mail_subject",
            &SUBJECT_DOC,
        ),
        reader(
            "text",
            CoreTy::Str,
            "nvs_core_test_sent_mail_text",
            &TEXT_DOC,
        ),
        reader(
            "cc",
            CoreTy::Array(&CoreTy::Str),
            "nvs_core_test_sent_mail_cc",
            &CC_DOC,
        ),
        reader(
            "bcc",
            CoreTy::Array(&CoreTy::Str),
            "nvs_core_test_sent_mail_bcc",
            &BCC_DOC,
        ),
        reader(
            "replyTo",
            CoreTy::Nullable(&CoreTy::Str),
            "nvs_core_test_sent_mail_reply_to",
            &REPLY_TO_DOC,
        ),
        reader(
            "html",
            CoreTy::Nullable(&CoreTy::Str),
            "nvs_core_test_sent_mail_html",
            &HTML_DOC,
        ),
    ],
    slots: SENT_MAIL_SLOTS,
    constants: &[],
};

/// `Core\Test::logs`'s reference card — `rule:core-api/reference-card`.
pub(crate) const LOGS_DOC: MethodDoc = MethodDoc {
    short: "Returns every log record written since this test started. This includes the records \
            of every request and run the test made. Each record in the list is also written to \
            the `[log] target`, the same as outside a test.",
    params: &[],
    ret: "One `Core\\Test\\LogRecord` for each call to `Core\\Log::write`, in the order written. A \
          record below `[log] level` is not written anywhere, so it is not in the list either. \
          The list is empty \
          when nothing was written.",
    errors: &[],
};

/// `Core\Test::sentMail`'s reference card — `rule:core-api/reference-card`.
pub(crate) const SENT_MAIL_DOC: MethodDoc = MethodDoc {
    short: "Returns every message `Core\\Mail::send` was given since this test started. This \
            includes the messages of every request and run the test made. Under `nvs test`, \
            `Core\\Mail::send` sends nothing and opens no connection.",
    params: &[],
    ret: "One `Core\\Test\\SentMail` for each message, in the order sent. The list is empty when \
          nothing was sent.",
    errors: &[],
};

/// `Core\Test\LogRecord`'s class card — `rule:core-api/reference-card`.
const LOG_RECORD_CARD: ClassDoc = ClassDoc {
    short: "One log record that a test, or a request or run it made, wrote with \
            `Core\\Log::write`. `Core\\Test::logs` returns them. You cannot create one yourself.",
};

/// `Core\Test\LogRecord::level`'s reference card.
const LEVEL_DOC: MethodDoc = MethodDoc {
    short: "Returns the level of the record, in lower case.",
    params: &[],
    ret: "`\"debug\"`, `\"info\"`, `\"warn\"`, `\"error\"` or `\"critical\"`. This is the \
          `level` a JSON log line has.",
    errors: &[],
};

/// `Core\Test\LogRecord::message`'s reference card.
const MESSAGE_DOC: MethodDoc = MethodDoc {
    short: "Returns the message of the record.",
    params: &[],
    ret: "The `$message` given to `Core\\Log::write`.",
    errors: &[],
};

/// `Core\Test\LogRecord::fields`'s reference card.
const FIELDS_DOC: MethodDoc = MethodDoc {
    short: "Returns the extra fields of the record, as the log wrote them.",
    params: &[],
    ret: "The `$fields` given to `Core\\Log::write`, in the order given. A string, a number, a \
          `bool` and `null` keep their value. JSON has no type for some values, such as `bytes`. \
          Such a value is a string: the same text that a JSON log line has for it. The array is \
          empty when the record has no fields.",
    errors: &[],
};

/// `Core\Test\SentMail`'s class card — `rule:core-api/reference-card`.
const SENT_MAIL_CARD: ClassDoc = ClassDoc {
    short: "One message that a test, or a request or run it made, gave to `Core\\Mail::send`. \
            Each method returns one parameter of that call. `Core\\Test::sentMail` returns them. \
            You cannot create one yourself.",
};

/// `Core\Test\SentMail::endpoint`'s reference card.
const ENDPOINT_DOC: MethodDoc = MethodDoc {
    short: "Returns the name of the `[mail.<name>]` block the message was sent through.",
    params: &[],
    ret: "The `$endpoint` given to `Core\\Mail::send`.",
    errors: &[],
};

/// `Core\Test\SentMail::to`'s reference card.
const TO_DOC: MethodDoc = MethodDoc {
    short: "Returns the addresses the message was sent to.",
    params: &[],
    ret: "The `$to` addresses, in the order given.",
    errors: &[],
};

/// `Core\Test\SentMail::subject`'s reference card.
const SUBJECT_DOC: MethodDoc = MethodDoc {
    short: "Returns the subject of the message.",
    params: &[],
    ret: "The `$subject` given to `Core\\Mail::send`.",
    errors: &[],
};

/// `Core\Test\SentMail::text`'s reference card.
const TEXT_DOC: MethodDoc = MethodDoc {
    short: "Returns the plain-text body of the message.",
    params: &[],
    ret: "The `$text` given to `Core\\Mail::send`.",
    errors: &[],
};

/// `Core\Test\SentMail::cc`'s reference card.
const CC_DOC: MethodDoc = MethodDoc {
    short: "Returns the `cc` addresses of the message.",
    params: &[],
    ret: "The `cc` addresses, in the order given. The list is empty when none were given.",
    errors: &[],
};

/// `Core\Test\SentMail::bcc`'s reference card.
const BCC_DOC: MethodDoc = MethodDoc {
    short: "Returns the `bcc` addresses of the message.",
    params: &[],
    ret: "The `bcc` addresses, in the order given. The list is empty when none were given.",
    errors: &[],
};

/// `Core\Test\SentMail::replyTo`'s reference card.
const REPLY_TO_DOC: MethodDoc = MethodDoc {
    short: "Returns the `replyTo` address of the message.",
    params: &[],
    ret: "The `replyTo` address, or `null` when none was given.",
    errors: &[],
};

/// `Core\Test\SentMail::html`'s reference card.
const HTML_DOC: MethodDoc = MethodDoc {
    short: "Returns the HTML body of the message.",
    params: &[],
    ret: "The `html` body, or `null` when none was given.",
    errors: &[],
};

/// `addresses` as an `array<string>` list.
fn list_of(addresses: &[String]) -> Value {
    let mut out = NvsArray::new();
    for address in addresses {
        out.append(Value::text(address));
    }
    Value::array(out)
}

/// `text` as a `?string`.
fn optional(text: Option<&str>) -> Value {
    text.map_or_else(Value::null, Value::text)
}

nvs_runtime::nvs_helper! {
    /// `Core\Test::logs(): array<Core\Test\LogRecord>` —
    /// `rule:testing/a-test-reads-its-logs-and-mail`.
    fn nvs_core_test_logs(ctx, args: [0]) {
        let _ = args;
        let mut out = NvsArray::new();
        if let Some(records) = ctx.test_records() {
            for record in records.logs().iter() {
                out.append(crate::instance::build(
                    &LOG_RECORD,
                    [
                        Value::text(record.level),
                        Value::text(&record.message),
                        Value::text(&record.fields),
                    ],
                ));
            }
        }
        Ok(Value::array(out))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Test::sentMail(): array<Core\Test\SentMail>` —
    /// `rule:testing/a-test-reads-its-logs-and-mail`.
    fn nvs_core_test_sent_mail(ctx, args: [0]) {
        let _ = args;
        let mut out = NvsArray::new();
        if let Some(records) = ctx.test_records() {
            for mail in records.sent().iter() {
                out.append(crate::instance::build(
                    &SENT_MAIL,
                    [
                        Value::text(&mail.endpoint),
                        list_of(&mail.to),
                        Value::text(&mail.subject),
                        Value::text(&mail.text),
                        list_of(&mail.cc),
                        list_of(&mail.bcc),
                        optional(mail.reply_to.as_deref()),
                        optional(mail.html.as_deref()),
                    ],
                ));
            }
        }
        Ok(Value::array(out))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Test\LogRecord::level(): string`.
    fn nvs_core_test_log_record_level(_ctx, args: [1]) {
        crate::instance::read_slot(args, &LOG_RECORD, LEVEL_SLOT, "level")
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Test\LogRecord::message(): string`.
    fn nvs_core_test_log_record_message(_ctx, args: [1]) {
        crate::instance::read_slot(args, &LOG_RECORD, MESSAGE_SLOT, "message")
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Test\LogRecord::fields(): array<mixed>`, read from the JSON text
    /// the record kept. Each call builds a new array.
    fn nvs_core_test_log_record_fields(_ctx, args: [1]) {
        let receiver = crate::instance::receiver(args[0], &LOG_RECORD, "fields")?;
        let held = crate::instance::slot(receiver, FIELDS_SLOT);
        // Unreachable from source: `nvs_core_test_logs` fills the slot with a
        // `string` and nothing else writes it.
        let text = held.as_text().ok_or_else(|| {
            Fault::fatal(format!("{LOG_RECORD_NAME}::fields found a non-`string` slot"))
        })?;
        if text.is_empty() {
            return Ok(Value::array(NvsArray::new()));
        }
        // Unreachable from source: the text is a JSON object `serde_json` wrote.
        crate::json::read(text, crate::json::DEFAULT_MAX_DEPTH_U32).map_err(|why| {
            Fault::fatal(format!("{LOG_RECORD_NAME}::fields could not read its fields: {why}"))
        })
    }
}

/// One [`SENT_MAIL`] reader: the slot named `name`.
fn sent_mail_slot(args: &[Value], name: &str) -> Result<Value, Fault> {
    let index = SENT_MAIL_SLOTS
        .iter()
        .position(|slot| *slot == name)
        .ok_or_else(|| Fault::fatal(format!("{SENT_MAIL_NAME} has no slot `{name}`")))?;
    crate::instance::read_slot(args, &SENT_MAIL, index, name)
}

nvs_runtime::nvs_helper! {
    /// `Core\Test\SentMail::endpoint(): string`.
    fn nvs_core_test_sent_mail_endpoint(_ctx, args: [1]) {
        sent_mail_slot(args, "endpoint")
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Test\SentMail::to(): array<string>`.
    fn nvs_core_test_sent_mail_to(_ctx, args: [1]) {
        sent_mail_slot(args, "to")
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Test\SentMail::subject(): string`.
    fn nvs_core_test_sent_mail_subject(_ctx, args: [1]) {
        sent_mail_slot(args, "subject")
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Test\SentMail::text(): string`.
    fn nvs_core_test_sent_mail_text(_ctx, args: [1]) {
        sent_mail_slot(args, "text")
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Test\SentMail::cc(): array<string>`.
    fn nvs_core_test_sent_mail_cc(_ctx, args: [1]) {
        sent_mail_slot(args, "cc")
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Test\SentMail::bcc(): array<string>`.
    fn nvs_core_test_sent_mail_bcc(_ctx, args: [1]) {
        sent_mail_slot(args, "bcc")
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Test\SentMail::replyTo(): ?string`.
    fn nvs_core_test_sent_mail_reply_to(_ctx, args: [1]) {
        sent_mail_slot(args, "replyTo")
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Test\SentMail::html(): ?string`.
    fn nvs_core_test_sent_mail_html(_ctx, args: [1]) {
        sent_mail_slot(args, "html")
    }
}

/// Where each member's helper lives.
pub(crate) fn address(symbol: &str) -> Option<nvs_runtime::Entry> {
    Some(nvs_runtime::Entry::borrowed(match symbol {
        "nvs_core_test_logs" => (nvs_core_test_logs as *const ()).cast(),
        "nvs_core_test_sent_mail" => (nvs_core_test_sent_mail as *const ()).cast(),
        "nvs_core_test_log_record_level" => (nvs_core_test_log_record_level as *const ()).cast(),
        "nvs_core_test_log_record_message" => {
            (nvs_core_test_log_record_message as *const ()).cast()
        }
        "nvs_core_test_log_record_fields" => (nvs_core_test_log_record_fields as *const ()).cast(),
        "nvs_core_test_sent_mail_endpoint" => {
            (nvs_core_test_sent_mail_endpoint as *const ()).cast()
        }
        "nvs_core_test_sent_mail_to" => (nvs_core_test_sent_mail_to as *const ()).cast(),
        "nvs_core_test_sent_mail_subject" => (nvs_core_test_sent_mail_subject as *const ()).cast(),
        "nvs_core_test_sent_mail_text" => (nvs_core_test_sent_mail_text as *const ()).cast(),
        "nvs_core_test_sent_mail_cc" => (nvs_core_test_sent_mail_cc as *const ()).cast(),
        "nvs_core_test_sent_mail_bcc" => (nvs_core_test_sent_mail_bcc as *const ()).cast(),
        "nvs_core_test_sent_mail_reply_to" => {
            (nvs_core_test_sent_mail_reply_to as *const ()).cast()
        }
        "nvs_core_test_sent_mail_html" => (nvs_core_test_sent_mail_html as *const ()).cast(),
        _ => return None,
    }))
}
