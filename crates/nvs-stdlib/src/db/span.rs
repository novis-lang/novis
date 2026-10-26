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
    /// `DebugFlags::TRACE` is on and the span is not traced, because the `debug.trace` grant is
    /// missing: [`QueryWatch::file`] says so once per process.
    ungranted: bool,
    /// The call site id the compiler passed, read by [`QueryWatch::at`] only
    /// when the span is traced (`rule:observability/a-query-span-names-its-call-site`).
    /// `None` for an untraced statement and for the zero id.
    site: Option<u32>,
}

/// Whether [`QueryWatch::file`] has already said that `trace` is on without its grant.
///
/// Once per process rather than per context, because a server builds a context per request and a
/// line per request repeats one fact about the configuration.
static TRACE_UNGRANTED_SAID: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(false);

/// What [`QueryWatch::taken`] takes off a span while the statement still lends it out.
pub(crate) struct Taken {
    /// The span's rendering, which the trace and the `slow_query` line read.
    line: String,
    /// The statement's duration.
    took: std::time::Duration,
    /// The span's fields, which the debug spool's `query` event records.
    query: nvs_runtime::spool::Query,
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
    ///
    /// A span is traced when the request is sampled, which needs no grant, or
    /// when `DebugFlags::TRACE` is on and the `debug.trace` grant is held
    /// (`rule:testing/debug-mode-directive`). The grant costs one
    /// configuration read per statement, on a run with that bit on only.
    pub(crate) fn named(ctx: &nvs_runtime::Ctx, block: Option<&str>) -> QueryWatch {
        let (traced, ungranted) = if ctx.debug_flags().contains(nvs_runtime::DebugFlags::TRACE) {
            let traced = ctx.trace_context().sampled()
                || nvs_runtime::capability::granted(
                    ctx,
                    nvs_config::capability::Cap::DebugTrace,
                    nvs_config::capability::Scope::Unscoped,
                );
            (traced, !traced)
        } else {
            (ctx.records_spans(), false)
        };
        QueryWatch {
            traced,
            slow: block.and_then(|name| slow_query_of(ctx, name)),
            ungranted,
            site: None,
        }
    }

    /// The same watch, for the call whose site id is `site`: the last
    /// argument of a `crate::registry::QUERY_SITE_MEMBERS` member.
    ///
    /// The id is read only when the span is traced, so a call nobody traces
    /// reads nothing. The zero id names no site, and reads as none.
    #[must_use]
    pub(crate) fn at(mut self, site: Value) -> QueryWatch {
        if self.traced {
            self.site = site
                .as_int()
                .and_then(|id| u32::try_from(id).ok())
                .filter(|&id| id != 0);
        }
        self
    }

    /// The span's facts, taken while the statement still lends them out.
    ///
    /// `None` when nothing is reading, and then the rendering and the clock are
    /// not paid for at all — which is every request on a deployment that has
    /// asked for neither.
    pub(crate) fn taken(self, span: &nvs_db::QuerySpan) -> Option<Taken> {
        (self.traced || self.slow.is_some()).then(|| Taken {
            line: span.to_string(),
            took: span.duration(),
            query: span.spooled(),
        })
    }

    /// Files what [`QueryWatch::taken`] took, once the statement has let the
    /// context go.
    pub(crate) fn file(self, ctx: &mut nvs_runtime::Ctx, taken: Option<Taken>) {
        if self.ungranted && !TRACE_UNGRANTED_SAID.swap(true, std::sync::atomic::Ordering::Relaxed)
        {
            trace_ungranted_record(ctx);
        }
        let Some(Taken {
            line,
            took,
            mut query,
        }) = taken
        else {
            return;
        };
        if self.traced {
            query.site = self.site.and_then(|id| ctx.query_site(id));
            ctx.record_query(&line, query, took);
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

/// The warning that `trace` is on and traces no query, because `debug.trace` is not granted
/// (`rule:testing/debug-mode-directive`). Its help is the line to add, from the same author as a
/// refusal's.
fn trace_ungranted_record(ctx: &mut nvs_runtime::Ctx) {
    let cap = nvs_config::capability::Cap::DebugTrace;
    let line = nvs_config::capability::grant_line(cap, nvs_config::capability::Scope::Unscoped);
    let message = format!(
        "The `trace` debug mode is on, but the capability `debug.trace` is not granted. No \
         database query is traced.\nhelp: add `{line}` under `[capabilities.{}]` in nvs.toml",
        cap.family()
    );
    let mut record = nvs_render::Record::at(nvs_render::Level::Warn);
    record.envelope.message = Some(nvs_render::Rendered::new(&message));
    ctx.stamp_envelope(&mut record.envelope);
    let _dropped = ctx.write_log_record(&record, nvs_runtime::LogChannel::Diagnostic);
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

#[cfg(test)]
mod tests {
    use super::QueryWatch;

    fn configured(written: &str) -> nvs_runtime::Ctx {
        let table: toml::Table = written.parse().expect("the case writes valid TOML");
        let snapshot = nvs_config::Snapshot {
            config: table
                .clone()
                .try_into()
                .expect("the case writes a block this tree has"),
            table,
            ..nvs_config::Snapshot::default()
        };
        let mut ctx = nvs_runtime::Ctx::buffered();
        ctx.set_config(std::sync::Arc::new(snapshot));
        ctx.set_debug_flags(nvs_runtime::DebugFlags::TRACE);
        ctx
    }

    #[test]
    fn a_traced_run_renders_a_query_span_only_under_the_debug_trace_grant() {
        let refused = configured("");
        assert!(!QueryWatch::named(&refused, None).traced);
        let granted = configured("[capabilities.debug]\ntrace = true\n");
        assert!(QueryWatch::named(&granted, None).traced);
        let mut off = configured("[capabilities.debug]\ntrace = true\n");
        off.set_debug_flags(nvs_runtime::DebugFlags::empty());
        assert!(
            !QueryWatch::named(&off, None).traced,
            "an unsampled run renders none"
        );
    }

    /// A traced span records the call site id its call passed, and the file and line the
    /// program's site table gives that id. An untraced call reads none
    /// (`rule:observability/a-query-span-names-its-call-site`). The zero id names no site.
    #[test]
    fn a_traced_span_records_its_call_site_and_an_untraced_call_reads_none() {
        let site = nvs_runtime::Value::int(2);
        let span = nvs_db::QuerySpan::opened(nvs_db::Driver::Sqlite, "select 1");
        let mut traced = configured("[capabilities.debug]\ntrace = true\n");
        let mut sites = nvs_runtime::QuerySites::new();
        sites.push("shop.nvs", 4, "aa");
        sites.push("shop.nvs", 9, "aa");
        traced.set_query_sites(std::sync::Arc::new(sites));
        let watch = QueryWatch::named(&traced, None).at(site);
        assert_eq!(watch.site, Some(2));
        let resolved = traced.query_site(2).expect("the table names id 2");
        assert_eq!(
            (
                resolved.file.as_str(),
                resolved.line,
                resolved.hash.as_str()
            ),
            ("shop.nvs", 9, "aa"),
            "the id is one past its row"
        );
        assert_eq!(
            traced.query_site(3),
            None,
            "an id past the table names no site"
        );
        assert!(watch.taken(&span).is_some(), "a traced span is taken");
        assert_eq!(
            QueryWatch::named(&traced, None)
                .at(nvs_runtime::Value::int(0))
                .site,
            None,
            "the zero id names no site"
        );

        let mut off = configured("[capabilities.debug]\ntrace = true\n");
        off.set_debug_flags(nvs_runtime::DebugFlags::empty());
        let untraced = QueryWatch::named(&off, None).at(site);
        assert_eq!(untraced.site, None, "an untraced call reads no site");
        assert!(untraced.taken(&span).is_none());
    }

    /// `trace` on without `debug.trace` is said, with the line to add; with the grant, or with
    /// `trace` off, there is nothing to say.
    #[test]
    fn a_traced_run_without_the_grant_says_so_naming_the_line_to_add() {
        let mut refused = configured("");
        assert!(QueryWatch::named(&refused, None).ungranted);
        assert!(
            !QueryWatch::named(&configured("[capabilities.debug]\ntrace = true\n"), None).ungranted
        );
        let mut off = configured("");
        off.set_debug_flags(nvs_runtime::DebugFlags::empty());
        assert!(!QueryWatch::named(&off, None).ungranted);

        refused.set_diagnostic_sink(nvs_runtime::OutputSink::Buffer(Vec::new()));
        super::trace_ungranted_record(&mut refused);
        let said = String::from_utf8(refused.take_buffered_diagnostic().unwrap_or_default())
            .expect("the warning is text");
        assert!(said.contains("`debug.trace` is not granted"), "{said}");
        assert!(
            said.contains("help: add `trace = true` under `[capabilities.debug]` in nvs.toml"),
            "{said}"
        );
    }
}
