//! [ADR 0067 § 11](/docs/decisions/0067.md)'s `query` trace event:
//! the facts one statement contributes to a trace, and why a bound parameter
//! is not one of them.
//!
//! § 11 fixes the field set — duration, driver, connection name, truncated SQL
//! text, rows returned, rows affected — and then says *never parameters*. This
//! module is where that second half is made structural rather than remembered:
//! [`QuerySpan::opened`] is handed the SQL and the clock and **is never handed
//! the values**, so there is no parameter in scope for a later field, a later
//! rendering or a later driver to leak. The rule it serves is § 8's, which
//! states it for the error path in the same words, and `pg::server_error`'s own
//! tests hold that half.
//!
//! **The SQL text is safe to carry for a reason this crate can point at.**
//! § 8 allows it on the error because it is developer-authored, and the goal's
//! standing decision that emulated prepares do not exist in any form is what
//! makes that true of a *span*: no driver here ever interpolates a value into
//! the text, so what a span carries is the statement as written, placeholders
//! still placeholders. A driver that formatted its own SQL would have put the
//! values back inside this field, which is the second reason that decision is
//! structural.
//!
//! # Why it is driver-shaped and not PostgreSQL-shaped
//!
//! The span carries the driver as a field, so every backend contributes the
//! same event and a trace reads across them. That is why this is its own
//! module rather than a struct in [`crate::pg`]: a further driver adds a
//! `Driver::MySql` at its own statement routine and nothing else.
//!
//! # What it spends
//!
//! One `Instant` read and one `String` of at most [`SQL_LIMIT`] bytes per
//! statement, held for as long as the statement's handle — O(in-flight
//! statements), never O(statements run). Against a network round trip that is
//! not measurable, which is why the span is built unconditionally rather than
//! behind the capability check § 11 gates the *output* on.
//!
//! **A span reaches a trace as its own rendering, not as its fields.**
//! `Core\Db`'s reader files one under `nvs_runtime::TraceKind::Query` once the
//! rows have ended, and what crosses is [`QuerySpan`]'s `Display` — `nvs-db`
//! depends on `nvs-runtime`, so the alternative is this field set written a
//! second time in that crate, and `Ctx::record_query` is where that is argued.
//! The gate that rendering waits on is `nvs_runtime::Ctx::records_spans` — this
//! request's trace is recorded, or `DebugFlags::TRACE` is on — and gap 1 is what
//! stands between that flag and § 11's grant.
//!
//! § 11's other half reads the same span: the `slow_query` threshold a
//! `[db.<name>]` block writes turns one of these into a `Core\Log` record, and
//! `Core\Db`'s `QueryWatch` is the single reader of both halves so that a
//! statement renders its span at most once.
//!
//! **A statement that lends no reader out opens its own span**, which is why
//! [`QuerySpan::opened`] is `pub` rather than something only [`crate::pg`]
//! reaches. `executeMany` answers with a count and never hands a
//! [`crate::PgRows`] back, so `Core\Db`'s routine opens the span around the
//! driver call and finishes it with the batch's sum — the same event, built one
//! layer up because there is no handle for the driver to hang it on. § 7's
//! `BEGIN`/`COMMIT`/`ROLLBACK` lend no reader out either, and those go the
//! other way: [`crate::PgConn::begin`] *answers* with the span, because which
//! command a nesting depth gets is the connection's answer and its caller has
//! no way to spell the text.
//!
//! # Known gaps
//!
//! 1. **A span renders on a flag no grant turns on.** § 11 has this event and
//!    the `slow_query` line inert unless the capability is granted, and
//!    `nvs_config::Capability::DebugTrace` — the `debug.trace` grant
//!    `rule:testing/debug-probes` writes — is the one it means. Outside a test
//!    nothing sets `DebugFlags::TRACE` and nothing reads the grant on the way
//!    to a render: a sampled request renders its span without the grant and an
//!    unsampled one renders none with it, because sampling is the other
//!    question `nvs_runtime::Ctx::records_spans` asks and the grant reaches
//!    neither. Both halves are above this crate, and neither is the one wire a
//!    seam would close. The bit is armed from `[debug] mode`, which
//!    `rule:testing/debug-mode-directive` states as a default and a ceiling and
//!    which `crates/nvs-config/src/tree.rs` records as a key the directive
//!    registry carries no row for; the grant says where a trace may be written
//!    rather than whether one is taken, so reading it into a request's `Ctx`
//!    gates a bit nothing can turn on and leaves the sampled path exactly as it
//!    is. `Core\Debug` and the `[debug]` section are where a program first asks
//!    for the bit, and the answer to which of the two gates a render is
//!    testable only once they exist. It is recorded here because § 11 is this
//!    span's own rule, and every other probe the same bitset carries reaches
//!    the same flag the same way.
//!    — owner: M10

use std::time::{Duration, Instant};

use crate::conn::Driver;

/// How much of a statement's text a span carries.
///
/// Long enough that a hand-written statement arrives whole, short enough that
/// one generated statement — an `inList` expanded to a thousand markers, a
/// migration's whole `create table` — cannot make a single span the largest
/// thing in a trace. A cut is marked rather than silent
/// ([`QuerySpan::is_truncated`]), because a reader who cannot tell a truncated
/// statement from a short one will read the wrong query into a slow trace.
pub const SQL_LIMIT: usize = 512;

/// One statement's [ADR 0067 § 11](/docs/decisions/0067.md) trace
/// event, from the moment it went out to the moment its rows ended.
///
/// **`Debug` is derived on purpose.** The claim this type exists to keep is
/// that no bound value appears *anywhere* in it, and
/// `a_query_span_contains_no_parameter_value_anywhere` asserts that over the
/// renderings rather than over the fields — so a field added later joins the
/// `Debug` output without anyone having to remember to add it, and the test
/// covers it on the commit that adds it.
#[derive(Clone, Debug)]
pub struct QuerySpan {
    /// Which backend ran it.
    driver: Driver,
    /// The `[db.<name>]` block this connection was opened from, where there is
    /// one. See [`QuerySpan::named`] for why the driver does not fill it.
    connection: Option<String>,
    /// The statement as written, cut to [`SQL_LIMIT`].
    sql: String,
    /// Whether the cut above actually cut anything.
    truncated: bool,
    /// When the statement went out. The duration is measured from here rather
    /// than from the first row, so a span reports what the caller waited.
    opened: Instant,
    /// Frozen by [`QuerySpan::finished`]; `None` while the statement is still
    /// running, which is what makes [`QuerySpan::duration`] read the clock.
    elapsed: Option<Duration>,
    /// Rows handed back so far.
    rows: u64,
    /// The server's own affected count, `None` for a command that carries none
    /// — `pg::affected_rows` owns which those are, and the two are different
    /// facts rather than a count of zero.
    affected: Option<u64>,
}

impl QuerySpan {
    /// Opens a span for a statement that is going out now.
    ///
    /// **The parameters are deliberately not an argument.** A driver calls this
    /// from the routine that is holding them, and the whole of § 11's "never
    /// parameters" is that they are not passed here — see the module docs.
    #[must_use]
    pub fn opened(driver: Driver, sql: &str) -> QuerySpan {
        let (sql, truncated) = truncate(sql);
        QuerySpan {
            driver,
            connection: None,
            sql,
            truncated,
            opened: Instant::now(),
            elapsed: None,
            rows: 0,
            affected: None,
        }
    }

    /// Names the `[db.<name>]` block this statement ran on.
    ///
    /// Separate from [`QuerySpan::opened`] because the driver does not know it:
    /// `rule:core-classes/db-connection-is-named` puts the name on the *config block*, and a connection built
    /// from a program-supplied `Db\Settings` through `open` has no name at all.
    /// So the layer that resolved the name puts it on, and a span without one
    /// is the honest answer rather than a placeholder.
    ///
    /// **In place rather than a builder**, because the layer that knows the name
    /// meets the span through the statement's handle — `Core\Db`'s reader has a
    /// `&mut PgRows`, which owns its span and cannot hand it over — so a
    /// consuming `named(self) -> Self` would be reachable only by replacing a
    /// running statement's span with a clone of itself.
    pub fn name(&mut self, connection: &str) {
        self.connection = Some(connection.to_owned());
    }

    /// Counts one row handed back to the caller.
    ///
    /// The count is of rows the stream *produced*, so an abandoned stream
    /// reports what was read and not what the server had — which is the number
    /// a reader of the trace is asking about.
    pub fn row(&mut self) {
        self.rows += 1;
    }

    /// Ends the span: freezes the duration and files the server's affected
    /// count.
    ///
    /// Idempotent in the direction that matters — a second call leaves the
    /// first duration in place, so a stream drained and then dropped reports
    /// the moment it ended rather than the moment its handle went out of scope.
    pub fn finished(&mut self, affected: Option<u64>) {
        if self.elapsed.is_none() {
            self.elapsed = Some(self.opened.elapsed());
            self.affected = affected;
        }
    }

    /// Which backend ran the statement.
    #[must_use]
    pub fn driver(&self) -> Driver {
        self.driver
    }

    /// The `[db.<name>]` block, where the statement ran on a named connection.
    #[must_use]
    pub fn connection(&self) -> Option<&str> {
        self.connection.as_deref()
    }

    /// The statement text, cut to [`SQL_LIMIT`].
    #[must_use]
    pub fn sql(&self) -> &str {
        &self.sql
    }

    /// Whether [`QuerySpan::sql`] is the whole statement or the head of it.
    #[must_use]
    pub fn is_truncated(&self) -> bool {
        self.truncated
    }

    /// Rows handed back.
    #[must_use]
    pub fn rows(&self) -> u64 {
        self.rows
    }

    /// The server's affected count, once the statement has ended.
    #[must_use]
    pub fn affected(&self) -> Option<u64> {
        self.affected
    }

    /// How long the statement took, or how long it has been running.
    #[must_use]
    pub fn duration(&self) -> Duration {
        self.elapsed.unwrap_or_else(|| self.opened.elapsed())
    }
}

impl std::fmt::Display for QuerySpan {
    /// The trace line, with the SQL last because it is the only field whose
    /// length is not bounded by a small constant.
    ///
    /// The driver is spelled as `Driver::matrix_name` has it — the same
    /// identifier a `[db.<name>]` block's own `driver` field carries, so an
    /// operator greps a trace for the word they already wrote — and this is a
    /// machine-read event rather than the user-facing rendering that method's
    /// doc declines.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "query driver={}", self.driver.matrix_name())?;
        if let Some(connection) = &self.connection {
            write!(f, " connection={connection}")?;
        }
        write!(f, " rows={}", self.rows)?;
        if let Some(affected) = self.affected {
            write!(f, " affected={affected}")?;
        }
        write!(f, " took={:?} sql={}", self.duration(), self.sql)?;
        if self.truncated {
            f.write_str("…")?;
        }
        Ok(())
    }
}

/// The statement text a span carries, and whether anything was cut.
///
/// The cut lands on a character boundary, walking back from [`SQL_LIMIT`]: SQL
/// is UTF-8 text and a literal in it may be any of it, so slicing at a fixed
/// byte would panic on the statement that happened to have a multi-byte
/// character across the bound.
fn truncate(sql: &str) -> (String, bool) {
    if sql.len() <= SQL_LIMIT {
        return (sql.to_owned(), false);
    }
    let mut end = SQL_LIMIT;
    while !sql.is_char_boundary(end) {
        end -= 1;
    }
    (sql[..end].to_owned(), true)
}

#[cfg(test)]
mod tests {
    use super::{QuerySpan, SQL_LIMIT, truncate};
    use crate::conn::Driver;

    /// § 11's bound, on both sides of it: the last statement carried whole and
    /// the first one cut, named together so a rewriting that stops one
    /// character early cannot pass against either half alone.
    #[test]
    fn a_statement_is_carried_whole_up_to_the_bound_and_cut_after_it() {
        let whole = "s".repeat(SQL_LIMIT);
        let (kept, truncated) = truncate(&whole);
        assert_eq!(kept.len(), SQL_LIMIT);
        assert!(!truncated);

        let over = "s".repeat(SQL_LIMIT + 1);
        let (kept, truncated) = truncate(&over);
        assert_eq!(kept.len(), SQL_LIMIT);
        assert!(truncated);
    }

    /// A cut lands on a character boundary rather than inside a code point —
    /// the case a fixed-byte slice panics on, written with the multi-byte
    /// character straddling the bound.
    #[test]
    fn a_cut_inside_a_multi_byte_character_walks_back_to_its_boundary() {
        // `é` is two bytes, so a statement of `SQL_LIMIT - 1` bytes plus one of
        // them puts the bound in the middle of it.
        let mut sql = "s".repeat(SQL_LIMIT - 1);
        sql.push('é');
        let (kept, truncated) = truncate(&sql);
        assert_eq!(kept.len(), SQL_LIMIT - 1);
        assert!(truncated);
    }

    /// The duration freezes when the statement ends, not when the span is
    /// read, and a second ending does not move it.
    #[test]
    fn a_finished_span_keeps_the_duration_its_first_ending_measured() {
        let mut span = QuerySpan::opened(Driver::Postgres, "select 1");
        span.finished(Some(1));
        let ended = span.duration();
        span.finished(Some(99));
        assert_eq!(span.duration(), ended);
        assert_eq!(span.affected(), Some(1));
    }
}
