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

#[cfg(test)]
mod tests {
    use super::*;
    use nvs_runtime::Ctx;
    use nvs_runtime::test_records::{KeptLog, KeptMail, TestRecords};

    /// Releases the reference a member's answer carries.
    fn dropped(value: Value) {
        #[expect(unsafe_code, reason = "the member transferred what it answered")]
        // SAFETY: a member's answer carries a reference of its own, and this
        // driver is the caller that would otherwise hold it.
        unsafe {
            value.release();
        }
    }

    /// The element of `list` at `index`, borrowed from the list.
    fn at(list: Value, index: i64) -> Value {
        list.as_array()
            .expect("the member returns an array")
            .get_index(index)
            .expect("the list has this element")
    }

    /// The number of elements in `list`.
    fn count(list: Value) -> usize {
        list.as_array()
            .expect("the member returns an array")
            .count()
    }

    /// `list`'s elements as owned text, in order.
    fn texts(list: Value) -> Vec<String> {
        let length = i64::try_from(count(list)).expect("a short list");
        (0..length)
            .map(|index| {
                at(list, index)
                    .as_text()
                    .expect("each element is a string")
                    .to_owned()
            })
            .collect()
    }

    /// Calls `member` on `record`, passes the answer to `with`, and releases it.
    fn read<T>(
        ctx: &mut Ctx,
        member: nvs_runtime::NvsFn,
        record: Value,
        with: impl FnOnce(Value) -> T,
    ) -> T {
        let answer = nvs_runtime::call(member, ctx, &[record]).expect("a reader cannot fail");
        let out = with(answer);
        dropped(answer);
        out
    }

    /// `level` and `message` return the text the record kept, `fields` reads the
    /// kept JSON object back into an array with its keys in order and a record
    /// with no fields has an empty one, every read is the same, and a receiver
    /// that is not a record throws.
    // covers: Core\Test\LogRecord::level, Core\Test\LogRecord::message, Core\Test\LogRecord::fields
    #[test]
    fn log_record_members_read_the_kept_record_and_no_fields_is_an_empty_array() {
        let mut ctx = Ctx::buffered();
        let records = TestRecords::new();
        records.log(KeptLog {
            level: "warn",
            message: "login failed".to_owned(),
            fields: r#"{"user":"ada","tries":3,"locked":false,"ip":null}"#.to_owned(),
        });
        records.log(KeptLog {
            level: "info",
            message: "started".to_owned(),
            fields: String::new(),
        });
        ctx.set_test_records(records);

        let logs = nvs_runtime::call(nvs_core_test_logs, &mut ctx, &[]).expect("logs cannot fail");
        assert_eq!(count(logs), 2, "one record for each write");
        let warning = at(logs, 0);
        let started = at(logs, 1);
        let text = |value: Value| value.as_text().expect("a string").to_owned();
        for _ in 0..2 {
            assert_eq!(
                read(&mut ctx, nvs_core_test_log_record_level, warning, text),
                "warn"
            );
            assert_eq!(
                read(&mut ctx, nvs_core_test_log_record_message, warning, text),
                "login failed"
            );
        }
        assert_eq!(
            read(&mut ctx, nvs_core_test_log_record_level, started, text),
            "info"
        );
        assert_eq!(
            read(&mut ctx, nvs_core_test_log_record_message, started, text),
            "started"
        );

        let fields = nvs_runtime::call(nvs_core_test_log_record_fields, &mut ctx, &[warning])
            .expect("the kept fields are a JSON object");
        {
            let array = fields.as_array().expect("fields are an array");
            assert_eq!(array.count(), 4, "every field is kept");
            assert_eq!(
                array
                    .get(b"user")
                    .and_then(|value| value.as_text().map(str::to_owned)),
                Some("ada".to_owned())
            );
            assert_eq!(array.get(b"tries").and_then(Value::as_int), Some(3));
            assert_eq!(array.get(b"locked").and_then(Value::as_bool), Some(false));
            assert!(
                array
                    .get(b"ip")
                    .is_some_and(|value| value.tag() == Some(nvs_runtime::Tag::Null)),
                "a `null` field keeps its value"
            );
        }
        dropped(fields);
        assert_eq!(
            read(&mut ctx, nvs_core_test_log_record_fields, started, count),
            0,
            "a record with no fields has an empty array"
        );

        assert!(
            nvs_runtime::call(nvs_core_test_log_record_level, &mut ctx, &[Value::int(7)]).is_err(),
            "a receiver that is not a record throws"
        );
        let _ = ctx.take_pending();
        dropped(logs);
    }

    /// Each `SentMail` reader returns the parameter of `Core\Mail::send` it is
    /// named after: a list keeps its order, an absent list is empty and an
    /// absent `replyTo` or `html` is `null`. A context with no records returns
    /// an empty list.
    // covers: Core\Test\SentMail::endpoint, Core\Test\SentMail::to, Core\Test\SentMail::subject, Core\Test\SentMail::text
    // covers: Core\Test\SentMail::cc, Core\Test\SentMail::bcc, Core\Test\SentMail::replyTo, Core\Test\SentMail::html
    #[test]
    fn sent_mail_members_read_each_parameter_and_an_absent_part_is_empty_or_null() {
        let mut ctx = Ctx::buffered();
        let none = nvs_runtime::call(nvs_core_test_sent_mail, &mut ctx, &[])
            .expect("sentMail cannot fail");
        assert_eq!(count(none), 0, "outside a test, nothing is kept");
        dropped(none);

        let records = TestRecords::new();
        records.mail(KeptMail {
            endpoint: "site".to_owned(),
            to: vec!["ada@example.com".to_owned(), "bo@example.com".to_owned()],
            subject: "News".to_owned(),
            text: "The news in text.".to_owned(),
            cc: vec!["editor@example.com".to_owned()],
            bcc: vec![
                "archive@example.com".to_owned(),
                "audit@example.com".to_owned(),
            ],
            reply_to: Some("desk@example.com".to_owned()),
            html: Some("<p>The news.</p>".to_owned()),
        });
        records.mail(KeptMail {
            endpoint: "alerts".to_owned(),
            to: vec!["ops@example.com".to_owned()],
            subject: "Disk full".to_owned(),
            text: String::new(),
            cc: Vec::new(),
            bcc: Vec::new(),
            reply_to: None,
            html: None,
        });
        ctx.set_test_records(records);

        let sent = nvs_runtime::call(nvs_core_test_sent_mail, &mut ctx, &[])
            .expect("sentMail cannot fail");
        assert_eq!(count(sent), 2, "one message for each send, in order");
        let news = at(sent, 0);
        let alert = at(sent, 1);
        let text = |value: Value| value.as_text().map(str::to_owned);

        assert_eq!(
            read(&mut ctx, nvs_core_test_sent_mail_endpoint, news, text).as_deref(),
            Some("site")
        );
        assert_eq!(
            read(&mut ctx, nvs_core_test_sent_mail_to, news, texts),
            ["ada@example.com", "bo@example.com"]
        );
        assert_eq!(
            read(&mut ctx, nvs_core_test_sent_mail_subject, news, text).as_deref(),
            Some("News")
        );
        assert_eq!(
            read(&mut ctx, nvs_core_test_sent_mail_text, news, text).as_deref(),
            Some("The news in text.")
        );
        assert_eq!(
            read(&mut ctx, nvs_core_test_sent_mail_cc, news, texts),
            ["editor@example.com"]
        );
        assert_eq!(
            read(&mut ctx, nvs_core_test_sent_mail_bcc, news, texts),
            ["archive@example.com", "audit@example.com"]
        );
        assert_eq!(
            read(&mut ctx, nvs_core_test_sent_mail_reply_to, news, text).as_deref(),
            Some("desk@example.com")
        );
        assert_eq!(
            read(&mut ctx, nvs_core_test_sent_mail_html, news, text).as_deref(),
            Some("<p>The news.</p>")
        );

        assert_eq!(
            read(&mut ctx, nvs_core_test_sent_mail_endpoint, alert, text).as_deref(),
            Some("alerts")
        );
        assert_eq!(
            read(&mut ctx, nvs_core_test_sent_mail_text, alert, text).as_deref(),
            Some("")
        );
        assert_eq!(read(&mut ctx, nvs_core_test_sent_mail_cc, alert, count), 0);
        assert_eq!(read(&mut ctx, nvs_core_test_sent_mail_bcc, alert, count), 0);
        let is_null = |value: Value| value.tag() == Some(nvs_runtime::Tag::Null);
        assert!(read(
            &mut ctx,
            nvs_core_test_sent_mail_reply_to,
            alert,
            is_null
        ));
        assert!(read(&mut ctx, nvs_core_test_sent_mail_html, alert, is_null));

        assert!(
            nvs_runtime::call(nvs_core_test_sent_mail_to, &mut ctx, &[Value::int(7)]).is_err(),
            "a receiver that is not a message throws"
        );
        let _ = ctx.take_pending();
        dropped(sent);
    }
}
