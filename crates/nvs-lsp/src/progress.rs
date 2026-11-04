//! The progress a client shows while the server builds its index.
//!
//! A start at the `Workspace` scope surveys every `.nvs` file under the root
//! and loads or plans the index before the loop answers anything, and the loop
//! then settles the waiting entries one at a time. On a large tree that is
//! seconds in which the editor would show nothing, so a client that declared
//! `window.workDoneProgress` is sent one run of work-done progress for it
//! (`rule:ide/the-index-is-incremental-and-persistent` § *A start shows its
//! progress*). A client that did not declare it is sent nothing, because a
//! `$/progress` for a token it never created is a protocol error to it.
//!
//! A run is a `window/workDoneProgress/create` request, then `$/progress`
//! `begin` before the survey, a `report` each time the share of settled entries
//! reaches a higher whole percent, and `end` once the queue is empty. That is at
//! most a hundred reports, however many files the tree holds. The `begin` is
//! sent right behind the `create` without waiting for its response, as
//! rust-analyzer does: the client handles the two in the order they were sent,
//! and the loop drops the response whenever it arrives.
//!
//! [`Indexing::begin`] ends a run that is still going before it starts the next,
//! and each run has a token of its own, so a client never sees two `begin`s
//! without an `end` between them. [`Indexing::track`] is called at the top of
//! every turn of the loop, so the queue emptying anywhere — the idle turn's
//! `settle_one`, a whole-tree request's `settle`, or a rebuild that left nothing
//! to settle — is followed by the `end` at the next turn.
//!
//! A run's `end` is followed by one `window/logMessage` at the `Log` level: the
//! index's [`Work`](crate::Work) since the session started, and the file-system
//! work the server's thread did, the survey of the tree included. `bun nv
//! lsp-start` reads it, and a client shows it in its log.
//!
//! What it spends is one counter and, during a run, one token and two numbers,
//! for the whole session. The check at each turn is one length read.

use lsp_server::{Connection, Notification, Request, RequestId};
use lsp_types::notification::{LogMessage, Notification as _, Progress};
use lsp_types::request::{Request as _, WorkDoneProgressCreate};
use lsp_types::{
    LogMessageParams, MessageType, NumberOrString, ProgressParams, ProgressParamsValue,
    WorkDoneProgress, WorkDoneProgressBegin,
    WorkDoneProgressCreateParams, WorkDoneProgressEnd, WorkDoneProgressReport,
};

use crate::server::ServerError;

/// The title the client shows for the whole run.
const TITLE: &str = "Indexing";

/// The message shown while the files under the root are read, before the
/// number of files to index is known.
const READING: &str = "Reading the workspace";

/// The start of every run's token, and of the id of the request that creates
/// it. A run's number follows it, so no token is used twice in a session.
const TOKEN: &str = "nvs/indexing";

/// The progress of the index build, as one client is shown it.
#[derive(Debug, Default)]
pub(crate) struct Indexing {
    /// Whether the client declared `window.workDoneProgress`. Nothing is sent
    /// when it did not.
    shown: bool,
    /// How many runs have begun, which numbers the next run's token.
    runs: u32,
    /// The run the client is showing now, if one is.
    run: Option<Run>,
}

/// One run between its `begin` and its `end`.
#[derive(Debug)]
struct Run {
    token: NumberOrString,
    /// The most entries the queue has held during the run.
    total: usize,
    /// The percentage the last `report` sent, and `None` before the first.
    percent: Option<u32>,
}

impl Indexing {
    /// Progress for a client that shows it when `shown` is true.
    pub(crate) fn new(shown: bool) -> Self {
        Self {
            shown,
            ..Self::default()
        }
    }

    /// Asks the client to create a fresh token, and sends its `begin`. A run
    /// still going is ended first.
    ///
    /// # Errors
    ///
    /// When the connection is gone.
    pub(crate) fn begin(&mut self, connection: &Connection) -> Result<(), ServerError> {
        if !self.shown {
            return Ok(());
        }
        self.end(connection, None)?;
        self.runs += 1;
        let name = format!("{TOKEN}/{}", self.runs);
        let token = NumberOrString::String(name.clone());
        connection.sender.send(
            Request::new(
                RequestId::from(name),
                WorkDoneProgressCreate::METHOD.to_owned(),
                WorkDoneProgressCreateParams {
                    token: token.clone(),
                },
            )
            .into(),
        )?;
        send(
            connection,
            &token,
            WorkDoneProgress::Begin(WorkDoneProgressBegin {
                title: TITLE.to_owned(),
                cancellable: None,
                message: Some(READING.to_owned()),
                // A percentage here is what lets the reports carry one.
                percentage: Some(0),
            }),
        )?;
        self.run = Some(Run {
            token,
            total: 0,
            percent: None,
        });
        Ok(())
    }

    /// Reports the run's progress with `pending` entries waiting, and ends it
    /// when none is. Nothing is sent outside a run, or when the percentage is
    /// not higher than the last one sent.
    ///
    /// # Errors
    ///
    /// When the connection is gone.
    pub(crate) fn track(
        &mut self,
        connection: &Connection,
        index: &crate::SymbolIndex,
    ) -> Result<(), ServerError> {
        let pending = index.pending().len();
        if pending == 0 {
            return self.end(connection, Some(index.work()));
        }
        let Some(run) = self.run.as_mut() else {
            return Ok(());
        };
        // An edit during the run may queue more entries than the run began
        // with, and the total grows to cover them.
        run.total = run.total.max(pending);
        let done = run.total - pending;
        let percent = u32::try_from(done * 100 / run.total).unwrap_or(100);
        if run.percent.is_some_and(|last| percent <= last) {
            return Ok(());
        }
        run.percent = Some(percent);
        let message = format!("{done} of {} files", run.total);
        let token = run.token.clone();
        send(
            connection,
            &token,
            WorkDoneProgress::Report(WorkDoneProgressReport {
                cancellable: None,
                message: Some(message),
                percentage: Some(percent),
            }),
        )
    }

    /// Sends the run's `end`, when a run is going, and then the work the
    /// index and the server's thread did. A run a new one cuts short has no
    /// `work`, because its index is being replaced, and logs nothing.
    fn end(
        &mut self,
        connection: &Connection,
        work: Option<crate::Work>,
    ) -> Result<(), ServerError> {
        let Some(run) = self.run.take() else {
            return Ok(());
        };
        send(
            connection,
            &run.token,
            WorkDoneProgress::End(WorkDoneProgressEnd { message: None }),
        )?;
        let Some(work) = work else {
            return Ok(());
        };
        let message = format!(
            "index work: parsed={} checked={} {}; server work: {}",
            work.parsed,
            work.checked,
            work.files,
            nvs_diagnostics::work::so_far()
        );
        connection.sender.send(
            Notification::new(
                LogMessage::METHOD.to_owned(),
                LogMessageParams {
                    typ: MessageType::LOG,
                    message,
                },
            )
            .into(),
        )?;
        Ok(())
    }
}

/// One `$/progress` notification for `token`.
fn send(
    connection: &Connection,
    token: &NumberOrString,
    value: WorkDoneProgress,
) -> Result<(), ServerError> {
    connection.sender.send(
        Notification::new(
            Progress::METHOD.to_owned(),
            ProgressParams {
                token: token.clone(),
                value: ProgressParamsValue::WorkDone(value),
            },
        )
        .into(),
    )?;
    Ok(())
}
