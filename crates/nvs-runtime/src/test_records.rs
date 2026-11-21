//! What a test reads back of what it caused: the log records written and the mail sent by the
//! test, its requests and its runs (`rule:testing/a-test-reads-its-logs-and-mail`).
//!
//! One [`TestRecords`] is made per test and shared by its task tree and every request and run it
//! starts, which all run on the test's own thread ([`crate::Ctx::test_records`]). A request's
//! values belong to the request's isolate and may not cross into the test's
//! (`rule:security/isolate-shares-nothing`), so a record is kept as owned text: the level's name,
//! the message, and the fields as the JSON object a `[log]` line writes for them. `nvs_stdlib`
//! builds the test's own values from that text when the test reads it.
//!
//! **What it spends:** the records and messages of one test, freed with it. Nothing on a context
//! no test armed.

use std::cell::{Ref, RefCell};
use std::rc::Rc;

/// One log record, as the test reads it back.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeptLog {
    /// The level's name as a `[log]` line writes it, such as `"warn"`.
    pub level: &'static str,
    /// The message.
    pub message: String,
    /// The fields as one JSON object, or empty for a record with none.
    pub fields: String,
}

/// One message `Core\Mail::send` was given, as the test reads it back.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeptMail {
    /// The `[mail.<name>]` block named.
    pub endpoint: String,
    /// The `to` addresses, in order.
    pub to: Vec<String>,
    /// The subject.
    pub subject: String,
    /// The plain-text body.
    pub text: String,
    /// The `cc` addresses, in order.
    pub cc: Vec<String>,
    /// The `bcc` addresses, in order.
    pub bcc: Vec<String>,
    /// The `replyTo` address, if one was given.
    pub reply_to: Option<String>,
    /// The HTML body, if one was given.
    pub html: Option<String>,
}

/// The log records and mail of one test, in the order each was written.
#[derive(Debug, Default)]
pub struct TestRecords {
    logs: RefCell<Vec<KeptLog>>,
    mail: RefCell<Vec<KeptMail>>,
}

impl TestRecords {
    /// An empty store, for one test.
    #[must_use]
    pub fn new() -> Rc<Self> {
        Rc::new(Self::default())
    }

    /// Keeps one log record.
    pub fn log(&self, record: KeptLog) {
        self.logs.borrow_mut().push(record);
    }

    /// Keeps one message.
    pub fn mail(&self, message: KeptMail) {
        self.mail.borrow_mut().push(message);
    }

    /// Every record kept so far, in the order written.
    #[must_use]
    pub fn logs(&self) -> Ref<'_, Vec<KeptLog>> {
        self.logs.borrow()
    }

    /// Every message kept so far, in the order sent.
    #[must_use]
    pub fn sent(&self) -> Ref<'_, Vec<KeptMail>> {
        self.mail.borrow()
    }
}
