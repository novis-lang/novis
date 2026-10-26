//! `rule:testing/the-debug-stream-reaches-the-service-through-the-disk`'s `request` line, written
//! at the door, where every answer the server counts is complete.
//!
//! # Which gate an answer is under
//!
//! A request that ran a program is recorded under the gate its isolate read where it finished
//! ([`nvs_runtime::host::Completion::store`]), so a request whose own `Core\Config::set` turned
//! `[debug] store` off is not recorded. An answer the door wrote alone, and a run whose completion
//! never came back, is recorded under the snapshot it answered under. A streamed answer's line is
//! written where its isolate is joined, once the body has ended ([`Pending`]), so it carries the
//! status the head went out with and the gate the program ended under. The health probe is not
//! recorded, as the access log does not write it.
//!
//! **What it spends:** with `[debug] store` off, one read of the resolved configuration per answer
//! the door wrote alone, and nothing for a request that ran. With it on, one `request` line built
//! and written per answer, and per streamed answer its method, path and route held until its body
//! ends.

use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use nvs_runtime::host::{Completion, Failure};
use nvs_runtime::spool::{self, Store};

/// What the door knows about how the answer it counts ended.
#[derive(Debug)]
pub enum Ending {
    /// The door wrote the answer alone, or the program's completion never came back, so the gate
    /// is read off the snapshot the answer was written under.
    Door,
    /// The program ended under this gate, with this uncaught exception.
    Ran(Option<Store>, Option<spool::Exception>),
    /// The head went out while the program still writes the body, so the line waits for the join.
    Later,
}

impl Ending {
    /// How `done` ended: its gate and its failure.
    #[must_use]
    pub fn of(done: &Completion) -> Self {
        Self::Ran(done.store, done.error.as_ref().map(exception))
    }
}

/// The uncaught exception `failure` names, as a `request` line records it. A failure that names no
/// site is recorded with an empty file and line `0`.
#[must_use]
pub fn exception(failure: &Failure) -> spool::Exception {
    let (file, line) = failure.site().unwrap_or(("", 0));
    spool::Exception {
        class: failure.class.clone(),
        message: failure.message.clone(),
        file: file.to_owned(),
        line,
    }
}

/// What a `request` line says about one answer.
#[derive(Debug)]
pub struct Answered<'a> {
    /// The request method.
    pub method: &'a str,
    /// The path, without the query.
    pub path: &'a str,
    /// The route pattern that matched, when one did.
    pub route: Option<&'a str>,
    /// The status the answer went out with.
    pub status: u16,
    /// How long the request ran.
    pub took: Duration,
    /// The exception the request did not catch.
    pub exception: Option<spool::Exception>,
}

/// The gate an answer the door wrote alone is under: `[debug] store` as `under` resolves it.
#[must_use]
pub fn door_store(under: &Arc<nvs_config::Snapshot>) -> Option<Store> {
    Store::of(&nvs_config::Request::new(Arc::clone(under)))
}

/// Writes the `request` line for `answered` in `trace`, under `store`. Writes nothing where the
/// store is off.
pub fn request(store: Option<Store>, trace: &nvs_runtime::TraceContext, answered: Answered<'_>) {
    let Some(store) = store else {
        return;
    };
    let duration_us = micros(answered.took);
    let now_us = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, micros);
    let line = spool::Request {
        start_us: now_us.saturating_sub(duration_us),
        duration_us,
        method: answered.method.to_owned(),
        route: answered.route.map(str::to_owned),
        path: answered.path.to_owned(),
        status: answered.status,
        exception: answered.exception,
        input: None,
        response: None,
    };
    spool::record_under(store, trace, spool::Record::Request(Box::new(line)));
}

/// A streamed answer's `request` line, held from the head until the body ends. Made only where the
/// snapshot has `[debug] store` on, because a request can turn the key off and never on.
#[derive(Debug)]
pub struct Pending {
    method: Box<str>,
    path: Box<str>,
    route: Option<Box<str>>,
    status: u16,
    began: Instant,
    trace: nvs_runtime::TraceContext,
    under: Store,
}

impl Pending {
    /// The line for an answer whose head went out with `status`, or `None` where `under` has the
    /// store off.
    #[must_use]
    pub fn of(
        answered: &Answered<'_>,
        began: Instant,
        trace: nvs_runtime::TraceContext,
        under: Option<Store>,
    ) -> Option<Self> {
        Some(Self {
            method: answered.method.into(),
            path: answered.path.into(),
            route: answered.route.map(Box::from),
            status: answered.status,
            began,
            trace,
            under: under?,
        })
    }

    /// Writes the line once the body has ended, under the gate `done` ended with, or the
    /// snapshot's where the completion never came back. The duration runs to the body's end,
    /// which is where the program ended.
    pub fn written(self, done: Option<&Completion>) {
        let (store, exception) = match done.map(Ending::of) {
            Some(Ending::Ran(store, exception)) => (store, exception),
            _ => (Some(self.under), None),
        };
        request(
            store,
            &self.trace,
            Answered {
                method: &self.method,
                path: &self.path,
                route: self.route.as_deref(),
                status: self.status,
                took: self.began.elapsed(),
                exception,
            },
        );
    }
}

fn micros(span: Duration) -> u64 {
    u64::try_from(span.as_micros()).unwrap_or(u64::MAX)
}
