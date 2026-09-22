//! The four executing members, and the per-driver arms beneath them.
//!
//! `query`, `queryAs`, `execute` and `executeMany` are one shape over five
//! drivers, so everything that varies is pushed to the bottom of this module:
//! the `*_rows` and `*_write` pairs are where a driver's own result becomes the
//! rows and the write [ADR 0067](/docs/decisions/0067.md) § 18
//! declares.
//!
//! **SQLite is the one that cannot be written like the others.** Its cells
//! carry a storage class and not a type, so its columns are decoded against
//! what the column *declared* — § 9's rule for that driver, and the reason its
//! decode sits here in full rather than beside the others in
//! [`mod@super::column`].

use super::*;

/// The ABI slot `rule:core-classes/db-statement-members`'s `{timeout?: Duration}` arrives in, on every
/// statement member.
///
/// One constant for all five, because the bag flattens to one trailing argument
/// and all five have the same three in front of it — the receiver, the statement
/// and its values. `queryAs` reads it here too: `crate::registry::WRITTEN_CLASS_MEMBERS`'
/// three leading constants are sliced off before [`queried_rows`] sees the
/// arguments at all, so the slot is the same number on both sides of that slice.
pub(super) const STATEMENT_TIMEOUT_ARG: usize = 3;

/// The instant this statement must have answered by, or `None` for a call that
/// named no `timeout`.
///
/// [`super::open::deadline_of`]'s twin for the statement path, reading the same
/// `Core\Time\Duration` off the same kind of slot. The two are separate
/// functions rather than one over an index because they refuse in their own
/// member's name, and a `connect` timeout and a statement timeout are bounds on
/// different things that a diagnostic should never blur.
///
/// # Errors
///
/// A thrown `RuntimeError` for a duration that is zero or negative — `rule:http-server/no-spelling-for-an-unbounded-wait`
/// has no spelling for an unbounded wait and a zero one is that spelling said
/// quietly — and a [`Fault::fatal`] for a slot that is neither a `Duration` nor
/// `Tag::Null`, which the row's type rules out.
pub(super) fn statement_deadline(
    args: &[Value],
    member: &str,
) -> Result<Option<std::time::Instant>, Fault> {
    if matches!(args[STATEMENT_TIMEOUT_ARG].tag(), Some(Tag::Null)) {
        return Ok(None);
    }
    let nanos = crate::time::nanos_of(args, STATEMENT_TIMEOUT_ARG, "timeout")?;
    if nanos <= 0 {
        return Err(Fault::thrown(format!(
            "{member}: `timeout` must be a positive duration, and this one is {nanos}ns"
        )));
    }
    Ok(Some(
        std::time::Instant::now() + std::time::Duration::from_nanos(nanos.unsigned_abs()),
    ))
}

/// [`filed_connection`], with § 4's deadline filed on it before the statement
/// goes out.
///
/// **Every statement path goes through this and not through
/// [`filed_connection`]**, including the ones whose caller named no timeout: the
/// bound belongs to the connection rather than to a call, so a statement that
/// named none has to *lift* the one the statement before it named. The other end
/// of that rule is [`warm_connection`], which lifts it again before § 13's reset
/// — the one exchange no program's clock may bound.
///
/// `nvs_db::Connection::set_deadline` owns what the instant reaches on each
/// driver: the socket on the four with a wire, and the lock wait on SQLite.
///
/// # Errors
///
/// [`filed_connection`]'s, plus a thrown `RuntimeError` for a connection that
/// would not take the bound at all — which only the SQLite arm can report.
pub(super) fn bound_connection<'a>(
    ctx: &'a mut nvs_runtime::Ctx,
    key: u64,
    named: &str,
    deadline: Option<std::time::Instant>,
) -> Result<&'a mut nvs_db::Connection, Fault> {
    let connection = filed_connection(ctx, key, named)?;
    connection.set_deadline(deadline).map_err(|refused| {
        Fault::thrown(format!(
            "{named}: the connection would not take a `timeout`: {refused}"
        ))
    })?;
    Ok(connection)
}

/// A connection `rule:core-classes/db-transactions`'s commands are written for, borrowed as one thing.
///
/// The drivers spell a transaction differently — `nvs_db::mysql`'s `begin`
/// owns the differences, from `START TRANSACTION` down to the release a nested
/// rollback does not owe — but they answer the same four questions, and
/// `transaction` asks them at five points around a closure it does not control.
/// An enum here rather than a trait in `nvs-db`: which commands a backend sends
/// is exactly what this goal's ADR slot refuses to flatten, and what this needs
/// is the *call sites* flattened rather than the drivers.
///
/// The key and the block are passed to [`transacting`] rather than a
/// [`Statement`] or a [`Batch`], because they are the only two fields it reads
/// and a batch is not a statement — the alternative is a `Statement` built with
/// an empty `binds` purely to reach this, which would be a shape nothing else
/// in this module means.
pub(super) enum Transacting<'a> {
    /// § 7 over the extended-query protocol's simple `Query`.
    Postgres(&'a mut nvs_db::PgConn),
    /// § 7 over `COM_QUERY`, with an isolation level as a command of its own.
    MySql(&'a mut nvs_db::MySqlConn),
    /// The same commands over the same framing — `nvs_db::mysql`'s `begin`,
    /// `commit` and `roll_back` are what `nvs_db::MariaConn` delegates to, as
    /// [`Framed`] says of the send path.
    MariaDb(&'a mut nvs_db::MariaConn),
    /// § 7 over `SQL_BATCH`, in T-SQL's own vocabulary and with the two
    /// differences `nvs_db::tds`'s `begin` owns: there is no read-only
    /// transaction to ask for, and an isolation level is a *session* setting
    /// this driver has to put back when the outermost transaction ends.
    SqlServer(&'a mut nvs_db::TdsConn),
    /// § 7 as ordinary SQL — `BEGIN`, `SAVEPOINT`, `RELEASE` — sent by a call
    /// on `nvs_host`'s blocking pool rather than over a wire, which is the one
    /// thing this arm does not share with the other four and is entirely
    /// `nvs_db::sqlite`'s to hold. What it does differently in *this* module's
    /// terms is only its refusals: `nvs_db::SqliteConn::begin` accepts all five
    /// isolation levels, SQLite being always serializable, and refuses
    /// `readOnly` at any depth because the property belongs to how the database
    /// was opened rather than to a transaction.
    Sqlite(&'a mut nvs_db::SqliteConn),
}

impl Transacting<'_> {
    /// How many of § 7's levels are open — 0 outside a transaction.
    pub(super) fn depth(&self) -> u32 {
        match self {
            Transacting::Postgres(postgres) => postgres.depth(),
            Transacting::MySql(mysql) => mysql.depth(),
            Transacting::MariaDb(maria) => maria.depth(),
            Transacting::SqlServer(tds) => tds.depth(),
            Transacting::Sqlite(sqlite) => sqlite.depth(),
        }
    }

    /// § 7's outermost `BEGIN`, or the `SAVEPOINT` a nested call opens.
    ///
    /// # Errors
    ///
    /// As the driver's own `begin`, including the refusal of a nested call that
    /// asked for either option.
    pub(super) fn begin(
        &mut self,
        isolation: Option<nvs_db::Isolation>,
        read_only: bool,
    ) -> std::io::Result<nvs_db::QuerySpan> {
        match self {
            Transacting::Postgres(postgres) => postgres.begin(isolation, read_only),
            Transacting::MySql(mysql) => mysql.begin(isolation, read_only),
            Transacting::MariaDb(maria) => maria.begin(isolation, read_only),
            Transacting::SqlServer(tds) => tds.begin(isolation, read_only),
            Transacting::Sqlite(sqlite) => sqlite.begin(isolation, read_only),
        }
    }

    /// § 7's `COMMIT`, or the release that closes a nested level.
    ///
    /// # Errors
    ///
    /// As the driver's own `commit`.
    pub(super) fn commit(&mut self) -> std::io::Result<nvs_db::QuerySpan> {
        match self {
            Transacting::Postgres(postgres) => postgres.commit(),
            Transacting::MySql(mysql) => mysql.commit(),
            Transacting::MariaDb(maria) => maria.commit(),
            Transacting::SqlServer(tds) => tds.commit(),
            Transacting::Sqlite(sqlite) => sqlite.commit(),
        }
    }

    /// § 7's `ROLLBACK`, or the undo of a nested level.
    ///
    /// # Errors
    ///
    /// As the driver's own `roll_back`.
    pub(super) fn roll_back(&mut self) -> std::io::Result<nvs_db::QuerySpan> {
        match self {
            Transacting::Postgres(postgres) => postgres.roll_back(),
            Transacting::MySql(mysql) => mysql.roll_back(),
            Transacting::MariaDb(maria) => maria.roll_back(),
            Transacting::SqlServer(tds) => tds.roll_back(),
            Transacting::Sqlite(sqlite) => sqlite.roll_back(),
        }
    }
}

/// The connection filed under `key`, as the driver § 7's commands run on.
///
/// **Total, and no longer a place a driver can be turned away.** It refused a
/// `sqlite` block while `nvs_db::sqlite` had the commands and this enum had no
/// arm over them; every `nvs_db::Connection` variant has one now, so the
/// roster this refusal rendered would have had nothing to leave out and the
/// message an operator read is gone rather than kept as an unreachable arm.
/// The `[db.<name>]` block the caller holds is no longer passed either — it was
/// here to be *named* in that sentence, and the one failure left names the key
/// instead because it is this crate's bookkeeping rather than a program's
/// configuration.
///
/// # Errors
///
/// A [`Fault::fatal`] for a key the request's own table does not hold, which is
/// this crate's paste error rather than a program's.
pub(super) fn transacting<'a>(
    ctx: &'a mut nvs_runtime::Ctx,
    key: u64,
    named: &str,
) -> Result<Transacting<'a>, Fault> {
    match filed_connection(ctx, key, named)? {
        nvs_db::Connection::Postgres(postgres) => Ok(Transacting::Postgres(postgres)),
        nvs_db::Connection::MySql(mysql) => Ok(Transacting::MySql(mysql)),
        nvs_db::Connection::MariaDb(maria) => Ok(Transacting::MariaDb(maria)),
        nvs_db::Connection::SqlServer(tds) => Ok(Transacting::SqlServer(tds)),
        nvs_db::Connection::Sqlite(sqlite) => Ok(Transacting::Sqlite(sqlite)),
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Db\Connection::query(string $sql, array<mixed> $params): Db\Rows`
    /// — `rule:core-classes/db-statement-members`'s buffered statement, and the first member of
    /// `Core\Db\Queryable` to land.
    ///
    /// **Every row is read before this returns**, which is § 4's default and
    /// the whole difference between this and `stream`: the connection is at a
    /// message boundary again by the time the caller has its answer, so the
    /// loop that reads rows and writes per row — the commonest one in web
    /// programming — needs no second connection and cannot meet § 4's
    /// connection-busy rule at runtime. [`ROWS`] is where what that spends is
    /// written down.
    ///
    /// **The bind order is the rewriter's and not the array's.** § 5's
    /// `rewrite` answers one `Source` per marker, and a repeated `:name` on a
    /// numbered dialect is one marker read twice — so the values go out in the
    /// order the *statement* asks for them, which is why nothing here counts
    /// placeholders itself and why an `inList`'s expansion needs no second
    /// pass.
    fn nvs_core_db_connection_query(ctx, args: [4]) {
        let answered = queried_rows(ctx, args, "query", QUERY)?;
        Ok(crate::instance::build(
            &ROWS,
            [
                Value::array(answered.rows),
                Value::null(),
                Value::array(answered.columns),
            ],
        ))
    }
}

/// What one statement answered, in the two shapes a [`ROWS`] holds it in.
///
/// The pair rather than the rows alone because they come out of one borrow and
/// are wanted at one place: [`ROWS_COLUMNS_SLOT`] says why the description is
/// built at query time, and the alternative — handing back a `Vec<PgColumn>`
/// for the caller to build objects from — would put half of that at each of
/// [`nvs_core_db_connection_query`] and
/// [`nvs_core_db_connection_query_as`] instead of neither.
pub(super) struct Answered {
    /// Every row, as [`ROWS_SLOT`] holds them.
    ///
    /// `pub(super)` for [`mod@super::schema`]'s catalog reads, which are the
    /// one caller that wants the rows without a `Core\Db\Rows` around them:
    /// `rule:core-classes/schema-introspection`'s introspection is two ordinary statements whose answer
    /// becomes a schema value rather than something a program sees.
    pub(super) rows: NvsArray,
    /// One [`COLUMN`] per described column, as [`ROWS_COLUMNS_SLOT`] holds
    /// them.
    columns: NvsArray,
}

/// One statement's rows and the columns it described, as the two arrays a
/// [`ROWS`] holds — the whole of what `query` and `queryAs` share, which is
/// everything except which class the result carries.
///
/// **`args` starts at the receiver**, so `queryAs` hands over the slice past
/// [`crate::registry::WRITTEN_CLASS_MEMBERS`]' three leading constants and both
/// members read one shape here. Nothing about the statement differs between
/// them: § 4 gives them one signature and one binding rule, and hydration is a
/// property of the result rather than of the wire.
///
/// # Errors
///
/// [`statement_of`]'s refusals, a thrown `RuntimeError` for a driver with no
/// send path yet, [`statement_failure`] for anything the server refused, and
/// [`column_value`]'s or [`mysql_column_value`]'s for a column whose value has
/// no Novis representation.
pub(super) fn queried_rows(
    ctx: &mut nvs_runtime::Ctx,
    args: &[Value],
    member: &str,
    named: &str,
) -> Result<Answered, Fault> {
    let statement = statement_of(ctx, args, member, named)?;
    // Read before the connection is in hand, because the refusal for a
    // nonsensical duration is the caller's mistake and owes no round trip.
    let deadline = statement_deadline(args, named)?;
    // § 18's `$sql` argument read a second time rather than [`Statement::sql`]:
    // what a refusal names is the text the program wrote, where that field is
    // § 5's rewrite of it. The tag is already known good — `statement_of`
    // refused anything else above — so the `None` arm here is unreachable and
    // costs no message of its own.
    let source = args[1].as_text();
    let sending: Vec<Option<&[u8]>> = statement.binds.wire();
    // Read before the statement takes the context, because it holds it for as
    // long as the rows do — see [`QueryWatch`] for the rest.
    let watch = QueryWatch::of(ctx, &statement.block);
    // The event is filed after the match and not inside it, because a driver's
    // rows borrow the connection and the connection borrows the context — so
    // the arm that read the span is still holding the thing the span is filed
    // on. Each arm hands back what [`QueryWatch::taken`] took, which is `None`
    // where nothing is reading rather than where a driver has no span.
    let (answered, taken) = match bound_connection(ctx, statement.key, named, deadline)? {
        nvs_db::Connection::Postgres(postgres) => {
            postgres_rows(postgres, &statement, &sending, source, watch, named)?
        }
        nvs_db::Connection::MySql(mysql) => mysql_rows(
            Framed::MySql(mysql),
            &statement,
            &sending,
            source,
            watch,
            named,
        )?,
        nvs_db::Connection::MariaDb(maria) => mysql_rows(
            Framed::MariaDb(maria),
            &statement,
            &sending,
            source,
            watch,
            named,
        )?,
        nvs_db::Connection::SqlServer(tds) => {
            tds_rows(tds, &statement, &sending, source, watch, named)?
        }
        // The one arm that does not read `sending`: its parameters went out as
        // storage classes, which [`Binds`] holds separately and this driver
        // takes owned.
        nvs_db::Connection::Sqlite(sqlite) => {
            sqlite_rows(sqlite, &statement, source, watch, named)?
        }
    };
    watch.file(ctx, taken);
    Ok(answered)
}

/// `rule:core-classes/db-statement-members`'s `Write`, as the driver answered it and before it becomes the
/// instance.
///
/// Two `Option`s and not two numbers: § 4 gives both fields `?uint`, and the
/// absence is a different fact from a zero on both drivers — a command that
/// carries no affected count at all, and a statement that generated no id.
pub(super) struct Written {
    /// The server's own affected count, `None` for a command that carries none.
    changed: Option<u64>,
    /// § 4's `lastId`, `None` where the statement generated no id.
    last_id: Option<u64>,
}

/// [`queried_rows`] over the PostgreSQL driver: the extended-query stream, § 9's
/// decode of every row, and `rule:observability/a-query-is-a-trace-event`'s span taken off the rows before they
/// are dropped.
///
/// # Errors
///
/// [`statement_failure`] for anything the server refused, and [`column_value`]'s
/// for a column whose value has no Novis representation.
pub(super) fn postgres_rows(
    postgres: &mut nvs_db::PgConn,
    statement: &Statement,
    sending: &[Option<&[u8]>],
    source: Option<&str>,
    watch: QueryWatch,
    named: &str,
) -> Result<(Answered, Option<(String, std::time::Duration)>), Fault> {
    // Read before the statement borrows the connection, and once for the whole
    // result: § 9's zone-less `TIMESTAMP` is decoded in the zone this
    // connection declared, and that is a property of the connection rather
    // than of the row.
    let zone = postgres.time_zone();
    let mut answered = postgres
        .query(&statement.sql, sending)
        .map_err(|refused| statement_failure(named, &statement.block, source, &refused))?;
    name_span(&mut answered, statement.block.as_text());
    // Taken before the first row: a `PgRows` lends its columns and its rows
    // out of one borrow, and the rows are read with it held mutably.
    let columns: Vec<nvs_db::PgColumn> = answered.columns().to_vec();
    let described = described_columns(&columns);

    let mut rows = NvsArray::new();
    loop {
        let Some(row) = answered
            .next_row()
            .map_err(|refused| statement_failure(named, &statement.block, source, &refused))?
        else {
            break;
        };
        // Built whole before it joins the result, so that a column this
        // driver cannot read back releases the row it was half way through
        // rather than leaving it in one — `NvsArray`'s own `Drop`.
        let mut one = NvsArray::new();
        for (index, column) in columns.iter().enumerate() {
            let body = row
                .column(index)
                .map_err(|refused| statement_failure(named, &statement.block, source, &refused))?;
            let scalar = column
                .scalar(body)
                .map_err(|refused| statement_failure(named, &statement.block, source, &refused))?;
            let value = column_value(scalar, zone, named, &column.name)?;
            one.set(NvsStr::new(column.name.as_bytes()), value);
        }
        rows.append(Value::array(one));
    }
    // After the drain, so the span carries the duration the caller waited and
    // the rows it actually got.
    let taken = watch.taken(answered.span());
    Ok((
        Answered {
            rows,
            columns: described,
        },
        taken,
    ))
}

/// A connection `nvs_db::mysql`'s statement path is written for, borrowed as
/// one thing.
///
/// **MariaDB is its own driver above the framing, not inside it.** `rule:core-classes/db-connection-is-named`
/// is emphatic that treating it as a MySQL flag is a design error, and
/// `nvs_db::maria` obeys that where it counts — its own targets, its own
/// authentication roster, its own § 8 code table. What it does not duplicate is
/// the wire: `nvs_db::MariaConn::query` is a two-line delegation into the same
/// `nvs_db::mysql::start_statement` that `nvs_db::MySqlConn::query` is, and it
/// hands back the same `nvs_db::MySqlRows`. So the only difference this module
/// can observe between the two on the send path is the *type of the borrow*,
/// and the three send members would otherwise each grow a second arm that
/// copies the first line for line.
///
/// An enum rather than a trait, for [`Transacting`]'s reason and no other: what
/// wants flattening is the call sites. Where the two drivers genuinely part —
/// MariaDB's `RETURNING`, which MySQL does not have — the arm belongs on the
/// connection in `nvs-db`, and nothing about it reaches here.
///
/// **`pub(crate)` because [`crate::queue`] sends over the same two drivers**, and
/// for [`QueryWatch`]'s reason: `rule:concurrency/enqueue-commits-with-your-write`'s members drive a result set themselves
/// rather than through this class's, so a second borrow-flattening enum over
/// there would be this one with the same two arms. It carries § 7's three
/// commands as well as the send, which [`Transacting`] also spells — the two are
/// not one type because that one covers PostgreSQL, whose arm the queue's splits
/// must not reach: a `Split` is what a driver *without* the single-statement
/// construct runs, and PostgreSQL runs the single statement instead.
pub(crate) enum Framed<'a> {
    /// § 1's two round trips as MySQL frames them.
    MySql(&'a mut nvs_db::MySqlConn),
    /// The same two, framed as MariaDB and authenticated by its own roster.
    MariaDb(&'a mut nvs_db::MariaConn),
}

impl Framed<'_> {
    /// § 9's zone a zone-less `DATETIME` off this connection is read in, as
    /// seconds east of UTC.
    pub(crate) fn time_zone(&self) -> i32 {
        match self {
            Framed::MySql(mysql) => mysql.time_zone(),
            Framed::MariaDb(maria) => maria.time_zone(),
        }
    }

    /// `rule:core-classes/db-one-api`'s round trips for one statement, and the rows it answers
    /// with.
    ///
    /// # Errors
    ///
    /// As the driver's own `query`, which on both is
    /// `nvs_db::mysql::start_statement`'s.
    pub(crate) fn query(
        &mut self,
        sql: &str,
        params: &[Option<&[u8]>],
    ) -> std::io::Result<nvs_db::MySqlRows<'_>> {
        match self {
            Framed::MySql(mysql) => mysql.query(sql, params),
            Framed::MariaDb(maria) => maria.query(sql, params),
        }
    }

    /// `rule:core-classes/db-streaming`'s `stream`: the same statement [`Self::query`] sends, left
    /// open with its read state parked on the connection.
    ///
    /// The description the driver answers with is dropped here rather than
    /// returned, for the reason the PostgreSQL arm drops its own: the row loop
    /// reads it back off the connection per step, so nothing about it is copied
    /// into the walk. What it is *not* dropped for is a type — a definition is
    /// `mysql_common`'s `Column` and this crate does not depend on that crate, so
    /// a definition can be passed through here but not named in a signature.
    ///
    /// # Errors
    ///
    /// As the driver's own `stream`, which on both is `nvs_db::mysql`'s
    /// `open_result`.
    pub(crate) fn stream(&mut self, sql: &str, params: &[Option<&[u8]>]) -> std::io::Result<()> {
        match self {
            Framed::MySql(mysql) => mysql.stream(sql, params).map(|_| ()),
            Framed::MariaDb(maria) => maria.stream(sql, params).map(|_| ()),
        }
    }

    /// The next row of the parked walk, or `None` once it has ended.
    ///
    /// # Errors
    ///
    /// As the driver's own `stream_next_row`, which on both is `nvs_db::mysql`'s
    /// `next_row_of`.
    pub(crate) fn stream_next_row(&mut self) -> std::io::Result<Option<nvs_db::MySqlRow>> {
        match self {
            Framed::MySql(mysql) => mysql.stream_next_row(),
            Framed::MariaDb(maria) => maria.stream_next_row(),
        }
    }

    /// `rule:observability/a-query-is-a-trace-event`'s event for the parked walk, or `None` where
    /// there is none.
    pub(crate) fn stream_span(&self) -> Option<&nvs_db::QuerySpan> {
        match self {
            Framed::MySql(mysql) => mysql.stream_span(),
            Framed::MariaDb(maria) => maria.stream_span(),
        }
    }

    /// Names the `[db.<name>]` block the parked walk is running on — the driver
    /// cannot work it out for itself, which is [`name_span`]'s reason.
    pub(crate) fn name_stream_connection(&mut self, connection: &str) {
        match self {
            Framed::MySql(mysql) => mysql.name_stream_connection(connection),
            Framed::MariaDb(maria) => maria.name_stream_connection(connection),
        }
    }

    /// Drops the parked walk, draining whatever is left of its result set.
    pub(crate) fn end_stream(&mut self) {
        match self {
            Framed::MySql(mysql) => mysql.end_stream(),
            Framed::MariaDb(maria) => maria.end_stream(),
        }
    }

    /// § 7's `START TRANSACTION`, or the `SAVEPOINT` a nested one is.
    ///
    /// # Errors
    ///
    /// As the driver's own `begin`.
    pub(crate) fn begin(
        &mut self,
        isolation: Option<nvs_db::Isolation>,
        read_only: bool,
    ) -> std::io::Result<nvs_db::QuerySpan> {
        match self {
            Framed::MySql(mysql) => mysql.begin(isolation, read_only),
            Framed::MariaDb(maria) => maria.begin(isolation, read_only),
        }
    }

    /// § 7's `COMMIT`, or the release that closes a nested level.
    ///
    /// # Errors
    ///
    /// As the driver's own `commit`.
    pub(crate) fn commit(&mut self) -> std::io::Result<nvs_db::QuerySpan> {
        match self {
            Framed::MySql(mysql) => mysql.commit(),
            Framed::MariaDb(maria) => maria.commit(),
        }
    }

    /// § 7's `ROLLBACK`, or the undo of a nested level.
    ///
    /// # Errors
    ///
    /// As the driver's own `roll_back`.
    pub(crate) fn roll_back(&mut self) -> std::io::Result<nvs_db::QuerySpan> {
        match self {
            Framed::MySql(mysql) => mysql.roll_back(),
            Framed::MariaDb(maria) => maria.roll_back(),
        }
    }
}

/// [`queried_rows`] over the two drivers [`Framed`] covers: `rule:core-classes/db-one-api`'s
/// `COM_STMT_EXECUTE`, and § 9's decode of the binary rows it answers with.
///
/// **The same shape as [`postgres_rows`] and deliberately not shared with it.**
/// The two drivers agree on what a row *is* — a keyed array under the labels the
/// result set described — and on nothing else in the walk: the description is
/// read off the stream here and off a cloned `PgColumn` there, a value is a
/// `MyValue` read against its own definition rather than a body the column
/// decodes, and the two `Scalar` enums are two sets of rows because MySQL has no
/// `UUID` and no array type. A trait over that would be four abstract methods
/// standing for eight concrete lines.
///
/// **§ 11's event is the one thing the two do share**, down to the line: a
/// `MySqlRows` opens its own span exactly as a `PgRows` does, so the pair this
/// hands back is [`postgres_rows`]' pair and a trace reads across the two
/// drivers without a field being spelled twice.
///
/// # Errors
///
/// [`statement_failure`] for anything the server refused, [`mysql_column_value`]'s
/// for a column whose value has no Novis representation, and a [`Fault::fatal`]
/// for a row narrower than the definitions it was decoded against, which is a
/// `nvs-db` bug rather than a program's.
pub(super) fn mysql_rows(
    mut framed: Framed<'_>,
    statement: &Statement,
    sending: &[Option<&[u8]>],
    source: Option<&str>,
    watch: QueryWatch,
    named: &str,
) -> Result<(Answered, Option<(String, std::time::Duration)>), Fault> {
    // As [`postgres_rows`], and § 9's zone rule is the connection's on both
    // drivers.
    let zone = framed.time_zone();
    let mut answered = framed
        .query(&statement.sql, sending)
        .map_err(|refused| statement_failure(named, &statement.block, source, &refused))?;
    name_span(&mut answered, statement.block.as_text());
    // Described before the first row, because the description is read out of a
    // shared borrow of the stream and the rows out of a mutable one — the same
    // ordering `postgres_rows` gets by cloning its columns, and here the clone
    // is needed anyway: `nvs_db::mysql::scalar` reads a value against the
    // definition it arrived under.
    let described = mysql_described_columns(&answered);
    let columns = answered.columns().to_vec();

    let mut rows = NvsArray::new();
    while let Some(row) = answered
        .next_row()
        .map_err(|refused| statement_failure(named, &statement.block, source, &refused))?
    {
        // Built whole before it joins the result, for [`postgres_rows`]' reason.
        let mut one = NvsArray::new();
        for (index, column) in columns.iter().enumerate() {
            // Unreachable: `nvs-db` decodes one value per definition, so a row
            // is exactly as wide as this loop. It is a `fatal` rather than a
            // refusal because a narrower row is that crate disagreeing with
            // itself and not something a statement can ask for.
            let body = row.value(index).ok_or_else(|| {
                Fault::fatal(format!(
                    "{named}: the row has no column {index}, where the result set described {}",
                    columns.len()
                ))
            })?;
            let scalar = nvs_db::mysql::scalar(column, body)
                .map_err(|refused| statement_failure(named, &statement.block, source, &refused))?;
            // A label is bytes on this driver and text on the other, and § 9
            // reads both as UTF-8: the lossy decode is for the message only,
            // where the key keeps the octets the server sent.
            let label = String::from_utf8_lossy(column.name_ref());
            let value = mysql_column_value(scalar, zone, named, &label)?;
            one.set(NvsStr::new(column.name_ref()), value);
        }
        rows.append(Value::array(one));
    }
    // After the drain, as [`postgres_rows`]: the terminator is what freezes the
    // duration, and the rows counted are the ones that came back.
    let taken = watch.taken(answered.span());
    Ok((
        Answered {
            rows,
            columns: described,
        },
        taken,
    ))
}

/// [`queried_rows`] over the SQL Server driver: `rule:core-classes/db-one-api`'s `sp_prepexec`,
/// and § 9's decode of the token stream it answers with.
///
/// **[`mysql_rows`]' shape a third time**, and that function's doc argues at
/// length why the three are not one walk. What differs here is smaller than
/// what differs between the other two: a value is the row's own octets read
/// against the column `COLMETADATA` described, which is `nvs_db::tds::scalar`'s
/// reading rather than this module's, and a label is a `String` because TDS
/// carries it as UCS-2 and the driver has already decoded it — so the key is
/// that text's octets and no lossy decode stands between the two.
///
/// # Errors
///
/// [`statement_failure`] for anything the server refused, [`tds_column_value`]'s
/// for a column whose value has no Novis representation, and a [`Fault::fatal`]
/// for a row narrower than the columns it was decoded against, which is a
/// `nvs-db` bug rather than a program's.
pub(super) fn tds_rows(
    tds: &mut nvs_db::TdsConn,
    statement: &Statement,
    sending: &[Option<&[u8]>],
    source: Option<&str>,
    watch: QueryWatch,
    named: &str,
) -> Result<(Answered, Option<(String, std::time::Duration)>), Fault> {
    // As the other two drivers, and § 9's zone is a property of the connection
    // on all three. What is this driver's own is that no server was told:
    // `nvs_db::tds::TdsTarget::time_zone` owns why SQL Server has nowhere to be.
    let zone = tds.time_zone();
    let mut answered = tds
        .query(&statement.sql, sending)
        .map_err(|refused| statement_failure(named, &statement.block, source, &refused))?;
    name_span(&mut answered, statement.block.as_text());
    // Cloned before the first row, for [`postgres_rows`]' reason: the columns
    // are lent out of a shared borrow and the rows out of a mutable one.
    let columns: Vec<nvs_db::tds::TdsColumn> = answered.columns().to_vec();
    let described = tds_described_columns(&answered);

    let mut rows = NvsArray::new();
    while let Some(row) = answered
        .next_row()
        .map_err(|refused| statement_failure(named, &statement.block, source, &refused))?
    {
        // Built whole before it joins the result, for [`postgres_rows`]' reason.
        let mut one = NvsArray::new();
        for (index, column) in columns.iter().enumerate() {
            // Unreachable and fatal for [`mysql_rows`]' reason: `nvs-db` reads
            // one value per described column, so a row is exactly as wide as
            // this loop.
            let body = row.column(index).ok_or_else(|| {
                Fault::fatal(format!(
                    "{named}: the row has no column {index}, where the result set described {}",
                    columns.len()
                ))
            })?;
            let scalar = nvs_db::tds::scalar(column, body)
                .map_err(|refused| statement_failure(named, &statement.block, source, &refused))?;
            let value = tds_column_value(scalar, zone, named, &column.name)?;
            one.set(NvsStr::new(column.name.as_bytes()), value);
        }
        rows.append(Value::array(one));
    }
    // After the drain, as the other two: the `DONE` token is what freezes the
    // duration and the count the span carries.
    let taken = watch.taken(answered.span());
    Ok((
        Answered {
            rows,
            columns: described,
        },
        taken,
    ))
}

/// `execute` over the PostgreSQL driver: the same stream [`postgres_rows`]
/// drains, read for its counts rather than its rows.
///
/// **The rows are drained and discarded, not skipped.** § 4 gives `execute` no
/// way to hand a `RETURNING` clause's rows back, and the completion tag that
/// carries the affected count is on the far side of them — so a member that
/// walked away would leave the connection mid-stream and would have no count to
/// answer with either.
///
/// # Errors
///
/// [`statement_failure`] for anything the server refused.
pub(super) fn postgres_write(
    postgres: &mut nvs_db::PgConn,
    statement: &Statement,
    sending: &[Option<&[u8]>],
    source: Option<&str>,
    watch: QueryWatch,
    named: &str,
) -> Result<(Written, Option<(String, std::time::Duration)>), Fault> {
    let mut answered = postgres
        .query(&statement.sql, sending)
        .map_err(|refused| statement_failure(named, &statement.block, source, &refused))?;
    name_span(&mut answered, statement.block.as_text());
    while answered
        .next_row()
        .map_err(|refused| statement_failure(named, &statement.block, source, &refused))?
        .is_some()
    {}

    let written = Written {
        changed: answered.affected(),
        last_id: answered.last_id(),
    };
    // § 11's event is a *statement's*, not a reader's: a write files one on the
    // same terms as `query`, carrying the affected count `finished` froze on
    // the span above.
    let taken = watch.taken(answered.span());
    Ok((written, taken))
}

/// `execute` over the MySQL driver: [`postgres_write`]'s shape, and § 4's two
/// counts read out of the status packet rather than out of a completion tag.
///
/// **`lastId` is where the two drivers differ and § 4 does not.** MySQL answers
/// `0` for a statement that generated no `AUTO_INCREMENT` value, and § 4's field
/// is `?uint` — so the zero is mapped to null here rather than handed to a
/// caller who would have to know to read it as absence. PostgreSQL reaches the
/// same answer by having no `RETURNING` id to read at all.
///
/// # Errors
///
/// [`statement_failure`] for anything the server refused.
pub(super) fn mysql_write(
    mut framed: Framed<'_>,
    statement: &Statement,
    sending: &[Option<&[u8]>],
    source: Option<&str>,
    watch: QueryWatch,
    named: &str,
) -> Result<(Written, Option<(String, std::time::Duration)>), Fault> {
    let mut answered = framed
        .query(&statement.sql, sending)
        .map_err(|refused| statement_failure(named, &statement.block, source, &refused))?;
    name_span(&mut answered, statement.block.as_text());
    // A write answers no result set, but a `CALL` does — and draining is what
    // ends the statement on this driver, as [`postgres_write`]'s does there.
    while answered
        .next_row()
        .map_err(|refused| statement_failure(named, &statement.block, source, &refused))?
        .is_some()
    {}

    let written = Written {
        changed: answered.affected(),
        last_id: answered.last_id().filter(|id| *id != 0),
    };
    let taken = watch.taken(answered.span());
    Ok((written, taken))
}

/// `execute` over the SQL Server driver: the statement [`tds_rows`] sends, read
/// for its count rather than for its rows.
///
/// **`lastId` is `null` on this driver and it is an answer, not a gap.** SQL
/// Server puts no generated key in the token stream at all — `SCOPE_IDENTITY()`
/// is a statement a caller writes — which is why `nvs_db::tds::TdsRows` has no
/// `last_id` for this function to have missed, and § 4's field is `?uint` for
/// the same absence PostgreSQL has.
///
/// **The rows are drained and discarded** for [`postgres_write`]'s reason, and
/// this backend adds one of its own: `nvs_db::tds::TdsRows::affected` answers
/// `None` until the stream has ended, because the count rides the `DONE` token
/// behind the last row rather than arriving in front of it.
///
/// # Errors
///
/// [`statement_failure`] for anything the server refused.
/// [`queried_rows`] over the SQLite driver: the statement off the blocking pool,
/// the rows already in hand, and § 11's span opened here rather than by the
/// driver.
///
/// **The span is this module's on this backend alone**, for `executeMany`'s
/// reason: `nvs_db::SqliteRows` lends none out, there being no round trip for a
/// driver-side one to time. What it does time is the handoff to
/// `nvs_host::blocking` and back, which is this driver's whole cost and the only
/// thing § 11 could usefully report about it.
///
/// **A cell arrives as the class it was stored in** and not as § 9's declared
/// type — [`sqlite_column_value`] owns that split, and the column's own
/// [`COLUMN`] carries the declared answer beside the value so a typed reader can
/// make it.
///
/// # Errors
///
/// [`statement_failure`] for anything SQLite refused, and a [`Fault::fatal`] for
/// a row narrower than the columns the statement described.
pub(super) fn sqlite_rows(
    sqlite: &mut nvs_db::SqliteConn,
    statement: &Statement,
    source: Option<&str>,
    watch: QueryWatch,
    named: &str,
) -> Result<(Answered, Option<(String, std::time::Duration)>), Fault> {
    let mut span = nvs_db::QuerySpan::opened(nvs_db::Driver::Sqlite, &statement.sql);
    if let Some(name) = statement.block.as_text() {
        span.name(name);
    }
    // Read before the statement borrows the connection, and once for the whole
    // result — [`postgres_rows`]' reason: § 9's zone-less `DATETIME` is decoded
    // in the zone this connection declared, which is a property of the
    // connection rather than of the row.
    let zone = sqlite.time_zone();
    let mut answered = sqlite
        .query(&statement.sql, statement.binds.sqlite())
        .map_err(|refused| statement_failure(named, &statement.block, source, &refused))?;
    // Cloned before the first row, for [`postgres_rows`]' reason: the columns
    // are lent out of a shared borrow and the rows out of a mutable one.
    let columns: Vec<nvs_db::SqliteColumn> = answered.columns().to_vec();
    let described = sqlite_described_columns(&columns);

    let mut rows = NvsArray::new();
    while let Some(row) = answered.next_row() {
        // Unreachable and fatal for [`tds_rows`]' reason: `nvs-db` steps one
        // value per described column, so a row is exactly as wide as the
        // description. Asked of the whole row rather than per cell because the
        // cells are *moved* into the result below — the owned buffer a `TEXT`
        // or `BLOB` came back in becomes the Novis value's, which is the one
        // copy this driver's materialized rows do not have to pay twice.
        if row.len() != columns.len() {
            return Err(Fault::fatal(format!(
                "{named}: the row holds {} column(s), where the result set described {}",
                row.len(),
                columns.len()
            )));
        }
        // Built whole before it joins the result, for [`postgres_rows`]' reason
        // — which this driver now needs for its own: a cell the column's
        // declaration does not describe throws part way through a row, and
        // `NvsArray`'s `Drop` releases what was already set.
        let mut one = NvsArray::new();
        for (column, cell) in columns.iter().zip(row) {
            one.set(
                NvsStr::new(column.name.as_bytes()),
                sqlite_column_value(column, cell, zone, named)?,
            );
        }
        span.row();
        rows.append(Value::array(one));
    }
    // No affected count on a read: `nvs_db::SqliteRows::affected` answers what
    // the last data-changing statement on this *connection* reported, which is
    // SQLite's own rule and belongs to [`sqlite_write`] alone.
    span.finished(None);
    let taken = watch.taken(&span);
    Ok((
        Answered {
            rows,
            columns: described,
        },
        taken,
    ))
}

/// `execute` over the SQLite driver: the same statement [`sqlite_rows`] runs,
/// read for its counts rather than its cells.
///
/// **`changed` is present for every statement this driver runs.** § 4's absent
/// case is a command tag that carries no count, and SQLite sends no tag at all:
/// a statement whose kind counts nothing cannot be told apart here from one
/// that changed no rows, so both report `0` and neither reports an absence.
/// That is what makes `Core\Db\Write::changed` answer on this driver exactly
/// what `affected` folds, where PostgreSQL keeps the two apart. `lastId` does
/// keep an absence, `0` being how SQLite spells "no row has ever been inserted
/// here".
///
/// The count is the statement's own rather than the connection's, which is
/// `nvs_db::SqliteRows::affected`'s work and its doc's to argue:
/// `sqlite3_changes` describes the last data-changing statement on the
/// connection, and `sqlite3_total_changes` moving is what tells this
/// statement's rows from an earlier statement's.
///
/// # Errors
///
/// [`statement_failure`] for anything SQLite refused.
pub(super) fn sqlite_write(
    sqlite: &mut nvs_db::SqliteConn,
    statement: &Statement,
    source: Option<&str>,
    watch: QueryWatch,
    named: &str,
) -> Result<(Written, Option<(String, std::time::Duration)>), Fault> {
    let mut span = nvs_db::QuerySpan::opened(nvs_db::Driver::Sqlite, &statement.sql);
    if let Some(name) = statement.block.as_text() {
        span.name(name);
    }
    let mut answered = sqlite
        .query(&statement.sql, statement.binds.sqlite())
        .map_err(|refused| statement_failure(named, &statement.block, source, &refused))?;
    // Drained rather than skipped, for [`postgres_write`]'s reason: § 4 gives
    // `execute` no way to hand a `RETURNING` clause's rows back, and the result
    // set is what holds the connection until it is done with.
    while answered.next_row().is_some() {}

    let written = Written {
        changed: Some(answered.affected()),
        last_id: u64::try_from(answered.last_insert_id())
            .ok()
            .filter(|id| *id != 0),
    };
    span.finished(written.changed);
    let taken = watch.taken(&span);
    Ok((written, taken))
}

/// One [`COLUMN`] per column a SQLite statement described.
///
/// [`described_columns`]' third sibling, and the one whose type is not a code a
/// server sent: `nvs_db::SqliteColumn::column_type` keys § 9 off the schema's
/// *declared* name, and that method's own doc is where the map and its three
/// refusals are argued.
pub(super) fn sqlite_described_columns(columns: &[nvs_db::SqliteColumn]) -> NvsArray {
    let mut described = NvsArray::new();
    for column in columns {
        described.append(crate::instance::build(
            &COLUMN,
            [
                Value::str(NvsStr::new(column.name.as_bytes())),
                column_type_value(column.column_type()),
                // As [`described_columns`]: § 9's own answer, and
                // [`COLUMN_NULLABLE_DOC`] is where it is written down. SQLite
                // describes no nullability at all on a stepped statement, so
                // there is not even a narrower server flag to sit beside it.
                Value::bool(true),
            ],
        ));
    }
    described
}

/// One stepped cell as [ADR 0067 § 9](/docs/decisions/0067.md)'s
/// value for the type its column was **declared** — the storage class it
/// arrived in wherever that declaration names something SQLite can hold, and a
/// `Core\Time`, a `Core\Uuid`, a `decimal` or a `bool` where it does not.
///
/// **The declaration and not the cell, and that is § 9's sentence for this
/// backend rather than a preference.** The other four drivers read a type code
/// the server sent and the encoding follows from it, so a `date` column can only
/// arrive as a date. SQLite stores five classes whatever the column was declared
/// as, so `'2026-09-03'` in a column declared `date` is text and `20260903` in
/// the same column is an integer — and § 9 answers that with "mapping keys off
/// the *declared* column type and throws on a value that does not parse", which
/// is the schema's word deciding and the cell either agreeing or being a
/// mistake. The five arms below are the whole of "agreeing"; everything else in
/// a converting column is [`unparsed_column`].
///
/// **So the *natural* type of a SQLite column is § 9's, exactly as it is on the
/// other four**, and that is what makes this the only edit the map needed:
/// `get`, `toArray`, all eleven of [`ROW`]'s typed readers, `queryAs<T>` and
/// [`converted`] read the value this built and none of them learns that SQLite
/// exists. A reader that parsed text on request instead would be § 6's
/// *conversion* rule doing § 9's *map*'s job — it would loosen `->date()` on
/// the four drivers that have a real `DATE`, and it could not reach a
/// `DATETIME` at all, § 6's eleven readers having no member for one.
///
/// **A `DECIMAL` accepts a `REAL`, which is the one place this loses precision
/// and it is lost before the read.** A column declared `decimal(10,2)` has
/// NUMERIC affinity, so SQLite converts the `TEXT` `nvs_db::sqlite::encode`
/// binds into a `REAL` on the way in; refusing one here would mean no `decimal`
/// column on this backend ever reads back. The shortest round-trip rendering is
/// therefore the honest answer for what is actually stored, and
/// `rule:types/decimal` is not weakened
/// by it — the value crossed binary floating point in the *engine*, and reading
/// it back as a `float` would only hide that.
///
/// `zone` is the connection's declared zone, for § 9's zone-less `DATETIME`; it
/// is [`postgres_rows`]' `zone` and read once for the same reason.
///
/// It still allocates nothing a value does not already own on the arms that do
/// not convert: the two owned ones move their buffer into the Novis string or
/// bytes rather than copying it a second time.
///
/// # Errors
///
/// [`unparsed_column`] for a cell the column's declaration does not describe.
pub(super) fn sqlite_column_value(
    column: &nvs_db::SqliteColumn,
    cell: nvs_db::SqliteValue,
    zone: i32,
    named: &str,
) -> Result<Value, Fault> {
    use nvs_db::{ColumnType, SqliteValue};

    // § 9 reads a SQL NULL back as `null` whatever the column was declared, so
    // the map below never has one to refuse.
    if matches!(cell, SqliteValue::Null) {
        return Ok(Value::null());
    }
    let declared = column.column_type();
    let want = match declared {
        ColumnType::Date => "a date",
        ColumnType::Time => "a time of day",
        ColumnType::DateTime => "a date and time",
        ColumnType::Uuid => "a UUID",
        ColumnType::Decimal => "a number",
        ColumnType::Bool => "`0` or `1`",
        // `Int`, `Float`, `Text`, `Bytes` and `Other` are storage classes
        // SQLite has, so the cell is already § 9's answer — and this map never
        // answers `Uint`, `Instant` or `Json`, which
        // `nvs_db::SqliteColumn::column_type` argues in full.
        _ => return Ok(sqlite_stored_value(cell)),
    };
    let built = match (declared, &cell) {
        (ColumnType::Date, SqliteValue::Text(text)) => crate::time::date_of_text(text),
        (ColumnType::Time, SqliteValue::Text(text)) => crate::time::time_of_day_of_text(text),
        (ColumnType::DateTime, SqliteValue::Text(text)) => {
            crate::time::datetime_of_text(text, zone)
        }
        (ColumnType::Uuid, SqliteValue::Text(text)) => crate::uuid::of_text(text),
        (ColumnType::Decimal, SqliteValue::Text(text)) => {
            nvs_runtime::Decimal::parse(text).map(Value::decimal)
        }
        (ColumnType::Decimal, SqliteValue::Int(int)) => {
            nvs_runtime::Decimal::parse(&int.to_string()).map(Value::decimal)
        }
        (ColumnType::Decimal, SqliteValue::Real(real)) => {
            nvs_runtime::Decimal::parse(&real.to_string()).map(Value::decimal)
        }
        // § 6's `bool` rule, said where § 9 has to say it: `0` and `1` are the
        // whole of what a flag column holds, and a stored `7` throws rather
        // than reading as PHP's `true`. [`requested_bool`] is the same rule on
        // the *request* side and the two agree by construction.
        (ColumnType::Bool, SqliteValue::Int(0)) => Some(Value::bool(false)),
        (ColumnType::Bool, SqliteValue::Int(1)) => Some(Value::bool(true)),
        _ => None,
    };
    built.ok_or_else(|| unparsed_column(named, column, &cell, want))
}

/// One cell as the Novis value of the storage class it arrived in — § 9's
/// answer for every column whose declared type names something SQLite holds
/// natively, and [`sqlite_column_value`]'s non-converting half.
pub(super) fn sqlite_stored_value(cell: nvs_db::SqliteValue) -> Value {
    match cell {
        nvs_db::SqliteValue::Null => Value::null(),
        nvs_db::SqliteValue::Int(int) => Value::int(int),
        nvs_db::SqliteValue::Real(real) => Value::float(real),
        nvs_db::SqliteValue::Text(text) => Value::str(NvsStr::new(text.as_bytes())),
        nvs_db::SqliteValue::Blob(bytes) => Value::bytes(NvsStr::new(&bytes)),
    }
}

/// § 9's SQLite throw: the column's declaration says what its values are, and
/// this cell is not one.
///
/// [`unrepresentable_column`]'s twin, and the division is which side wrote the
/// value — that one is a reading the server rendered correctly and no
/// `Core\Time` type has, this one is a cell the schema's own word does not
/// describe. Both are a [`Fault::thrown`] rather than a `Db\DbError`, because
/// neither is anything a driver reported: the statement succeeded and the
/// decode is this crate's.
///
/// The cell is quoted, bounded by [`sqlite_storage_class`], because the column
/// is what the program has to go and fix.
pub(super) fn unparsed_column(
    named: &str,
    column: &nvs_db::SqliteColumn,
    cell: &nvs_db::SqliteValue,
    want: &str,
) -> Fault {
    Fault::thrown(format!(
        "{named}: the column `{}` is declared `{}` and holds {}, which is not {want} — `rule:core-classes/db-one-api` \
         § 9 keys SQLite off the declared type, a storage class saying nothing about what a value \
         was meant as, and throws on one that does not parse",
        column.name,
        // Always present: a column with no declared type describes as
        // `ColumnType::Other`, which never reaches this throw.
        column.declared.as_deref().unwrap_or("an expression"),
        sqlite_storage_class(cell)
    ))
}

/// What a cell holds, for [`unparsed_column`]'s sentence: its storage class,
/// and the value itself where that is short enough to be worth reading.
///
/// Bounded at 40 characters because a column is program data and a message is a
/// log line — [`crate::uuid`]'s `shown` is the same bound for the same reason.
pub(super) fn sqlite_storage_class(cell: &nvs_db::SqliteValue) -> String {
    match cell {
        nvs_db::SqliteValue::Null => "SQL `NULL`".to_owned(),
        nvs_db::SqliteValue::Int(int) => format!("the `INTEGER` {int}"),
        nvs_db::SqliteValue::Real(real) => format!("the `REAL` {real}"),
        nvs_db::SqliteValue::Text(text) => {
            let shown: String = text.chars().take(40).collect();
            if shown.len() == text.len() {
                format!("the `TEXT` \"{shown}\"")
            } else {
                format!("the `TEXT` \"{shown}…\"")
            }
        }
        nvs_db::SqliteValue::Blob(bytes) => format!("a `BLOB` of {} byte(s)", bytes.len()),
    }
}

pub(super) fn tds_write(
    tds: &mut nvs_db::TdsConn,
    statement: &Statement,
    sending: &[Option<&[u8]>],
    source: Option<&str>,
    watch: QueryWatch,
    named: &str,
) -> Result<(Written, Option<(String, std::time::Duration)>), Fault> {
    let mut answered = tds
        .query(&statement.sql, sending)
        .map_err(|refused| statement_failure(named, &statement.block, source, &refused))?;
    name_span(&mut answered, statement.block.as_text());
    while answered
        .next_row()
        .map_err(|refused| statement_failure(named, &statement.block, source, &refused))?
        .is_some()
    {}

    let written = Written {
        changed: answered.affected(),
        last_id: None,
    };
    let taken = watch.taken(answered.span());
    Ok((written, taken))
}

nvs_runtime::nvs_helper! {
    /// `Core\Db\Connection::queryAs<T>(string $sql, array<mixed> $params):
    /// Db\Rows<T>` — `rule:core-classes/db-statement-members`'s statement over § 18's hydrating result.
    ///
    /// **Arguments 0 to 2 are what the call site wrote as its type argument** —
    /// the class, whether it was written as `array<...>` of one, and an inline
    /// shape's wire contract — not values, and the receiver is argument 3:
    /// `crate::registry::WRITTEN_CLASS_MEMBERS` owns that ABI and this is its
    /// first *instance* member. So the arity here is three more than `query`'s,
    /// which is otherwise the same call.
    ///
    /// **The statement is [`queried_rows`], unchanged**: § 4 gives `query` and
    /// this member one signature and one binding rule, so what differs is the
    /// class the result carries and nothing on the wire. That class goes into
    /// [`ROWS_CLASS_SLOT`] and is read only when a row is handed out, so a
    /// caller that just counts pays for no construction.
    ///
    /// **The construction is [`hydrate`]'s**, reached through [`row_object`]
    /// when `all`, `first` or a `foreach` asks for a row — so this body's own
    /// refusals are the two that are about the *call site* rather than about a
    /// row, and they are raised before the statement goes out.
    fn nvs_core_db_connection_query_as(ctx, args: [7]) {
        // Unreachable from source, exactly as `Core\Json::decodeAs`'s own
        // reading of these two slots is: `nvs_ir::lower` writes the descriptor
        // and the flag out of the type argument at the call site, and a call
        // naming none is `E0442` before any of this runs.
        if args[0].as_class_desc().is_none() {
            return Err(Fault::fatal(format!(
                "internal error: `{QUERY_AS}` was called with no class in argument 0"
            )));
        }
        // Unreachable from source for the same reason and refused by the same
        // `E0442`: slot 1 is the `ConstBool` the lowering emits beside the
        // descriptor, so a call that has one has the other.
        let list = args[1].as_bool().ok_or_else(|| Fault::fatal(format!(
            "internal error: `{QUERY_AS}` was called with no list flag in argument 1"
        )))?;
        // Refused before the statement goes out, because it cannot mean
        // anything downstream: `Core\Json::decodeAs`'s list form is a document
        // that *is* a JSON array, and a result set is already one row per row.
        // A compile-time home would be better and gap 4 says what it waits on.
        if list {
            return Err(Fault::thrown_as(
                ThrownClass::Logic,
                format!(
                    "{QUERY_AS}: `array<...>` is not a type argument this member takes — a result \
                     set is already one `{ROWS_NAME}` of one row each, so write \
                     `queryAs<Person>(…)` and read the list off the result"
                ),
            ));
        }
        // The receiver and the two value parameters, past the block of three
        // `WRITTEN_CLASS_MEMBERS` puts ahead of everything.
        let answered = queried_rows(ctx, &args[3..], "queryAs", QUERY_AS)?;
        // `args[0]` carries no reference — a descriptor rides in the payload
        // half of an otherwise-`null` value — so the slot takes it as it is.
        Ok(crate::instance::build(
            &ROWS,
            [
                Value::array(answered.rows),
                args[0],
                Value::array(answered.columns),
            ],
        ))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Db\Connection::execute(string $sql, array<mixed> $params):
    /// Db\Write` — `rule:core-classes/db-statement-members`'s counting half of the same statement path.
    ///
    /// **The difference from [`nvs_core_db_connection_query`] is what becomes
    /// of the rows, and nothing else.** The statement goes out the same way,
    /// through the same [`statement_of`], because § 4 gives the two members one
    /// signature and one binding rule — so `execute` is not a second, weaker
    /// path a caller could reach a difference through.
    ///
    /// **The rows are read to the end and dropped**, which is not a waste: a
    /// PostgreSQL statement is a stream either way, ending it is what returns
    /// the connection to idle, and `lastId` is taken as each row goes past
    /// ([`nvs_db::PgRows::last_id`]) — so the drain is also what finds it.
    /// Nothing is decoded, so an `insert … returning` costs no column work at
    /// all here, which is the one thing this member does differently with the
    /// same stream `query` reads.
    ///
    /// **Both counts come off `CommandComplete`**, so neither exists until that
    /// stream has ended, and the pair is § 4's own: `affected` folds a command
    /// whose tag carries no count at all — a `create table` — to `0`, and
    /// `changed` keeps the absence, which is the only thing the two say
    /// differently on this driver.
    fn nvs_core_db_connection_execute(ctx, args: [4]) {
        let statement = statement_of(ctx, args, "execute", EXECUTE)?;
        // As `query`'s, and read at the same point for the same reason.
        let deadline = statement_deadline(args, EXECUTE)?;
        // As `query`, and for the reason given there: a refusal names the
        // caller's own text rather than the rewrite of it that reached the wire.
        let source = args[1].as_text();
        let sending: Vec<Option<&[u8]>> = statement.binds.wire();
        // As `query`, and for the reason [`QueryWatch`] gives.
        let watch = QueryWatch::of(ctx, &statement.block);
        // Branched as [`queried_rows`] is, and the arms hand the event back for
        // the same borrow reason: the rows hold the connection, which holds the
        // context the span is filed on.
        let (written, taken) = match bound_connection(ctx, statement.key, EXECUTE, deadline)? {
            nvs_db::Connection::Postgres(postgres) => {
                postgres_write(postgres, &statement, &sending, source, watch, EXECUTE)?
            }
            nvs_db::Connection::MySql(mysql) => {
                mysql_write(Framed::MySql(mysql), &statement, &sending, source, watch, EXECUTE)?
            }
            nvs_db::Connection::MariaDb(maria) => mysql_write(
                Framed::MariaDb(maria),
                &statement,
                &sending,
                source,
                watch,
                EXECUTE,
            )?,
            nvs_db::Connection::SqlServer(tds) => {
                tds_write(tds, &statement, &sending, source, watch, EXECUTE)?
            }
            // As `query`'s: this arm's parameters are [`Binds::sqlite`]'s and
            // `sending` is not what it reads.
            nvs_db::Connection::Sqlite(sqlite) => {
                sqlite_write(sqlite, &statement, source, watch, EXECUTE)?
            }
        };
        watch.file(ctx, taken);
        Ok(crate::instance::build(
            &WRITE,
            [
                Value::uint(written.changed.unwrap_or(0)),
                written.changed.map_or_else(Value::null, Value::uint),
                written.last_id.map_or_else(Value::null, Value::uint),
            ],
        ))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Db\Connection::executeMany(string $sql, array<array<mixed>> $sets):
    /// uint` — `rule:core-classes/db-statement-members`'s batch, and § 1's reason for having no `Statement`
    /// object at all.
    ///
    /// **This is the member a `prepare` handle would have existed for.** § 1
    /// removes the handle because the per-connection cache already buys what it
    /// bought, and names the batch as the one case that would otherwise still
    /// want one — so a loop of `execute` calls and this member differ in round
    /// trips and in nothing else a caller can see.
    ///
    /// **Every set is bound by [`statement_of`], and they must agree**; that
    /// rule and why it is checked on the rewritten text rather than on a count
    /// are [`batch_of`]'s.
    ///
    /// **The batch opens and files `rule:observability/a-query-is-a-trace-event`'s span itself**, which is the
    /// one thing it does that `execute` leaves to the driver.
    /// Every driver's `execute_many` answers with a count and lends no row
    /// handle out, so there is no handle a driver-built span could ride on and
    /// be read off afterwards — the span is opened here, around the same round
    /// trips, and finished with the batch's sum as its affected count and no
    /// rows at all, which is what a batch contributes to a trace.
    /// `nvs_db::QuerySpan` owns the field set and why a bound value is not in
    /// it, and `Ctx::record_query` owns why what crosses is a rendering.
    ///
    /// **What it answers is a `uint` and not a [`WRITE`].** § 4 gives the batch
    /// a sum, because `changed` and `lastId` would each have to pick one
    /// execution to be about — and the sum is what a caller writing the loop by
    /// hand would have accumulated anyway. The batch is also **not** a
    /// transaction: a failure part way through leaves the writes before it
    /// standing and the sets after it still attempted, and `transaction` is the
    /// member that asks for all or nothing. The drivers reach that one
    /// observable from opposite ends of their protocols — a `Sync` per execution
    /// on PostgreSQL, a command per execution on MySQL, an `sp_execute` per set
    /// against one `sp_prepexec` on SQL Server — and each `execute_many`'s own
    /// doc argues its half.
    fn nvs_core_db_connection_execute_many(ctx, args: [4]) {
        let batch = batch_of(ctx, args, "executeMany", EXECUTE_MANY)?;
        // § 4's `timeout` bounds the *batch*, which is what a caller asked to
        // bound: the sets are one statement run N times over one connection, and
        // there is no per-set answer for a per-set clock to belong to.
        let deadline = statement_deadline(args, EXECUTE_MANY)?;
        // Two hops rather than one: the driver borrows each set as a slice, so
        // the per-set `Vec` has to outlive the slice taken of it.
        let sending: Vec<Vec<Option<&[u8]>>> = batch.binds.iter().map(Binds::wire).collect();
        let sets: Vec<&[Option<&[u8]>]> = sending.iter().map(Vec::as_slice).collect();

        // As `execute`, and for the reason [`QueryWatch`] gives.
        let watch = QueryWatch::of(ctx, &batch.block);
        let connection = bound_connection(ctx, batch.key, EXECUTE_MANY, deadline)?;
        let driver = connection.driver();
        // § 11's span, opened where the driver opens `execute`'s: after the
        // connection is in hand, so the duration is the statement's wait and
        // not the pool's. It carries the rewritten text, which is what reaches
        // the wire and what a driver-opened span would have been handed, and the
        // connection's own driver, so a trace reads the batch beside the
        // statements around it rather than as PostgreSQL's whatever ran it.
        let mut span = nvs_db::QuerySpan::opened(driver, &batch.sql);
        // One statement over many parameter sets, so the batch has exactly the
        // one text to name and it is the caller's, as `execute`'s is. What the
        // two drivers do with the sets differs and what a caller observes does
        // not — `nvs_db::mysql`'s own `execute_many` is where that is argued.
        let written = match connection {
            nvs_db::Connection::Postgres(postgres) => postgres.execute_many(&batch.sql, &sets),
            nvs_db::Connection::MySql(mysql) => mysql.execute_many(&batch.sql, &sets),
            // Not through [`Framed`]: that seam exists to stop a *body* being
            // written twice, and this arm is the whole body. § 4 runs N
            // executions on both drivers and `nvs_db::mysql::execute_many` is
            // the one that runs them.
            nvs_db::Connection::MariaDb(maria) => maria.execute_many(&batch.sql, &sets),
            // The one driver whose batch costs no request shape of its own:
            // § 1's cache makes the first set an `sp_prepexec` and every set
            // after it an `sp_execute`, so `nvs_db::tds::execute_many` is the
            // loop and nothing below it changed for this member.
            nvs_db::Connection::SqlServer(tds) => tds.execute_many(&batch.sql, &sets),
            // The one driver whose whole loop is a single handoff off the core
            // rather than N round trips, so its sets are handed over owned and
            // all at once — `nvs_db::SqliteConn::execute_many` is where that is
            // argued, and § 4's "a batch is not a transaction" holds there as
            // it does on the other four.
            nvs_db::Connection::Sqlite(sqlite) => sqlite.execute_many(
                &batch.sql,
                batch.binds.iter().map(Binds::sqlite).collect(),
            ),
        }
        .map_err(|refused| {
            statement_failure(EXECUTE_MANY, &batch.block, args[1].as_text(), &refused)
        })?;
        // § 4's sum is the batch's affected count, and the span's rows stay at
        // zero: nothing was handed back, and a batch that inserted a thousand
        // rows reporting a thousand rows *returned* would read as a select.
        span.finished(Some(written));
        file_span(ctx, watch, &batch.block, span);
        Ok(Value::uint(written))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Drops the one reference [`sqlite_column_value`] handed back, which every
    /// case below owns exactly as a member's caller would.
    fn released(value: Value) {
        #[expect(
            unsafe_code,
            reason = "the reference released here is the one this frame was \
                      handed, and nothing else holds the instance"
        )]
        unsafe {
            value.release();
        }
    }

    /// [ADR 0067 § 4](/docs/decisions/0067.md)'s `{timeout?: Duration}`, over
    /// the three links that make it a bound rather than an option that parses:
    /// the duration becomes an instant, the instant reaches the socket, and a
    /// socket that gave up on it becomes a throw.
    ///
    /// **The middle link is the claim, and it is asserted on a real socket.**
    /// `nvs_host::net::Deadline` is the trait every driver's `Wire::set_deadline`
    /// forwards through, and `nvs_db::Connection::set_deadline` — what
    /// [`bound_connection`] calls — is five arms of exactly that call. This crate
    /// cannot build an `nvs_db::Connection` at all (the playbook's own bullet
    /// owns why), so what is pinned here is the seam the member files the
    /// deadline *on*, read back off the stream and then made to expire. The
    /// live-server half is `tools/db-matrix.py`'s.
    ///
    /// The read runs off a core, where the bound is the poll's own timeout, so
    /// this needs neither a scheduler nor a reactor — `nvs_host::net`'s
    /// `a_read_off_a_core_is_bounded_by_the_same_deadline` is the same shape one
    /// crate down.
    // covers: Core\Db\Connection::execute
    #[test]
    fn a_statement_timeout_reaches_the_socket_and_throws_on_expiry() {
        // 1. § 18's option becomes the instant the statement must answer by.
        let named = crate::time::duration_of(50_000_000);
        let asked = [Value::null(), Value::null(), Value::null(), named];
        let deadline = statement_deadline(&asked, QUERY)
            .expect("a positive `timeout` is a deadline")
            .expect("a `timeout` that was written is not an absent one");
        assert!(
            deadline > std::time::Instant::now(),
            "a 50ms `timeout` produced a deadline that had already passed"
        );
        released(named);

        // The absence is the absence, so a statement that named none lifts
        // whatever the statement before it on this connection named.
        let silent = [Value::null(), Value::null(), Value::null(), Value::null()];
        assert!(
            statement_deadline(&silent, QUERY)
                .expect("an omitted `timeout` is not a refusal")
                .is_none(),
            "an omitted `timeout` produced a bound"
        );

        // And a zero is refused rather than read as "unbounded", which is the
        // spelling `rule:http-server/no-spelling-for-an-unbounded-wait` does not have.
        let zero = crate::time::duration_of(0);
        let refused = [Value::null(), Value::null(), Value::null(), zero];
        assert!(
            matches!(statement_deadline(&refused, QUERY), Err(Fault::Thrown(..))),
            "a zero `timeout` was accepted as a bound"
        );
        released(zero);

        // 2. The instant reaches the socket, through the trait the drivers
        //    forward it through.
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("the OS refused a port");
        let address = listener
            .local_addr()
            .expect("a bound listener has an address");
        let client = std::net::TcpStream::connect(address).expect("the listener refused a connect");
        // Held for the length of the case: a peer that has gone leaves the read
        // below answering end-of-stream instead of waiting for its deadline.
        let peer = listener.accept().expect("the connect was never accepted");
        let mut socket = nvs_host::net::NvsTcp::from_std(client)
            .expect("the platform refused a non-blocking socket");
        let expiring = std::time::Instant::now() + std::time::Duration::from_millis(20);
        nvs_host::net::Deadline::set_deadline(&mut socket, Some(expiring));
        assert_eq!(
            nvs_host::net::Deadline::deadline(&socket),
            Some(expiring),
            "the deadline did not land on the stream that waits"
        );

        // 3. A read past it gives up, and this module's own mapping turns that
        //    into § 10's `IOError` — the connection was abandoned part way
        //    through a message and is spent, which is what that class says.
        let mut buffer = [0_u8; 8];
        let started = std::time::Instant::now();
        let expired = std::io::Read::read(&mut socket, &mut buffer)
            .expect_err("a read behind a deadline, from a peer that says nothing, answered");
        assert_eq!(expired.kind(), std::io::ErrorKind::TimedOut);
        assert!(
            started.elapsed() >= std::time::Duration::from_millis(20),
            "the read gave up before the deadline it was given"
        );
        let thrown = statement_failure(QUERY, &Value::null(), Some("select 1"), &expired);
        assert!(
            matches!(thrown, Fault::Thrown(ThrownClass::Io, _)),
            "a statement that ran out of time did not throw an `IOError`"
        );
        drop(peer);
    }

    /// [ADR 0067 § 4](/docs/decisions/0067.md)'s buffered read, which is the
    /// whole difference between `query` and `stream`: every row is in memory
    /// before the call returns, and the connection is free from that moment.
    ///
    /// **Asserted on a real engine**, because a claim about buffering that
    /// never stepped a statement would be a claim about this crate's guess at
    /// one. [`sqlite_rows`] is the arm [`queried_rows`] takes for this driver,
    /// and what it answers is exactly the two arrays a `Core\Db\Rows` is built
    /// from — so the member's own shape is what is read here, one layer under
    /// the receiver this crate cannot build (the playbook's bullet owns why).
    ///
    /// Three claims, because the first two alone look right: the result holds
    /// every row before anything walks it, the descriptions belong to the
    /// statement rather than to a row, and the connection runs the next
    /// statement with the first result still in hand. A member that streamed
    /// would pass the first two and fail the third.
    ///
    /// The `.nvst` half runs the same read through a program, where `count`
    /// and the walk are reached the way a caller reaches them.
    // covers: Core\Db\Connection::query
    #[test]
    fn a_read_holds_every_row_before_it_returns_and_frees_the_connection() {
        let block = nvs_config::tree::Database {
            driver: Some(String::from("sqlite")),
            path: Some(String::from(":memory:")),
            ..nvs_config::tree::Database::default()
        };
        let target = nvs_db::SqliteTarget::resolve(&block).expect("a `sqlite` block resolves");
        let mut conn = nvs_db::sqlite::open(&target).expect("an in-memory database opens");
        conn.query(
            "create table notes (id integer primary key, text text not null)",
            Vec::new(),
        )
        .expect("the schema is applied");
        conn.query(
            "insert into notes (text) values ('buy milk'), ('call Ana'), ('book train')",
            Vec::new(),
        )
        .expect("the rows are written");

        // The statement a bound `query` reaches this arm with: § 5's rewrite has
        // already run, so the text is in the driver's own placeholder spelling
        // and the binds are in the *statement's* order.
        let ctx = nvs_runtime::Ctx::buffered();
        let statement = crate::db::bind::Statement {
            key: 0,
            block: Value::null(),
            sql: String::from("select id, text from notes where id >= ? order by id"),
            binds: crate::db::bind::Binds::Sqlite(vec![nvs_db::SqliteValue::Int(2)]),
        };
        let (answered, _) = sqlite_rows(
            &mut conn,
            &statement,
            None,
            crate::db::span::QueryWatch::named(&ctx, None),
            "query",
        )
        .expect("a `select` over an open connection answers its rows");

        // 1. Every row is already in the array. Nothing has walked the result,
        //    and the count is the server's rather than a guess at it.
        assert_eq!(
            answered.rows.count(),
            2,
            "a buffered read answered a result that does not hold every row"
        );

        // 2. One description per column the *statement* named, and not per cell
        //    of a row — `id` and `text`, whatever the rows under them hold.
        assert_eq!(
            answered.columns.count(),
            2,
            "the descriptions do not describe the statement's own columns"
        );

        // 3. And the connection is free with the result still in hand, which is
        //    what `stream` cannot say: its walk holds the connection until the
        //    rows are gone.
        conn.query("insert into notes (text) values ('pay rent')", Vec::new())
            .expect("the connection is free the moment the read returns");
        assert_eq!(
            answered.rows.count(),
            2,
            "the result changed under a statement that ran after it"
        );
    }

    /// **`Core\Db\Write::changed` is the count the server itself reported, and
    /// on SQLite every statement reports one.** § 4's absent case is a command
    /// tag that carries no count, and this driver sends no tag at all: what
    /// [`sqlite_write`] can read is a number for every statement kind, so the
    /// `?uint` is present throughout and holds whatever `affected` folds.
    ///
    /// **Asserted after a statement that really changed rows**, which is
    /// `nvs_db::sqlite`'s own reason: `sqlite3_changes` describes the last
    /// data-changing statement on the connection, so a count read on a quiet
    /// connection passes whether this statement's own count is reported or an
    /// earlier statement's is.
    ///
    /// The `.nvst` half runs the same statements through a program, where the
    /// `?uint` is unwrapped the way a caller unwraps it.
    // covers: Core\Db\Write::changed
    #[test]
    fn a_sqlite_write_reports_a_count_for_every_statement_kind() {
        let block = nvs_config::tree::Database {
            driver: Some(String::from("sqlite")),
            path: Some(String::from(":memory:")),
            ..nvs_config::tree::Database::default()
        };
        let target = nvs_db::SqliteTarget::resolve(&block).expect("a `sqlite` block resolves");
        let mut conn = nvs_db::sqlite::open(&target).expect("an in-memory database opens");
        let ctx = nvs_runtime::Ctx::buffered();

        // The statement a bound `execute` reaches this arm with, for the binds
        // the sibling case above gives: nothing below binds a value, so every
        // count is the statement's own rather than an argument's.
        let mut changed = |sql: &str| -> Option<u64> {
            let statement = crate::db::bind::Statement {
                key: 0,
                block: Value::null(),
                sql: String::from(sql),
                binds: crate::db::bind::Binds::Sqlite(Vec::new()),
            };
            let (written, _) = sqlite_write(
                &mut conn,
                &statement,
                None,
                crate::db::span::QueryWatch::named(&ctx, None),
                EXECUTE,
            )
            .expect("a statement over an open connection answers its counts");
            written.changed
        };

        // 1. A schema and three rows: the write's count is the rows it changed.
        assert_eq!(
            changed("create table t (id integer primary key, name text not null)"),
            Some(0)
        );
        assert_eq!(
            changed("insert into t (name) values ('ada'), ('grace'), ('alan')"),
            Some(3)
        );

        // 2. Three rows are now behind every statement below, so a count that
        //    is not the statement's own has something to report as this one's.
        for quiet in [
            "create table u (id integer primary key)",
            "drop table u",
            "update t set name = 'nobody' where id = 99",
        ] {
            assert_eq!(
                changed(quiet),
                Some(0),
                "`{quiet}` reported no count, or reported the insert's rows"
            );
        }

        // 3. And the driver goes on counting after them.
        assert_eq!(changed("delete from t"), Some(3));
    }

    /// **`Core\Db\Write::lastId` is the key of the row this statement inserted,
    /// and `null` where there is no key a `uint` holds.** One statement writing
    /// several rows answers the last row's key, which is all SQLite reports and
    /// all § 4 promises; a row written with a negative key has none to report,
    /// since [`sqlite_write`]'s `u64::try_from` is where a `?uint` stops.
    ///
    /// **What a statement that inserted nothing answers is not asserted here**,
    /// because it is wrong: `sqlite3_last_insert_rowid` belongs to the
    /// connection, so an `update` after an insert reports the insert's key.
    /// `nvs_db::sqlite`'s `# Known gaps` carries it, and the example under
    /// `docs/examples/core/Db-Write/lastId` is the proof that fails on it.
    ///
    /// The `.nvst` half runs the same statements through a program, where the
    /// `?uint` is unwrapped the way a caller unwraps it.
    // covers: Core\Db\Write::lastId
    #[test]
    fn a_sqlite_write_carries_the_key_of_the_row_it_inserted() {
        let block = nvs_config::tree::Database {
            driver: Some(String::from("sqlite")),
            path: Some(String::from(":memory:")),
            ..nvs_config::tree::Database::default()
        };
        let target = nvs_db::SqliteTarget::resolve(&block).expect("a `sqlite` block resolves");
        let mut conn = nvs_db::sqlite::open(&target).expect("an in-memory database opens");
        let ctx = nvs_runtime::Ctx::buffered();

        let mut last_id = |sql: &str| -> Option<u64> {
            let statement = crate::db::bind::Statement {
                key: 0,
                block: Value::null(),
                sql: String::from(sql),
                binds: crate::db::bind::Binds::Sqlite(Vec::new()),
            };
            let (written, _) = sqlite_write(
                &mut conn,
                &statement,
                None,
                crate::db::span::QueryWatch::named(&ctx, None),
                EXECUTE,
            )
            .expect("a statement over an open connection answers its key");
            written.last_id
        };

        // 1. A schema hands back no key, and the first row written is row 1.
        assert_eq!(
            last_id("create table t (id integer primary key, name text not null)"),
            None
        );
        assert_eq!(last_id("insert into t (name) values ('ada')"), Some(1));

        // 2. Three rows in one statement, and the key is the last row's.
        assert_eq!(
            last_id("insert into t (name) values ('grace'), ('alan'), ('kay')"),
            Some(4)
        );

        // 3. A key below zero is no `uint`, so the write carries no key at all
        //    rather than a number that wrapped around.
        assert_eq!(
            last_id("insert into t (id, name) values (-5, 'below zero')"),
            None
        );
    }

    /// The rewritten text of one set, as [`batch_of`] reads it before comparing
    /// it with the first set's — a statement binding one `inList` of `ids`.
    ///
    /// The set is released here, so the caller owes nothing: what it is handed
    /// back is the text alone, which is the whole of what the agreement is
    /// decided on.
    fn batched_sql(dialect: nvs_db::Dialect, encode: Encoder, ids: &[i64]) -> String {
        let mut list = NvsArray::new();
        for id in ids {
            list.append(Value::int(*id));
        }
        let mut set = NvsArray::new();
        set.append(crate::instance::build(&IN_LIST, [Value::array(list)]));
        let set = Value::array(set);
        let sql = Value::str(NvsStr::new(b"select id from notes where id in (?)"));
        let statement = statement_in(
            dialect,
            encode,
            0,
            Value::null(),
            &[Value::null(), sql, set],
            "executeMany",
        )
        .expect("a set binding one `inList` rewrites");
        released(set);
        statement.sql
    }

    /// [ADR 0067 § 4](/docs/decisions/0067.md)'s batch agreement, asserted at
    /// the seam [`batch_of`] decides it on: every set of one `executeMany`
    /// rewrites to the same statement text, or the sets are two statements and
    /// the batch is refused.
    ///
    /// **The comparison is on the rewritten text and not on a count of
    /// `$params`**, which is the whole of the rule. `rule:core-classes/db-parameters`'s `inList`
    /// expands to one marker per element, so the two sets below each hold one
    /// argument and bind a different number of values — a batch comparing
    /// lengths would accept the pair and send the second set against the first
    /// set's statement.
    ///
    /// [`batch_of`] itself is one layer above what this crate can reach: it
    /// reads the driver off a filed connection, and no `nvs_db::Connection` can
    /// be built in a `-p nvs-stdlib` test at all, which the playbook's own
    /// bullet owns. [`rendering_for`] is that read's pure half, so what runs
    /// below is the rewrite the member performs, set by set.
    // covers: Core\Db\Connection::executeMany
    #[test]
    fn a_batch_is_one_statement_only_where_every_set_rewrites_the_same_way() {
        let (dialect, encode) = rendering_for(nvs_db::Driver::Sqlite);

        let two = batched_sql(dialect, encode, &[1, 2]);
        let other_two = batched_sql(dialect, encode, &[7, 8]);
        let three = batched_sql(dialect, encode, &[1, 2, 3]);

        // 1. Two sets of the same width are one statement, whatever they bind:
        //    the values differ and the text the batch agreed on does not.
        assert_eq!(
            two, other_two,
            "two sets of the same width rewrote to different statements, so a batch of them \
             would be refused for binding the values it was given"
        );

        // 2. And one element wider is a different statement, which is what the
        //    member throws a `LogicError` for rather than sending.
        assert_ne!(
            two, three,
            "a wider `inList` rewrote to the same statement, so a batch would send a set \
             against a text with a marker too few"
        );
    }

    /// One row of [`one_sqlite_cell_reads_as_five_things_under_five_declarations`]'s
    /// sweep: a declared type, a cell every row of the sweep shares the storage
    /// class of, and the question the answer has to say yes to.
    type Declared = (&'static str, &'static str, fn(Value) -> bool);

    /// One SQLite result column, declared as `declared` — the half of
    /// [`nvs_db::SqliteColumn`] every case below varies.
    fn sqlite_column(declared: &str) -> nvs_db::SqliteColumn {
        nvs_db::SqliteColumn {
            name: "c".to_owned(),
            declared: Some(declared.to_owned()),
        }
    }

    /// [ADR 0067 § 9](/docs/decisions/0067.md)'s SQLite rule, asserted
    /// as the **agreement** it is: one cell, five declarations, five different
    /// answers.
    ///
    /// The point is that the storage class is constant across the sweep and the
    /// answer is not, which is the whole of "mapping keys off the *declared*
    /// column type" — a decoder that read the cell instead would answer `TEXT`
    /// five times and still look right on any one line of it.
    #[test]
    fn one_sqlite_cell_reads_as_five_things_under_five_declarations() {
        let text = |held: &str| nvs_db::SqliteValue::Text(held.to_owned());
        let cases: [Declared; 5] = [
            ("date", "2026-09-03", |value| {
                crate::instance::is_instance(value, &crate::time::DATE)
            }),
            ("datetime", "2026-09-03 10:11:12", |value| {
                crate::instance::is_instance(value, &crate::time::DATETIME)
            }),
            ("time", "10:11:12", |value| {
                crate::instance::is_instance(value, &crate::time::TIME_OF_DAY)
            }),
            ("uuid", "3f2504e0-4f89-41d3-9a0c-0305e82c3301", |value| {
                crate::instance::is_instance(value, &crate::uuid::CLASS)
            }),
            ("text", "2026-09-03", |value| value.tag() == Some(Tag::Str)),
        ];
        for (declared, held, is_wanted) in cases {
            let read = sqlite_column_value(&sqlite_column(declared), text(held), 0, "query")
                .unwrap_or_else(|_| panic!("`{held}` is what a `{declared}` column holds"));
            assert!(
                is_wanted(read),
                "a `{declared}` column holding \"{held}\" read back as the wrong type"
            );
            released(read);
        }
    }

    /// § 9's other half — "throws on a value that does not parse" — asserted on
    /// **both sides of the bound**: the same declaration, one cell it describes
    /// and one it does not.
    ///
    /// `20260903` is the case the decoder is written around: it is a perfectly
    /// good `INTEGER` and a plausible spelling of the date beside it, and the
    /// column's own word is the only thing that says it is not one. A decoder
    /// that guessed would answer a date for both lines.
    #[test]
    fn a_sqlite_cell_the_declaration_does_not_describe_throws() {
        let column = sqlite_column("date");
        let accepted = sqlite_column_value(
            &column,
            nvs_db::SqliteValue::Text("2026-09-03".to_owned()),
            0,
            "query",
        )
        .expect("a rendered date is what a `date` column holds");
        assert!(crate::instance::is_instance(accepted, &crate::time::DATE));
        released(accepted);

        for cell in [
            nvs_db::SqliteValue::Int(20_260_903),
            nvs_db::SqliteValue::Text("the third".to_owned()),
        ] {
            let refused = sqlite_column_value(&column, cell, 0, "query")
                .expect_err("neither is a date, whatever it would be in another column");
            let Fault::Thrown(_, why) = refused else {
                panic!("§ 9's refusal is a throw a program can catch");
            };
            assert!(
                why.contains("is declared `date`"),
                "the refusal names the declaration and not the cell: {why}"
            );
        }
    }

    /// A `DECIMAL` column reads back through every storage class SQLite's
    /// NUMERIC affinity can leave in one, which is what makes the column usable
    /// at all — [`sqlite_column_value`]'s own doc argues the `REAL` arm.
    ///
    /// A `BOOLEAN` is the same shape with § 6's bound on it: `0` and `1` cross
    /// and a stored `7` does not, which is [`requested_bool`]'s rule reached
    /// from the map rather than from a request.
    #[test]
    fn a_sqlite_decimal_crosses_from_three_classes_and_a_bool_from_two() {
        let exact = |value: Value| {
            value
                .as_decimal()
                .map(|held| held.to_string())
                .expect("the column is declared `decimal`")
        };
        let column = sqlite_column("decimal(10,2)");
        for (cell, rendered) in [
            (nvs_db::SqliteValue::Text("1.25".to_owned()), "1.25"),
            (nvs_db::SqliteValue::Int(3), "3"),
            (nvs_db::SqliteValue::Real(1.25), "1.25"),
        ] {
            let read = sqlite_column_value(&column, cell, 0, "query").expect("a number crosses");
            assert_eq!(exact(read), rendered);
        }
        assert!(
            sqlite_column_value(
                &column,
                nvs_db::SqliteValue::Text("not a number".to_owned()),
                0,
                "query"
            )
            .is_err()
        );

        let flag = sqlite_column("boolean");
        for (held, want) in [(0, false), (1, true)] {
            let read = sqlite_column_value(&flag, nvs_db::SqliteValue::Int(held), 0, "query")
                .expect("`0` and `1` are the whole of what a flag column holds");
            assert_eq!(read.as_bool(), Some(want));
        }
        assert!(
            sqlite_column_value(&flag, nvs_db::SqliteValue::Int(7), 0, "query").is_err(),
            "a stored `7` throws rather than reading as PHP's `true`"
        );
    }
}
