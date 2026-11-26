//! The kinds, the delivery report, the ID a message answers and the bounce tag, over the
//! hand-written samples in `mail/samples/`.

use super::{AUTO_REPLY, Action, Classified, Kind};

/// A sample's bytes, by file name.
fn sample(name: &str) -> &'static [u8] {
    match name {
        "bounce" => include_bytes!("../../samples/bounce.eml"),
        "delay" => include_bytes!("../../samples/delay.eml"),
        "complaint" => include_bytes!("../../samples/complaint.eml"),
        "auto-reply" => include_bytes!("../../samples/auto-reply.eml"),
        "reply" => include_bytes!("../../samples/reply.eml"),
        "other" => include_bytes!("../../samples/other.eml"),
        _ => panic!("no sample `{name}`"),
    }
}

/// What `raw` is classified as.
fn classified(raw: &[u8]) -> Classified {
    super::super::parse(raw).classified
}

/// A plain message with `header` added above the others.
fn with_header(header: &str) -> Vec<u8> {
    format!(
        "{header}\r\nFrom: ada@example.org\r\nTo: shop@example.com\r\nSubject: Away\r\n\r\nAway.\r\n"
    )
    .into_bytes()
}

// covers: Core\Mail\Message::kind
#[test]
fn a_failed_delivery_report_is_a_bounce() {
    let bounce = classified(sample("bounce"));
    assert_eq!(bounce.kind, Kind::Bounce);
    assert!(bounce.report.is_some());
}

// covers: Core\Mail\Message::kind
#[test]
fn a_delayed_only_report_is_a_delay() {
    let delay = classified(sample("delay"));
    assert_eq!(delay.kind, Kind::Delay);
    let actions: Vec<Action> = delay
        .report
        .expect("a delivery report has a report")
        .iter()
        .map(|recipient| recipient.action)
        .collect();
    assert_eq!(actions, [Action::Delayed, Action::Delivered]);
}

// covers: Core\Mail\Message::report, Core\Mail\Report::recipients
// covers: Core\Mail\ReportRecipient::address, Core\Mail\ReportRecipient::action
// covers: Core\Mail\ReportRecipient::status, Core\Mail\ReportRecipient::diagnostic
#[test]
fn each_recipient_has_its_action_status_and_diagnostic() {
    let report = classified(sample("bounce"))
        .report
        .expect("a bounce has a report");
    assert_eq!(report.len(), 2);
    assert_eq!(report[0].address, "ada@example.org");
    assert_eq!(report[0].action, Action::Failed);
    assert_eq!(report[0].status, "5.1.1");
    assert_eq!(
        report[0].diagnostic.as_deref(),
        Some("550 5.1.1 <ada@example.org>: Recipient address rejected: User unknown")
    );
    assert_eq!(report[1].address, "bo@example.org");
    assert_eq!(report[1].action, Action::Delayed);
    assert_eq!(report[1].status, "4.4.1");
    assert_eq!(report[1].diagnostic, None);
}

// covers: Core\Mail\ReportRecipient::isPermanent
#[test]
fn a_5xx_status_is_permanent_and_a_4xx_is_not() {
    let report = classified(sample("bounce"))
        .report
        .expect("a bounce has a report");
    assert!(report[0].is_permanent());
    assert!(!report[1].is_permanent());
}

// covers: Core\Mail\Message::answers
#[test]
fn a_bounce_answers_the_message_id_in_its_returned_headers() {
    let bounce = classified(sample("bounce"));
    assert_eq!(bounce.answers.as_deref(), Some("order-42@shop.example.com"));
    // A delay returns the whole message as a `message/rfc822` part.
    let delay = classified(sample("delay"));
    assert_eq!(delay.answers.as_deref(), Some("invoice-7@shop.example.com"));
}

// covers: Core\Mail\Message::kind, Core\Mail\Message::answers
#[test]
fn a_feedback_report_is_a_complaint() {
    let complaint = classified(sample("complaint"));
    assert_eq!(complaint.kind, Kind::Complaint);
    assert_eq!(complaint.answers.as_deref(), Some("news-9@shop.example.com"));
    assert_eq!(complaint.report, None);
}

// covers: Core\Mail\Message::kind
#[test]
fn an_auto_submitted_message_is_an_auto_reply() {
    let reply = classified(sample("auto-reply"));
    assert_eq!(reply.kind, Kind::AutoReply);
    assert_eq!(reply.answers.as_deref(), Some("order-42@shop.example.com"));
    assert_eq!(
        classified(&with_header("Auto-Submitted: auto-generated")).kind,
        Kind::AutoReply
    );
    assert_eq!(classified(&with_header("Auto-Submitted: no")).kind, Kind::Other);
}

// covers: Core\Mail\Message::kind
#[test]
fn each_auto_reply_header_in_the_table_is_recognized() {
    for (name, needs) in AUTO_REPLY {
        let header = format!("{name}: {}", needs.unwrap_or("yes"));
        assert_eq!(classified(&with_header(&header)).kind, Kind::AutoReply, "{header}");
        if needs.is_some() {
            let other = format!("{name}: bulk");
            assert_eq!(classified(&with_header(&other)).kind, Kind::Other, "{other}");
        }
    }
    assert_eq!(
        classified(&with_header("X-Auto-Response-Suppress: DR, OOF, AutoReply")).kind,
        Kind::AutoReply
    );
}

// covers: Core\Mail\Message::kind, Core\Mail\Message::answers
#[test]
fn a_message_with_in_reply_to_is_a_reply_and_answers_it() {
    let reply = classified(sample("reply"));
    assert_eq!(reply.kind, Kind::Reply);
    assert_eq!(reply.answers.as_deref(), Some("order-42@shop.example.com"));
    // With no `In-Reply-To`, the last `References` ID is the answer.
    let referenced = classified(
        b"From: ada@example.org\r\nReferences: <a@example.com> <b@example.com>\r\n\r\nHi.\r\n",
    );
    assert_eq!(referenced.kind, Kind::Other);
    assert_eq!(referenced.answers.as_deref(), Some("b@example.com"));
}

// covers: Core\Mail\Message::kind, Core\Mail\Message::report, Core\Mail\Message::bounceTag
#[test]
fn a_message_with_none_of_these_is_other() {
    assert_eq!(classified(sample("other")), Classified::default());
    assert_eq!(classified(b""), Classified::default());
}

// covers: Core\Mail\Message::bounceTag
#[test]
fn the_bounce_tag_is_read_from_delivered_to() {
    assert_eq!(classified(sample("bounce")).bounce_tag.as_deref(), Some("order-42"));
    // With no `Delivered-To` or `X-Original-To`, the `To` address is read.
    assert_eq!(classified(sample("reply")).bounce_tag.as_deref(), Some("support"));
    // `X-Original-To` comes before `To`.
    let original = classified(
        b"X-Original-To: b+first@example.com\r\nTo: b+second@example.com\r\n\r\nHi.\r\n",
    );
    assert_eq!(original.bounce_tag.as_deref(), Some("first"));
    // A tag that `send` would not write is not read.
    let invalid = classified(b"Delivered-To: <b+a/b@example.com>\r\n\r\nHi.\r\n");
    assert_eq!(invalid.bounce_tag, None);
}
