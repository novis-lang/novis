//! A [`Connection`] driven straight from an operator's command, with no request
//! behind it: `rule:core-classes/schema-introspection`'s two catalog reads, and
//! one statement at a time.
//!
//! [`crate::catalog`] is sans-io — a statement is text and a row is a struct —
//! and something has to put the two together over a live socket. Inside a
//! request that is `nvs-stdlib`, which runs a catalog read through the same five
//! arms every other `Core\Db` statement goes through so that a trace event, a
//! slow-query threshold and a pool's busy state all apply to it. `nvs schema`
//! has none of those: it holds no `Ctx`, serves no request, and opens the block
//! it proved for the length of one command. This module is that second caller's
//! path, and the reason it is here rather than in `nvs-cli` is
//! `rule:core-classes/db-crate-boundary` — the wire is this crate's, so a
//! command that reads a catalog asks for rows and never learns what a `PgScalar`
//! or a TDS token is.
//!
//! ## Every cell is read as text
//!
//! A catalog row carries names, type spellings, ordinals and flags, and nothing
//! else — so one reading serves all five backends: each cell becomes the text
//! the server would print for it, and [`ColumnRow`] and [`IndexRow`] are built
//! off that. A flag arrives as `1` or `0`, which is what four of the five
//! catalogs answer already, and an integer arrives as its digits. That is why
//! this module names one scalar variant per driver and no type map at all: the
//! type map is `rule:core-classes/db-column-types`' and belongs to the layer
//! that hands values to a program.
//!
//! **Cells are read by position and not by name.** The select list of every
//! catalog query is [`catalog::Read::row`]'s in that order, which the queries
//! themselves guarantee — and a column *name* is the one thing a catalog row
//! cannot be keyed by safely, since SQLite's column read selects `m.name` beside
//! `p.name` and a by-name reader collapses the two.
//!
//! ## What it costs
//!
//! Two statements per introspection and one owned `String` per cell, both
//! released with the command. A catalog holds tens of tables and a table tens of
//! columns, so the whole answer is kilobytes; nothing here is on a request path,
//! and no reading is cached because a command runs once.

use std::io::{self, Read, Write};

use crate::catalog::{self, ColumnRow, IndexRow};
use crate::conn::Connection;
use crate::schema::{Schema, SchemaError};
use crate::sql::Dialect;
use crate::tds::{TdsRows, TdsScalar};
use crate::{MySqlRows, MySqlScalar, PgRows, PgScalar, SqliteRows, SqliteValue};

/// Every row of one read, each cell as its text and `None` for SQL `NULL`.
type Cells = Vec<Vec<Option<String>>>;

/// What an introspection can answer instead of a schema.
///
/// **Three failures rather than one**, because an operator does something
/// different about each: a server that refused the query, a catalog whose rows
/// stopped matching what this module asks for, and a database holding a type
/// `rule:core-classes/schema-vocabulary-is-closed` has no name for. Only the
/// last is a limit of the design rather than of the deployment.
#[derive(Debug)]
pub enum ReadError {
    /// The server refused a catalog query, or the wire failed under it.
    Wire(io::Error),
    /// A row that does not carry what the query asked the server for, which
    /// means the query and this reader have stopped agreeing about a driver.
    Row(String),
    /// The database holds something the closed vocabulary cannot name, so no
    /// plan against it would be total.
    Vocabulary(SchemaError),
}

impl std::fmt::Display for ReadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ReadError::Wire(err) => write!(f, "the catalog read failed: {err}"),
            ReadError::Row(what) => write!(f, "{what}"),
            ReadError::Vocabulary(why) => write!(
                f,
                "this database holds something the schema vocabulary cannot name, so no plan \
                 against it would be total: {why}"
            ),
        }
    }
}

impl std::error::Error for ReadError {}

impl From<io::Error> for ReadError {
    fn from(err: io::Error) -> ReadError {
        ReadError::Wire(err)
    }
}

/// `rule:core-classes/schema-introspection`'s live read: the database behind
/// `conn`, as the schema value a diff compares against.
///
/// Two statements and no parser, which is that rule's whole content — the
/// catalog is structured tables, and [`catalog::assemble`] is the one place rows
/// become a value, through the same builders a declared schema goes through.
///
/// # Errors
///
/// [`ReadError`]'s three, in the order this walks them.
pub fn schema_of(conn: &mut Connection) -> Result<Schema, ReadError> {
    let dialect = Dialect::of(conn.driver());

    let read = catalog::Read::Columns;
    let cells = drive(conn, catalog::query(read, dialect), read.arity())?;
    let mut columns = Vec::with_capacity(cells.len());
    for (index, row) in cells.iter().enumerate() {
        columns.push(ColumnRow {
            table: text_at(row, 0).ok_or_else(|| short(read, index))?,
            column: text_at(row, 1).ok_or_else(|| short(read, index))?,
            ordinal: int_at(row, 2).ok_or_else(|| short(read, index))?,
            ty: text_at(row, 3).ok_or_else(|| short(read, index))?,
            nullable: flag_at(row, 4),
            default: text_at(row, 5),
            identity: flag_at(row, 6),
        });
    }

    let read = catalog::Read::Indexes;
    let cells = drive(conn, catalog::query(read, dialect), read.arity())?;
    let mut indexes = Vec::with_capacity(cells.len());
    for (index, row) in cells.iter().enumerate() {
        indexes.push(IndexRow {
            table: text_at(row, 0).ok_or_else(|| short(read, index))?,
            index: text_at(row, 1).ok_or_else(|| short(read, index))?,
            column: text_at(row, 2).ok_or_else(|| short(read, index))?,
            ordinal: int_at(row, 3).ok_or_else(|| short(read, index))?,
            unique: flag_at(row, 4),
            primary: flag_at(row, 5),
        });
    }

    catalog::assemble(&columns, &indexes, dialect).map_err(ReadError::Vocabulary)
}

/// One statement on `conn`, with whatever it answered drained.
///
/// One statement at a time and never one string with several `;` in it:
/// `rule:core-classes/db-one-api`'s driver takes a statement, and a
/// multi-statement text is what § 10 of that rule's record refuses on a literal
/// query. DDL answers no rows, so the drain is what matters — a connection is
/// usable only at a message boundary, and an abandoned result is not one.
///
/// # Errors
///
/// The server's own refusal, or the wire failure under it.
pub fn run(conn: &mut Connection, sql: &str) -> io::Result<()> {
    drive(conn, sql, 0).map(|_| ())
}

/// One statement, and the first `arity` cells of every row it answered.
///
/// **The one `match` over the five drivers**, per
/// `rule:core-classes/db-drivers-are-an-enum`: the arms differ in which decoder
/// names a cell, and everything either caller does with the answer is written
/// once above.
fn drive(conn: &mut Connection, sql: &str, arity: usize) -> io::Result<Cells> {
    match conn {
        Connection::Postgres(postgres) => postgres_cells(&mut postgres.query(sql, &[])?, arity),
        Connection::MySql(mysql) => mysql_cells(&mut mysql.query(sql, &[])?, arity),
        Connection::MariaDb(maria) => mysql_cells(&mut maria.query(sql, &[])?, arity),
        Connection::SqlServer(tds) => tds_cells(&mut tds.query(sql, &[])?, arity),
        Connection::Sqlite(sqlite) => sqlite_cells(&mut sqlite.query(sql, Vec::new())?, arity),
    }
}

/// PostgreSQL's rows, decoded against the row description the portal answered.
fn postgres_cells<S: Read + Write>(rows: &mut PgRows<'_, S>, arity: usize) -> io::Result<Cells> {
    let mut out = Vec::new();
    while let Some(row) = rows.next_row()? {
        let mut cells = Vec::with_capacity(arity);
        for index in 0..arity {
            let body = row.column(index)?;
            let text = match rows.columns().get(index) {
                Some(column) => postgres_text(column.scalar(body)?),
                None => None,
            };
            cells.push(text);
        }
        out.push(cells);
    }
    Ok(out)
}

/// MySQL's and MariaDB's rows, which are one binary protocol and one decoder.
fn mysql_cells<S: Read + Write>(rows: &mut MySqlRows<'_, S>, arity: usize) -> io::Result<Cells> {
    let mut out = Vec::new();
    while let Some(row) = rows.next_row()? {
        let mut cells = Vec::with_capacity(arity);
        for index in 0..arity {
            let text = match (rows.columns().get(index), row.value(index)) {
                (Some(column), Some(value)) => mysql_text(crate::mysql::scalar(column, value)?),
                _ => None,
            };
            cells.push(text);
        }
        out.push(cells);
    }
    Ok(out)
}

/// SQL Server's rows, decoded against the column metadata token.
fn tds_cells<S: Read + Write>(rows: &mut TdsRows<'_, S>, arity: usize) -> io::Result<Cells> {
    let mut out = Vec::new();
    while let Some(row) = rows.next_row()? {
        let mut cells = Vec::with_capacity(arity);
        for index in 0..arity {
            let text = match (rows.columns().get(index), row.column(index)) {
                (Some(column), Some(value)) => tds_text(crate::tds::scalar(column, value)?),
                _ => None,
            };
            cells.push(text);
        }
        out.push(cells);
    }
    Ok(out)
}

/// SQLite's rows, which arrive already owned off the blocking pool.
fn sqlite_cells(rows: &mut SqliteRows<'_>, arity: usize) -> io::Result<Cells> {
    let mut out = Vec::new();
    while let Some(row) = rows.next_row() {
        let mut cells = Vec::with_capacity(arity);
        for index in 0..arity {
            cells.push(row.get(index).and_then(sqlite_text));
        }
        out.push(cells);
    }
    Ok(out)
}

/// One PostgreSQL cell as the text a catalog row carries.
///
/// The four readings below are one rule written four times because the scalar
/// types are four: a catalog answers names, type spellings, ordinals and flags,
/// so text is itself, a number is its digits, and a boolean is the `1`/`0` the
/// other catalogs already answer. Anything else is a column this reader was
/// never asked for, and `None` is what a caller turns into the refusal that
/// names it.
fn postgres_text(scalar: PgScalar<'_>) -> Option<String> {
    match scalar {
        PgScalar::Text(text) => Some(text.into_owned()),
        PgScalar::Int(number) => Some(number.to_string()),
        PgScalar::UInt(number) => Some(number.to_string()),
        PgScalar::Bool(flag) => Some(bit(flag)),
        _ => None,
    }
}

/// One MySQL or MariaDB cell, as [`postgres_text`] reads PostgreSQL's.
fn mysql_text(scalar: MySqlScalar<'_>) -> Option<String> {
    match scalar {
        MySqlScalar::Text(text) => Some(text.to_owned()),
        MySqlScalar::Bytes(bytes) => String::from_utf8(bytes.to_vec()).ok(),
        MySqlScalar::Int(number) => Some(number.to_string()),
        MySqlScalar::UInt(number) => Some(number.to_string()),
        MySqlScalar::Bool(flag) => Some(bit(flag)),
        _ => None,
    }
}

/// One SQL Server cell, as [`postgres_text`] reads PostgreSQL's.
fn tds_text(scalar: TdsScalar<'_>) -> Option<String> {
    match scalar {
        TdsScalar::Text(text) => Some(text.into_owned()),
        TdsScalar::Int(number) => Some(number.to_string()),
        TdsScalar::Bool(flag) => Some(bit(flag)),
        _ => None,
    }
}

/// One SQLite cell, as [`postgres_text`] reads PostgreSQL's.
///
/// There is no boolean storage class, so a `pragma`'s flag is an `INTEGER` and
/// arrives on the number arm — which is exactly what [`flag_at`] reads.
fn sqlite_text(value: &SqliteValue) -> Option<String> {
    match value {
        SqliteValue::Text(text) => Some(text.clone()),
        SqliteValue::Int(number) => Some(number.to_string()),
        _ => None,
    }
}

/// A boolean as the digit every catalog spells one with.
fn bit(flag: bool) -> String {
    if flag { "1".to_owned() } else { "0".to_owned() }
}

/// Cell `at` as text, where the row has one.
fn text_at(row: &[Option<String>], at: usize) -> Option<String> {
    row.get(at).cloned().flatten()
}

/// Cell `at` as an integer — the two `ordinal` positions.
fn int_at(row: &[Option<String>], at: usize) -> Option<i64> {
    text_at(row, at)?.trim().parse().ok()
}

/// Cell `at` as a flag — `nullable`, `identity`, `unique` and `primary`.
///
/// An unreadable cell is `false`, which **over**-reports a difference rather
/// than under-reporting one: a plan gaining a step is visible to whoever reads
/// it, and a plan losing one is not. [`catalog::assemble`]'s own doc argues the
/// same direction.
fn flag_at(row: &[Option<String>], at: usize) -> bool {
    text_at(row, at)
        .is_some_and(|text| matches!(text.trim(), "1" | "t" | "true" | "TRUE" | "YES" | "y" | "Y"))
}

/// The refusal for a row that does not carry what its query asked for.
fn short(read: catalog::Read, index: usize) -> ReadError {
    ReadError::Row(format!(
        "the server's catalog answered a row this read cannot use — row {index} of the {} read is \
         missing one of the {} columns the query asked it for, which means the catalog query and \
         this reader have stopped agreeing about a driver",
        match read {
            catalog::Read::Columns => "column",
            catalog::Read::Indexes => "index",
        },
        read.arity()
    ))
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::sync::{Arc, Mutex};

    use super::{Connection, Dialect, run, schema_of};
    use crate::conn::{SqliteConn, State};
    use crate::schema::{Column, ScalarType, Schema, Table};

    /// A connection with no server behind it, as [`crate::conn`]'s own tests
    /// build one: SQLite is the variant this crate can always open, because it
    /// has no wire for a unit test to have to script.
    fn in_memory() -> Connection {
        Connection::Sqlite(SqliteConn {
            handle: Arc::new(Mutex::new(
                rusqlite::Connection::open_in_memory().expect("an in-memory database opens"),
            )),
            state: Cell::new(State::Idle),
            depth: Cell::new(0),
            time_zone: 0,
            reading: None,
        })
    }

    /// One column, by the spelling the array form writes.
    fn column(name: &str, spelling: &str) -> Column {
        Column::new(
            name,
            ScalarType::from_spelling(spelling).expect("the vocabulary spells this type"),
        )
        .expect("a bare identifier and a spelled type build a column")
    }

    /// `rule:core-classes/schema-converges`' acceptance property, over the path
    /// a command drives rather than the one a request drives: apply a schema
    /// value through [`run`], read it back through [`schema_of`], and the plan
    /// between them is empty.
    ///
    /// The two entry points are asserted together on purpose — separately, each
    /// can be wrong in the direction the other is wrong in, and the plan being
    /// empty is exactly the assertion that cannot be satisfied by two matching
    /// mistakes.
    #[test]
    fn an_applied_schema_reads_back_through_the_connection_a_command_opens() {
        let mut conn = in_memory();
        let want = Schema::new(vec![
            Table::new(
                "notes",
                vec![
                    column("id", "int64")
                        .identity()
                        .expect("an identity column is the primary key's own"),
                    column("title", "text(200)"),
                    column("body", "text").null(),
                ],
            )
            .expect("three columns and a bare name build a table")
            .primary_key(&["id"])
            .expect("the key names a column the table has")
            .unique("notes_title", &["title"])
            .expect("the constraint names a column the table has"),
        ])
        .expect("one table builds a schema");

        let empty = Schema::new(Vec::new()).expect("no tables build a schema");
        for step in crate::diff(&want, &empty, Dialect::Sqlite).runnable() {
            for sql in step.sql() {
                run(&mut conn, sql)
                    .expect("the emitter's SQL runs on the dialect it was written in");
            }
        }

        let have = schema_of(&mut conn).expect("the database it just wrote reads back");
        let left = crate::diff(&want, &have, Dialect::Sqlite);
        assert!(
            left.is_empty(),
            "the schema this path applied does not read back through it: {left}"
        );
    }
}
