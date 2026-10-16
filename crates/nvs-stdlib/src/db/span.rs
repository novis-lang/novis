//! What a statement's span is worth reporting to, asked once before the
//! statement borrows the context.
//!
//! [`QueryWatch`] is the whole of it, and its own doc owns why `rule:observability/trace-events-carry-a-kind`'s trace
//! and `rule:observability/a-query-is-a-trace-event`'s `slow_query`
//! line are one type rather than two readers.

use super::*;

/// What is reading this statement's span — `rule:observability/trace-events-carry-a-kind`
/// 's trace, `rule:observability/a-slow-query-is-logged-past-a-threshold`'s `slow_query` line, both or neither — asked
/// **before** a statement borrows the context.
///
/// A statement holds `ctx` mutably for as long as its rows do
/// ([`transacting`]), so neither can be read at the point the event is filed.
/// Reading them early also means a request that turns tracing on midway through
/// a statement does not get half an event — the span is either filed whole or
/// not at all, unlike a call site's pair, which `rule:testing/debug-probes` deliberately lets
/// straddle a change.
///
/// **The two readers are one type because they read one span.** § 11 gives the
/// threshold the same facts the trace event carries, so a statement renders its
/// span at most once however many readers there are — and a second rendering is
/// the only way the two could ever describe one statement differently.
///
/// **`pub(crate)` because § 11 is about statements and not about `Core\Db`.**
/// [`crate::queue`]'s four members drive a `PgRows` themselves rather than
/// through this class's own, and a trace that showed every statement but
/// theirs would be describing a request that never happened.
#[derive(Clone, Copy)]
pub(crate) struct QueryWatch {
    /// `rule:observability/trace-events-carry-a-kind`'s trace is recording this request.
    traced: bool,
    /// § 11's threshold, for the block that wrote one.
    slow: Option<std::time::Duration>,
}

impl QueryWatch {
    /// What the context and the connection's block say, before the statement
    /// goes out.
    pub(super) fn of(ctx: &nvs_runtime::Ctx, block: &Value) -> QueryWatch {
        QueryWatch::named(ctx, block.as_text())
    }

    /// The same, for a caller that holds the block's *name* rather than the
    /// `Value` § 2's `Connection` carries it as.
    ///
    /// [`crate::queue`] is that caller: its connection is named by `rule:core-classes/queue-storage-is-a-table`'s `[queue] connection` and reached by key, so there is no `Value` to
    /// read a name out of. `None` is § 2's unnamed `open` and means the same
    /// thing here as there — nothing to look a threshold up under.
    pub(crate) fn named(ctx: &nvs_runtime::Ctx, block: Option<&str>) -> QueryWatch {
        QueryWatch {
            traced: ctx.records_spans(),
            slow: block.and_then(|name| slow_query_of(ctx, name)),
        }
    }

    /// The span's facts, taken while the statement still lends them out.
    ///
    /// `None` when nothing is reading, and then the rendering and the clock are
    /// not paid for at all — which is every request on a deployment that has
    /// asked for neither.
    pub(crate) fn taken(self, span: &nvs_db::QuerySpan) -> Option<(String, std::time::Duration)> {
        (self.traced || self.slow.is_some()).then(|| (span.to_string(), span.duration()))
    }

    /// Files what [`QueryWatch::taken`] took, once the statement has let the
    /// context go.
    pub(crate) fn file(
        self,
        ctx: &mut nvs_runtime::Ctx,
        taken: Option<(String, std::time::Duration)>,
    ) {
        let Some((line, took)) = taken else {
            return;
        };
        if self.traced {
            ctx.record_query(&line);
        }
        if self.slow.is_some_and(|threshold| took >= threshold) {
            slow_query_record(ctx, &line);
        }
    }
}

/// `rule:observability/a-slow-query-is-logged-past-a-threshold`'s threshold for the `[db.<name>]` block a statement is running
/// on, or `None` for a statement nothing is timing.
///
/// Three cases answer `None` and they are one answer: the block wrote no
/// threshold, the connection has no block at all (§ 2's `open`, whose settings
/// the program wrote and no operator named — [`QueryWatch::named`] answers that
/// one before this is reached), and a value that would not parse —
/// which `nvs_config::db::validate` refused at boot, so it is unreachable here.
/// Off is the right answer to all three: a threshold nobody can read is not a
/// reason to fail a statement, and § 11's output is inert until asked for.
pub(super) fn slow_query_of(ctx: &nvs_runtime::Ctx, name: &str) -> Option<std::time::Duration> {
    let snapshot = ctx.config()?.snapshot();
    let written = snapshot.config.db.get(name)?;
    nvs_config::db::slow_query_for(name, written, &std::collections::BTreeMap::new())
        .ok()
        .flatten()
}

/// § 11's slow-query line: the span `rule:observability/trace-events-carry-a-kind`'s trace event carries, written to
/// `Core\Log` as one record.
///
/// **The message is the span's own rendering and not a bag of fields**, which is
/// what "the same facts" costs here: `nvs_db::QuerySpan`'s `Display` is the one
/// home of § 11's field set, and a field-shaped second spelling of it in this
/// module would be the copy that goes stale the day a driver adds one. `Warn`
/// because a threshold is written by an operator asking to be told, and it is
/// the quietest level a log pipeline is not configured to drop.
///
/// A record that cannot be written is dropped rather than retried or thrown —
/// `Core\Log::write`'s own rule, and a statement that already ran is not failed
/// by the line describing it.
pub(super) fn slow_query_record(ctx: &mut nvs_runtime::Ctx, line: &str) {
    let mut record = nvs_render::Record::at(nvs_render::Level::Warn);
    record.envelope.message = Some(nvs_render::Rendered::new(line));
    let _dropped = ctx.write_log_record(&record, nvs_runtime::LogChannel::Output);
}

/// Files `rule:observability/a-query-is-a-trace-event`'s event for a statement that never lent a `PgRows` out —
/// § 7's three commands, and the batch `executeMany` is.
///
/// The block goes on here rather than through [`name_span`], which takes the
/// rows those statements never have; the rule it applies is the same one and
/// `nvs_db::QuerySpan::name` owns it. The span arrives finished — the driver
/// froze it when its command came back, or the caller did with its own count —
/// so this is only the naming, the taking and the filing, in the order
/// [`QueryWatch`] requires.
pub(super) fn file_span(
    ctx: &mut nvs_runtime::Ctx,
    watch: QueryWatch,
    block: &Value,
    mut span: nvs_db::QuerySpan,
) {
    if let Some(name) = block.as_text() {
        span.name(name);
    }
    let taken = watch.taken(&span);
    watch.file(ctx, taken);
}

/// Puts the `[db.<name>]` block on a running statement's span.
///
/// `nvs_db::QuerySpan::name` owns why the driver cannot do this itself. The
/// block is the `Statement`'s own, so a `connect`'d connection names itself and
/// `rule:core-classes/db-connection-is-named`'s unnamed `open` — which has no block at all — leaves the field
/// empty rather than carrying a made-up name.
///
/// It takes the name and not the `Value`, so [`crate::queue`]'s statements —
/// whose block is `[queue] connection`'s name — put it on their spans through
/// this one rule rather than a second spelling of it.
pub(crate) fn name_span<R: NamesConnection>(rows: &mut R, block: Option<&str>) {
    if let Some(name) = block {
        rows.name_connection(name);
    }
}

/// A running statement's handle, on whichever driver — joined by the one method
/// [`name_span`] needs of it.
///
/// **One method and not a result-set trait**, which [`mysql_rows`] argues at
/// length for the walk above: the two handles agree on nothing else, and what is
/// shared here is the *rule* that a block names its own span, not the reading of
/// a row. Written as a trait rather than as two calls so a third driver's
/// statement cannot quietly file a nameless span.
pub(crate) trait NamesConnection {
    /// Puts `connection` on this statement's span.
    fn name_connection(&mut self, connection: &str);
}

impl NamesConnection for nvs_db::PgRows<'_> {
    fn name_connection(&mut self, connection: &str) {
        nvs_db::PgRows::name_connection(self, connection);
    }
}

impl NamesConnection for nvs_db::MySqlRows<'_> {
    fn name_connection(&mut self, connection: &str) {
        nvs_db::MySqlRows::name_connection(self, connection);
    }
}

impl NamesConnection for nvs_db::tds::TdsRows<'_> {
    fn name_connection(&mut self, connection: &str) {
        nvs_db::tds::TdsRows::name_connection(self, connection);
    }
}
