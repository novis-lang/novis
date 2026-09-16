//! `rule:core-classes/schema-plan`'s DDL: a [`Schema`] value as SQL an operator could paste.
//!
//! Every statement this module returns is **complete, terminated and
//! dialect-correct**, including the ones the applier will refuse to run. § 8's
//! whole argument for that is the deployment where the application's own
//! credentials cannot issue DDL at all and a DBA runs the change from a ticket:
//! a plan whose risky steps are elided into "3 unsafe changes" is useless to
//! that person.
//!
//! # Two halves: the whole table, and one change to it
//!
//! [`create_table`] writes a table that does not exist yet, and [`step`]
//! answers one [`Change`] to a table that does — with its SQL, its § 6 grade
//! and the sentence that justifies the grade, which are one dialect's three
//! answers to the same reading and are computed together for that reason.
//! [`crate::plan`] holds the vocabulary and no SQL at all.
//!
//! # Four emitters, not five
//!
//! Keyed on [`Dialect`] and never on [`Driver`](crate::Driver). MariaDB and
//! MySQL share their SQL text exactly and are two drivers for authentication
//! plugins and error tables, neither of which reaches DDL —
//! `the_emitters_follow_dialect_rather_than_driver` asserts that as an
//! *agreement* over all five drivers rather than as five expected texts.
//!
//! # Why this is not an injection sink
//!
//! DDL binds nothing: there is no parameter marker in a `CREATE TABLE`, so the
//! rewriter in [`crate::sql`] has nothing to do here and every byte of these
//! statements is written rather than bound. Two closed sets are what make that
//! safe, and both are [`crate::schema`]'s rather than this module's:
//!
//! - **An identifier is validated and then delimited** — a [`Ident`] is ASCII
//!   letters, digits and `_` within [`MAX_IDENTIFIER`](crate::MAX_IDENTIFIER),
//!   and [`delimited`] writes it in the dialect's own quotes. The validation is
//!   what makes the delimiting total: no name the vocabulary admits can carry
//!   the delimiter that would end it, so the first closing delimiter a parser
//!   meets is the one this module wrote, and a name the server reserves —
//!   `RANK` on MySQL 8 — is still the name the schema gave it.
//!   `Core\Db::quoteIdentifier` validates and stops there because that member
//!   has no connection and so no dialect to quote for
//!   (`rule:security/tainted-qualifier`); every call here has one.
//! - **A literal is one of [`ColumnDefault`]'s cases**, each with a
//!   spelling per dialect save the read-only [`ColumnDefault::Opaque`], which
//!   no statement written here can carry. A program writes no expression
//!   defaults, so the one place a string reaches statement text —
//!   [`ColumnDefault::Text`] — is quoted here, in the dialect's own escaping.
//!
//! # An engine's own limit is the dialect's rule
//!
//! Each of these is one dialect's rule rather than a shape this emitter has
//! not written: the statement a dialect can say is the statement it emits, and
//! what that costs the round-trip is § 5's reading rather than a spelling to
//! invent here.
//!
//! - **SQL Server cannot index an unbounded text or bytes column at all** — a
//!   `MAX` type is not a key column there. The statement is still emitted,
//!   because § 8 says a step is shown in full even when it will not run, and
//!   § 5's grading is where it becomes a refusal rather than a failure.
//! - **A SQL Server unique key over a nullable column is a filtered index**,
//!   because its `UNIQUE` constraint is the one spelling of the four that reads
//!   that column's nulls as equal and refuses the second of them.
//!   `rule:core-classes/a-unique-key-reads-nulls-as-distinct` is the guarantee
//!   the vocabulary makes, so the dialect is brought into line here rather than
//!   worked around by every schema that needs one. [`filtered_unique_index`] is
//!   both doors: inside a `CREATE TABLE` and after the fact.
//! - **SQLite's identity is the rowid or nothing.** `AUTOINCREMENT` is legal
//!   only in the exact `INTEGER PRIMARY KEY AUTOINCREMENT` form, so an
//!   identity column that is one of several primary-key columns is emitted
//!   without it. [`Table`]'s own rule is only that an identity is *in* the
//!   primary key.
//! - **A SQLite unique constraint added after the fact is an index**, where
//!   one written into a `CREATE TABLE` is a constraint the catalog reports
//!   under an `sqlite_autoindex_…` name the schema never gave it. [`add_key`]
//!   takes the index form deliberately, and the two paths reaching the same
//!   catalog by different spellings is § 5's input.
//! - **SQLite's declared types carry affinity, and some of them convert.** A
//!   `JSON` or `DECIMAL` column has NUMERIC affinity, so a document that is a
//!   bare number and an exact decimal with trailing zeros are stored as
//!   numbers. That is the SQLite driver's binding question, answered in
//!   [`crate::sqlite`] where `DECIMAL`'s already is, and not by choosing a
//!   different declared type here, which would only move the cost into § 5's
//!   normalisation (`crate::plan`'s `stored_as`).
//!
//! # Known gaps
//!
//! 1. **A SQL Server default is a separate named constraint, so a change to
//!    one is not emitted.** `ALTER COLUMN` carries a type and a nullability
//!    there and nothing else; replacing a default means dropping the
//!    constraint holding it, by the name the server generated, and neither
//!    [`Change`] nor [`crate::catalog`]'s reads carry that name. A SQL Server
//!    default change is therefore a step whose SQL brings the type and the
//!    nullability across and leaves the default alone.
//!    Decided: Look the name up at apply time (sys.default_constraints) in the emitted batch — Works on
//!    any existing database with no vocabulary change; the SQL Server step becomes dynamic SQL.
//!    — owner: unowned-closures

use crate::plan::{Change, Grade, KeyKind, Step};
use crate::schema::{
    Column, ColumnDefault, FloatWidth, Ident, IntWidth, Key, ScalarType, Schema, Table,
};
use crate::sql::Dialect;

/// How many leading bytes of an unbounded text or bytes column MySQL indexes.
///
/// MySQL cannot index a `LONGTEXT` or `LONGBLOB` without a prefix length, and
/// 255 is what fits InnoDB's 3,072-byte key limit at `utf8mb4`'s four bytes a
/// character with room for a second column beside it —
/// `crates/nvs-stdlib/src/queue.rs`'s own DDL picked the same number for the
/// same reason. A prefix key is **stricter** than the constraint asked for, not
/// weaker: it refuses two distinct values sharing their first 255 bytes.
const TEXT_KEY_PREFIX: u32 = 255;

/// The statements that create every table in `schema`, in the schema's order.
#[must_use]
pub fn create_schema(schema: &Schema, dialect: Dialect) -> Vec<String> {
    schema
        .tables()
        .iter()
        .flat_map(|table| create_table(table, dialect))
        .collect()
}

/// The statements that create `table`: its columns, its keys and its indexes.
///
/// MySQL declares its indexes inside the `CREATE TABLE` and the other three
/// declare them beside it, which is the departure
/// `mysql_declares_an_index_inside_its_create_table_having_no_if_not_exists`
/// pins. MySQL has no `CREATE INDEX IF NOT EXISTS` and no transactional DDL, so
/// a table created by one statement and indexed by a second has a half-built
/// state that no re-run can complete; declaring the index inside the
/// `CREATE TABLE` means the table and its indexes arrive together or not at
/// all. The other three either have the guard or roll the pair back.
///
/// SQL Server leaves the table for a second reason: a unique key over a
/// nullable column is [`filtered_unique_index`]'s statement of its own there
/// rather than a clause, because the constraint form reads that column's nulls
/// as equal.
#[must_use]
pub fn create_table(table: &Table, dialect: Dialect) -> Vec<String> {
    let mut statements = vec![create_table_statement(
        &delimited(table.name().as_str(), dialect),
        table,
        dialect,
    )];
    statements.extend(
        table
            .unique_keys()
            .iter()
            .filter_map(|key| filtered_unique_index(table, key, dialect)),
    );
    if dialect != Dialect::MySql {
        statements.extend(
            table
                .indexes()
                .iter()
                .map(|key| create_index(table, key, dialect)),
        );
    }
    statements
}

/// The one `CREATE TABLE` for `table`, written under `name` rather than under
/// its own.
///
/// The two are the same everywhere but in [`sqlite_rebuild`], which builds the
/// table it is about to become under a staging name and renames it into place.
/// `name` arrives already [`delimited`], because the staging one is no
/// [`Ident`] and both callers hold the dialect that says how.
fn create_table_statement(name: &str, table: &Table, dialect: Dialect) -> String {
    let rowid = rowid_identity(table, dialect);
    let mut clauses: Vec<String> = table
        .columns()
        .iter()
        .map(|column| column_clause(column, dialect, rowid == Some(column.name())))
        .collect();

    // The rowid form carries the key in the column, and SQLite refuses a second
    // declaration of it.
    if rowid.is_none() && !table.primary_key_columns().is_empty() {
        clauses.push(format!(
            "PRIMARY KEY ({})",
            key_columns(table, table.primary_key_columns(), dialect)
        ));
    }
    for key in table.unique_keys() {
        // SQL Server's spelling of a nullable unique key is an index, which
        // [`create_table`] writes beside the table rather than inside it.
        if null_filter(table, key, dialect).is_none() {
            clauses.push(format!(
                "CONSTRAINT {} UNIQUE ({})",
                delimited(key.name().as_str(), dialect),
                key_columns(table, key.columns(), dialect)
            ));
        }
    }
    if dialect == Dialect::MySql {
        for key in table.indexes() {
            clauses.push(format!(
                "INDEX {} ({})",
                delimited(key.name().as_str(), dialect),
                key_columns(table, key.columns(), dialect)
            ));
        }
    }

    format!("CREATE TABLE {name} (\n    {}\n);", clauses.join(",\n    "))
}

/// SQL Server's spelling of a unique key over a nullable column, or `None`
/// where the `UNIQUE` constraint form already says the same thing.
///
/// A unique key's nulls are distinct on every backend
/// (`rule:core-classes/a-unique-key-reads-nulls-as-distinct`), and SQL Server's
/// constraint is the one spelling that reads them as equal: it admits one null
/// row and refuses the second. A unique index over the same columns, filtered
/// to the rows whose nullable members are present, is that key with the null
/// rows outside it — which is what the standard says and what the other three
/// give directly. The key keeps the name the schema gave it, so
/// [`crate::catalog`] reads it back as the same key and a second `plan`
/// converges.
fn filtered_unique_index(table: &Table, key: &Key, dialect: Dialect) -> Option<String> {
    let predicate = null_filter(table, key, dialect)?;
    Some(format!(
        "CREATE UNIQUE INDEX {} ON {} ({}) WHERE {predicate};",
        delimited(key.name().as_str(), dialect),
        delimited(table.name().as_str(), dialect),
        key_columns(table, key.columns(), dialect)
    ))
}

/// The `WHERE` that index carries: its nullable key columns, each present.
///
/// A `NOT NULL` column excludes no row and is left out, so the predicate is
/// empty exactly when the constraint form is correct as written — which is
/// every key on the other three dialects, and a key over columns SQL Server
/// already knows are present.
fn null_filter(table: &Table, key: &Key, dialect: Dialect) -> Option<String> {
    if dialect != Dialect::SqlServer {
        return None;
    }
    let terms: Vec<String> = key
        .columns()
        .iter()
        .filter(|name| table.column(name).is_some_and(Column::is_nullable))
        .map(|name| format!("{} IS NOT NULL", delimited(name.as_str(), dialect)))
        .collect();
    if terms.is_empty() {
        return None;
    }
    Some(terms.join(" AND "))
}

/// One `CREATE INDEX`, for the three dialects that declare one outside the
/// table it belongs to.
fn create_index(table: &Table, key: &Key, dialect: Dialect) -> String {
    format!(
        "CREATE INDEX {} ON {} ({});",
        delimited(key.name().as_str(), dialect),
        delimited(table.name().as_str(), dialect),
        key_columns(table, key.columns(), dialect)
    )
}

/// The SQL type `ty` is written as in `dialect`.
///
/// The spelling is **SQL's and never the canonical one**:
/// [`ScalarType`]'s `Display` is § 1's array form — `int64`, `text(200)` — and
/// a canonical spelling reaching a statement would be a schema file's syntax in
/// a server's parser. Every case here is upper case, which is what
/// `every_construct_in_the_vocabulary_emits_in_all_four_dialects` asserts
/// rather than listing every expected string.
#[must_use]
pub fn column_type(ty: &ScalarType, dialect: Dialect) -> String {
    match dialect {
        Dialect::PostgreSql => postgres_type(ty),
        Dialect::MySql => mysql_type(ty),
        Dialect::Sqlite => sqlite_type(ty),
        Dialect::SqlServer => sqlserver_type(ty),
    }
}

/// PostgreSQL: one `BYTEA` for both binary widths, `JSONB` rather than `JSON`
/// because the binary form is the one with an index, and `NUMERIC(20, 0)` for
/// the unsigned 64-bit column no backend but MySQL has.
fn postgres_type(ty: &ScalarType) -> String {
    match ty {
        ScalarType::Int(IntWidth::Small) => "SMALLINT".to_owned(),
        ScalarType::Int(IntWidth::Normal) | ScalarType::Uint(IntWidth::Small) => {
            "INTEGER".to_owned()
        }
        ScalarType::Int(IntWidth::Big) | ScalarType::Uint(IntWidth::Normal) => "BIGINT".to_owned(),
        ScalarType::Uint(IntWidth::Big) => "NUMERIC(20, 0)".to_owned(),
        ScalarType::Float(FloatWidth::Single) => "REAL".to_owned(),
        ScalarType::Float(FloatWidth::Double) => "DOUBLE PRECISION".to_owned(),
        ScalarType::Decimal { precision, scale } => format!("NUMERIC({precision}, {scale})"),
        ScalarType::Text { max: Some(max) } => format!("VARCHAR({max})"),
        ScalarType::Text { max: None } => "TEXT".to_owned(),
        ScalarType::Bytes { .. } => "BYTEA".to_owned(),
        ScalarType::Bool => "BOOLEAN".to_owned(),
        ScalarType::Date => "DATE".to_owned(),
        ScalarType::Time => "TIME".to_owned(),
        ScalarType::DateTime => "TIMESTAMP".to_owned(),
        ScalarType::Instant => "TIMESTAMPTZ".to_owned(),
        ScalarType::Uuid => "UUID".to_owned(),
        ScalarType::Json => "JSONB".to_owned(),
    }
}

/// MySQL and MariaDB: the only backend with an unsigned integer, and the one
/// that spells a boolean `BIT(1)`.
///
/// `BOOLEAN` is not written here even though MySQL accepts it, because MySQL's
/// `BOOLEAN` *is* `TINYINT(1)` in the catalog and
/// [ADR 0067 § 9](/docs/decisions/0067.md) reads a `TINYINT(1)` as an `int`.
/// A column written as a boolean that introspects as an integer is § 5's empty
/// plan failing at the first `dump`. `CHAR(36)` for a UUID is the other
/// direction of the same trade and it is unavoidable: MySQL has no UUID type,
/// and `BINARY(16)` reads back as `bytes`.
fn mysql_type(ty: &ScalarType) -> String {
    match ty {
        ScalarType::Int(IntWidth::Small) => "SMALLINT".to_owned(),
        ScalarType::Int(IntWidth::Normal) => "INT".to_owned(),
        ScalarType::Int(IntWidth::Big) => "BIGINT".to_owned(),
        ScalarType::Uint(IntWidth::Small) => "SMALLINT UNSIGNED".to_owned(),
        ScalarType::Uint(IntWidth::Normal) => "INT UNSIGNED".to_owned(),
        ScalarType::Uint(IntWidth::Big) => "BIGINT UNSIGNED".to_owned(),
        ScalarType::Float(FloatWidth::Single) => "FLOAT".to_owned(),
        ScalarType::Float(FloatWidth::Double) => "DOUBLE".to_owned(),
        ScalarType::Decimal { precision, scale } => format!("DECIMAL({precision}, {scale})"),
        ScalarType::Text { max: Some(max) } => format!("VARCHAR({max})"),
        ScalarType::Text { max: None } => "LONGTEXT".to_owned(),
        ScalarType::Bytes { max: Some(max) } => format!("VARBINARY({max})"),
        ScalarType::Bytes { max: None } => "LONGBLOB".to_owned(),
        ScalarType::Bool => "BIT(1)".to_owned(),
        ScalarType::Date => "DATE".to_owned(),
        ScalarType::Time => "TIME".to_owned(),
        ScalarType::DateTime => "DATETIME".to_owned(),
        ScalarType::Instant => "TIMESTAMP".to_owned(),
        ScalarType::Uuid => "CHAR(36)".to_owned(),
        ScalarType::Json => "JSON".to_owned(),
    }
}

/// SQLite: a declared type is a name and an affinity rather than a constraint,
/// so the vocabulary's own spelling is written wherever one exists.
///
/// That is not decoration. `pragma table_info` reports the declared name back
/// verbatim, so writing `TIMESTAMPTZ` for an [`ScalarType::Instant`] — a type
/// SQLite has no idea about — is what lets stage 4 read the same value back and
/// § 5's plan be empty. The one place it cannot help is binary: `BLOB` is the
/// only storage class there is, so both [`ScalarType::Bytes`] widths collapse
/// into it exactly as PostgreSQL's `BYTEA` does.
fn sqlite_type(ty: &ScalarType) -> String {
    match ty {
        ScalarType::Int(IntWidth::Small) => "SMALLINT".to_owned(),
        ScalarType::Int(IntWidth::Normal) | ScalarType::Uint(IntWidth::Small) => {
            "INTEGER".to_owned()
        }
        ScalarType::Int(IntWidth::Big) | ScalarType::Uint(IntWidth::Normal) => "BIGINT".to_owned(),
        ScalarType::Uint(IntWidth::Big) => "NUMERIC(20, 0)".to_owned(),
        ScalarType::Float(FloatWidth::Single) => "REAL".to_owned(),
        ScalarType::Float(FloatWidth::Double) => "DOUBLE PRECISION".to_owned(),
        ScalarType::Decimal { precision, scale } => format!("DECIMAL({precision}, {scale})"),
        ScalarType::Text { max: Some(max) } => format!("VARCHAR({max})"),
        ScalarType::Text { max: None } => "TEXT".to_owned(),
        ScalarType::Bytes { .. } => "BLOB".to_owned(),
        ScalarType::Bool => "BOOLEAN".to_owned(),
        ScalarType::Date => "DATE".to_owned(),
        ScalarType::Time => "TIME".to_owned(),
        ScalarType::DateTime => "DATETIME".to_owned(),
        ScalarType::Instant => "TIMESTAMPTZ".to_owned(),
        ScalarType::Uuid => "UUID".to_owned(),
        ScalarType::Json => "JSON".to_owned(),
    }
}

/// SQL Server: `N`-prefixed national text throughout, `MAX` rather than an
/// unbounded type of its own, and `DATETIME2` rather than the `DATETIME` that
/// rounds to 1/300th of a second.
fn sqlserver_type(ty: &ScalarType) -> String {
    match ty {
        ScalarType::Int(IntWidth::Small) => "SMALLINT".to_owned(),
        ScalarType::Int(IntWidth::Normal) | ScalarType::Uint(IntWidth::Small) => "INT".to_owned(),
        ScalarType::Int(IntWidth::Big) | ScalarType::Uint(IntWidth::Normal) => "BIGINT".to_owned(),
        ScalarType::Uint(IntWidth::Big) => "DECIMAL(20, 0)".to_owned(),
        ScalarType::Float(FloatWidth::Single) => "REAL".to_owned(),
        ScalarType::Float(FloatWidth::Double) => "FLOAT(53)".to_owned(),
        ScalarType::Decimal { precision, scale } => format!("DECIMAL({precision}, {scale})"),
        ScalarType::Text { max: Some(max) } => format!("NVARCHAR({max})"),
        ScalarType::Text { max: None } => "NVARCHAR(MAX)".to_owned(),
        ScalarType::Bytes { max: Some(max) } => format!("VARBINARY({max})"),
        ScalarType::Bytes { max: None } => "VARBINARY(MAX)".to_owned(),
        ScalarType::Bool => "BIT".to_owned(),
        ScalarType::Date => "DATE".to_owned(),
        ScalarType::Time => "TIME".to_owned(),
        ScalarType::DateTime => "DATETIME2".to_owned(),
        ScalarType::Instant => "DATETIMEOFFSET".to_owned(),
        ScalarType::Uuid => "UNIQUEIDENTIFIER".to_owned(),
        ScalarType::Json => "NVARCHAR(MAX)".to_owned(),
    }
}

/// One column of a `CREATE TABLE`.
///
/// `rowid` is SQLite's identity form and is the whole column: it carries the
/// primary key, the type and the autoincrement in three words that have to be
/// written in that order and nowhere else.
fn column_clause(column: &Column, dialect: Dialect, rowid: bool) -> String {
    let name = delimited(column.name().as_str(), dialect);
    let mut parts: Vec<String> = if rowid {
        vec![name, "INTEGER PRIMARY KEY AUTOINCREMENT".to_owned()]
    } else {
        let mut parts = vec![name, column_type(column.ty(), dialect)];
        if !column.is_nullable() {
            parts.push("NOT NULL".to_owned());
        }
        if let Some(default) = column.default_value() {
            parts.push(format!(
                "DEFAULT {}",
                literal(default, column.ty(), dialect)
            ));
        }
        if let Some(spelling) = identity(dialect).filter(|_| column.is_identity()) {
            parts.push(spelling.to_owned());
        }
        parts
    };
    if let Some(check) = unsigned_check(column, dialect) {
        parts.push(check);
    }
    parts.join(" ")
}

/// How `dialect` says the server supplies this column's value, or `None` for
/// SQLite, whose only spelling is the rowid form [`column_clause`] writes.
fn identity(dialect: Dialect) -> Option<&'static str> {
    match dialect {
        Dialect::PostgreSql => Some("GENERATED BY DEFAULT AS IDENTITY"),
        Dialect::MySql => Some("AUTO_INCREMENT"),
        Dialect::SqlServer => Some("IDENTITY(1,1)"),
        Dialect::Sqlite => None,
    }
}

/// The check that stands in for the unsigned integer type only MySQL has.
///
/// [`ScalarType::Uint`]'s own doc states the rule this implements: the other
/// three take the next width up, which leaves the negative half of that width
/// admissible, and the check is what closes it. It is not one of § 11's
/// excluded check constraints — those are a construct a schema could *ask* for,
/// and this is a type's own spelling on a backend that lacks the type.
fn unsigned_check(column: &Column, dialect: Dialect) -> Option<String> {
    if dialect == Dialect::MySql || !matches!(column.ty(), ScalarType::Uint(_)) {
        return None;
    }
    Some(format!(
        "CHECK ({} >= 0)",
        delimited(column.name().as_str(), dialect)
    ))
}

/// The columns of a key, with MySQL's prefix length where it needs one.
fn key_columns(table: &Table, columns: &[Ident], dialect: Dialect) -> String {
    columns
        .iter()
        .map(|name| {
            let unbounded = table.column(name).is_some_and(|column| {
                matches!(
                    column.ty(),
                    ScalarType::Text { max: None } | ScalarType::Bytes { max: None }
                )
            });
            let written = delimited(name.as_str(), dialect);
            if dialect == Dialect::MySql && unbounded {
                format!("{written}({TEXT_KEY_PREFIX})")
            } else {
                written
            }
        })
        .collect::<Vec<String>>()
        .join(", ")
}

/// The identity column SQLite writes as the rowid, if this table has one.
///
/// Only when it is the *whole* primary key: `AUTOINCREMENT` is legal in the
/// exact `INTEGER PRIMARY KEY AUTOINCREMENT` form and in no other, so an
/// identity inside a composite key has no SQLite spelling at all.
///
/// `pub(crate)` for [`crate::plan::diff`], which needs the same predicate for
/// the opposite reason: the width this form erases is § 5's to normalise out
/// of a comparison, and a second reading of when SQLite writes a rowid would
/// be a second answer to drift from this one.
pub(crate) fn rowid_identity(table: &Table, dialect: Dialect) -> Option<&Ident> {
    if dialect != Dialect::Sqlite {
        return None;
    }
    let [only] = table.primary_key_columns() else {
        return None;
    };
    table
        .column(only)
        .filter(|column| column.is_identity())
        .map(Column::name)
}

/// One of [`ColumnDefault`]'s writable cases as a literal `dialect` reads.
///
/// The column's own type is read for one case and it is not a nicety.
/// [`ColumnDefault::Now`]'s doc calls `CURRENT_TIMESTAMP` the one spelling all
/// five share, and it is — but on SQL Server it is a `datetime`, so writing it
/// as a [`ScalarType::Instant`]'s default converts the server's *local* time
/// into a `datetimeoffset` labelled `+00:00`. That is a wrong instant on every
/// server not running in UTC, silently, in a column whose whole point is that
/// it carries its zone. `SYSDATETIMEOFFSET()` is the same clock read as the
/// type it is being stored in.
pub(crate) fn literal(default: &ColumnDefault, ty: &ScalarType, dialect: Dialect) -> String {
    match default {
        ColumnDefault::Int(value) => value.to_string(),
        ColumnDefault::Uint(value) => value.to_string(),
        ColumnDefault::Float(value) => format!("{value:?}"),
        ColumnDefault::Decimal(digits) => digits.clone(),
        ColumnDefault::Text(text) => quoted(text, dialect),
        ColumnDefault::Bool(yes) => match (dialect, *yes) {
            (Dialect::PostgreSql, true) => "TRUE".to_owned(),
            (Dialect::PostgreSql, false) => "FALSE".to_owned(),
            // MySQL's `BIT(1)` takes a bit literal and not a `1`.
            (Dialect::MySql, true) => "b'1'".to_owned(),
            (Dialect::MySql, false) => "b'0'".to_owned(),
            (_, true) => "1".to_owned(),
            (_, false) => "0".to_owned(),
        },
        // The one spelling all five share, which is why § 2's closed set has a
        // case for it rather than an expression — with the one exception this
        // function's doc argues.
        ColumnDefault::Now => match (dialect, ty) {
            (Dialect::SqlServer, ScalarType::Instant) => "SYSDATETIMEOFFSET()".to_owned(),
            _ => "CURRENT_TIMESTAMP".to_owned(),
        },
        // The arm exists to be unreachable rather than to spell anything. Every
        // step emits the *wanted* schema's default, a program wrote that
        // schema, and `ColumnDefault::from_node` refuses the only key this case
        // could have arrived under — so reaching here is a schema read off a
        // server handed to the emitter, and the one thing that must not happen
        // is the server's words going back out as statement text.
        ColumnDefault::Opaque(text) => {
            unreachable!("an opaque default reached the emitter: {text}")
        }
    }
}

/// An identifier in `dialect`'s own delimiters.
///
/// Every name this module writes goes through here. A backend reserves words
/// no other one does — `RANK` is a keyword on MySQL 8 — and a schema's names
/// are not this emitter's to rename, so the quotes are what keep a `CREATE
/// TABLE` parseable wherever it is pasted. Nothing is escaped on the way
/// through and nothing has to be: [`crate::schema::is_bare_identifier`] admits
/// ASCII letters, digits and `_`, and no delimiter is one of those.
///
/// SQL Server takes the bracket form rather than the double quote it also
/// accepts, because the quoted form there is only an identifier while
/// `QUOTED_IDENTIFIER` is on and a session may have turned it off. PostgreSQL
/// folds an undelimited name to lower case and a delimited one exactly, which
/// is one of the reasons [`Ident`] compares case-insensitively rather than
/// trusting either.
fn delimited(name: &str, dialect: Dialect) -> String {
    match dialect {
        Dialect::PostgreSql | Dialect::Sqlite => format!("\"{name}\""),
        Dialect::MySql => format!("`{name}`"),
        Dialect::SqlServer => format!("[{name}]"),
    }
}

/// A text literal in `dialect`'s own escaping.
///
/// Doubling the quote is the standard every backend implements. The backslash
/// is MySQL's alone, and the judgement is `Dialect::backslash_escapes`'s
/// rather than a second copy of it: a value
/// ending in `\` would otherwise escape the quote that closes it, which is the
/// classic way a "quoted" string stops being one.
fn quoted(text: &str, dialect: Dialect) -> String {
    let escaped = if dialect.backslash_escapes() {
        text.replace('\\', "\\\\").replace('\'', "''")
    } else {
        text.replace('\'', "''")
    };
    // `N` is what makes a SQL Server literal national rather than a codepage's,
    // and every text column this module writes there is `NVARCHAR`.
    if dialect == Dialect::SqlServer {
        format!("N'{escaped}'")
    } else {
        format!("'{escaped}'")
    }
}

/// The step that makes `change` in `dialect`: its SQL, its grade and the
/// reason for that grade.
///
/// This is the only place a [`Step`] is built, because § 6 puts the grade in
/// the dialect emitter: the same change is not the same risk on two backends,
/// and the two answers are computed from the same reading of the change.
///
/// **The SQL is complete and terminated even when the step is a report** — a
/// [`Change::is_report`] is § 7's "carried and never applied", not "carried
/// and summarised".
#[must_use]
pub fn step(change: Change, dialect: Dialect) -> Step {
    let sql = sql_for(&change, dialect);
    let (grade, reason) = grade_of(&change, dialect);
    Step::new(change, grade, reason, sql)
}

/// The statements that make `change` in `dialect`, each terminated.
fn sql_for(change: &Change, dialect: Dialect) -> Vec<String> {
    match change {
        Change::CreateTable(table) => create_table(table, dialect),
        Change::DropTable(name) => {
            vec![format!("DROP TABLE {};", delimited(name.as_str(), dialect))]
        }
        Change::AddColumn { table, column } => add_column(table, column, dialect),
        Change::DropColumn { table, column } => {
            if dialect == Dialect::Sqlite {
                sqlite_rebuild(table, None)
            } else {
                vec![format!(
                    "ALTER TABLE {} DROP COLUMN {};",
                    delimited(table.name().as_str(), dialect),
                    delimited(column.as_str(), dialect)
                )]
            }
        }
        Change::ChangeColumn { table, from, to } => change_column(table, from, to, dialect),
        Change::AddKey { table, key, kind } => vec![add_key(table, key, *kind, dialect)],
        Change::DropKey { table, key, kind } => drop_key(table, key, *kind, dialect),
    }
}

/// `ALTER TABLE … ADD COLUMN`, or SQLite's rebuild where it cannot.
///
/// SQL Server is the one that does not write the word `COLUMN`, and it is not
/// optional there — `ADD COLUMN c INT` is a syntax error, where the other
/// three take the keyword and PostgreSQL and MySQL also take it away.
fn add_column(table: &Table, column: &Column, dialect: Dialect) -> Vec<String> {
    if dialect == Dialect::Sqlite && !sqlite_can_add(column) {
        return sqlite_rebuild(table, Some(column.name()));
    }
    let keyword = if dialect == Dialect::SqlServer {
        ""
    } else {
        "COLUMN "
    };
    vec![format!(
        "ALTER TABLE {} ADD {keyword}{};",
        delimited(table.name().as_str(), dialect),
        column_clause(column, dialect, false)
    )]
}

/// Whether SQLite's own `ALTER TABLE … ADD COLUMN` can carry this column.
///
/// Each refusal is SQLite's own rather than a caution of ours: the
/// default must be a constant, so [`ColumnDefault::Now`] is out; a `NOT NULL`
/// column must have one, because every existing row needs a value; and an
/// identity is the rowid there, which is the primary key and cannot be added
/// to a table that already has one.
fn sqlite_can_add(column: &Column) -> bool {
    if column.is_identity() {
        return false;
    }
    match column.default_value() {
        Some(ColumnDefault::Now) => false,
        Some(_) => true,
        None => column.is_nullable(),
    }
}

/// A column's type, nullability and default brought to `to`.
///
/// Three shapes for four dialects. MySQL restates the whole definition in one
/// `MODIFY COLUMN`, so nothing has to be compared; SQL Server restates the
/// type and the nullability together and keeps its default in a separate named
/// constraint; PostgreSQL spells each property in its own statement, and gets
/// only the ones that changed — a `TYPE` alter it did not need is a full table
/// rewrite. SQLite has no spelling for any of it and rebuilds.
fn change_column(table: &Table, from: &Column, to: &Column, dialect: Dialect) -> Vec<String> {
    let name = delimited(table.name().as_str(), dialect);
    let column = delimited(to.name().as_str(), dialect);
    match dialect {
        Dialect::Sqlite => sqlite_rebuild(table, None),
        Dialect::MySql => vec![format!(
            "ALTER TABLE {name} MODIFY COLUMN {};",
            column_clause(to, dialect, false)
        )],
        Dialect::SqlServer => vec![format!(
            "ALTER TABLE {name} ALTER COLUMN {column} {} {};",
            column_type(to.ty(), dialect),
            if to.is_nullable() { "NULL" } else { "NOT NULL" }
        )],
        Dialect::PostgreSql => {
            let retype = format!(
                "ALTER TABLE {name} ALTER COLUMN {column} TYPE {};",
                column_type(to.ty(), dialect)
            );
            let mut statements = Vec::new();
            if from.ty() != to.ty() {
                statements.push(retype.clone());
            }
            if from.is_nullable() != to.is_nullable() {
                statements.push(format!(
                    "ALTER TABLE {name} ALTER COLUMN {column} {};",
                    if to.is_nullable() {
                        "DROP NOT NULL"
                    } else {
                        "SET NOT NULL"
                    }
                ));
            }
            if from.default_value() != to.default_value() {
                statements.push(match to.default_value() {
                    Some(default) => format!(
                        "ALTER TABLE {name} ALTER COLUMN {column} SET DEFAULT {};",
                        literal(default, to.ty(), dialect)
                    ),
                    None => format!("ALTER TABLE {name} ALTER COLUMN {column} DROP DEFAULT;"),
                });
            }
            // A step is never empty, whatever the caller handed us.
            if statements.is_empty() {
                statements.push(retype);
            }
            statements
        }
    }
}

/// One `CREATE INDEX` or one `ADD CONSTRAINT … UNIQUE`.
///
/// Two dialects say a unique key as an index instead, for two unrelated
/// reasons, and both keep the name the schema gave it so the introspector
/// matches it back. SQLite cannot add a constraint to an existing table at all,
/// and a unique index is the same refusal by another name where the constraint
/// form would leave an `sqlite_autoindex_…`. SQL Server can, but the constraint
/// reads a nullable column's nulls as equal, so a key over one is
/// [`filtered_unique_index`]'s filtered index.
fn add_key(table: &Table, key: &Key, kind: KeyKind, dialect: Dialect) -> String {
    match (kind, dialect) {
        (KeyKind::Index, _) => create_index(table, key, dialect),
        (KeyKind::Unique, Dialect::Sqlite) => format!(
            "CREATE UNIQUE INDEX {} ON {} ({});",
            delimited(key.name().as_str(), dialect),
            delimited(table.name().as_str(), dialect),
            key_columns(table, key.columns(), dialect)
        ),
        (KeyKind::Unique, _) => filtered_unique_index(table, key, dialect).unwrap_or_else(|| {
            format!(
                "ALTER TABLE {} ADD CONSTRAINT {} UNIQUE ({});",
                delimited(table.name().as_str(), dialect),
                delimited(key.name().as_str(), dialect),
                key_columns(table, key.columns(), dialect)
            )
        }),
    }
}

/// The `DROP` for a key, which every dialect spells differently.
///
/// MySQL has no `DROP CONSTRAINT` before 8.0.19 and drops the index the
/// constraint is; MySQL and SQL Server both need the table named on a
/// `DROP INDEX` where PostgreSQL and SQLite refuse it, because an index is a
/// schema-level object there and a table-level one here. SQLite cannot drop a
/// constraint at all and rebuilds.
fn drop_key(table: &Table, key: &Ident, kind: KeyKind, dialect: Dialect) -> Vec<String> {
    let name = delimited(table.name().as_str(), dialect);
    let key = delimited(key.as_str(), dialect);
    match (kind, dialect) {
        (KeyKind::Unique, Dialect::Sqlite) => sqlite_rebuild(table, None),
        (KeyKind::Unique, Dialect::MySql) => {
            vec![format!("ALTER TABLE {name} DROP INDEX {key};")]
        }
        (KeyKind::Unique, _) => vec![format!("ALTER TABLE {name} DROP CONSTRAINT {key};")],
        (KeyKind::Index, Dialect::MySql | Dialect::SqlServer) => {
            vec![format!("DROP INDEX {key} ON {name};")]
        }
        (KeyKind::Index, _) => vec![format!("DROP INDEX {key};")],
    }
}

/// SQLite's create-copy-drop-rename, for every alter its own `ALTER TABLE`
/// cannot express.
///
/// `table` is the table as it should be **after** the change, which is why
/// [`Change`]'s own doc fixes that convention: the rebuild builds exactly that
/// under a staging name, copies what both shapes have in common, drops the
/// original and renames. `added` names the one column a rebuild must *not*
/// select, because it is the column being added and the old table has no such
/// value to copy — the new one takes its default, or, having none, refuses the
/// insert on a table that already holds rows, which is precisely what
/// `NOT NULL` means on every other backend too.
///
/// Indexes come last and not with the `CREATE TABLE`: an index name is unique
/// across a SQLite database, so the staging table cannot carry the names the
/// original still holds. Dropping the original takes its indexes with it,
/// which is why they are re-created here rather than left alone.
///
/// The applier wraps this in a transaction — SQLite has transactional DDL, so
/// a rebuild that fails half way leaves the original table untouched, which is
/// the one thing that makes a data copy tolerable as a schema change at all.
fn sqlite_rebuild(table: &Table, added: Option<&Ident>) -> Vec<String> {
    let dialect = Dialect::Sqlite;
    let name = delimited(table.name().as_str(), dialect);
    let staging = delimited(&format!("{}_nvs_rebuild", table.name()), dialect);
    let copied: Vec<String> = table
        .columns()
        .iter()
        .map(Column::name)
        .filter(|column| !added.is_some_and(|new| new == *column))
        .map(|column| delimited(column.as_str(), dialect))
        .collect();

    let mut statements = vec![create_table_statement(&staging, table, dialect)];
    // A rebuild whose every column is the new one has nothing to copy, and
    // `INSERT INTO t () SELECT FROM u` is not a statement.
    if !copied.is_empty() {
        let columns = copied.join(", ");
        statements.push(format!(
            "INSERT INTO {staging} ({columns})\n    SELECT {columns} FROM {name};"
        ));
    }
    statements.push(format!("DROP TABLE {name};"));
    statements.push(format!("ALTER TABLE {staging} RENAME TO {name};"));
    statements.extend(
        table
            .indexes()
            .iter()
            .map(|key| create_index(table, key, dialect)),
    );
    statements
}

/// Whether SQLite answers `change` with [`sqlite_rebuild`] rather than with an
/// `ALTER TABLE` of its own.
///
/// This is the *grading* half of a judgement [`sql_for`]'s arms make again
/// when they emit, and the two are held together by
/// `sqlite_rebuilds_the_table_for_an_alter_it_cannot_express`, which asserts
/// the agreement over every change in the vocabulary rather than either
/// answer on its own.
fn sqlite_rebuilds(change: &Change, dialect: Dialect) -> bool {
    dialect == Dialect::Sqlite
        && match change {
            Change::AddColumn { column, .. } => !sqlite_can_add(column),
            Change::DropColumn { .. } | Change::ChangeColumn { .. } => true,
            Change::DropKey { kind, .. } => *kind == KeyKind::Unique,
            Change::CreateTable(_) | Change::DropTable(_) | Change::AddKey { .. } => false,
        }
}

/// § 6's grade for `change` in `dialect`, and the sentence an operator reads
/// instead of taking our word for it.
fn grade_of(change: &Change, dialect: Dialect) -> (Grade, String) {
    let (grade, reason) = base_grade(change);
    if sqlite_rebuilds(change, dialect) {
        return (
            grade.up_to(Grade::Destructive),
            format!(
                "{reason} SQLite cannot express this alter, so it is a \
                 create-copy-drop-rename rebuild, which copies every row — destructive \
                 unconditionally, whatever the change would have cost elsewhere."
            ),
        );
    }
    (grade, reason.to_owned())
}

/// The grade every dialect agrees on, before SQLite's rebuild is considered.
///
/// The version-keyed half of § 6 is where the grade-up rule does its work:
/// adding a column with a default is instant on PostgreSQL 11+ and MySQL
/// 8.0.12+ and a full table rewrite on anything older, and nothing in a
/// sans-io emitter knows which server it is talking to. It grades up, and the
/// reason says so, so an operator who does know can overrule it by reading.
fn base_grade(change: &Change) -> (Grade, &'static str) {
    match change {
        Change::CreateTable(_) => (
            Grade::Safe,
            "A new table holds no rows: nothing to lose, nothing to validate, nothing to block.",
        ),
        Change::DropTable(_) => (
            Grade::Destructive,
            "Every row in the table. Reported and never applied: a schema value describes what \
             its author knows about, and this database holds a table it does not name.",
        ),
        Change::AddColumn { column, .. } => {
            if column.default_value().is_some() {
                (
                    Grade::Locking,
                    "A column with a default is a catalog write on PostgreSQL 11+ and MySQL \
                     8.0.12+ and a full table rewrite on anything older. The server's version is \
                     not known here, so this grades up.",
                )
            } else if column.is_nullable() {
                (
                    Grade::Safe,
                    "A nullable column with no default is a catalog write on all four: every \
                     existing row already has its value, and it is null.",
                )
            } else {
                (
                    Grade::Locking,
                    "A NOT NULL column with no default has no value for the rows that are \
                     already there, so it fails outright on a table that is not empty.",
                )
            }
        }
        Change::DropColumn { .. } => (
            Grade::Destructive,
            "Every value in the column. Reported and never applied, per the same rule as a \
             dropped table.",
        ),
        Change::ChangeColumn { from, to, .. } => {
            if widens(from.ty(), to.ty()) {
                if from.is_nullable() && !to.is_nullable() {
                    (
                        Grade::Locking,
                        "The new type holds every value the old one did, but NOT NULL is checked \
                         against every existing row and fails on the first null.",
                    )
                } else {
                    (
                        Grade::Locking,
                        "The new type holds every value the old one did, but most servers rewrite \
                         the table to change one and block writes while they do.",
                    )
                }
            } else {
                (
                    Grade::Destructive,
                    "The new type does not hold every value the old one did, so the server either \
                     truncates what does not fit or refuses the whole statement. Where the \
                     emitter has no rule that says otherwise, § 6 grades up.",
                )
            }
        }
        Change::AddKey {
            kind: KeyKind::Unique,
            ..
        } => (
            Grade::Locking,
            "A unique key is validated against every existing row, and fails where two of them \
             already collide.",
        ),
        Change::AddKey {
            kind: KeyKind::Index,
            ..
        } => (
            Grade::Locking,
            "An index is built over every existing row and holds a lock while it is built; v1 \
             emits no concurrent build.",
        ),
        Change::DropKey { .. } => (
            Grade::Destructive,
            "A key is not data, but dropping is an operation someone writes rather than one this \
             tool performs. Reported and never applied.",
        ),
    }
}

/// Whether every value of `from` fits in `to`.
///
/// Conservative on purpose, and the default arm is § 6's grade-up rule: a pair
/// this has no rule for is *not* a widening, which makes the change
/// destructive and costs an operator a confirmation. The reverse mistake costs
/// them the column. Nothing here crosses type families — an integer into a
/// float loses exactness past 2^53 and a date into a text is a spelling, not a
/// conversion — so the rules are within a family and the identity.
fn widens(from: &ScalarType, to: &ScalarType) -> bool {
    match (from, to) {
        (ScalarType::Int(narrow), ScalarType::Int(wide))
        | (ScalarType::Uint(narrow), ScalarType::Uint(wide)) => wide.bits() >= narrow.bits(),
        // An unsigned value fits a signed type only with a bit to spare for
        // the sign, which the three portable widths supply one step apart.
        (ScalarType::Uint(narrow), ScalarType::Int(wide)) => wide.bits() > narrow.bits(),
        (ScalarType::Float(narrow), ScalarType::Float(wide)) => wide.bits() >= narrow.bits(),
        (
            ScalarType::Decimal {
                precision: from_precision,
                scale: from_scale,
            },
            ScalarType::Decimal {
                precision: to_precision,
                scale: to_scale,
            },
        ) => {
            to_precision >= from_precision
                && to_scale >= from_scale
                && to_precision - to_scale >= from_precision - from_scale
        }
        (ScalarType::Text { max: from_max }, ScalarType::Text { max: to_max })
        | (ScalarType::Bytes { max: from_max }, ScalarType::Bytes { max: to_max }) => {
            match (from_max, to_max) {
                (_, None) => true,
                (None, Some(_)) => false,
                (Some(narrow), Some(wide)) => wide >= narrow,
            }
        }
        _ => from == to,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::conn::Driver;
    use crate::plan::Plan;

    /// The four dialects, in the order [`Dialect`] declares them.
    const DIALECTS: [Dialect; 4] = [
        Dialect::PostgreSql,
        Dialect::MySql,
        Dialect::Sqlite,
        Dialect::SqlServer,
    ];

    /// How each dialect delimits an identifier, written out here rather than
    /// asked of [`delimited`]: a case that takes its expectation from the
    /// function under test asserts that the function equals itself.
    fn delimiters(dialect: Dialect) -> (char, char) {
        match dialect {
            Dialect::PostgreSql | Dialect::Sqlite => ('"', '"'),
            Dialect::MySql => ('`', '`'),
            Dialect::SqlServer => ('[', ']'),
        }
    }

    /// `name` as `dialect` writes it.
    fn as_written(name: &str, dialect: Dialect) -> String {
        let (open, close) = delimiters(dialect);
        format!("{open}{name}{close}")
    }

    /// Whether `statement` names `name` as a word of its own without
    /// `dialect`'s delimiters around it.
    ///
    /// A word of its own is the whole of the judgement: `wide` inside
    /// `wide_nvs_rebuild` is not this identifier, and the delimited staging
    /// name it sits in would otherwise read as a bare one.
    fn written_bare(statement: &str, name: &str, dialect: Dialect) -> bool {
        let (open, close) = delimiters(dialect);
        let bytes = statement.as_bytes();
        let mut from = 0;
        while let Some(found) = statement[from..].find(name) {
            let start = from + found;
            let end = start + name.len();
            from = end;
            let before = start.checked_sub(1).map(|at| bytes[at] as char);
            let after = bytes.get(end).map(|byte| *byte as char);
            let boundary = |side: Option<char>| {
                side.is_none_or(|char| !char.is_ascii_alphanumeric() && char != '_')
            };
            if boundary(before) && boundary(after) && (before != Some(open) || after != Some(close))
            {
                return true;
            }
        }
        false
    }

    /// Every name [`every_construct`] and [`every_change`] hand the emitter,
    /// including the staging table only SQLite's rebuild writes.
    fn fixture_identifiers() -> Vec<String> {
        let schema = every_construct();
        let mut names = Vec::new();
        for table in schema.tables() {
            names.push(table.name().to_string());
            names.extend(
                table
                    .columns()
                    .iter()
                    .map(|column| column.name().to_string()),
            );
            names.extend(
                table
                    .unique_keys()
                    .iter()
                    .chain(table.indexes().iter())
                    .map(|key| key.name().to_string()),
            );
        }
        names.extend(
            [
                "note",
                "owner",
                "touched_at",
                "wide_gone_key",
                "wide_gone_idx",
                "wide_nvs_rebuild",
            ]
            .map(str::to_owned),
        );
        names
    }

    /// A schema naming every construct the vocabulary has: all the scalar
    /// types at both of their widths, an identity, a nullable and a
    /// non-nullable column, every kind of default, a single and a composite
    /// primary key, a unique constraint, and two indexes — one of them over an
    /// unbounded text column, which is the one MySQL cannot index whole.
    ///
    /// No column is named after a keyword any of the four reserves, because an
    /// identifier is validated and never delimited: `double` and `order` are
    /// not names this vocabulary can carry, and a fixture pretending otherwise
    /// would be pinning SQL no server accepts.
    fn every_construct() -> Schema {
        let wide = Table::new(
            "wide",
            vec![
                Column::new("id", ScalarType::Int(IntWidth::Big))
                    .unwrap()
                    .identity()
                    .unwrap(),
                Column::new("small_int", ScalarType::Int(IntWidth::Small)).unwrap(),
                Column::new("normal_int", ScalarType::Int(IntWidth::Normal))
                    .unwrap()
                    .default(ColumnDefault::Int(-7))
                    .unwrap(),
                Column::new("u_small", ScalarType::Uint(IntWidth::Small)).unwrap(),
                Column::new("u_normal", ScalarType::Uint(IntWidth::Normal)).unwrap(),
                Column::new("u_big", ScalarType::Uint(IntWidth::Big))
                    .unwrap()
                    .default(ColumnDefault::Uint(u64::MAX))
                    .unwrap(),
                Column::new("f_single", ScalarType::Float(FloatWidth::Single)).unwrap(),
                Column::new("f_double", ScalarType::Float(FloatWidth::Double))
                    .unwrap()
                    .default(ColumnDefault::Float(1.5))
                    .unwrap(),
                Column::new(
                    "money",
                    ScalarType::Decimal {
                        precision: 10,
                        scale: 2,
                    },
                )
                .unwrap()
                .default(ColumnDefault::Decimal("10.50".to_owned()))
                .unwrap(),
                Column::new("label", ScalarType::Text { max: Some(200) })
                    .unwrap()
                    .default(ColumnDefault::Text("it's \\ fine".to_owned()))
                    .unwrap(),
                Column::new("body", ScalarType::Text { max: None })
                    .unwrap()
                    .null(),
                Column::new("thumb", ScalarType::Bytes { max: Some(64) })
                    .unwrap()
                    .null(),
                Column::new("payload", ScalarType::Bytes { max: None })
                    .unwrap()
                    .null(),
                Column::new("live", ScalarType::Bool)
                    .unwrap()
                    .default(ColumnDefault::Bool(true))
                    .unwrap(),
                Column::new("day", ScalarType::Date).unwrap(),
                Column::new("clock", ScalarType::Time).unwrap(),
                Column::new("seen_at", ScalarType::DateTime)
                    .unwrap()
                    .default(ColumnDefault::Now)
                    .unwrap(),
                Column::new("happened_at", ScalarType::Instant)
                    .unwrap()
                    .default(ColumnDefault::Now)
                    .unwrap(),
                Column::new("token", ScalarType::Uuid).unwrap(),
                Column::new("doc", ScalarType::Json).unwrap().null(),
            ],
        )
        .unwrap()
        .primary_key(&["id"])
        .unwrap()
        .unique("wide_label_key", &["label"])
        .unwrap()
        .index("wide_body_idx", &["body"])
        .unwrap()
        .index("wide_day_clock_idx", &["day", "clock"])
        .unwrap();

        let link = Table::new(
            "link",
            vec![
                Column::new("left_id", ScalarType::Int(IntWidth::Big)).unwrap(),
                Column::new("right_id", ScalarType::Int(IntWidth::Big)).unwrap(),
            ],
        )
        .unwrap()
        .primary_key(&["left_id", "right_id"])
        .unwrap();

        Schema::new(vec![wide, link]).unwrap()
    }

    /// How each dialect says a column is the table's identity.
    fn identity_marker(dialect: Dialect) -> &'static str {
        identity(dialect).unwrap_or("AUTOINCREMENT")
    }

    /// `rule:core-classes/schema-plan`: every construct the vocabulary has emits in all four
    /// dialects.
    ///
    /// Asserted as an **invariance over a sweep** and not as four expected
    /// texts: the counts are derived from the schema value, so a construct that
    /// silently emits nothing fails here, where a golden text would only fail
    /// on the line someone remembered to write. The type spellings are checked
    /// by a property rather than by a table of expected strings — each is upper
    /// case, and each differs from [`ScalarType`]'s canonical `Display`, which
    /// is the one wrong answer that would look plausible in every position.
    #[test]
    fn every_construct_in_the_vocabulary_emits_in_all_four_dialects() {
        let schema = every_construct();
        let wide = &schema.tables()[0];

        for dialect in DIALECTS {
            let statements = create_schema(&schema, dialect);
            // Two tables, and two indexes that are separate statements
            // everywhere but MySQL.
            let wanted = if dialect == Dialect::MySql { 2 } else { 4 };
            assert_eq!(statements.len(), wanted, "{dialect:?} statement count");
            for statement in &statements {
                assert!(statement.starts_with("CREATE "), "{statement}");
                assert!(statement.ends_with(';'), "{statement} is not terminated");
            }

            let create = &statements[0];
            let clauses: Vec<&str> = create.lines().map(str::trim).collect();
            // One line per column and one per key, between the two lines the
            // statement itself is made of: a construct that emitted nothing
            // leaves this short rather than looking plausible further down.
            let keys = usize::from(dialect != Dialect::Sqlite)
                + wide.unique_keys().len()
                + if dialect == Dialect::MySql {
                    wide.indexes().len()
                } else {
                    0
                };
            assert_eq!(
                clauses.len(),
                2 + wide.columns().len() + keys,
                "{dialect:?} clause count"
            );

            for column in wide.columns() {
                let clause = clauses
                    .iter()
                    .find(|line| {
                        line.starts_with(&format!(
                            "{} ",
                            as_written(column.name().as_str(), dialect)
                        ))
                    })
                    .unwrap_or_else(|| panic!("{dialect:?} has no clause for `{}`", column.name()));
                let sql = column_type(column.ty(), dialect);
                assert!(!sql.is_empty(), "{:?} has no spelling", column.ty());
                assert_eq!(
                    sql,
                    sql.to_uppercase(),
                    "{sql} is not written in SQL's case"
                );
                assert_ne!(
                    sql,
                    column.ty().to_string(),
                    "{sql} is the canonical spelling, not the SQL one"
                );
                // SQLite's identity is `INTEGER PRIMARY KEY AUTOINCREMENT` and
                // carries no separate type, no `NOT NULL` and no default.
                let rowid = dialect == Dialect::Sqlite && column.is_identity();
                if !rowid {
                    assert!(clause.contains(&sql), "{dialect:?} does not write {sql}");
                }
                // PostgreSQL's identity spelling carries a `DEFAULT` of its
                // own, and it is not this column's default.
                let bare = clause.replace(identity_marker(dialect), "");
                assert_eq!(
                    bare.contains(" DEFAULT "),
                    column.default_value().is_some(),
                    "{dialect:?} default on `{}`",
                    column.name()
                );
                assert_eq!(
                    bare.contains("NOT NULL"),
                    !column.is_nullable() && !rowid,
                    "{dialect:?} NOT NULL on `{}`",
                    column.name()
                );
            }

            assert!(
                create.contains(identity_marker(dialect)),
                "{dialect:?} does not write an identity"
            );
            assert!(create.contains("PRIMARY KEY"), "{dialect:?} has no key");
            assert!(
                create.contains(&format!(
                    "CONSTRAINT {} UNIQUE ({})",
                    as_written("wide_label_key", dialect),
                    as_written("label", dialect)
                )),
                "{dialect:?} has no unique constraint"
            );
            assert!(
                statements.last().unwrap().contains(&format!(
                    "PRIMARY KEY ({}, {})",
                    as_written("left_id", dialect),
                    as_written("right_id", dialect)
                )),
                "{dialect:?} has no composite key"
            );

            // The unsigned columns: MySQL has the type, the other three take
            // the next width up and a check.
            if dialect == Dialect::MySql {
                assert_eq!(create.matches(" UNSIGNED").count(), 3);
                assert!(!create.contains("CHECK"));
            } else {
                assert_eq!(create.matches("CHECK (").count(), 3, "{dialect:?} checks");
                assert!(!create.contains("UNSIGNED"), "{dialect:?} has no such type");
            }

            // A text default is quoted in the dialect's own escaping, which is
            // the one string in this schema that reaches statement text.
            let quoted = match dialect {
                Dialect::MySql => "'it''s \\\\ fine'",
                Dialect::SqlServer => "N'it''s \\ fine'",
                _ => "'it''s \\ fine'",
            };
            assert!(create.contains(quoted), "{dialect:?} escaping");

            // `Now` on a zone-carrying column, where SQL Server's shared
            // spelling is the wrong clock — [`literal`]'s doc says why.
            let now = if dialect == Dialect::SqlServer {
                "SYSDATETIMEOFFSET()"
            } else {
                "CURRENT_TIMESTAMP"
            };
            assert!(
                create.contains(&format!(
                    "{} {} NOT NULL DEFAULT {now}",
                    as_written("happened_at", dialect),
                    column_type(&ScalarType::Instant, dialect)
                )),
                "{dialect:?} defaults an instant to the wrong clock"
            );

            // Every index is declared exactly once, wherever this dialect puts
            // it.
            for key in wide.indexes() {
                assert_eq!(
                    statements
                        .iter()
                        .filter(|statement| statement.contains(key.name().as_str()))
                        .count(),
                    1,
                    "{dialect:?} declares {} more than once",
                    key.name()
                );
            }
        }
    }

    /// `rule:core-classes/schema-plan`: every identifier the emitter writes is
    /// delimited for its dialect, so a name a backend reserves is still the
    /// name the schema gave it.
    ///
    /// **A property over the sweep rather than a list of quoted strings**:
    /// every statement the vocabulary emits, in every dialect, is searched for
    /// every name the fixtures use, and each occurrence that stands as a word
    /// of its own has to sit between that dialect's delimiters. An emitter that
    /// quoted the table and forgot one key column reads plausibly on every line
    /// a golden text would have checked.
    #[test]
    fn every_identifier_the_emitter_writes_is_delimited_for_its_dialect() {
        for dialect in DIALECTS {
            let (open, _) = delimiters(dialect);
            let mut statements = create_schema(&every_construct(), dialect);
            for change in every_change() {
                statements.extend(step(change, dialect).sql().to_vec());
            }
            for statement in &statements {
                assert!(
                    statement.contains(open),
                    "{dialect:?} wrote a statement naming nothing: {statement}"
                );
                for name in fixture_identifiers() {
                    assert!(
                        !written_bare(statement, &name, dialect),
                        "{dialect:?} writes `{name}` bare: {statement}"
                    );
                }
            }

            // The case the gap named: a table and two columns every backend has
            // a keyword for, which the vocabulary accepts and no emitter may
            // hand to a parser bare.
            let reserved = Schema::new(vec![
                Table::new(
                    "order",
                    vec![
                        Column::new("rank", ScalarType::Int(IntWidth::Normal)).unwrap(),
                        Column::new("select", ScalarType::Text { max: Some(10) }).unwrap(),
                    ],
                )
                .unwrap()
                .primary_key(&["rank"])
                .unwrap()
                .index("order_select_idx", &["select"])
                .unwrap(),
            ])
            .unwrap();
            for statement in create_schema(&reserved, dialect) {
                for name in ["order", "rank", "select", "order_select_idx"] {
                    assert!(
                        !written_bare(&statement, name, dialect),
                        "{dialect:?} writes the reserved `{name}` bare: {statement}"
                    );
                }
            }
        }
    }

    /// `rule:core-classes/schema-plan`: the emitters follow `Dialect`, and five drivers are four
    /// texts.
    ///
    /// An **agreement** rather than five expected outputs: MariaDB earns no
    /// emitter of its own, and the assertion that would fail if it grew one by
    /// accident is that its text and MySQL's are the same bytes. The distinct
    /// count is the other half — four texts, so no two dialects have quietly
    /// converged either.
    #[test]
    fn the_emitters_follow_dialect_rather_than_driver() {
        let schema = every_construct();
        let written = [
            Driver::Postgres,
            Driver::MySql,
            Driver::MariaDb,
            Driver::SqlServer,
            Driver::Sqlite,
        ]
        .map(|driver| create_schema(&schema, Dialect::of(driver)));

        assert_eq!(
            written[1], written[2],
            "MySQL and MariaDB share their SQL text exactly"
        );

        let mut distinct: Vec<&Vec<String>> = Vec::new();
        for text in &written {
            if !distinct.contains(&text) {
                distinct.push(text);
            }
        }
        assert_eq!(distinct.len(), 4, "five drivers, four dialects");
    }

    /// `rule:core-classes/schema-plan`: one of MySQL's departures — the index is inside the
    /// `CREATE TABLE`.
    ///
    /// The reason is in [`create_table`]'s doc and it is not style: MySQL has
    /// neither `CREATE INDEX IF NOT EXISTS` nor transactional DDL, so a table
    /// and its indexes have to arrive in one statement or there is a half-built
    /// state no re-run can complete.
    #[test]
    fn mysql_declares_an_index_inside_its_create_table_having_no_if_not_exists() {
        let schema = every_construct();
        let mysql = create_schema(&schema, Dialect::MySql);

        assert_eq!(mysql.len(), 2, "one statement per table");
        assert!(mysql[0].contains("INDEX `wide_body_idx` ("));
        assert!(mysql[0].contains("INDEX `wide_day_clock_idx` (`day`, `clock`)"));
        assert!(
            !mysql
                .iter()
                .any(|statement| statement.contains("IF NOT EXISTS")),
            "MySQL has no such guard to write"
        );
        assert!(
            !mysql
                .iter()
                .any(|statement| statement.starts_with("CREATE INDEX")),
            "MySQL declares no index outside a table"
        );

        for dialect in [Dialect::PostgreSql, Dialect::Sqlite, Dialect::SqlServer] {
            let statements = create_schema(&schema, dialect);
            assert!(
                statements.iter().any(|statement| {
                    *statement
                        == format!(
                            "CREATE INDEX {} ON {} ({});",
                            as_written("wide_body_idx", dialect),
                            as_written("wide", dialect),
                            as_written("body", dialect)
                        )
                }),
                "{dialect:?} declares an index of its own"
            );
        }
    }

    /// `rule:core-classes/schema-plan`: another of MySQL's departures — an indexed unbounded
    /// text column carries a prefix length.
    ///
    /// A **bound asserted on both sides**: the `LONGTEXT` column takes the
    /// prefix and the `VARCHAR(200)` beside it does not, so an emitter that
    /// wrote one on every text column would fail here while still looking right
    /// on the line that matters.
    #[test]
    fn mysql_gives_an_indexed_text_column_a_prefix_length() {
        let schema = every_construct();
        let mysql = create_schema(&schema, Dialect::MySql);

        assert!(
            mysql[0].contains(&format!(
                "INDEX `wide_body_idx` (`body`({TEXT_KEY_PREFIX}))"
            )),
            "MySQL cannot index a LONGTEXT column whole"
        );
        assert!(
            mysql[0].contains("CONSTRAINT `wide_label_key` UNIQUE (`label`)"),
            "a VARCHAR(200) is indexable as it stands"
        );

        for dialect in [Dialect::PostgreSql, Dialect::Sqlite, Dialect::SqlServer] {
            assert!(
                !create_schema(&schema, dialect)
                    .iter()
                    .any(|statement| statement
                        .contains(&format!("{}(", as_written("body", dialect)))),
                "{dialect:?} indexes a text column whole"
            );
        }
    }

    /// A table carrying both spellings of a unique key: one over a nullable
    /// column and one over a column that is `not null`.
    ///
    /// The queue's `dedupe_pending` is the first of them
    /// (`rule:core-classes/queue-storage-is-a-table`), and the second is what
    /// makes every assertion below a bound on both sides: an emitter that
    /// filtered *every* unique key would satisfy each one that named only the
    /// nullable column.
    fn both_unique_spellings() -> Table {
        Table::new(
            "jobs",
            vec![
                Column::new("id", ScalarType::Int(IntWidth::Big))
                    .unwrap()
                    .identity()
                    .unwrap(),
                Column::new("dedupe_pending", ScalarType::Text { max: Some(255) })
                    .unwrap()
                    .null(),
                Column::new("token", ScalarType::Uuid).unwrap(),
            ],
        )
        .unwrap()
        .primary_key(&["id"])
        .unwrap()
        .unique("jobs_dedupe", &["dedupe_pending"])
        .unwrap()
        .unique("jobs_token", &["token"])
        .unwrap()
    }

    /// The step that adds one of that table's unique keys after the fact.
    fn add_unique(table: &Table, at: usize, dialect: Dialect) -> Step {
        step(
            Change::AddKey {
                table: table.clone(),
                key: table.unique_keys()[at].clone(),
                kind: KeyKind::Unique,
            },
            dialect,
        )
    }

    /// `rule:core-classes/a-unique-key-reads-nulls-as-distinct`: SQL Server's unique key over a
    /// nullable column is a filtered index, at both doors.
    ///
    /// The constraint form is the one spelling of the four that reads that
    /// column's nulls as equal, so a second null row is refused where the
    /// standard and the other three admit it. Both doors are asserted because
    /// the two are written apart — a clause inside [`create_table_statement`]
    /// and a statement in [`add_key`] — and a key that converged one way and
    /// not the other would be a database whose plan never empties.
    #[test]
    fn a_unique_key_over_a_nullable_column_is_a_filtered_index_on_sql_server() {
        let table = both_unique_spellings();
        const FILTERED: &str = "CREATE UNIQUE INDEX [jobs_dedupe] ON [jobs] ([dedupe_pending]) \
                                WHERE [dedupe_pending] IS NOT NULL;";

        let statements = create_table(&table, Dialect::SqlServer);
        assert!(
            !statements[0].contains("jobs_dedupe"),
            "the nullable key is not a clause of the table: {}",
            statements[0]
        );
        assert!(
            statements[0].contains("CONSTRAINT [jobs_token] UNIQUE ([token])"),
            "a NOT NULL column keeps the constraint form: {}",
            statements[0]
        );
        assert!(
            statements.iter().any(|statement| statement == FILTERED),
            "{statements:?}"
        );

        let filtered = add_unique(&table, 0, Dialect::SqlServer);
        assert_eq!(filtered.sql(), [FILTERED]);
        // § 6 grades the dialect's spelling as the construct: an index built
        // over a table that already holds rows is what a constraint costs
        // there, and no more.
        assert_eq!(filtered.grade(), Grade::Locking);
        assert_eq!(
            add_unique(&table, 1, Dialect::SqlServer).sql(),
            ["ALTER TABLE [jobs] ADD CONSTRAINT [jobs_token] UNIQUE ([token]);"]
        );

        for dialect in [Dialect::PostgreSql, Dialect::MySql, Dialect::Sqlite] {
            let written = create_table(&table, dialect);
            assert!(
                written[0].contains(&format!(
                    "CONSTRAINT {} UNIQUE ({})",
                    as_written("jobs_dedupe", dialect),
                    as_written("dedupe_pending", dialect)
                )),
                "{dialect:?} needs no filter for a nullable unique key: {}",
                written[0]
            );
            assert!(
                !written
                    .iter()
                    .chain(add_unique(&table, 0, dialect).sql())
                    .any(|statement| statement.contains("IS NOT NULL")),
                "{dialect:?} wrote a predicate it does not need"
            );
        }
    }

    /// The name every alter fixture below is about.
    fn wide_name() -> Ident {
        Ident::new("wide").unwrap()
    }

    /// [`every_construct`]'s `wide`, with `extra` appended and `dropped`
    /// removed — the **after** shape a [`Change`] carries, by that enum's own
    /// convention.
    ///
    /// Only `thumb` is ever dropped here, because it is the one column of the
    /// fixture that no key names and the builder refuses a key over a column
    /// that is not there.
    fn wide_after(extra: Option<Column>, dropped: Option<&str>) -> Table {
        let schema = every_construct();
        let wide = schema.table(&wide_name()).unwrap();
        let mut columns: Vec<Column> = wide
            .columns()
            .iter()
            .filter(|column| dropped.is_none_or(|name| column.name().as_str() != name))
            .cloned()
            .collect();
        columns.extend(extra);
        Table::new("wide", columns)
            .unwrap()
            .primary_key(&["id"])
            .unwrap()
            .unique("wide_label_key", &["label"])
            .unwrap()
            .index("wide_body_idx", &["body"])
            .unwrap()
            .index("wide_day_clock_idx", &["day", "clock"])
            .unwrap()
    }

    /// `label` as the fixture has it, which every `ChangeColumn` below starts
    /// from.
    fn label_column() -> Column {
        every_construct()
            .table(&wide_name())
            .unwrap()
            .column(&Ident::new("label").unwrap())
            .unwrap()
            .clone()
    }

    /// A `label` of `max` characters, keeping the default the fixture gave it.
    fn label_of(max: u32) -> Column {
        Column::new("label", ScalarType::Text { max: Some(max) })
            .unwrap()
            .default(ColumnDefault::Text("it's \\ fine".to_owned()))
            .unwrap()
    }

    /// One change of every variant the vocabulary has, including every shape
    /// of `AddColumn` and both ends of a type change.
    ///
    /// The § 7 reports among them — a dropped table, column, unique key and
    /// index — are here rather than in a fixture of their own because the
    /// property under test is that they are written in full *alongside* the
    /// steps that will run.
    fn every_change() -> Vec<Change> {
        let plain = wide_after(None, None);
        let note = Column::new("note", ScalarType::Text { max: Some(40) })
            .unwrap()
            .null();
        let owner = Column::new("owner", ScalarType::Int(IntWidth::Big)).unwrap();
        let touched = Column::new("touched_at", ScalarType::DateTime)
            .unwrap()
            .default(ColumnDefault::Now)
            .unwrap();

        vec![
            Change::CreateTable(plain.clone()),
            Change::DropTable(wide_name()),
            Change::AddColumn {
                table: wide_after(Some(note.clone()), None),
                column: note,
            },
            Change::AddColumn {
                table: wide_after(Some(owner.clone()), None),
                column: owner,
            },
            Change::AddColumn {
                table: wide_after(Some(touched.clone()), None),
                column: touched,
            },
            Change::DropColumn {
                table: wide_after(None, Some("thumb")),
                column: Ident::new("thumb").unwrap(),
            },
            Change::ChangeColumn {
                table: wide_after(Some(label_of(400)), Some("label")),
                from: label_column(),
                to: label_of(400),
            },
            Change::ChangeColumn {
                table: wide_after(Some(label_of(20)), Some("label")),
                from: label_column(),
                to: label_of(20),
            },
            Change::AddKey {
                table: plain.clone(),
                key: plain.unique_keys()[0].clone(),
                kind: KeyKind::Unique,
            },
            Change::AddKey {
                table: plain.clone(),
                key: plain.indexes()[0].clone(),
                kind: KeyKind::Index,
            },
            Change::DropKey {
                table: plain.clone(),
                key: Ident::new("wide_gone_key").unwrap(),
                kind: KeyKind::Unique,
            },
            Change::DropKey {
                table: plain,
                key: Ident::new("wide_gone_idx").unwrap(),
                kind: KeyKind::Index,
            },
        ]
    }

    /// § 8: every step's SQL is complete, terminated and dialect-correct — and
    /// the four steps the applier refuses are written out in the same detail
    /// as the eight it would run.
    ///
    /// **Invariance over a sweep**: asserted by walking every change in the
    /// vocabulary in every dialect and *counting*, rather than by reading one
    /// expected statement off a line. An emitter that elided a report — the
    /// natural shortcut, since nothing will ever run it — answers plausibly
    /// statement by statement and fails the count here.
    #[test]
    fn every_step_carries_terminated_executable_sql_including_the_ones_apply_refuses() {
        for dialect in DIALECTS {
            let plan = Plan::new(
                every_change()
                    .into_iter()
                    .map(|change| step(change, dialect))
                    .collect(),
            );
            assert_eq!(plan.len(), 12, "{dialect:?} lost a change");

            for taken in plan.steps() {
                let where_ = format!("{dialect:?} {}", taken.change());
                assert!(!taken.sql().is_empty(), "{where_} carries no SQL at all");
                for statement in taken.sql() {
                    assert!(
                        statement.ends_with(';'),
                        "{where_}: `{statement}` is unterminated"
                    );
                    assert_eq!(
                        statement.matches(';').count(),
                        1,
                        "{where_}: `{statement}` is more than one statement in one string"
                    );
                    assert!(
                        statement.starts_with("CREATE ")
                            || statement.starts_with("ALTER ")
                            || statement.starts_with("DROP ")
                            || statement.starts_with("INSERT "),
                        "{where_}: `{statement}` does not open with a DDL verb"
                    );
                }
                assert!(
                    taken
                        .sql()
                        .concat()
                        .contains(taken.change().table_name().as_str()),
                    "{where_} does not name the table it changes"
                );
                assert!(
                    taken.reason().ends_with('.'),
                    "{where_} grades without a reason an operator can read"
                );
            }

            // § 7's four reports, each Destructive, each written out in full.
            let reports: Vec<&Step> = plan
                .steps()
                .iter()
                .filter(|step| step.is_report())
                .collect();
            assert_eq!(reports.len(), 4, "{dialect:?} lost a report");
            assert_eq!(reports.len() + plan.runnable().count(), plan.len());
            for report in reports {
                assert_eq!(report.grade(), Grade::Destructive);
                assert!(!report.sql().is_empty(), "a report the plan did not show");
            }

            // And the reason that distinction exists: reports are Destructive,
            // every plan against a shared database has them, and `applySafe`
            // has to stay usable anyway.
            let carried = Plan::new(
                every_change()
                    .into_iter()
                    .filter(Change::is_report)
                    .map(|change| step(change, dialect))
                    .collect(),
            );
            assert_eq!(carried.count(Grade::Destructive), 4);
            assert!(
                carried.first_refused().is_none(),
                "{dialect:?}: a plan of nothing but reports refused applySafe"
            );
            assert!(
                plan.first_refused().is_some(),
                "{dialect:?}: a plan that adds a NOT NULL column ran under applySafe"
            );

            // The document itself, which is what a DBA is handed.
            let document = plan.to_string();
            assert!(
                document.contains("-- [destructive] drop table wide, reported and never applied")
            );
            assert!(document.contains("-- [safe] create table wide\n"));
            for taken in plan.steps() {
                for statement in taken.sql() {
                    assert!(
                        document.contains(statement),
                        "the document elided a statement"
                    );
                }
            }
        }
    }

    /// The standing decision SQLite forces: every alter its own `ALTER TABLE`
    /// cannot express is a create-copy-drop-rename, graded `Destructive`
    /// unconditionally because it copies every row.
    ///
    /// **Agreement**, over the whole vocabulary: [`sqlite_rebuilds`] grades a
    /// change and [`sql_for`]'s arms emit it, and the two read the same
    /// judgement in two places. The sweep asserts they answer the same, so a
    /// case that grew a rebuild in one half and not the other fails here while
    /// looking right on its own line.
    #[test]
    fn sqlite_rebuilds_the_table_for_an_alter_it_cannot_express() {
        let mut rebuilt = 0;
        for dialect in DIALECTS {
            for change in every_change() {
                let graded = sqlite_rebuilds(&change, dialect);
                let taken = step(change, dialect);
                let emitted = taken
                    .sql()
                    .iter()
                    .any(|statement| statement.contains("wide_nvs_rebuild"));
                assert_eq!(
                    graded,
                    emitted,
                    "{dialect:?} {}: the grade and the SQL disagree about rebuilding",
                    taken.change()
                );
                if emitted {
                    rebuilt += 1;
                    assert_eq!(dialect, Dialect::Sqlite, "only SQLite rebuilds");
                    assert_eq!(
                        taken.grade(),
                        Grade::Destructive,
                        "a rebuild copies every row"
                    );
                }
            }
        }
        assert_eq!(
            rebuilt, 6,
            "six of the twelve changes are past SQLite's ALTER"
        );

        // The steps of the rebuild, in the order that survives an index name
        // being unique across the database.
        let taken = step(
            Change::DropColumn {
                table: wide_after(None, Some("thumb")),
                column: Ident::new("thumb").unwrap(),
            },
            Dialect::Sqlite,
        );
        let sql = taken.sql();
        assert_eq!(sql.len(), 6, "create, copy, drop, rename and two indexes");
        assert!(sql[0].starts_with("CREATE TABLE \"wide_nvs_rebuild\" ("));
        assert!(sql[1].starts_with("INSERT INTO \"wide_nvs_rebuild\" (\"id\", \"small_int\""));
        assert!(sql[1].ends_with("FROM \"wide\";"));
        assert!(
            !sql[1].contains("thumb"),
            "the dropped column has nowhere to go"
        );
        assert_eq!(sql[2], "DROP TABLE \"wide\";");
        assert_eq!(
            sql[3],
            "ALTER TABLE \"wide_nvs_rebuild\" RENAME TO \"wide\";"
        );
        assert!(
            sql[4..]
                .iter()
                .all(|statement| statement.starts_with("CREATE INDEX ")
                    && statement.contains(" ON \"wide\" (")),
            "the indexes went with the table the drop took, and come back named"
        );

        // A column being added has no value in the old table to copy, so the
        // rebuild declares it and does not select it.
        let owner = Column::new("owner", ScalarType::Int(IntWidth::Big)).unwrap();
        let added = step(
            Change::AddColumn {
                table: wide_after(Some(owner.clone()), None),
                column: owner,
            },
            Dialect::Sqlite,
        );
        assert!(added.sql()[0].contains("\"owner\" BIGINT NOT NULL"));
        assert!(
            !added.sql()[1].contains("owner"),
            "there is no value to copy"
        );

        // The same change is one statement everywhere else, and is not
        // Destructive: the rebuild is SQLite's price, not the change's.
        for dialect in [Dialect::PostgreSql, Dialect::MySql, Dialect::SqlServer] {
            let elsewhere = step(
                Change::ChangeColumn {
                    table: wide_after(Some(label_of(400)), Some("label")),
                    from: label_column(),
                    to: label_of(400),
                },
                dialect,
            );
            assert_eq!(
                elsewhere.grade(),
                Grade::Locking,
                "{dialect:?} widened a column"
            );
            assert!(!elsewhere.sql().concat().contains("_nvs_rebuild"));
        }
    }

    /// § 6's `Safe`, which is the only grade `applySafe` will run unasked.
    ///
    /// All four agree, SQLite included: a nullable column with no default is
    /// the one add that dialect can express as an alter ([`sqlite_can_add`]),
    /// so it is the one that does not become a rebuild and grade up.
    #[test]
    fn adding_a_nullable_column_is_safe() {
        let owner = Column::new("owner", ScalarType::Int(IntWidth::Big))
            .unwrap()
            .null();
        for dialect in DIALECTS {
            let added = step(
                Change::AddColumn {
                    table: wide_after(Some(owner.clone()), None),
                    column: owner.clone(),
                },
                dialect,
            );
            assert_eq!(added.grade(), Grade::Safe, "{dialect:?}");
            assert!(
                !added.sql().concat().contains("_nvs_rebuild"),
                "{dialect:?} rebuilt the table for a column every row already has"
            );
        }
    }

    /// § 6's `Locking`, at the case that gives the grade its name: a unique key
    /// is validated against every row that is already there.
    ///
    /// Both kinds, because an index earns the same grade for the *other* half
    /// of the reason — it is built rather than validated, and v1 emits no
    /// concurrent build — and a grader that collapsed the two would still print
    /// plausibly on either line alone.
    #[test]
    fn a_unique_index_over_existing_rows_is_locking() {
        let table = wide_after(None, None);
        let unique = table.unique_keys()[0].clone();
        let plain = table.indexes()[0].clone();
        for dialect in DIALECTS {
            let validated = step(
                Change::AddKey {
                    table: table.clone(),
                    key: unique.clone(),
                    kind: KeyKind::Unique,
                },
                dialect,
            );
            assert_eq!(validated.grade(), Grade::Locking, "{dialect:?}");
            assert!(
                validated.reason().contains("collide"),
                "{dialect:?}: {}",
                validated.reason()
            );
            let built = step(
                Change::AddKey {
                    table: table.clone(),
                    key: plain.clone(),
                    kind: KeyKind::Index,
                },
                dialect,
            );
            assert_eq!(built.grade(), Grade::Locking, "{dialect:?}");
            assert!(
                built.reason().contains("holds a lock"),
                "{dialect:?}: {}",
                built.reason()
            );
        }
    }

    /// § 6's `Locking` at its other end: `NOT NULL` is checked against every
    /// existing row and fails on the first null, whether it arrives as a new
    /// column or as a tightening of one already there.
    ///
    /// SQLite grades *up* rather than down, and the two claims are one test
    /// because that is the whole shape of § 6 — the same change is not the same
    /// risk on two backends. It can express neither of these as an alter, so
    /// both are a create-copy-drop-rename and `Destructive` unconditionally.
    #[test]
    fn not_null_on_a_populated_column_is_locking() {
        let owner = Column::new("owner", ScalarType::Int(IntWidth::Big)).unwrap();
        let tightened = Column::new("body", ScalarType::Text { max: None }).unwrap();
        for dialect in DIALECTS {
            let rebuilds = dialect == Dialect::Sqlite;
            let want = if rebuilds {
                Grade::Destructive
            } else {
                Grade::Locking
            };
            let added = step(
                Change::AddColumn {
                    table: wide_after(Some(owner.clone()), None),
                    column: owner.clone(),
                },
                dialect,
            );
            assert_eq!(added.grade(), want, "{dialect:?}");
            let changed = step(
                Change::ChangeColumn {
                    table: wide_after(Some(tightened.clone()), Some("body")),
                    from: Column::new("body", ScalarType::Text { max: None })
                        .unwrap()
                        .null(),
                    to: tightened.clone(),
                },
                dialect,
            );
            assert_eq!(changed.grade(), want, "{dialect:?}");
            assert!(
                changed.reason().contains("fails on the first null"),
                "{dialect:?}: {}",
                changed.reason()
            );
        }
    }

    /// § 6's `Destructive` at the two changes that reach it for different
    /// reasons: one because § 7 makes every removal a report, one because the
    /// new type does not hold every value the old one did.
    ///
    /// The pair also separates the grade from the report: they share a grade
    /// and only one of them is carried and never applied.
    #[test]
    fn dropping_a_column_and_narrowing_a_type_are_destructive() {
        for dialect in DIALECTS {
            let dropped = step(
                Change::DropColumn {
                    table: wide_after(None, Some("thumb")),
                    column: Ident::new("thumb").unwrap(),
                },
                dialect,
            );
            assert_eq!(dropped.grade(), Grade::Destructive, "{dialect:?}");
            assert!(dropped.is_report(), "{dialect:?}");
            let narrowed = step(
                Change::ChangeColumn {
                    table: wide_after(Some(label_of(40)), Some("label")),
                    from: label_column(),
                    to: label_of(40),
                },
                dialect,
            );
            assert_eq!(narrowed.grade(), Grade::Destructive, "{dialect:?}");
            assert!(
                !narrowed.is_report(),
                "{dialect:?} would not apply a change the operator asked for"
            );
        }
    }

    /// § 6's version-keyed half, as the claim that a grade is the *server's*
    /// and not the loaded driver's.
    ///
    /// Adding a column with a default is a catalog write on PostgreSQL 11+ and
    /// MySQL 8.0.12+ and a full table rewrite on anything older. A sans-io
    /// emitter knows the dialect and not the version, so all four grade up to
    /// `Locking` and the reason names the versions rather than asking to be
    /// believed. The answer PostgreSQL alone would have earned is `Safe`, and
    /// taking it on the driver's word is the optimism § 6 refuses.
    #[test]
    fn an_added_default_grades_on_the_servers_version_and_not_the_driver_alone() {
        let owner = Column::new("owner", ScalarType::Int(IntWidth::Big))
            .unwrap()
            .null()
            .default(ColumnDefault::Int(0))
            .unwrap();
        for dialect in DIALECTS {
            let added = step(
                Change::AddColumn {
                    table: wide_after(Some(owner.clone()), None),
                    column: owner.clone(),
                },
                dialect,
            );
            assert_eq!(added.grade(), Grade::Locking, "{dialect:?}");
            assert!(
                added.reason().contains("11+") && added.reason().contains("8.0.12+"),
                "{dialect:?} graded on the driver alone: {}",
                added.reason()
            );
        }
    }

    /// § 6's grade-up rule, at both places it is spent.
    ///
    /// [`Grade::up_to`] is the operation, and what it must be is a maximum over
    /// the three — asserted over the whole nine-pair table rather than on a
    /// line, so an ordering that grew a case still fails here. [`widens`]'s
    /// default arm is the rule applied: a pair of types it has no rule for is
    /// **not** a widening, so a date respelled as text is `Destructive` and an
    /// operator is asked to confirm something that may have been free. The
    /// reverse mistake costs them the column.
    #[test]
    fn a_grade_the_emitter_cannot_determine_grades_up() {
        let grades = [Grade::Safe, Grade::Locking, Grade::Destructive];
        for one in grades {
            for other in grades {
                assert_eq!(one.up_to(other), one.max(other));
                assert_eq!(one.up_to(other), other.up_to(one));
            }
        }
        let spelled = Column::new("day", ScalarType::Text { max: Some(10) }).unwrap();
        for dialect in DIALECTS {
            let crossed = step(
                Change::ChangeColumn {
                    table: wide_after(Some(spelled.clone()), Some("day")),
                    from: Column::new("day", ScalarType::Date).unwrap(),
                    to: spelled.clone(),
                },
                dialect,
            );
            assert_eq!(crossed.grade(), Grade::Destructive, "{dialect:?}");
            assert!(
                crossed.reason().contains("grades up"),
                "{dialect:?} took a guess instead: {}",
                crossed.reason()
            );
        }
    }
}
