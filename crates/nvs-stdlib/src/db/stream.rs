//! ADR 0067 § 4's one member that does not buffer: the walk over a portal the
//! connection is still holding open.
//!
//! # Decision: a stream is its own class, and never [`crate::cursor`]
//!
//! Every other `Core` collection answers `iterate()` with a cursor over a
//! *snapshot* — [`crate::cursor`]'s own module doc argues why, and
//! [`nvs_core_db_rows_iterate`] is `Core\Db` taking that answer for a buffered
//! result set. A streamed row cannot be in a snapshot, because it does not
//! exist until [`nvs_db::PgConn::stream_next_row`] asks the server for it. So
//! this class carries the `advance()`/`current()` pair itself and *is* its own
//! iterator, exactly as `Core\Request\BodyStream` and `Core\Request\Files` are
//! and for the same reason: naming the walk is not reading it.
//!
//! **What it holds is the connection's key and one row.** The rows live on the
//! connection — [`nvs_db::PgCursor`] is the read state § 4 parks there — so
//! this object owns nothing a request teardown does not already own, and an
//! escaped `$stream` keeps no portal alive past the request that opened it. The
//! one slot that holds a value is [`STREAM_ROW_SLOT`], overwritten by every
//! `advance()` and cleared at the end of the walk, which is what makes the
//! member's promise measurable: one row is held at a time whatever the result
//! set's size.
//!
//! **What it spends:** one object per `foreach`, plus one row's columns for as
//! long as the loop body holds them. That is O(1) in the rows the statement
//! answered, which is the whole difference from `query` and the reason
//! [AGENTS.md](/AGENTS.md)'s ordering admits the member at all — it buys
//! priority 5 back at no cost to any of the four above it.
//!
//! # The connection is busy for the walk's whole length
//!
//! § 4 states it as the member's price: a statement attempted while a stream is
//! open throws `LogicError`. Nothing here enforces that — `nvs_db`'s own
//! `State::Streaming` does, and [`statement_failure`] is what turns the
//! driver's `InvalidInput` into the class § 4 names. The rule is therefore the
//! same one whichever member the second statement is, and there is no second
//! place for it to be stated.
//!
//! A walk read to its end returns the connection to `State::Idle` on the
//! `advance()` that answers `false`, and that same call is where ADR 0067
//! § 11's event is filed: a stream's span is the whole statement's, so it is
//! worth reporting once the statement is over rather than per row. A walk the
//! program abandons half way holds the connection until the request ends,
//! which is § 4's documented price and not a leak — the reset § 13 runs before
//! the connection is poolable drops the parked cursor.

use super::*;

/// `Core\Db\Connection::stream`, as its own refusals spell it — see
/// [`crate::db::open`]'s [`QUERY`] for why both spellings of a member's name
/// travel together, and [`QUERY_AS`] for why a call through a
/// `Core\Db\Transaction` names the connection's member here too.
pub(super) const STREAM_MEMBER: &str = r"Core\Db\Connection::stream";

/// Spec § 18's `Iterable<Db\Row>`, as a class a return type can name.
///
/// A named class because [`CoreTy::Iterated`] is parameter position only, which
/// is `Core\IO\Lines`' reason for existing as well; [`crate::registry`]'s
/// `ITERABLES` is where the element type is declared, and
/// [`crate::instance`]'s dispatch roster is what gives it the three names a
/// `foreach` reaches.
pub(crate) const STREAM: CoreClass = CoreClass {
    name: STREAM_NAME,
    methods: &[],
    instance: &[],
    slots: &[HANDLE_SLOT, CONNECTION_NAME_SLOT, STREAM_ROW_SLOT],
    constants: &[],
};

/// The row the last `advance()` read, or `null` before the first one and after
/// the last — a [`Fault::fatal`] for a receiver that is not a stream.
fn stream_row(value: Value, member: &str) -> Result<Value, Fault> {
    let receiver = crate::instance::receiver(value, &STREAM, member)?;
    let held = crate::instance::slot(receiver, STREAM_ROW_AT);
    #[expect(
        unsafe_code,
        reason = "the slot keeps its reference until the next `advance`, so the \
                  value handed back needs one of its own"
    )]
    unsafe {
        held.retain();
    }
    Ok(held)
}

/// One step of the walk: the next row parked into [`STREAM_ROW_AT`], and
/// whether there was one.
///
/// The span is filed on the step that ends the walk and on no other, because a
/// stream's § 11 event describes the statement rather than a row — and by then
/// the driver has let the context go, which is [`QueryWatch::file`]'s own
/// requirement.
///
/// # Errors
///
/// [`filed_connection`]'s `LogicError` for a connection spec § 18's `close` has
/// released, [`statement_failure`]'s for anything the server refused mid-walk
/// or a wire that failed under it, and [`column_value`]'s for a column with no
/// Novis representation. A [`Fault::fatal`] for a receiver whose slots hold the
/// wrong shape, or for a connection that is not the driver [`STREAM`] was built
/// over — both this crate's paste errors.
fn stream_step(ctx: &mut nvs_runtime::Ctx, value: Value) -> Result<Value, Fault> {
    let member = nvs_runtime::sequence::ADVANCE;
    let receiver = crate::instance::receiver(value, &STREAM, member)?;
    let key = crate::instance::slot(receiver, HANDLE_AT)
        .as_uint()
        .ok_or_else(|| {
            Fault::fatal(format!(
                "{STREAM_NAME}::{member} expected {:?} in its `{}` slot",
                Tag::Uint,
                STREAM.slots[HANDLE_AT]
            ))
        })?;
    let block = crate::instance::slot(receiver, BLOCK_AT);
    // Read before the connection is borrowed, for [`QueryWatch`]'s reason: the
    // arm below holds the thing the span is filed on.
    let watch = QueryWatch::of(ctx, &block);
    let (row, taken) = match filed_connection(ctx, key, STREAM_MEMBER)? {
        nvs_db::Connection::Postgres(postgres) => {
            let read = postgres
                .stream_next_row()
                .map_err(|refused| statement_failure(STREAM_MEMBER, &block, None, &refused))?;
            match read {
                Some(read) => {
                    // Read after the row and not before it: the row is owned, so
                    // the description can be borrowed rather than copied per
                    // row — which is what keeps this walk's per-row cost the
                    // decode and nothing else.
                    let zone = postgres.time_zone();
                    let columns = postgres.stream_columns().unwrap_or(&[]);
                    // Built whole before it is parked, on
                    // [`postgres_rows`]' reasoning: a column this driver cannot
                    // read back releases the half-built row rather than leaving
                    // it in the slot.
                    let mut one = NvsArray::new();
                    for (index, column) in columns.iter().enumerate() {
                        let body = read.column(index).map_err(|refused| {
                            statement_failure(STREAM_MEMBER, &block, None, &refused)
                        })?;
                        let scalar = column.scalar(body).map_err(|refused| {
                            statement_failure(STREAM_MEMBER, &block, None, &refused)
                        })?;
                        let value = column_value(scalar, zone, STREAM_MEMBER, &column.name)?;
                        one.set(NvsStr::new(column.name.as_bytes()), value);
                    }
                    (Some(one), None)
                }
                None => {
                    let taken = postgres.stream_span().and_then(|span| watch.taken(span));
                    // The portal is already drained — `stream_next_row` read the
                    // `ReadyForQuery` that ended it — so this drops the parked
                    // state rather than draining anything, and the connection is
                    // idle and poolable from here.
                    postgres.end_stream();
                    (None, taken)
                }
            }
        }
        // Unreachable: [`nvs_core_db_connection_stream`] refuses every other
        // driver before it builds one of these, and a connection cannot change
        // driver under a walk.
        other => {
            return Err(Fault::fatal(format!(
                "{STREAM_NAME}::{member} found a `{}` connection under a stream",
                other.driver().matrix_name()
            )));
        }
    };
    watch.file(ctx, taken);
    let Some(row) = row else {
        // Cleared rather than left holding the last row, on
        // `Core\Request\BodyStream`'s reasoning: the loop is over, so keeping it
        // would hold one row's columns for as long as the program held the walk.
        crate::instance::set_slot(receiver, STREAM_ROW_AT, Value::null());
        return Ok(Value::bool(false));
    };
    crate::instance::set_slot(
        receiver,
        STREAM_ROW_AT,
        crate::instance::build(&ROW, [Value::array(row)]),
    );
    Ok(Value::bool(true))
}

nvs_runtime::nvs_helper! {
    /// `$c->stream(string $sql, array<mixed> $params): Iterable<Db\Row>` — ADR
    /// 0067 § 4's constant-memory read, and the one member that holds the
    /// connection until the walk is drained.
    ///
    /// The statement is bound exactly as [`nvs_core_db_connection_query`] binds
    /// one — [`statement_of`] is the whole of §§ 5 and 18's rules and neither
    /// member states any of them itself — and the only difference is what is
    /// done with the portal afterwards: `query` drains it before it answers and
    /// this one leaves it open, parked on the connection where
    /// [`nvs_db::PgCursor`] can outlive the borrow that opened it.
    ///
    /// **The answer is empty of rows on purpose.** Nothing has been read when
    /// this returns — the first row arrives at the first `advance()` — so a
    /// program that names a stream and never walks it has still made the
    /// connection busy, which is § 4's price stated where the price is paid.
    ///
    /// # Errors
    ///
    /// [`statement_of`]'s refusals, [`filed_connection`]'s `LogicError` for a
    /// closed connection, a thrown `RuntimeError` for a driver with no parked
    /// cursor yet — [`crate::db`]'s known gap 5 — and [`statement_failure`] for
    /// anything the server refused, which for a connection that is already
    /// streaming is § 4's `LogicError`.
    fn nvs_core_db_connection_stream(ctx, args: [3]) {
        let statement = statement_of(ctx, args, "stream", STREAM_MEMBER)?;
        // The caller's own text and not [`Statement::sql`], for
        // [`queried_rows`]' reason: a refusal names what the program wrote.
        let source = args[1].as_text();
        let sending: Vec<Option<&[u8]>> = statement.binds.wire();
        match filed_connection(ctx, statement.key, STREAM_MEMBER)? {
            nvs_db::Connection::Postgres(postgres) => {
                // The description is answered here and read again per row off
                // the connection, so nothing about it is copied into this
                // object: it belongs to the statement rather than to the walk.
                postgres
                    .stream(&statement.sql, &sending)
                    .map_err(|refused| {
                        statement_failure(STREAM_MEMBER, &statement.block, source, &refused)
                    })?;
                if let Some(name) = statement.block.as_text() {
                    postgres.name_stream_connection(name);
                }
            }
            other => {
                return Err(unstreamed(other.driver()));
            }
        }
        Ok(crate::instance::build(
            &STREAM,
            [
                Value::uint(statement.key),
                statement.block,
                Value::null(),
            ],
        ))
    }
}

/// The refusal for a driver whose wire half has no parked cursor yet.
///
/// A `RuntimeError` and not a `LogicError`: the call is well formed and the
/// same call is answered on PostgreSQL, so it is this runtime that is short and
/// not the program. It names `query` because that is the member every driver
/// answers with the same rows, which is the whole of what a caller can do about
/// it today.
fn unstreamed(driver: nvs_db::Driver) -> Fault {
    Fault::thrown(format!(
        "{STREAM_MEMBER}: this connection's driver has no streaming read yet — only PostgreSQL \
         parks a cursor, so read this statement with `query` here, or open the block on a \
         PostgreSQL server (`{}` is what it names today)",
        driver.matrix_name()
    ))
}

nvs_runtime::nvs_helper! {
    /// `Iterable<Db\Row>::iterate(): Iterator<Db\Row>` — the walk itself,
    /// because the next row does not exist when the walk is named.
    ///
    /// [`nvs_core_request_files_iterate`](crate::request)'s shape and for its
    /// reason: the receiver's transferred reference is handed straight back out,
    /// so nothing is allocated and the cursor *is* the open portal.
    fn nvs_core_db_stream_iterate(_ctx, args: [1]) {
        crate::instance::receiver(args[0], &STREAM, nvs_runtime::sequence::ITERATE)?;
        Ok(args[0])
    }
}

nvs_runtime::nvs_helper! {
    /// `Iterator<Db\Row>::advance(): bool` — reads the next row off the parked
    /// portal, answering `false` at the end of the result set and freeing the
    /// connection there.
    ///
    /// This is the only place a `foreach` over a statement suspends, and it
    /// parks the isolate rather than a thread: every read underneath is
    /// `nvs_db`'s over `nvs_host`'s parking stream.
    fn nvs_core_db_stream_advance(ctx, args: [1]) {
        let stepped = stream_step(ctx, args[0]);
        crate::cursor::consume(args[0]);
        stepped
    }
}

nvs_runtime::nvs_helper! {
    /// `Iterator<Db\Row>::current(): Core\Db\Row` — the row the last
    /// `advance()` read, and the only one this walk is holding.
    fn nvs_core_db_stream_current(_ctx, args: [1]) {
        let read = stream_row(args[0], nvs_runtime::sequence::CURRENT);
        crate::cursor::consume(args[0]);
        read
    }
}
