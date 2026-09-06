//! [ADR 0145](/docs/adr/0145-a-schema-is-a-value-core-db-schema-converges-a-closed.md)
//! § 8's DDL: a [`Schema`] value as SQL an operator could paste.
//!
//! Every statement this module returns is **complete, terminated and
//! dialect-correct**, including the ones the applier will refuse to run. § 8's
//! whole argument for that is the deployment where the application's own
//! credentials cannot issue DDL at all and a DBA runs the change from a ticket:
//! a plan whose risky steps are elided into "3 unsafe changes" is useless to
//! that person.
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
//! - **An identifier is validated, never delimited** — a [`Ident`] is ASCII
//!   letters, digits and `_` within [`MAX_IDENTIFIER`](crate::MAX_IDENTIFIER),
//!   so it is written bare and carries nothing into the statement it lands in.
//!   The cost of that judgement is that a table named `order` is not sayable,
//!   which is the trade
//!   [ADR 0024](/docs/adr/0024-taint-tracking-for-injection-sinks.md) already
//!   made for `Core\Db::quoteIdentifier`.
//! - **A literal is one of [`ColumnDefault`]'s seven cases**, each with a
//!   spelling per dialect. There are no expression defaults, so the one place a
//!   string reaches statement text — [`ColumnDefault::Text`] — is quoted here,
//!   in the dialect's own escaping.
//!
//! # Known gaps
//!
//! 1. **Three round-trips are lossy, and § 5's normalization owns them, not
//!    this module.** PostgreSQL has one `BYTEA` for both
//!    [`ScalarType::Bytes`] widths; MySQL has no `UUID` type and takes
//!    `CHAR(36)`; SQL Server has no JSON type and takes `NVARCHAR(MAX)`, which
//!    is also its unbounded text. An `uint` is a fourth: only MySQL has the
//!    type, so the other three take the next width up and a `CHECK`, and no
//!    catalog reports the check as a type. Each is a column an introspector
//!    reads back as a *different* vocabulary case, and the diff has to know it
//!    before it converges.
//! 2. **SQL Server cannot index an unbounded text or bytes column at all** — a
//!    `MAX` type is not a key column there. The statement is still emitted,
//!    because § 8 says a step is shown in full even when it will not run; § 5's
//!    grading is where it becomes a refusal rather than a failure.
//! 3. **SQLite's identity is the rowid or nothing.** `AUTOINCREMENT` is legal
//!    only in the exact `INTEGER PRIMARY KEY AUTOINCREMENT` form, so an
//!    identity column that is one of several primary-key columns has no SQLite
//!    spelling and is emitted without it. [`Table`]'s own rule is only that an
//!    identity is *in* the primary key.
//! 4. **SQLite's declared types carry affinity, and two of them convert.** A
//!    `JSON` or `DECIMAL` column has NUMERIC affinity, so a document that is a
//!    bare number and an exact decimal with trailing zeros are stored as
//!    numbers. That is the SQLite driver's binding question — the same one
//!    `DECIMAL` already poses there — and it is not answered by choosing a
//!    different declared type here, which would cost the round-trip in gap 1.

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
/// One statement on MySQL and one plus an index on the other three, which is
/// the departure `mysql_declares_an_index_inside_its_create_table_having_no_if_not_exists`
/// pins. MySQL has no `CREATE INDEX IF NOT EXISTS` and no transactional DDL, so
/// a table created by one statement and indexed by a second has a half-built
/// state that no re-run can complete; declaring the index inside the
/// `CREATE TABLE` means the table and its indexes arrive together or not at
/// all. The other three either have the guard or roll the pair back.
#[must_use]
pub fn create_table(table: &Table, dialect: Dialect) -> Vec<String> {
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
        clauses.push(format!(
            "CONSTRAINT {} UNIQUE ({})",
            key.name(),
            key_columns(table, key.columns(), dialect)
        ));
    }
    if dialect == Dialect::MySql {
        for key in table.indexes() {
            clauses.push(format!(
                "INDEX {} ({})",
                key.name(),
                key_columns(table, key.columns(), dialect)
            ));
        }
    }

    let mut statements = vec![format!(
        "CREATE TABLE {} (\n    {}\n);",
        table.name(),
        clauses.join(",\n    ")
    )];
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

/// One `CREATE INDEX`, for the three dialects that declare one outside the
/// table it belongs to.
fn create_index(table: &Table, key: &Key, dialect: Dialect) -> String {
    format!(
        "CREATE INDEX {} ON {} ({});",
        key.name(),
        table.name(),
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
/// rather than listing 80 expected strings.
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
/// [ADR 0067 § 9](/docs/adr/0067-core-db.md) reads a `TINYINT(1)` as an `int`.
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
    let mut parts: Vec<String> = if rowid {
        vec![
            column.name().to_string(),
            "INTEGER PRIMARY KEY AUTOINCREMENT".to_owned(),
        ]
    } else {
        let mut parts = vec![column.name().to_string(), column_type(column.ty(), dialect)];
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
    Some(format!("CHECK ({} >= 0)", column.name()))
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
            if dialect == Dialect::MySql && unbounded {
                format!("{name}({TEXT_KEY_PREFIX})")
            } else {
                name.to_string()
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
fn rowid_identity(table: &Table, dialect: Dialect) -> Option<&Ident> {
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

/// One of [`ColumnDefault`]'s seven cases as a literal `dialect` reads.
///
/// The column's own type is read for one case and it is not a nicety.
/// [`ColumnDefault::Now`]'s doc calls `CURRENT_TIMESTAMP` the one spelling all
/// five share, and it is — but on SQL Server it is a `datetime`, so writing it
/// as a [`ScalarType::Instant`]'s default converts the server's *local* time
/// into a `datetimeoffset` labelled `+00:00`. That is a wrong instant on every
/// server not running in UTC, silently, in a column whose whole point is that
/// it carries its zone. `SYSDATETIMEOFFSET()` is the same clock read as the
/// type it is being stored in.
fn literal(default: &ColumnDefault, ty: &ScalarType, dialect: Dialect) -> String {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::conn::Driver;

    /// The four dialects, in the order [`Dialect`] declares them.
    const DIALECTS: [Dialect; 4] = [
        Dialect::PostgreSql,
        Dialect::MySql,
        Dialect::Sqlite,
        Dialect::SqlServer,
    ];

    /// A schema naming every construct the vocabulary has: all thirteen scalar
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

    /// ADR 0145 § 8: every construct the vocabulary has emits in all four
    /// dialects.
    ///
    /// Asserted as an **invariance over a sweep** and not as four expected
    /// texts: the counts are derived from the schema value, so a construct that
    /// silently emits nothing fails here, where a golden text would only fail
    /// on the line someone remembered to write. The type spellings are checked
    /// by a property rather than by a table of eighty strings — each is upper
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
                    .find(|line| line.starts_with(&format!("{} ", column.name())))
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
                create.contains("CONSTRAINT wide_label_key UNIQUE (label)"),
                "{dialect:?} has no unique constraint"
            );
            assert!(
                statements
                    .last()
                    .unwrap()
                    .contains("PRIMARY KEY (left_id, right_id)"),
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
                    "happened_at {} NOT NULL DEFAULT {now}",
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

    /// ADR 0145 § 8: the emitters follow `Dialect`, and five drivers are four
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

    /// ADR 0145 § 8: MySQL's first departure — the index is inside the
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
        assert!(mysql[0].contains("INDEX wide_body_idx ("));
        assert!(mysql[0].contains("INDEX wide_day_clock_idx (day, clock)"));
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
                statements
                    .iter()
                    .any(|statement| statement == "CREATE INDEX wide_body_idx ON wide (body);"),
                "{dialect:?} declares an index of its own"
            );
        }
    }

    /// ADR 0145 § 8: MySQL's second departure — an indexed unbounded text
    /// column carries a prefix length.
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
            mysql[0].contains(&format!("INDEX wide_body_idx (body({TEXT_KEY_PREFIX}))")),
            "MySQL cannot index a LONGTEXT column whole"
        );
        assert!(
            mysql[0].contains("CONSTRAINT wide_label_key UNIQUE (label)"),
            "a VARCHAR(200) is indexable as it stands"
        );

        for dialect in [Dialect::PostgreSql, Dialect::Sqlite, Dialect::SqlServer] {
            assert!(
                !create_schema(&schema, dialect)
                    .iter()
                    .any(|statement| statement.contains("body(")),
                "{dialect:?} indexes a text column whole"
            );
        }
    }
}
