//! A `[mail.<name>] capture = true` block: the message is kept and no connection is opened, as it
//! is under `nvs test`. Outside a test, a kept message is written to the debug spool as one `mail`
//! line, so the viewer's Mail page can show it.
//!
//! **Only a capture block writes its content to disk.** A block that sends files only its `mail`
//! event, with the number of recipients and whether the send returned. A sent message can carry a
//! password-reset link, and its content must not land in the spool.
//!
//! **What it spends:** per kept message, one copy of its bodies and its attachments as base64,
//! for the length of the write, and one more parse of the inlined HTML for the CSS report. All of
//! it only where `[debug] store` is on.

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use nvs_runtime::spool::{Mail, MailPart, Record};
use nvs_runtime::test_records::KeptMail;
use nvs_runtime::{Ctx, Value};

/// Writes `kept` to the debug spool as a `mail` line, when `[debug] store` is on for the request.
/// `body` is the call's `$body`, read again for the HTML as the program wrote it.
pub(super) fn spool(ctx: &Ctx, from: &str, body: Value, kept: &KeptMail) {
    if ctx.config().and_then(nvs_runtime::spool::Store::of).is_none() {
        return;
    }
    nvs_runtime::spool::record(ctx, Record::Mail(Box::new(line(from, body, kept))));
}

/// The `mail` line of `kept`.
fn line(from: &str, body: Value, kept: &KeptMail) -> Mail {
    let report = kept.html.as_deref().map(super::css::report).unwrap_or_default();
    let html_written = crate::html::markup_slot(body, "body")
        .ok()
        .and_then(|markup| markup.as_text().map(str::to_owned));
    Mail {
        endpoint: kept.endpoint.clone(),
        message_id: kept.message_id.clone(),
        from: from.to_owned(),
        from_name: kept.from_name.clone(),
        to: kept.to.clone(),
        cc: kept.cc.clone(),
        bcc: kept.bcc.clone(),
        reply_to: kept.reply_to.clone(),
        subject: kept.subject.clone(),
        text: kept.text.clone(),
        html_written,
        html: kept.html.clone(),
        kept_css: report.kept,
        unused_css: report.unused,
        attachments: kept
            .attachments
            .iter()
            .map(|part| MailPart {
                name: part.name.clone(),
                media_type: part.media_type.clone(),
                content_id: part.content_id.clone(),
                size: u64::try_from(part.data.len()).unwrap_or(u64::MAX),
                base64: STANDARD.encode(&part.data),
            })
            .collect(),
        in_reply_to: kept.in_reply_to.clone(),
        references: kept.references.clone(),
        unsubscribe: kept.unsubscribe.clone(),
        auto_submitted: kept.auto_submitted,
        bounce_tag: kept.bounce_tag.clone(),
        elided: false,
    }
}
