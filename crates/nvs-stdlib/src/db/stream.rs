//! `rule:core-classes/db-statement-members`'s one member that does not buffer: the walk over a portal the
//! connection is still holding open.
//!
//! # Decision: a stream is its own class, and never [`crate::cursor`]
//!
//! Every other `Core` collection answers `iterate()` with a cursor over a
//! *snapshot* — [`crate::cursor`]'s own module doc argues why, and
//! [`nvs_core_db_rows_iterate`] is `Core\Db` taking that answer for a buffered
//! result set. A streamed row cannot be in a snapshot, because it does not
//! exist until the driver's own `stream_next_row` asks the server for it. So
//! this class carries the `advance()`/`current()` pair itself and *is* its own
//! iterator, exactly as `Core\Request\BodyStream` and `Core\Request\Files` are
//! and for the same reason: naming the walk is not reading it.
//!
//! **What it holds is the connection's key and one row.** The rows live on the
//! connection — `nvs_db`'s `PgCursor`, `MySqlCursor`, `TdsCursor` and
//! `SqliteCursor` are `rule:core-classes/a-stream-parks-its-read-on-the-connection`'s read state, one per
//! driver and every driver — so
//! this object owns nothing a request teardown does not already own, and an
//! escaped `$stream` keeps no portal alive past the request that opened it. The
//! one slot that holds a value is [`STREAM_ROW_SLOT`], overwritten by every
//! `advance()` and cleared at the end of the walk, which is what makes the
//! member's promise measurable: one row is held at a time whatever the result
//! set's size. [`park_row`] is where that is kept, and
//! `stream_answers_rows_without_holding_the_result_set` counts it over a
//! thousand rows.
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
//! `advance()` that answers `false`, and that same call is where `rule:observability/a-query-is-a-trace-event`
//! 's event is filed: a stream's span is the whole statement's, so it is
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

/// `Core\Db\Connection::streamAs`, as its own refusals spell it —
/// [`STREAM_MEMBER`]'s reason, and a second name because the two members are
/// told apart by what a row becomes rather than by the walk under it. A refusal
/// raised *between* two rows names that walk and so spells [`STREAM_MEMBER`]
/// whichever member opened it.
pub(super) const STREAM_AS_MEMBER: &str = r"Core\Db\Connection::streamAs";

/// Spec § 18's `Iterable<Db\Row>` and `Iterable<T>`, as a class a return type
/// can name.
///
/// A named class because [`CoreTy::Iterated`] is parameter position only, which
/// is `Core\IO\Lines`' reason for existing as well; [`crate::registry`]'s
/// `ITERABLES` is where the element is declared — this class's own `T`, since
/// `stream` and `streamAs` are one walk at two arguments — and
/// [`crate::instance`]'s dispatch roster is what gives it the three names a
/// `foreach` reaches.
pub(crate) const STREAM: CoreClass = CoreClass {
    name: STREAM_NAME,
    doc: None,
    methods: &[],
    instance: &[],
    slots: &[
        HANDLE_SLOT,
        CONNECTION_NAME_SLOT,
        STREAM_ROW_SLOT,
        STREAM_CLASS_SLOT,
    ],
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

/// Parks `row` as the one row this walk is holding — a [`ROW`], or an instance
/// of the class [`STREAM_CLASS_AT`] names — and answers what `advance()`
/// answers: `true` for a row, `false` for the end of the result set.
///
/// [`stream_step`]'s tail, split out because this is where § 4's promise is
/// *kept* rather than merely stated — a member that held its result set would
/// differ from this one in exactly this call — and because everything above it
/// in that function needs a server.
/// [`nvs_runtime::nvs_object_field_set`] releases the value it displaces, so
/// the row the previous `advance()` parked is freed right here unless the loop
/// body is still holding it, and the walk's footprint is one row whatever the
/// statement answered.
///
/// The `None` arm clears the slot rather than leaving the last row in it, on
/// `Core\Request\BodyStream`'s reasoning: the loop is over, so keeping it would
/// hold one row's columns for as long as the program held the walk.
///
/// **A `streamAs<T>` builds its `T` here and parks that**, where
/// [`nvs_core_db_connection_query_as`] builds one per row handed out: a walk
/// hands out every row it reads, so there is nothing a later construction could
/// save, and parking the class rather than the columns is what keeps the
/// member's footprint one object instead of one object and the array it came
/// from.
///
/// # Errors
///
/// [`hydrate`]'s, for a row that does not match that class.
fn park_row(
    ctx: &mut nvs_runtime::Ctx,
    receiver: *mut nvs_runtime::ObjHeader,
    row: Option<NvsArray>,
    class: Option<*const nvs_runtime::ClassDesc>,
) -> Result<Value, Fault> {
    let Some(row) = row else {
        crate::instance::set_slot(receiver, STREAM_ROW_AT, Value::null());
        return Ok(Value::bool(false));
    };
    let built = match class {
        None => crate::instance::build(&ROW, [Value::array(row)]),
        Some(class) => {
            #[expect(
                unsafe_code,
                reason = "the descriptor came out of a `ClassDescConst` the compiled \
                          unit owns, written into this receiver's own slot by \
                          `nvs_core_db_connection_stream_as`, so it outlives this call"
            )]
            // The columns are read and never kept — [`hydrate`] takes its own
            // reference to each value it holds on to — so the array this step
            // built is released when it goes out of scope here, whichever way
            // the construction went.
            unsafe {
                hydrate(ctx, class, &row)?
            }
        }
    };
    crate::instance::set_slot(receiver, STREAM_ROW_AT, built);
    Ok(Value::bool(true))
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
/// or a wire that failed under it, [`column_value`]'s for a column with no
/// Novis representation, and — on a `streamAs<T>` walk — [`park_row`]'s
/// `ParseError` for a row that does not match `T`. A [`Fault::fatal`] for a
/// receiver whose slots hold the wrong shape, or for a connection that is not
/// the driver [`STREAM`] was built over — both this crate's paste errors.
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
    // The class a `streamAs<T>` wrote, or `None` for the walk that answers
    // `Core\Db\Row`s — read here rather than at the park below because the
    // borrow the driver arms take is the receiver's connection and this is the
    // receiver's own slot.
    let class = crate::instance::slot(receiver, STREAM_CLASS_AT).as_class_desc();
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
        nvs_db::Connection::MySql(mysql) => mysql_step(Framed::MySql(mysql), &block, &watch)?,
        nvs_db::Connection::MariaDb(maria) => mysql_step(Framed::MariaDb(maria), &block, &watch)?,
        nvs_db::Connection::SqlServer(tds) => tds_step(tds, &block, &watch)?,
        nvs_db::Connection::Sqlite(sqlite) => sqlite_step(sqlite, &block, &watch)?,
    };
    watch.file(ctx, taken);
    park_row(ctx, receiver, row, class)
}

/// What one step of a walk produced: the row where there was one, and
/// `rule:observability/a-query-is-a-trace-event`'s event where this step is the one that ended the
/// statement.
///
/// A name because the pair is what every arm of [`stream_step`] answers with,
/// where the buffered members spell their own twin of it out at each signature.
type Stepped = (Option<NvsArray>, Option<(String, std::time::Duration)>);

/// One step of a walk over either of the two drivers [`Framed`] covers, for
/// [`stream_step`]'s arms.
///
/// A function over that enum rather than two arms that read alike, and for its
/// own reason: `nvs_db::MariaConn::stream_next_row` is a delegation into the
/// same parked read `nvs_db::MySqlConn::stream_next_row` is, so the only thing
/// the two arms could differ in is the type of the borrow.
///
/// **The per-row work is [`mysql_rows`]' and not [`stream_step`]'s**, down to
/// the lossy label in the message beside the octets in the key: what a row *is*
/// on this driver is one decision, and a walk that read a column differently
/// from the buffered member would be two.
///
/// # Errors
///
/// [`statement_failure`]'s for anything the server refused mid-walk or a wire
/// that failed under it, and [`mysql_column_value`]'s for a column with no Novis
/// representation. A [`Fault::fatal`] for a row narrower than the definitions it
/// was decoded against, which is a `nvs-db` bug rather than a program's.
fn mysql_step(mut framed: Framed<'_>, block: &Value, watch: &QueryWatch) -> Result<Stepped, Fault> {
    let read = framed
        .stream_next_row()
        .map_err(|refused| statement_failure(STREAM_MEMBER, block, None, &refused))?;
    let Some(read) = read else {
        let taken = framed.stream_span().and_then(|span| watch.taken(span));
        // The result set is already drained — this step is the one that read the
        // terminator — so this drops the parked state rather than draining
        // anything, and the connection is idle and poolable from here.
        framed.end_stream();
        return Ok((None, taken));
    };

    // Read after the row and not before it, for the PostgreSQL arm's reason: the
    // row is owned, so the definitions can be borrowed rather than copied per
    // row.
    let zone = framed.time_zone();
    // Matched here rather than reached through a [`Framed`] member, because a
    // definition is `mysql_common`'s `Column` and this crate does not depend on
    // that crate — [`Framed::stream`] owns that sentence.
    let columns = match &framed {
        Framed::MySql(mysql) => mysql.stream_columns(),
        Framed::MariaDb(maria) => maria.stream_columns(),
    }
    .unwrap_or_default();

    // Built whole before it is parked, on [`park_row`]'s reasoning: a column this
    // driver cannot read back releases the half-built row rather than leaving it
    // in the slot.
    let mut one = NvsArray::new();
    for (index, column) in columns.iter().enumerate() {
        // Unreachable, as in [`mysql_rows`]: `nvs-db` decodes one value per
        // definition, so a row is exactly as wide as this loop.
        let body = read.value(index).ok_or_else(|| {
            Fault::fatal(format!(
                "{STREAM_MEMBER}: the row has no column {index}, where the result set described {}",
                columns.len()
            ))
        })?;
        let scalar = nvs_db::mysql::scalar(column, body)
            .map_err(|refused| statement_failure(STREAM_MEMBER, block, None, &refused))?;
        let label = String::from_utf8_lossy(column.name_ref());
        let value = mysql_column_value(scalar, zone, STREAM_MEMBER, &label)?;
        one.set(NvsStr::new(column.name_ref()), value);
    }
    Ok((Some(one), None))
}

/// One step of a walk over SQL Server, for [`stream_step`]'s arm.
///
/// [`mysql_step`]'s shape, over `nvs_db::TdsCursor`'s parked read: the row comes
/// back owned, the description is borrowed off the connection after it, and the
/// per-row work is [`tds_rows`]' rather than this function's.
///
/// # Errors
///
/// [`statement_failure`]'s for anything the server refused mid-walk or a wire
/// that failed under it, and [`tds_column_value`]'s for a column with no Novis
/// representation. A [`Fault::fatal`] for a row narrower than the description it
/// was decoded against, which is a `nvs-db` bug rather than a program's.
fn tds_step(
    tds: &mut nvs_db::TdsConn,
    block: &Value,
    watch: &QueryWatch,
) -> Result<Stepped, Fault> {
    let read = tds
        .stream_next_row()
        .map_err(|refused| statement_failure(STREAM_MEMBER, block, None, &refused))?;
    let Some(read) = read else {
        let taken = tds.stream_span().and_then(|span| watch.taken(span));
        // The answer is already read to its `DONE` — that is what ended the
        // walk — so this drops the parked state rather than draining anything.
        tds.end_stream();
        return Ok((None, taken));
    };

    let zone = tds.time_zone();
    let columns = tds.stream_columns().unwrap_or(&[]);
    // Built whole before it is parked, on [`park_row`]'s reasoning.
    let mut one = NvsArray::new();
    for (index, column) in columns.iter().enumerate() {
        // Unreachable and fatal for [`tds_rows`]' reason: `nvs-db` reads one
        // value per described column, so a row is exactly as wide as this loop.
        let body = read.column(index).ok_or_else(|| {
            Fault::fatal(format!(
                "{STREAM_MEMBER}: the row has no column {index}, where the result set described {}",
                columns.len()
            ))
        })?;
        let scalar = nvs_db::tds::scalar(column, body)
            .map_err(|refused| statement_failure(STREAM_MEMBER, block, None, &refused))?;
        let value = tds_column_value(scalar, zone, STREAM_MEMBER, &column.name)?;
        one.set(NvsStr::new(column.name.as_bytes()), value);
    }
    Ok((Some(one), None))
}

/// One step of a walk over SQLite, for [`stream_step`]'s arm.
///
/// The parked read here is a pool thread rather than a message boundary
/// (`nvs_db::SqliteCursor`), which changes nothing this side of the call: one
/// owned row per step, decoded against a description that outlives it.
///
/// **The cells are moved and not copied**, exactly as [`sqlite_rows`] moves
/// them: the buffer a `TEXT` or `BLOB` crossed the thread in becomes the Novis
/// value's, so a walk pays one copy per cell and not two.
///
/// # Errors
///
/// [`statement_failure`]'s for anything SQLite refused mid-walk, and
/// [`sqlite_column_value`]'s for a cell the column's declaration does not
/// describe. A [`Fault::fatal`] for a row that is not as wide as the
/// description, which is a `nvs-db` bug rather than a program's.
fn sqlite_step(
    sqlite: &mut nvs_db::SqliteConn,
    block: &Value,
    watch: &QueryWatch,
) -> Result<Stepped, Fault> {
    let read = sqlite
        .stream_next_row()
        .map_err(|refused| statement_failure(STREAM_MEMBER, block, None, &refused))?;
    let Some(cells) = read else {
        let taken = sqlite.stream_span().and_then(|span| watch.taken(span));
        // The thread let go of the statement and the connection before it
        // answered this step, so this drops the description and the span alone.
        sqlite.end_stream();
        return Ok((None, taken));
    };

    let zone = sqlite.time_zone();
    let columns = sqlite.stream_columns().unwrap_or(&[]);
    if cells.len() != columns.len() {
        return Err(Fault::fatal(format!(
            "{STREAM_MEMBER}: the row holds {} column(s), where the result set described {}",
            cells.len(),
            columns.len()
        )));
    }

    // Built whole before it is parked, on [`park_row`]'s reasoning.
    let mut one = NvsArray::new();
    for (column, cell) in columns.iter().zip(cells) {
        one.set(
            NvsStr::new(column.name.as_bytes()),
            sqlite_column_value(column, cell, zone, STREAM_MEMBER)?,
        );
    }
    Ok((Some(one), None))
}

/// Opens the walk on either of the two drivers [`Framed`] covers, for
/// [`nvs_core_db_connection_stream`]'s arms.
///
/// [`mysql_step`]'s reason for existing, at the other end of the walk.
///
/// # Errors
///
/// [`statement_failure`] for anything the server refused, which for a connection
/// that is already streaming is `rule:core-classes/db-streaming`'s `LogicError`.
fn stream_over(
    mut framed: Framed<'_>,
    statement: &Statement,
    sending: &[Option<&[u8]>],
    source: Option<&str>,
    named: &str,
) -> Result<(), Fault> {
    framed
        .stream(&statement.sql, sending)
        .map_err(|refused| statement_failure(named, &statement.block, source, &refused))?;
    if let Some(name) = statement.block.as_text() {
        framed.name_stream_connection(name);
    }
    Ok(())
}

/// Opens the walk both of § 18's streaming members are: the portal left parked
/// on the connection, and a [`STREAM`] over it carrying `class` — a descriptor
/// for `streamAs<T>`, `null` for `stream`.
///
/// A function over the two rather than two bodies, for [`queried_rows`]'
/// reason: § 4 gives `stream` and `streamAs` one statement, one binding rule
/// and one deadline, so the only thing that can differ between them is what a
/// row becomes, and that is one slot.
///
/// # Errors
///
/// [`statement_of`]'s refusals, [`filed_connection`]'s `LogicError` for a
/// closed connection, and [`statement_failure`] for anything the server
/// refused, which for a connection that is already streaming is § 4's
/// `LogicError`. **No driver is refused**: every one of the five parks a read,
/// which is what `rule:core-classes/db-streaming` requires of both members.
fn open_stream(
    ctx: &mut nvs_runtime::Ctx,
    args: &[Value],
    member: &str,
    named: &str,
    class: Value,
) -> Result<Value, Fault> {
    let statement = statement_of(ctx, args, member, named)?;
    // § 4's `timeout` bounds the *walk* on this member and not the call that
    // opens it: the deadline stays filed on the connection while the portal
    // is open, so it is every `advance()` up to the last row that is bounded
    // — which is the wait a streaming caller actually has to survive. The
    // release lifts it, as it does for a buffered statement.
    let deadline = statement_deadline(args, named)?;
    // The caller's own text and not [`Statement::sql`], for
    // [`queried_rows`]' reason: a refusal names what the program wrote.
    let source = args[1].as_text();
    let sending: Vec<Option<&[u8]>> = statement.binds.wire();
    match bound_connection(ctx, statement.key, named, deadline)? {
        nvs_db::Connection::Postgres(postgres) => {
            // The description is answered here and read again per row off
            // the connection, so nothing about it is copied into this
            // object: it belongs to the statement rather than to the walk.
            postgres
                .stream(&statement.sql, &sending)
                .map_err(|refused| statement_failure(named, &statement.block, source, &refused))?;
            if let Some(name) = statement.block.as_text() {
                postgres.name_stream_connection(name);
            }
        }
        nvs_db::Connection::MySql(mysql) => {
            stream_over(Framed::MySql(mysql), &statement, &sending, source, named)?;
        }
        nvs_db::Connection::MariaDb(maria) => {
            stream_over(Framed::MariaDb(maria), &statement, &sending, source, named)?;
        }
        nvs_db::Connection::SqlServer(tds) => {
            tds.stream(&statement.sql, &sending)
                .map_err(|refused| statement_failure(named, &statement.block, source, &refused))?;
            if let Some(name) = statement.block.as_text() {
                tds.name_stream_connection(name);
            }
        }
        nvs_db::Connection::Sqlite(sqlite) => {
            // The one driver whose parameters arrive owned, exactly as
            // [`sqlite_rows`] sends them: there is no wire to encode for, so
            // `nvs_db` takes the values themselves.
            sqlite
                .stream(&statement.sql, statement.binds.sqlite())
                .map_err(|refused| statement_failure(named, &statement.block, source, &refused))?;
            if let Some(name) = statement.block.as_text() {
                sqlite.name_stream_connection(name);
            }
        }
    }
    // The block name is the *connection's*, read out of its slot by
    // [`handle_of`] and borrowed for the length of the call like every value
    // this crate reads out of an argument — and [`crate::instance::build`]
    // takes a reference over rather than making one. So the walk retains it
    // here, where it stops being the caller's and becomes this object's.
    // Without this the stream's release frees the connection's own name a
    // second time, which is a heap corruption the program never sees: the
    // right answer is printed and the process exits 127 with nothing on
    // stderr.
    #[expect(
        unsafe_code,
        reason = "the receiver's slot keeps its own reference for as long as \
                  the connection is alive, so the copy parked in this object \
                  needs one of its own"
    )]
    unsafe {
        statement.block.retain();
    }
    // `class` carries no reference — a descriptor rides in the payload half of
    // an otherwise-`null` value — so the slot takes it as it is, exactly as
    // [`nvs_core_db_connection_query_as`]'s does.
    Ok(crate::instance::build(
        &STREAM,
        [
            Value::uint(statement.key),
            statement.block,
            Value::null(),
            class,
        ],
    ))
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
    /// [`open_stream`]'s, which are this member's whole refusal set.
    fn nvs_core_db_connection_stream(ctx, args: [4]) {
        open_stream(ctx, args, "stream", STREAM_MEMBER, Value::null())
    }
}

nvs_runtime::nvs_helper! {
    /// `$c->streamAs<T>(string $sql, array<mixed> $params): Iterable<T>` — the
    /// walk above at the class its call site wrote, which is `queryAs`'s
    /// hydration over the one member that does not buffer.
    ///
    /// **Arguments 0 to 2 are what the call site wrote as its type argument**,
    /// and the receiver is argument 3:
    /// [`nvs_core_db_connection_query_as`] owns that ABI, and this is the
    /// second instance member on
    /// [`crate::registry::WRITTEN_CLASS_MEMBERS`]. So the arity is three more
    /// than `stream`'s, which is otherwise the same call.
    ///
    /// **Nothing about the statement differs**, down to the deadline and the
    /// parked portal: [`open_stream`] is the whole of it, and the descriptor
    /// goes into the walk's own slot to be read a row at a time. The refusals
    /// that are the *call site's* — a list type argument, a `T` carrying no
    /// `#[Db\Derive]`, a constructor no mapping fills — are
    /// `nvs_types::derive`'s `check_row_sites` while compiling, and the two
    /// below are what is left for a class built where no diagnostic could see
    /// it.
    ///
    /// # Errors
    ///
    /// [`open_stream`]'s, and a `LogicError` for an `array<...>` type argument.
    /// Per row, [`park_row`]'s `ParseError` for a row that does not match `T`
    /// and its `LogicError` for a `T` carrying no mapping at all.
    fn nvs_core_db_connection_stream_as(ctx, args: [7]) {
        // Unreachable from source, exactly as
        // [`nvs_core_db_connection_query_as`]'s reading of the same two slots
        // is: `nvs_ir::lower` writes the descriptor and the flag out of the
        // type argument at the call site, and a call naming none is `E0442`
        // before any of this runs.
        if args[0].as_class_desc().is_none() {
            return Err(Fault::fatal(format!(
                "internal error: `{STREAM_AS_MEMBER}` was called with no class in argument 0"
            )));
        }
        let list = args[1].as_bool().ok_or_else(|| Fault::fatal(format!(
            "internal error: `{STREAM_AS_MEMBER}` was called with no list flag in argument 1"
        )))?;
        // A walk is already one `T` per row, so a list form asks for the plural
        // twice — `queryAs`'s refusal over a result set, said of the member
        // that never holds one.
        if list {
            return Err(Fault::thrown_as(
                ThrownClass::Logic,
                format!(
                    "{STREAM_AS_MEMBER}: `array<...>` is not a type argument this member takes \
                     — a walk answers one row at a time, so write `streamAs<Person>(…)` and \
                     collect the rows yourself if a list is what you want"
                ),
            ));
        }
        open_stream(ctx, &args[3..], "streamAs", STREAM_AS_MEMBER, args[0])
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Drops the one reference this frame owns, exactly as a member's caller
    /// would.
    fn released(value: Value) {
        #[expect(
            unsafe_code,
            reason = "the reference released here is the one this frame holds, \
                      and the sweep below accounts for every other one"
        )]
        unsafe {
            value.release();
        }
    }

    /// How many owners hold `row` — for a row of the sweep below, the test's
    /// own reference plus whatever the walk is still holding.
    fn owners(row: Value) -> usize {
        let ptr = row.obj_ptr().expect("every row of the sweep is an object");
        #[expect(
            unsafe_code,
            reason = "the sweep keeps a reference of its own to every row it \
                      asks about, so each one is live for the whole test"
        )]
        unsafe {
            nvs_runtime::NvsObj::refcount_of(ptr)
        }
    }

    /// `rule:core-classes/db-statement-members`'s promise, measured where [`park_row`] keeps it: a walk
    /// over a result set of any size holds **one** row, because parking the
    /// next one releases the last and the end of the walk clears the slot.
    ///
    /// **Asserted by counting the rows the walk still holds**, rather than by
    /// reading the slot: a member that appended its rows to something as it
    /// went — which is what buffering *is* — would answer `current()` correctly
    /// on every line of the walk and still fail this, and one that cleared
    /// nothing at the end would pass the count and fail the two lines after it.
    /// A thousand rows rather than three, because the number the answer must
    /// not depend on is the result set's size.
    ///
    /// The rows a server would have sent are stood in for by the arrays
    /// [`stream_step`] builds a `DataRow` into, which is exactly where the wire
    /// ends and this crate begins: a socket is what no `-p nvs-stdlib` test
    /// has, and `nvs_db::PgConn::stream_next_row` is where the other half of
    /// the member is pinned.
    // covers: Core\Db\Connection::stream
    #[test]
    fn stream_answers_rows_without_holding_the_result_set() {
        const ROWS: u64 = 1_000;

        // The fourth slot is the class a `streamAs<T>` would have written, and
        // this walk is `stream`'s: the rows stay `Db\Row`s, which is what makes
        // the count below one about the parking and not about a constructor.
        let stream = crate::instance::build(
            &STREAM,
            [Value::uint(0), Value::null(), Value::null(), Value::null()],
        );
        let receiver = stream.obj_ptr().expect("`build` answers an object");
        let mut ctx = nvs_runtime::Ctx::new(nvs_runtime::OutputSink::Sink);

        // One reference of the test's own per row, so that a row the walk has
        // let go of is still live enough to be counted.
        let mut rows: Vec<Value> = Vec::new();
        for n in 0..ROWS {
            let mut one = NvsArray::new();
            one.set(NvsStr::new(b"n"), Value::uint(n));
            assert_eq!(
                park_row(&mut ctx, receiver, Some(one), None)
                    .expect("a row that stays a `Db\\Row` is built from nothing")
                    .as_bool(),
                Some(true),
                "a row parked is an `advance()` that answers `true`"
            );

            let held = crate::instance::slot(receiver, STREAM_ROW_AT);
            assert!(
                crate::instance::is_instance(held, &ROW),
                "the walk parks spec § 18's `Db\\Row` and not the array it holds"
            );
            #[expect(
                unsafe_code,
                reason = "the slot keeps its own reference until the next \
                          `park_row`, so the one this sweep keeps is its own"
            )]
            unsafe {
                held.retain();
            }
            rows.push(held);
        }

        // `current()` hands back a reference of its own, which is what lets a
        // loop body outlive the `advance()` that parked what it is reading.
        let read = stream_row(stream, nvs_runtime::sequence::CURRENT)
            .expect("the walk is standing on its last row");
        assert_eq!(
            owners(read),
            3,
            "`current` retains the row rather than lending out the slot's own reference"
        );
        let columns =
            super::row::row_columns(std::slice::from_ref(&read), nvs_runtime::sequence::CURRENT)
                .expect("a parked row holds the columns it was built over");
        assert_eq!(
            columns.get(b"n").and_then(Value::as_uint),
            Some(ROWS - 1),
            "`current` answers the row the last `advance` read"
        );
        // `columns` is borrowed rather than owned — a `ManuallyDrop`, so
        // letting it fall out of scope releases nothing — and `read` is the one
        // reference this frame owns.
        released(read);

        let still_held: Vec<usize> = rows
            .iter()
            .enumerate()
            .filter(|(_, row)| owners(**row) == 2)
            .map(|(at, _)| at)
            .collect();
        assert_eq!(
            still_held,
            vec![rows.len() - 1],
            "one row is held whatever the result set's size, and it is the one last read"
        );

        assert_eq!(
            park_row(&mut ctx, receiver, None, None)
                .expect("the end of a walk builds nothing")
                .as_bool(),
            Some(false),
            "the end of the result set is an `advance()` that answers `false`"
        );
        assert!(
            rows.iter().all(|row| owners(*row) == 1),
            "the last row is cleared at the end of the walk rather than held for as long as \
             the program holds the stream"
        );
        let after = stream_row(stream, nvs_runtime::sequence::CURRENT)
            .expect("`current` past the end of the walk is answerable");
        assert_eq!(
            after.tag(),
            Some(Tag::Null),
            "and what it answers is `null`, which is what the slot was cleared to"
        );

        for row in rows {
            released(row);
        }
        released(stream);
    }

    /// § 4's price, and the sharper half of the pair: a second statement
    /// written to a connection that is still streaming is a `LogicError`
    /// rather than an answer.
    ///
    /// **It is sharper because a member that had buffered could not fail it.**
    /// A buffered read finishes its statement before it returns, leaving the
    /// connection [`nvs_db::State::Idle`] — so it would *answer* the second
    /// query. Only a walk that is still holding the portal open refuses one,
    /// which is why this is the pair's evidence that nothing was buffered.
    ///
    /// The rule is split across two crates on purpose, and both halves are
    /// asserted here because neither is worth much alone: `nvs-db`'s
    /// [`nvs_db::State::may_start_statement`] is what refuses, asserted over
    /// the whole roster by **counting** the states that admit a statement, and
    /// [`statement_failure`] is what turns that refusal into the class § 4
    /// names — asserted over every member that can be the second statement,
    /// since this module's docs say the rule is the same one whichever member
    /// that is and a member that grew its own answer would still look right on
    /// its own line.
    #[test]
    fn a_second_statement_on_a_streaming_connection_is_a_logic_error() {
        let admitting: Vec<nvs_db::State> = [
            nvs_db::State::Idle,
            nvs_db::State::Executing,
            nvs_db::State::Streaming,
            nvs_db::State::Poisoned,
        ]
        .into_iter()
        .filter(|state| state.may_start_statement())
        .collect();
        assert_eq!(
            admitting,
            vec![nvs_db::State::Idle],
            "a statement is written to an idle connection and to no other, so an open walk \
             refuses one and a buffered read would not have to"
        );

        // The driver's own refusal, whose wording is `pg.rs`'s
        // `second_statement` and whose *kind* is the whole of what this side
        // reads — that function is `pub(crate)` there, and this crate's half of
        // the rule is the mapping rather than the sentence.
        let busy = std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!(
                "a statement was written to a connection that is {:?}, and `rule:core-classes/db-statement-members` allows \
                 one at a time",
                nvs_db::State::Streaming
            ),
        );
        let block = Value::null();
        for member in [STREAM_MEMBER, QUERY, QUERY_AS, EXECUTE, EXECUTE_MANY] {
            let refused = statement_failure(member, &block, Some("select 1"), &busy);
            let Fault::Thrown(ThrownClass::Logic, why) = refused else {
                panic!("{member} refuses a second statement as § 4's `LogicError`");
            };
            assert!(
                why.starts_with(member),
                "the refusal names the member the program called: {why}"
            );
            assert!(
                why.contains("Streaming"),
                "and carries the driver's own sentence, which names both of § 4's fixes: {why}"
            );
        }
    }
}
