//! `rule:core-classes/schema-introspection`'s catalog readers: the SQL that reads a database back, per dialect.
//!
//! § 4 refuses a DDL parser at every tier, so the reverse direction of
//! [`crate::ddl`] is **live introspection** — `pg_catalog` for PostgreSQL,
//! `information_schema` for MySQL, MariaDB and SQL Server, and
//! `sqlite_master` with the `pragma_*` table-valued functions for SQLite. This
//! module is the text half of that and holds no rows: [`query`] answers one
//! statement, `nvs-stdlib` issues it over a connection the program already has
//! (§ 9 makes planning an ordinary read under `db.connect`), [`scalar_type`]
//! and [`column_default`] read the two spellings in that row back into the
//! vocabulary, and [`assemble`] turns the two reads' rows into one
//! [`Schema`].
//!
//! # One row shape per read, and every dialect answers it
//!
//! The point of keying on [`Dialect`] rather than on
//! [`Driver`](crate::Driver) is not that the SQL is shared — none of it is —
//! but that the **rows are**. [`Read`] names the reads an introspection makes
//! and [`Read::row`] names the columns each answers, in order, so the code
//! that turns rows into a schema is written once over positions rather than
//! once per server's column names. Every value that could differ in spelling
//! is normalised in the statement instead: a boolean is `0`/`1` everywhere, an
//! ordinal is one-based everywhere, and the type is one string in the server's
//! own declared spelling rather than the `information_schema` columns it is
//! scattered across.
//!
//! Consequences of that are worth stating because they look like omissions:
//!
//! - **The primary key arrives through [`Read::Indexes`], not through
//!   [`Read::Columns`]** — with `primary` set — because a server reports it as
//!   an index, or can be made to. SQLite is the one that has to be made to: an
//!   `INTEGER PRIMARY KEY` is the rowid and has no entry in `pragma_index_list`
//!   at all, so its statement is a `UNION ALL` whose first branch synthesises
//!   the key from `pragma_table_info`'s `pk` and whose second branch drops the
//!   `origin = 'pk'` index that would otherwise report it twice.
//! - **SQL Server's index read is the one statement that reaches `sys`.** Its
//!   `INFORMATION_SCHEMA` has views for constraints and none for indexes, so a
//!   plain non-unique index is invisible there. Its *column* read is
//!   `INFORMATION_SCHEMA.COLUMNS` like MySQL's, which is what
//!   `postgres_reads_pg_catalog_and_the_others_read_information_schema`
//!   asserts.
//! - **An index outside § 11's vocabulary is not reported at all.** A partial
//!   or expression index is dropped — by `indexprs`/`indpred` on PostgreSQL,
//!   `partial` and a null `pragma_index_info` name on SQLite, and a null
//!   `column_name` on MySQL, each in the statement itself. A [`Schema`] cannot
//!   hold one, so reporting it with its predicate dropped would put a *false*
//!   index in the value and the diff would then agree with a server it does
//!   not match. The cost is the other way round: a plan that creates an index
//!   whose name is already taken by a partial one fails on the server rather
//!   than in the plan.
//! - **SQL Server's filtered index is the one predicate that is read rather
//!   than dropped**, because a unique key over a nullable column *is* a
//!   filtered index there (`rule:core-classes/a-unique-key-reads-nulls-as-distinct`).
//!   Its read carries `filter_definition` and [`predicate_is_the_key_itself`]
//!   matches it against that key's own nullable columns, so the `WHERE`
//!   [`crate::ddl`] wrote reads back as the key that asked for it and a second
//!   `plan` over a converged database is empty. Every other predicate is
//!   dropped exactly as above. The match is in [`assemble`] and not in the
//!   statement because it takes the index's columns and their nullability
//!   together, and only the assembly holds both.
//! - **The read is one schema deep.** A PostgreSQL search path with two
//!   schemas on it, or a SQL Server object under a schema other than the
//!   login's default, is out of view. § 11 has no cross-schema construct, so
//!   there is nothing in the vocabulary to lose.
//!
//! # Nothing here binds a parameter
//!
//! Every statement scopes itself to the connection's own schema with the
//! server's own function — `current_schema()`, `DATABASE()`, `SCHEMA_NAME()`,
//! and for SQLite the file it is open on — so a catalog read carries no
//! parameter marker and [`crate::sql`]'s rewriter has nothing to do with one.
//! `no_catalog_query_carries_a_parameter_marker` holds that: a reader that
//! grew a bind would be the first place an introspection could be pointed at a
//! database the caller did not connect to.
//!
//! # Known gaps
//!
//! 1. **SQL Server's type spelling is assembled out of the catalog's separate
//!    columns and covers lengths and `decimal` only.** `DATETIME_PRECISION` is
//!    not folded in, so a `datetime2(7)` reads back as `datetime2`. That is
//!    § 5's normalisation to own rather than this module's, since the write
//!    direction in [`crate::ddl`] emits one precision for every instant column.
//!    Decided: One normalisation pass that folds both sides through the dialect's map before the diff —
//!    One function that owns every lossy case; the diff stays a plain equality.
//!    — owner: unowned-closures
//! 2. **[`scalar_type`] is a choice function, and § 5 owes the other half.**
//!    The map back is not injective — a dialect with no unsigned integer
//!    spells one as the width above it — so a column
//!    written as `uint32` reads back as `int64` and the plan is empty only
//!    once § 5 normalises the *declared* side the same way. Every case is
//!    named in that function's own doc; nothing here hides one.
//!    Decided: One normalisation pass that folds both sides through the dialect's map before the diff —
//!    One function that owns every lossy case; the diff stays a plain equality.
//!    — owner: unowned-closures
//! 3. **An unquoted spelling on a text column is read as that text.**
//!    [`unquote`] falls back to the whole string where a server printed no
//!    quotes, because MySQL's `information_schema` prints a literal that way —
//!    but PostgreSQL, SQL Server and SQLite always quote a string default, so
//!    on those an unquoted spelling is an *expression* and reading it as a
//!    value is the direction [`column_default`]'s own doc calls unrecoverable:
//!    the plan then proposes a default the server will never report back, and
//!    no number of applies converges. Narrowing the fallback to the dialect
//!    that needs it is a change to that function and the tests over it, not to
//!    [`assemble`].
//!    Decided: Add an opaque, read-only ColumnDefault case holding the raw text, compared verbatim and
//!    never emitted — The plan converges and nothing is lost; costs one vocabulary case that programs
//!    cannot construct.
//!    — owner: unowned-closures

use crate::schema::{
    Column, ColumnDefault, FloatWidth, IntWidth, ScalarType, Schema, SchemaError, Table,
};
use crate::sql::Dialect;

/// One of the two reads an introspection makes.
///
/// Each answers a fixed row shape — [`Read::row`] — that is the same on every
/// driver, which is what lets the assembly above this module be written once.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Read {
    /// Every column of every base table, one row each.
    Columns,
    /// Every key column of every primary key, unique constraint and index, one
    /// row each.
    Indexes,
}

impl Read {
    /// The columns this read's rows carry, in the order every dialect selects
    /// them.
    ///
    /// The names are this module's rather than any server's, and **every
    /// statement aliases its select list to them**, so a reader may match a
    /// column by name and not only by position. That is not decoration: SQLite's
    /// column read selects `m.name` beside `p.name`, and a consumer that
    /// describes a row by the server's own names — which `Core\Db\Row` does —
    /// collapses the two into one entry and silently shifts every column after
    /// it. The bug is invisible against an empty database, where there are no
    /// rows to collapse.
    ///
    /// The `nvs_` prefix is what makes one alias legal on every backend
    /// unquoted: `table`, `column`, `type`, `default`, `index`, `unique` and
    /// `primary` are reserved words somewhere, and the backends do not agree on
    /// how an identifier is delimited — which is the same wall
    /// `Core\Db::quoteIdentifier` states.
    ///
    /// - [`Read::Columns`] — `nvs_table`, `nvs_column`, `nvs_ordinal` (one-based
    ///   declaration order, an ordering key and not an index — PostgreSQL
    ///   leaves gaps where a column was dropped), `nvs_type` (the server's own
    ///   declared spelling, parameters included), `nvs_nullable` (`1` when the
    ///   column accepts null), `nvs_default` (the server's spelling, or null),
    ///   `nvs_identity` (`1` when the server assigns the value).
    /// - [`Read::Indexes`] — `nvs_table`, `nvs_index` (the constraint or index
    ///   name), `nvs_column`, `nvs_ordinal` (one-based position within the key),
    ///   `nvs_unique` (`1` when a duplicate is refused), `nvs_primary` (`1` when
    ///   this is the table's primary key).
    #[must_use]
    pub fn row(self) -> &'static [&'static str] {
        match self {
            Read::Columns => &[
                "nvs_table",
                "nvs_column",
                "nvs_ordinal",
                "nvs_type",
                "nvs_nullable",
                "nvs_default",
                "nvs_identity",
            ],
            Read::Indexes => &[
                "nvs_table",
                "nvs_index",
                "nvs_column",
                "nvs_ordinal",
                "nvs_unique",
                "nvs_primary",
                "nvs_filter",
            ],
        }
    }

    /// How many values one row of this read carries.
    #[must_use]
    pub fn arity(self) -> usize {
        self.row().len()
    }
}

/// The statement `dialect` answers `read` with, over the schema the connection
/// is already on.
///
/// Keyed on [`Dialect`] and never on [`Driver`](crate::Driver), as
/// [`crate::ddl`] is: MariaDB reads MySQL's `information_schema` with the same
/// text, and `the_catalog_queries_follow_dialect_rather_than_driver` asserts
/// that as an agreement over every driver.
#[must_use]
pub fn query(read: Read, dialect: Dialect) -> &'static str {
    match (read, dialect) {
        (Read::Columns, Dialect::PostgreSql) => PG_COLUMNS,
        (Read::Columns, Dialect::MySql) => MYSQL_COLUMNS,
        (Read::Columns, Dialect::Sqlite) => SQLITE_COLUMNS,
        (Read::Columns, Dialect::SqlServer) => SQLSERVER_COLUMNS,
        (Read::Indexes, Dialect::PostgreSql) => PG_INDEXES,
        (Read::Indexes, Dialect::MySql) => MYSQL_INDEXES,
        (Read::Indexes, Dialect::Sqlite) => SQLITE_INDEXES,
        (Read::Indexes, Dialect::SqlServer) => SQLSERVER_INDEXES,
    }
}

/// PostgreSQL's columns, from `pg_catalog` rather than `information_schema`.
///
/// `format_type` is the reason: it prints the declared type and its parameters
/// as one string, where `information_schema.columns` scatters the same fact
/// across `data_type`, `character_maximum_length`, `numeric_precision` and
/// `numeric_scale` and then spells the type in the standard's words rather than
/// the server's. `attidentity` is non-empty exactly for the `GENERATED … AS
/// IDENTITY` [`crate::ddl`] writes.
const PG_COLUMNS: &str = r"SELECT c.relname AS nvs_table,
       a.attname AS nvs_column,
       a.attnum AS nvs_ordinal,
       pg_catalog.format_type(a.atttypid, a.atttypmod) AS nvs_type,
       CASE WHEN a.attnotnull THEN 0 ELSE 1 END AS nvs_nullable,
       pg_catalog.pg_get_expr(d.adbin, d.adrelid) AS nvs_default,
       CASE WHEN a.attidentity <> '' THEN 1 ELSE 0 END AS nvs_identity
FROM pg_catalog.pg_class c
JOIN pg_catalog.pg_namespace n ON n.oid = c.relnamespace
JOIN pg_catalog.pg_attribute a ON a.attrelid = c.oid
LEFT JOIN pg_catalog.pg_attrdef d ON d.adrelid = c.oid AND d.adnum = a.attnum
WHERE c.relkind = 'r'
  AND n.nspname = pg_catalog.current_schema()
  AND a.attnum > 0
  AND NOT a.attisdropped
ORDER BY c.relname, a.attnum";

/// PostgreSQL's key columns, primary keys included.
///
/// `indkey` is an `int2vector`, so it is cast to an array before `unnest …
/// WITH ORDINALITY` gives each column its position. A zero entry there is an
/// expression, and the `indexprs`/`indpred` filter has already dropped the
/// index it belonged to — which is why `nvs_filter` is a typed null here: the
/// column is SQL Server's answer to a question this dialect settles in its own
/// `WHERE`, and the row shape is one shape for all four.
const PG_INDEXES: &str = r"SELECT c.relname AS nvs_table,
       i.relname AS nvs_index,
       a.attname AS nvs_column,
       k.ord AS nvs_ordinal,
       CASE WHEN ix.indisunique THEN 1 ELSE 0 END AS nvs_unique,
       CASE WHEN ix.indisprimary THEN 1 ELSE 0 END AS nvs_primary,
       CAST(NULL AS text) AS nvs_filter
FROM pg_catalog.pg_index ix
JOIN pg_catalog.pg_class c ON c.oid = ix.indrelid
JOIN pg_catalog.pg_class i ON i.oid = ix.indexrelid
JOIN pg_catalog.pg_namespace n ON n.oid = c.relnamespace
JOIN LATERAL unnest(ix.indkey::int2[]) WITH ORDINALITY AS k(attnum, ord) ON TRUE
JOIN pg_catalog.pg_attribute a ON a.attrelid = c.oid AND a.attnum = k.attnum
WHERE c.relkind = 'r'
  AND n.nspname = pg_catalog.current_schema()
  AND ix.indexprs IS NULL
  AND ix.indpred IS NULL
ORDER BY c.relname, i.relname, k.ord";

/// MySQL's and MariaDB's columns.
///
/// `column_type` is MySQL's extension to `information_schema` and is the same
/// fact `format_type` gives on PostgreSQL: the declared spelling with its
/// length, its `unsigned` and its enumeration intact. `extra` is where
/// `AUTO_INCREMENT` is reported.
const MYSQL_COLUMNS: &str = r"SELECT c.table_name AS nvs_table,
       c.column_name AS nvs_column,
       c.ordinal_position AS nvs_ordinal,
       c.column_type AS nvs_type,
       CASE WHEN c.is_nullable = 'YES' THEN 1 ELSE 0 END AS nvs_nullable,
       c.column_default AS nvs_default,
       CASE WHEN c.extra LIKE '%auto_increment%' THEN 1 ELSE 0 END AS nvs_identity
FROM information_schema.columns c
JOIN information_schema.tables t
  ON t.table_schema = c.table_schema AND t.table_name = c.table_name
WHERE c.table_schema = DATABASE()
  AND t.table_type = 'BASE TABLE'
ORDER BY c.table_name, c.ordinal_position";

/// MySQL's and MariaDB's key columns, out of `information_schema.statistics`.
///
/// A functional index reports a null `column_name`, and the `NOT EXISTS`
/// removes the whole index rather than the one row, so a partly-read index
/// never reaches a schema value. The `expression` column that would say the
/// same thing directly exists on MySQL 8 and not on MariaDB, and this text is
/// one text for both.
const MYSQL_INDEXES: &str = r"SELECT s.table_name AS nvs_table,
       s.index_name AS nvs_index,
       s.column_name AS nvs_column,
       s.seq_in_index AS nvs_ordinal,
       CASE WHEN s.non_unique = 0 THEN 1 ELSE 0 END AS nvs_unique,
       CASE WHEN s.index_name = 'PRIMARY' THEN 1 ELSE 0 END AS nvs_primary,
       CAST(NULL AS char) AS nvs_filter
FROM information_schema.statistics s
JOIN information_schema.tables t
  ON t.table_schema = s.table_schema AND t.table_name = s.table_name
WHERE s.table_schema = DATABASE()
  AND t.table_type = 'BASE TABLE'
  AND NOT EXISTS (
        SELECT 1 FROM information_schema.statistics x
        WHERE x.table_schema = s.table_schema
          AND x.table_name = s.table_name
          AND x.index_name = s.index_name
          AND x.column_name IS NULL)
ORDER BY s.table_name, s.index_name, s.seq_in_index";

/// SQL Server's columns, from `INFORMATION_SCHEMA.COLUMNS`.
///
/// The declared spelling has to be rebuilt here because SQL Server has no
/// `column_type` of its own: a `-1` length is the `(max)` form, any other
/// length is written in parentheses, and `decimal` and `numeric` take their
/// precision and scale. `COLUMNPROPERTY` is how an identity is asked for
/// without leaving `INFORMATION_SCHEMA` for `sys.columns`.
const SQLSERVER_COLUMNS: &str = r"SELECT c.TABLE_NAME AS nvs_table,
       c.COLUMN_NAME AS nvs_column,
       c.ORDINAL_POSITION AS nvs_ordinal,
       CASE
         WHEN c.CHARACTER_MAXIMUM_LENGTH = -1 THEN c.DATA_TYPE + '(max)'
         WHEN c.CHARACTER_MAXIMUM_LENGTH IS NOT NULL
           THEN c.DATA_TYPE + '(' + CAST(c.CHARACTER_MAXIMUM_LENGTH AS varchar(11)) + ')'
         WHEN c.DATA_TYPE IN ('decimal', 'numeric')
           THEN c.DATA_TYPE + '(' + CAST(c.NUMERIC_PRECISION AS varchar(11))
                + ',' + CAST(c.NUMERIC_SCALE AS varchar(11)) + ')'
         ELSE c.DATA_TYPE
       END AS nvs_type,
       CASE WHEN c.IS_NULLABLE = 'YES' THEN 1 ELSE 0 END AS nvs_nullable,
       c.COLUMN_DEFAULT AS nvs_default,
       COLUMNPROPERTY(
         OBJECT_ID(QUOTENAME(c.TABLE_SCHEMA) + '.' + QUOTENAME(c.TABLE_NAME)),
         c.COLUMN_NAME,
         'IsIdentity') AS nvs_identity
FROM INFORMATION_SCHEMA.COLUMNS c
JOIN INFORMATION_SCHEMA.TABLES t
  ON t.TABLE_SCHEMA = c.TABLE_SCHEMA AND t.TABLE_NAME = c.TABLE_NAME
WHERE c.TABLE_SCHEMA = SCHEMA_NAME()
  AND t.TABLE_TYPE = 'BASE TABLE'
ORDER BY c.TABLE_NAME, c.ORDINAL_POSITION";

/// SQL Server's key columns, and the one statement here that reaches `sys`.
///
/// `INFORMATION_SCHEMA` has `TABLE_CONSTRAINTS` and `KEY_COLUMN_USAGE` and no
/// view over indexes at all, so a plain non-unique index — which § 11 very much
/// has — is unreadable there. `type IN (1, 2)` keeps clustered and nonclustered
/// b-trees and drops the heap, the XML, spatial and columnstore forms, and an
/// included column is not a key column.
///
/// **`filter_definition` is carried rather than filtered on**, because this is
/// the dialect where a filtered index is a unique key of the vocabulary's own
/// (`rule:core-classes/a-unique-key-reads-nulls-as-distinct`) and not only a
/// predicate a `Schema` cannot hold. Which of the two it is depends on the
/// index's columns and their nullability, so [`assemble`] reads it where both
/// are already in hand.
const SQLSERVER_INDEXES: &str = r"SELECT t.name AS nvs_table,
       i.name AS nvs_index,
       c.name AS nvs_column,
       ic.key_ordinal AS nvs_ordinal,
       CASE WHEN i.is_unique = 1 THEN 1 ELSE 0 END AS nvs_unique,
       CASE WHEN i.is_primary_key = 1 THEN 1 ELSE 0 END AS nvs_primary,
       i.filter_definition AS nvs_filter
FROM sys.indexes i
JOIN sys.tables t ON t.object_id = i.object_id
JOIN sys.schemas s ON s.schema_id = t.schema_id
JOIN sys.index_columns ic
  ON ic.object_id = i.object_id AND ic.index_id = i.index_id
JOIN sys.columns c ON c.object_id = ic.object_id AND c.column_id = ic.column_id
WHERE s.name = SCHEMA_NAME()
  AND i.type IN (1, 2)
  AND i.is_hypothetical = 0
  AND ic.is_included_column = 0
ORDER BY t.name, i.name, ic.key_ordinal";

/// SQLite's columns, from `sqlite_master` joined to the `pragma_table_info`
/// table-valued function.
///
/// SQLite has neither catalog, and the pragma functions are how one statement
/// covers every table rather than one statement per table. `notnull` is a
/// keyword and has to be quoted. `type` is the *declared* type, which is the
/// text this vocabulary's own emitter wrote, since SQLite stores the spelling
/// and applies only its affinity.
///
/// A primary-key column is reported as not nullable whatever `notnull` says,
/// because SQLite does not record one: the `NOT NULL` an `INTEGER PRIMARY KEY`
/// carries is implicit in being the rowid, and for the other key forms SQLite
/// keeps a documented compatibility hole that lets a null into a primary key
/// at all. [`Table`](crate::Table) refuses a nullable primary key, so a reader
/// passing `notnull` through would answer rows no [`Schema`](crate::Schema)
/// can be built from.
///
/// Identity is the rowid alias — a single-column `INTEGER PRIMARY KEY` — and
/// deliberately not the `AUTOINCREMENT` keyword, which lives in
/// `sqlite_sequence` and would need `sqlite_master.sql` read as text to find
/// otherwise. The two forms differ in whether a rowid is reused and not in
/// anything a [`Schema`](crate::Schema) can say, so reading both as an identity
/// is what makes a round trip empty.
const SQLITE_COLUMNS: &str = r#"SELECT m.name AS nvs_table,
       p.name AS nvs_column,
       p.cid + 1 AS nvs_ordinal,
       p.type AS nvs_type,
       CASE WHEN p."notnull" = 0 AND p.pk = 0 THEN 1 ELSE 0 END AS nvs_nullable,
       p.dflt_value AS nvs_default,
       CASE WHEN p.pk = 1 AND UPPER(p.type) = 'INTEGER'
                 AND (SELECT COUNT(*) FROM pragma_table_info(m.name) q WHERE q.pk > 0) = 1
            THEN 1 ELSE 0 END AS nvs_identity
FROM sqlite_master m
JOIN pragma_table_info(m.name) p
WHERE m.type = 'table'
  AND m.name NOT LIKE 'sqlite_%'
ORDER BY m.name, p.cid"#;

/// SQLite's key columns, in two branches.
///
/// The first synthesises the primary key from `pragma_table_info`'s `pk`,
/// because an `INTEGER PRIMARY KEY` is the rowid and `pragma_index_list`
/// reports no index for it; the name is this statement's own, since a
/// [`Table`](crate::Table)'s primary key has no name in the vocabulary. The
/// second is every other index, with `origin = 'pk'` dropped so a composite
/// key's `sqlite_autoindex_…` is not reported a second time. The `ORDER BY` is
/// by position because a compound select takes the first branch's column names.
const SQLITE_INDEXES: &str = r#"SELECT m.name AS nvs_table, 'PRIMARY' AS nvs_index, p.name AS nvs_column,
       p.pk AS nvs_ordinal, 1 AS nvs_unique, 1 AS nvs_primary, NULL AS nvs_filter
FROM sqlite_master m
JOIN pragma_table_info(m.name) p
WHERE m.type = 'table'
  AND m.name NOT LIKE 'sqlite_%'
  AND p.pk > 0
UNION ALL
SELECT m.name, il.name, ii.name, ii.seqno + 1,
       CASE WHEN il."unique" = 1 THEN 1 ELSE 0 END,
       0,
       NULL
FROM sqlite_master m
JOIN pragma_index_list(m.name) il
JOIN pragma_index_info(il.name) ii
WHERE m.type = 'table'
  AND m.name NOT LIKE 'sqlite_%'
  AND il.origin <> 'pk'
  AND il.partial = 0
  AND ii.name IS NOT NULL
ORDER BY 1, 2, 4"#;

/// The [`ScalarType`] a server's own type spelling names, or `None` for a
/// spelling § 11's vocabulary has not got.
///
/// This is the reverse of [`crate::ddl::column_type`] and **not** of
/// [`ScalarType::from_spelling`]: that reader takes this vocabulary's own
/// canonical names — `int64`, `text(200)` — and this one takes the words a
/// catalog printed, which are the *server's* and not the emitter's.
/// PostgreSQL's `format_type` answers `character varying(200)` where the
/// emitter wrote `VARCHAR(200)`, and `timestamp with time zone` where it wrote
/// `TIMESTAMPTZ`. Both spellings are accepted, because both are the same
/// column.
///
/// A spelling is read case-insensitively, with runs of whitespace collapsed
/// and a single parenthesised group lifted out of wherever it sits — so
/// `timestamp(3) without time zone` is the head `timestamp without time zone`
/// with an argument, and a precision the vocabulary cannot hold is dropped
/// rather than refused, which is gap 2 of this module's doc in the other
/// direction. Nothing here parses SQL: it is a closed table of names per
/// dialect, exactly as [`ScalarType::from_spelling`] is a closed table over
/// the canonical ones, and § 4 refuses the alternative at every tier.
///
/// # The map is not injective, and these are the choices it makes
///
/// The emitter writes one spelling for two types in the places below, so
/// reading is a choice and each one costs a normalisation § 5 must make on the
/// declared side for the round trip to be empty:
///
/// - **`INTEGER`/`INT` is [`IntWidth::Normal`] signed, and `BIGINT` is
///   [`IntWidth::Big`] signed**, on the dialects with no unsigned integer. A
///   `uint16` reads back as `int32` and a `uint32` as `int64` —
///   the server genuinely holds the wider column, so the *reading* is right
///   and it is the declared side that has to be widened before the diff.
/// - **`NUMERIC(20, 0)`/`DECIMAL(20, 0)` is a [`ScalarType::Decimal`]**, never
///   a `uint64`, on PostgreSQL and SQL Server. Decimal is the general answer
///   and `uint64` the case only MySQL has a type for. SQLite is the exception
///   and needs no choice at all: a declared type there is a name rather than a
///   constraint, so the emitter's `NUMERIC(20, 0)` for `uint64` and its
///   `DECIMAL(p, s)` for a decimal come back as two different words.
/// - **`BYTEA` and `BLOB` are [`ScalarType::Bytes`] unbounded**: PostgreSQL and
///   SQLite hold no declared length for binary, so a `bytes(64)` reads back
///   without its 64. There is nothing in the server to recover it from.
/// - **SQL Server's `NVARCHAR(MAX)` is [`ScalarType::Text`] unbounded**, never
///   [`ScalarType::Json`], because that backend has no JSON type and the
///   emitter writes the same words for both.
/// - **MySQL's `CHAR(36)` is [`ScalarType::Uuid`]**, which is safe rather than
///   lucky: the emitter writes `CHAR(36)` for nothing else and text is always
///   a `VARCHAR`, so no vocabulary type is being taken away from.
/// - **PostgreSQL's unbounded `character varying` is `TEXT`**, and its `JSON`
///   is `JSONB`. Neither is a spelling the emitter writes; both are what that
///   server means, and refusing them would make a hand-made column unreadable
///   rather than merely unnormalised.
///
/// `every_catalog_spelling_the_emitter_wrote_reads_back_as_its_own_type` holds
/// the whole of that as a round trip rather than as a table of expected names:
/// for every type and every dialect, the spelling the emitter wrote must read
/// back as a type whose own emission is that same string.
#[must_use]
pub fn scalar_type(spelling: &str, dialect: Dialect) -> Option<ScalarType> {
    let (head, args) = split_spelling(spelling)?;
    let args = args.as_deref();
    let ty = match dialect {
        Dialect::PostgreSql => postgres_scalar(&head, args),
        Dialect::MySql => mysql_scalar(&head, args),
        Dialect::Sqlite => sqlite_scalar(&head, args),
        Dialect::SqlServer => sqlserver_scalar(&head, args),
    }?;
    ty.check().ok()?;
    Some(ty)
}

/// A spelling as a lower-case head and the one parenthesised group it carries.
///
/// The group is lifted out of wherever it sits rather than required at the
/// end, because PostgreSQL writes the parameter in the middle of the name —
/// `timestamp(3) without time zone`. A second group, or an unclosed one, is
/// refused: that is a type this vocabulary does not have.
fn split_spelling(spelling: &str) -> Option<(String, Option<String>)> {
    /// One space between words, and none at either end.
    fn words(text: &str) -> String {
        text.split_whitespace().collect::<Vec<_>>().join(" ")
    }

    let lower = spelling.to_ascii_lowercase();
    let Some((before, rest)) = lower.split_once('(') else {
        return (!lower.trim().is_empty()).then(|| (words(&lower), None));
    };
    let (args, after) = rest.split_once(')')?;
    if args.contains('(') || after.contains(['(', ')']) {
        return None;
    }
    Some((words(&format!("{before} {after}")), Some(words(args))))
}

/// A single unsigned parameter — a length, or a display width to ignore.
fn one_arg(args: Option<&str>) -> Option<u32> {
    args?.parse().ok()
}

/// `p, s`, with a missing scale reading as zero, as a catalog printing only a
/// precision means. A precision no vocabulary decimal could carry does not
/// parse as a `u8` and so refuses here rather than in
/// [`ScalarType::check`](ScalarType).
fn decimal_args(args: Option<&str>) -> Option<ScalarType> {
    let text = args?;
    let (precision, scale) = match text.split_once(',') {
        Some((precision, scale)) => (precision.trim(), scale.trim()),
        None => (text, "0"),
    };
    Some(ScalarType::Decimal {
        precision: precision.parse().ok()?,
        scale: scale.parse().ok()?,
    })
}

/// PostgreSQL, as `format_type` prints it — the standard's long spellings
/// alongside the abbreviations a person types.
fn postgres_scalar(head: &str, args: Option<&str>) -> Option<ScalarType> {
    Some(match (head, args) {
        ("smallint" | "int2", None) => ScalarType::Int(IntWidth::Small),
        ("integer" | "int" | "int4", None) => ScalarType::Int(IntWidth::Normal),
        ("bigint" | "int8", None) => ScalarType::Int(IntWidth::Big),
        ("real" | "float4", None) => ScalarType::Float(FloatWidth::Single),
        ("double precision" | "float8", None) => ScalarType::Float(FloatWidth::Double),
        ("numeric" | "decimal", Some(_)) => decimal_args(args)?,
        ("character varying" | "varchar", Some(_)) => ScalarType::Text {
            max: Some(one_arg(args)?),
        },
        ("text" | "character varying" | "varchar", None) => ScalarType::Text { max: None },
        ("bytea", None) => ScalarType::Bytes { max: None },
        ("boolean" | "bool", None) => ScalarType::Bool,
        ("date", None) => ScalarType::Date,
        ("time" | "time without time zone", _) => ScalarType::Time,
        ("timestamp" | "timestamp without time zone", _) => ScalarType::DateTime,
        ("timestamptz" | "timestamp with time zone", _) => ScalarType::Instant,
        ("uuid", None) => ScalarType::Uuid,
        ("json" | "jsonb", None) => ScalarType::Json,
        _ => return None,
    })
}

/// MySQL and MariaDB, as `column_type` prints it: `unsigned` is part of the
/// name, and a display width MySQL 8 no longer prints is accepted and ignored
/// where an older server still does.
fn mysql_scalar(head: &str, args: Option<&str>) -> Option<ScalarType> {
    Some(match (head, args) {
        ("smallint", _) => ScalarType::Int(IntWidth::Small),
        ("int" | "integer", _) => ScalarType::Int(IntWidth::Normal),
        ("bigint", _) => ScalarType::Int(IntWidth::Big),
        ("smallint unsigned", _) => ScalarType::Uint(IntWidth::Small),
        ("int unsigned" | "integer unsigned", _) => ScalarType::Uint(IntWidth::Normal),
        ("bigint unsigned", _) => ScalarType::Uint(IntWidth::Big),
        ("float", None) => ScalarType::Float(FloatWidth::Single),
        ("double", None) => ScalarType::Float(FloatWidth::Double),
        ("decimal" | "numeric", Some(_)) => decimal_args(args)?,
        ("varchar", Some(_)) => ScalarType::Text {
            max: Some(one_arg(args)?),
        },
        ("longtext", None) => ScalarType::Text { max: None },
        ("varbinary", Some(_)) => ScalarType::Bytes {
            max: Some(one_arg(args)?),
        },
        ("longblob", None) => ScalarType::Bytes { max: None },
        ("bit", Some("1")) => ScalarType::Bool,
        ("date", None) => ScalarType::Date,
        ("time", _) => ScalarType::Time,
        ("datetime", _) => ScalarType::DateTime,
        ("timestamp", _) => ScalarType::Instant,
        ("char", Some("36")) => ScalarType::Uuid,
        ("json", None) => ScalarType::Json,
        _ => return None,
    })
}

/// SQLite, where `pragma_table_info` reports the declared name back verbatim,
/// so this is very nearly [`crate::ddl`]'s own words read backwards.
fn sqlite_scalar(head: &str, args: Option<&str>) -> Option<ScalarType> {
    Some(match (head, args) {
        ("smallint", None) => ScalarType::Int(IntWidth::Small),
        ("integer" | "int", None) => ScalarType::Int(IntWidth::Normal),
        ("bigint", None) => ScalarType::Int(IntWidth::Big),
        ("real", None) => ScalarType::Float(FloatWidth::Single),
        ("double precision" | "double", None) => ScalarType::Float(FloatWidth::Double),
        // SQLite is the one dialect that tells `uint64` from the decimal of
        // the same shape, and that is the emitter's doing rather than the
        // server's: a declared type here is a name and not a constraint, so
        // its `NUMERIC(20, 0)` and its `DECIMAL(20, 0)` both come back out of
        // `pragma_table_info` as themselves.
        ("numeric" | "decimal", Some(_)) => match (head, decimal_args(args)?) {
            (
                "numeric",
                ScalarType::Decimal {
                    precision: 20,
                    scale: 0,
                },
            ) => ScalarType::Uint(IntWidth::Big),
            (_, ty) => ty,
        },
        ("varchar", Some(_)) => ScalarType::Text {
            max: Some(one_arg(args)?),
        },
        ("text", None) => ScalarType::Text { max: None },
        ("blob", None) => ScalarType::Bytes { max: None },
        ("boolean", None) => ScalarType::Bool,
        ("date", None) => ScalarType::Date,
        ("time", None) => ScalarType::Time,
        ("datetime", None) => ScalarType::DateTime,
        ("timestamptz", None) => ScalarType::Instant,
        ("uuid", None) => ScalarType::Uuid,
        ("json", None) => ScalarType::Json,
        _ => return None,
    })
}

/// SQL Server, as this module's own `CASE` assembles it out of `DATA_TYPE` and
/// the length columns: `(max)` for the unbounded forms and nothing at all for
/// a type whose parameter that catalog does not report.
fn sqlserver_scalar(head: &str, args: Option<&str>) -> Option<ScalarType> {
    Some(match (head, args) {
        ("smallint", None) => ScalarType::Int(IntWidth::Small),
        ("int", None) => ScalarType::Int(IntWidth::Normal),
        ("bigint", None) => ScalarType::Int(IntWidth::Big),
        ("real", None) => ScalarType::Float(FloatWidth::Single),
        // `FLOAT(n)` is `real` at 24 bits or fewer and a double above it,
        // which is the server's own rule; a bare `float` is the 53-bit one.
        ("float", None) => ScalarType::Float(FloatWidth::Double),
        ("float", Some(_)) => match one_arg(args)? {
            0..=24 => ScalarType::Float(FloatWidth::Single),
            _ => ScalarType::Float(FloatWidth::Double),
        },
        ("decimal" | "numeric", Some(_)) => decimal_args(args)?,
        ("nvarchar" | "varchar", Some("max")) => ScalarType::Text { max: None },
        ("nvarchar" | "varchar", Some(_)) => ScalarType::Text {
            max: Some(one_arg(args)?),
        },
        ("varbinary" | "binary", Some("max")) => ScalarType::Bytes { max: None },
        ("varbinary" | "binary", Some(_)) => ScalarType::Bytes {
            max: Some(one_arg(args)?),
        },
        ("bit", None) => ScalarType::Bool,
        ("date", None) => ScalarType::Date,
        ("time", _) => ScalarType::Time,
        ("datetime2", _) => ScalarType::DateTime,
        ("datetimeoffset", _) => ScalarType::Instant,
        ("uniqueidentifier", None) => ScalarType::Uuid,
        _ => return None,
    })
}

/// The [`ColumnDefault`] a server's own default expression names, or `None`
/// for one § 2's closed set cannot hold.
///
/// The reverse of `literal` in [`crate::ddl`], and **type-directed** in the
/// same way that function is: `1` is a `bool` on a `bool` column and an
/// integer on an integer one, and `CURRENT_TIMESTAMP` is
/// [`ColumnDefault::Now`] only where a timestamp could be stored. Reading it
/// any other way would need to know what the server meant, which is the
/// question § 4 refuses to answer with a parser.
///
/// The wrappers below are the server's and never an emitter's, and each is
/// removed before the literal is read:
///
/// - **SQL Server parenthesises a default, sometimes twice** — `((0))`,
///   `(N'hi')`, `(getdate())` — and the parentheses carry no meaning.
/// - **PostgreSQL labels a literal with its type**: `pg_get_expr` prints
///   `'hi'::character varying` and `'12.34'::numeric`. The label is the
///   column's own type, so dropping it loses nothing this function did not
///   already have.
/// - **MySQL prints a literal with no quotes at all.** `COLUMN_DEFAULT` gives
///   `hi`, not `'hi'`, so an unquoted value is read as the literal it is
///   rather than refused — which is also why this reader is type-directed and
///   not shape-directed.
/// - **`NULL` is the keyword and never a value**, so a column defaulting to it
///   is the column with no default at all — which is what SQL means by the two
///   as well. MariaDB is where that is load-bearing: its `information_schema`
///   prints this keyword for every nullable column that was never given a
///   default, where MySQL prints null itself. The one spelling it costs is a
///   MySQL text default of the four characters `NULL`, which arrives unquoted
///   and so cannot be told from the keyword; MariaDB quotes a text default and
///   is unambiguous, and the alternative is every nullable column on that
///   server reading back as a default no apply converges on.
///
/// # What is refused, and why refusing is the safe direction
///
/// `None` is the answer for an expression default — `nextval(…)`,
/// `uuid_generate_v4()`, a string with an escape this vocabulary does not
/// write — because § 2's set is closed and an expression is the value a server
/// is most likely to spell back differently. A default read as `None` costs a
/// spurious step in § 5's plan; a default read *wrongly* costs a plan that
/// says nothing needs doing when it does, and only one of those is recoverable
/// by looking at the plan. What the assembly does with a `None` is its own
/// doc's business, not this function's.
#[must_use]
pub fn column_default(spelling: &str, ty: &ScalarType, dialect: Dialect) -> Option<ColumnDefault> {
    let bare = strip_cast(unwrap_parens(spelling.trim()));
    if bare.eq_ignore_ascii_case("null") {
        return None;
    }
    if matches!(ty, ScalarType::DateTime | ScalarType::Instant) && is_now(bare) {
        return Some(ColumnDefault::Now);
    }
    let value = unquote(bare, dialect)?;
    let default = match ty {
        ScalarType::Int(_) => ColumnDefault::Int(value.parse().ok()?),
        ScalarType::Uint(_) => ColumnDefault::Uint(value.parse().ok()?),
        ScalarType::Float(_) => ColumnDefault::Float(value.parse().ok()?),
        ScalarType::Decimal { .. } => {
            // Not a number this reader parses: a decimal literal keeps the
            // digits the schema wrote, and turning them into a float and back
            // is exactly the rounding `rule:types/decimal` gives a `decimal` to avoid.
            let digits = value.trim_start_matches(['+', '-']);
            if digits.is_empty()
                || !digits.chars().all(|c| c.is_ascii_digit() || c == '.')
                || digits.matches('.').count() > 1
            {
                return None;
            }
            ColumnDefault::Decimal(value)
        }
        ScalarType::Text { .. } => ColumnDefault::Text(value),
        ScalarType::Bool => match value.to_ascii_lowercase().as_str() {
            "1" | "true" | "t" => ColumnDefault::Bool(true),
            "0" | "false" | "f" => ColumnDefault::Bool(false),
            _ => return None,
        },
        _ => return None,
    };
    // The one rule, and the same one the builder applies: a literal that could
    // not have been written on every backend is not one an introspection
    // invents either.
    default.fits(ty).then_some(default)
}

/// SQL Server's wrapping parentheses, however many it added.
///
/// `(getdate())` is a wrapper around a call and `getdate()` is not, which is
/// why the inside is checked for balance rather than the outside for shape.
fn unwrap_parens(text: &str) -> &str {
    /// Whether `text` closes every parenthesis it opens outside a literal.
    fn balanced(text: &str) -> bool {
        let mut depth = 0i32;
        let mut quoted = false;
        for c in text.chars() {
            match c {
                // A doubled quote toggles twice and so nets out.
                '\'' => quoted = !quoted,
                '(' if !quoted => depth += 1,
                ')' if !quoted => {
                    depth -= 1;
                    if depth < 0 {
                        return false;
                    }
                }
                _ => {}
            }
        }
        depth == 0 && !quoted
    }

    let mut text = text;
    while let Some(inner) = text
        .strip_prefix('(')
        .and_then(|rest| rest.strip_suffix(')'))
        .map(str::trim)
        .filter(|inner| balanced(inner))
    {
        text = inner;
    }
    text
}

/// PostgreSQL's `::type` label, dropped from the end of a literal.
fn strip_cast(text: &str) -> &str {
    let mut quoted = false;
    for (at, c) in text.char_indices() {
        match c {
            '\'' => quoted = !quoted,
            ':' if !quoted && text[at..].starts_with("::") => return text[..at].trim_end(),
            _ => {}
        }
    }
    text
}

/// The spellings a server prints for `CURRENT_TIMESTAMP`, with the precision
/// some of them add ignored.
fn is_now(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    let head = lower
        .split_once('(')
        .map_or(lower.as_str(), |(head, _)| head);
    matches!(
        head.trim(),
        "current_timestamp" | "now" | "getdate" | "sysdatetimeoffset" | "localtimestamp"
    )
}

/// The value inside a quoted literal, or the whole text where the server
/// printed it unquoted.
///
/// `N'…'` is SQL Server's national literal and `b'…'` MySQL's bit literal, and
/// both are the emitter's own spellings. Inside, a doubled quote is the one
/// escape every backend shares and a backslash is MySQL's alone — an escape
/// outside those two is refused rather than guessed at, because this is a
/// closed set of literals and not a lexer.
fn unquote(text: &str, dialect: Dialect) -> Option<String> {
    let opened = ["N'", "n'", "b'", "B'", "'"]
        .into_iter()
        .find_map(|prefix| text.strip_prefix(prefix));
    let Some(body) = opened else {
        return Some(text.to_owned());
    };
    let body = body.strip_suffix('\'')?;
    let mut out = String::with_capacity(body.len());
    let mut chars = body.chars();
    while let Some(c) = chars.next() {
        match c {
            '\'' => {
                // A lone quote means the literal ended before the text did, so
                // this is an expression rather than one value.
                if chars.next() != Some('\'') {
                    return None;
                }
                out.push('\'');
            }
            '\\' if dialect.backslash_escapes() => match chars.next()? {
                escaped @ ('\\' | '\'') => out.push(escaped),
                _ => return None,
            },
            other => out.push(other),
        }
    }
    Some(out)
}

/// One row of [`Read::Columns`], in this module's own names.
///
/// The fields are [`Read::row`]'s positions in order, and each carries the
/// neutral type rather than any driver's value — `nvs-db` exports `PgRow`,
/// `MySqlRow` and `SqliteValue` and nothing common, so a row of a catalog read
/// is read by `nvs-stdlib` off whichever driver answered and filled in here.
/// That is what lets [`assemble`] be one function rather than one per driver, and
/// what keeps this crate sans-io per `rule:core-classes/db-drivers-are-an-enum`: the statement is text, the row
/// is a struct, and neither end of the module touches a socket.
///
/// `ty` carries the `type` position, because `type` is a keyword; nothing else
/// is renamed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColumnRow {
    /// The table the column belongs to.
    pub table: String,
    /// The column's name, in the server's own case.
    pub column: String,
    /// One-based declaration order. An **ordering key and not an index**:
    /// PostgreSQL leaves gaps where a column was dropped, so [`assemble`] sorts
    /// on this and never indexes with it.
    pub ordinal: i64,
    /// The server's own declared spelling, parameters included.
    pub ty: String,
    /// Whether the column accepts null.
    pub nullable: bool,
    /// The server's spelling of the default, where the column has one.
    pub default: Option<String>,
    /// Whether the server assigns the value.
    pub identity: bool,
}

/// One row of [`Read::Indexes`], in this module's own names.
///
/// [`ColumnRow`]'s note on the field types is this struct's too. A row is one
/// **column of one key**, so a composite key arrives as as many rows as it has
/// columns and `ordinal` is their order within it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndexRow {
    /// The table the key belongs to.
    pub table: String,
    /// The constraint or index name, which is unique across the schema.
    pub index: String,
    /// One column of that key.
    pub column: String,
    /// One-based position within the key.
    pub ordinal: i64,
    /// Whether a duplicate is refused.
    pub unique: bool,
    /// Whether this is the table's primary key.
    pub primary: bool,
    /// The server's spelling of the index's predicate, where it has one.
    ///
    /// Only SQL Server answers it: the other three drop a partial index in
    /// their own statement, having no predicate the vocabulary can name.
    pub filter: Option<String>,
}

/// The two reads' rows as one [`Schema`], built through the same builders a
/// declared schema goes through.
///
/// Nothing here constructs a [`Table`] or a [`Column`] directly, so a value
/// that comes back from a server has passed every rule a value written by hand
/// passes — an introspection cannot mint a column [`crate::ddl`] would refuse
/// to emit, which is the whole reason those builders are the only door.
///
/// # The order of construction, which the row shapes decide
///
/// The primary key arrives on the **index** rows rather than the column ones
/// ([`Read`]'s own doc says why), so every column of a table is built before
/// any key is added to it: [`Table::new`] takes the columns, and
/// [`Table::primary_key`] then resolves names against them. Within a table,
/// columns are sorted by [`ColumnRow::ordinal`] and keys by
/// [`IndexRow::ordinal`]; tables, and the keys within one table, keep the order
/// they arrive in, which is the order every statement's `ORDER BY` fixes —
/// `every_catalog_query_orders_the_rows_the_assembly_walks` is that agreement,
/// and normalising two schemas that list one table's indexes in two orders is
/// § 5's job rather than this function's.
///
/// # What is refused, and what is merely dropped
///
/// The answers a row can be unreadable in are not the same kind of mistake,
/// and the direction of the error is what separates them:
///
/// - **A type spelling outside the vocabulary fails the whole read**, as
///   [`SchemaError::UnknownType`]. Dropping the column instead would
///   *under*-report: the diff would not propose removing a column the server
///   has, so `applySafe` would report convergence over a table that has not
///   converged — the plan saying nothing needs doing when it does, which is the
///   one failure [`column_default`]'s doc calls unrecoverable.
/// - **A default this reader cannot spell back is dropped**, exactly as
///   [`column_default`] answers `None`. That *over*-reports: the plan gains a
///   step setting a default the column may already have, which is visible in
///   the plan and costs a rewrite the server would accept.
/// - **An index row naming a table the column read did not report is
///   ignored**, because a base table with no columns is not a table this
///   vocabulary can hold at all. The table is then absent from the value, and
///   [`Schema`]'s own doc owns what the diff does with that.
///
/// # Errors
///
/// [`SchemaError::UnknownType`] as above, and every refusal the builders state
/// — a name that is not a bare identifier, a duplicate key name, an identity
/// column outside its primary key. A server can hold any of them; this is
/// where they are found rather than at the sink.
pub fn assemble(
    columns: &[ColumnRow],
    indexes: &[IndexRow],
    dialect: Dialect,
) -> Result<Schema, SchemaError> {
    let mut tables = Vec::new();
    for name in first_seen(columns.iter().map(|row| row.table.as_str())) {
        let mut rows: Vec<&ColumnRow> = columns.iter().filter(|row| row.table == name).collect();
        rows.sort_by_key(|row| row.ordinal);
        let built = rows
            .iter()
            .map(|row| column_of(row, dialect))
            .collect::<Result<Vec<Column>, SchemaError>>()?;
        let mut table = Table::new(name, built)?;
        let nullable: Vec<&str> = rows
            .iter()
            .filter(|row| row.nullable)
            .map(|row| row.column.as_str())
            .collect();
        let keys = indexes.iter().filter(|row| row.table == name);
        for key in first_seen(keys.clone().map(|row| row.index.as_str())) {
            let mut key_rows: Vec<&IndexRow> =
                keys.clone().filter(|row| row.index == key).collect();
            key_rows.sort_by_key(|row| row.ordinal);
            let names: Vec<&str> = key_rows.iter().map(|row| row.column.as_str()).collect();
            if !predicate_is_the_key_itself(&key_rows, &names, &nullable, dialect) {
                continue;
            }
            table = if key_rows[0].primary {
                table.primary_key(&names)?
            } else if key_rows[0].unique {
                table.unique(key, &names)?
            } else {
                table.index(key, &names)?
            };
        }
        tables.push(table);
    }
    Schema::new(tables)
}

/// Whether an index carrying a predicate is [`crate::ddl`]'s spelling of the
/// unique key over `names`, rather than a partial index the vocabulary cannot
/// hold.
///
/// An index with no predicate is every dialect's ordinary key and is kept. One
/// with a predicate is kept on SQL Server alone, because that is where a unique
/// key over a nullable column *is* a filtered index
/// (`rule:core-classes/a-unique-key-reads-nulls-as-distinct`), and only when
/// the predicate requires exactly this key's nullable columns to be present —
/// which is the emitter's own `WHERE` and nothing else. Anything else is
/// dropped, as a partial index has always been: reporting one with its
/// predicate discarded would put a *false* key in the value, and the diff would
/// then agree with a server it does not match.
fn predicate_is_the_key_itself(
    rows: &[&IndexRow],
    names: &[&str],
    nullable: &[&str],
    dialect: Dialect,
) -> bool {
    let Some(filter) = rows[0].filter.as_deref() else {
        return true;
    };
    if dialect != Dialect::SqlServer || rows[0].primary || !rows[0].unique {
        return false;
    }
    let wanted: Vec<&str> = names
        .iter()
        .copied()
        .filter(|name| {
            nullable
                .iter()
                .any(|other| other.eq_ignore_ascii_case(name))
        })
        .collect();
    present_columns(filter).is_some_and(|present| {
        present.len() == wanted.len()
            && wanted
                .iter()
                .all(|name| present.iter().any(|other| other.eq_ignore_ascii_case(name)))
    })
}

/// The columns a predicate requires to be present, or `None` for a predicate
/// that says anything else at all.
///
/// This is not a SQL parser and refuses everything it does not recognise. SQL
/// Server stores a predicate in its own normalised form — `WHERE dedupe_pending
/// IS NOT NULL` comes back as `([dedupe_pending] IS NOT NULL)`, delimited and
/// parenthesised however it was written — so the one shape accepted here is a
/// conjunction of `IS NOT NULL` over bare column names, which is the only shape
/// [`crate::ddl`] writes. A predicate this returns `None` for is an index the
/// value drops, which is the same answer it gave before any of them were read.
fn present_columns(filter: &str) -> Option<Vec<String>> {
    let mut rest = filter.trim();
    if let Some(inner) = rest
        .strip_prefix('(')
        .and_then(|text| text.strip_suffix(')'))
    {
        rest = inner;
    }
    let mut columns = Vec::new();
    for term in rest.split(" AND ") {
        let name = term.trim().strip_suffix("IS NOT NULL")?.trim();
        let name = name
            .strip_prefix('[')
            .and_then(|text| text.strip_suffix(']'))
            .unwrap_or(name);
        if name.is_empty() || !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
            return None;
        }
        columns.push(name.to_owned());
    }
    Some(columns)
}

/// The distinct values of `names`, in the order they first appear.
///
/// A catalog holds tens of tables and a table tens of columns, so this is a
/// scan rather than a map: the ordering is part of the answer, and a `HashMap`
/// would have to have it added back.
fn first_seen<'a>(names: impl Iterator<Item = &'a str>) -> Vec<&'a str> {
    let mut seen: Vec<&str> = Vec::new();
    for name in names {
        if !seen.contains(&name) {
            seen.push(name);
        }
    }
    seen
}

/// One [`ColumnRow`] through [`Column`]'s builders, in the order they compose.
///
/// [`Column::identity`] clears both nullability and the default, because the
/// server supplies the value — so an identity column takes neither step, and
/// the sequence expression a server prints as its default is never read at all.
fn column_of(row: &ColumnRow, dialect: Dialect) -> Result<Column, SchemaError> {
    let ty =
        scalar_type(&row.ty, dialect).ok_or_else(|| SchemaError::UnknownType(row.ty.clone()))?;
    let mut column = Column::new(&row.column, ty.clone())?;
    if row.nullable {
        column = column.null();
    }
    if row.identity {
        return column.identity();
    }
    match row
        .default
        .as_deref()
        .and_then(|text| column_default(text, &ty, dialect))
    {
        Some(default) => column.default(default),
        None => Ok(column),
    }
}

#[cfg(test)]
mod tests {
    use std::net::{SocketAddr, ToSocketAddrs};
    use std::time::{Duration, Instant};

    use super::*;
    use crate::conn::{Connection, Driver};
    use crate::ddl;
    use crate::matrix::{Location, Server};
    use crate::schema::Ident;
    use crate::{
        MariaConn, MariaTarget, MySqlConn, MySqlTarget, PgConn, PgTarget, TdsConn, TdsTarget,
    };

    /// Every read, in the order [`Read`] declares them.
    const READS: [Read; 2] = [Read::Columns, Read::Indexes];

    /// Every dialect, in the order [`Dialect`] declares them.
    const DIALECTS: [Dialect; 4] = [
        Dialect::PostgreSql,
        Dialect::MySql,
        Dialect::Sqlite,
        Dialect::SqlServer,
    ];

    #[test]
    fn postgres_reads_pg_catalog_and_the_others_read_information_schema() {
        for read in READS {
            let sql = query(read, Dialect::PostgreSql);
            assert!(sql.contains("pg_catalog."), "{read:?} left pg_catalog");
            assert!(
                !sql.to_ascii_lowercase().contains("information_schema"),
                "{read:?} reached information_schema"
            );
        }
        // The column read is `information_schema` on both of the others. The
        // index read is not, and cannot be on SQL Server: that catalog has no
        // view over indexes, which the module doc owns.
        for dialect in [Dialect::MySql, Dialect::SqlServer] {
            let sql = query(Read::Columns, dialect).to_ascii_lowercase();
            assert!(
                sql.contains("information_schema.columns"),
                "{dialect:?} read its columns from somewhere else"
            );
            assert!(
                !sql.contains("pg_catalog"),
                "{dialect:?} reached pg_catalog"
            );
        }
        let mysql = query(Read::Indexes, Dialect::MySql).to_ascii_lowercase();
        assert!(mysql.contains("information_schema.statistics"));
        assert!(query(Read::Indexes, Dialect::SqlServer).contains("sys.indexes"));
    }

    #[test]
    fn the_catalog_queries_follow_dialect_rather_than_driver() {
        // An agreement rather than a table of expected texts: MariaDB and
        // MySQL are two drivers for their authentication plugins and error
        // tables, and neither of those reaches a catalog query. Every other
        // pairing must differ, or a dialect would be reading a catalog it has
        // not got.
        const DRIVERS: [Driver; 5] = [
            Driver::Postgres,
            Driver::MySql,
            Driver::MariaDb,
            Driver::Sqlite,
            Driver::SqlServer,
        ];
        for read in READS {
            for left in DRIVERS {
                for right in DRIVERS {
                    let same = query(read, Dialect::of(left)) == query(read, Dialect::of(right));
                    assert_eq!(
                        same,
                        Dialect::of(left) == Dialect::of(right),
                        "{read:?} on {left:?} and {right:?} agrees with the driver, not the dialect"
                    );
                }
            }
        }
    }

    #[test]
    fn no_catalog_query_carries_a_parameter_marker() {
        for read in READS {
            for dialect in DIALECTS {
                let sql = query(read, dialect);
                for marker in ["?", "$1", "@p1"] {
                    assert!(
                        !sql.contains(marker),
                        "{read:?} on {dialect:?} carries {marker}"
                    );
                }
            }
        }
    }

    /// Two tables covering both branches of [`SQLITE_INDEXES`]: `wide`'s
    /// primary key is the rowid, which `pragma_index_list` does not report at
    /// all, and `pair`'s is a composite, which it reports as an
    /// `sqlite_autoindex_…` the second branch has to drop.
    fn sqlite_fixture() -> Vec<Table> {
        let wide = Table::new(
            "wide",
            vec![
                Column::new("id", ScalarType::Int(IntWidth::Big))
                    .unwrap()
                    .identity()
                    .unwrap(),
                Column::new("label", ScalarType::Text { max: Some(200) }).unwrap(),
                Column::new("note", ScalarType::Text { max: None })
                    .unwrap()
                    .null(),
            ],
        )
        .unwrap()
        .primary_key(&["id"])
        .unwrap()
        .unique("wide_label", &["label"])
        .unwrap()
        .index("wide_note", &["note"])
        .unwrap();
        let pair = Table::new(
            "pair",
            vec![
                Column::new("left_id", ScalarType::Int(IntWidth::Normal)).unwrap(),
                Column::new("right_id", ScalarType::Int(IntWidth::Normal)).unwrap(),
            ],
        )
        .unwrap()
        .primary_key(&["left_id", "right_id"])
        .unwrap();
        vec![wide, pair]
    }

    #[test]
    fn sqlite_reads_sqlite_master_and_its_pragmas() {
        for read in READS {
            let sql = query(read, Dialect::Sqlite);
            assert!(sql.contains("sqlite_master"), "{read:?} left sqlite_master");
            assert!(!sql.to_ascii_lowercase().contains("information_schema"));
            assert!(!sql.contains("pg_catalog"));
        }
        assert!(query(Read::Columns, Dialect::Sqlite).contains("pragma_table_info("));
        let indexes = query(Read::Indexes, Dialect::Sqlite);
        assert!(indexes.contains("pragma_index_list("));
        assert!(indexes.contains("pragma_index_info("));

        // And it runs. SQLite is the one dialect whose server is a file, so
        // its catalog read is the one that can be asserted against a real
        // database with no container: `rusqlite` is already this crate's
        // driver, and what is applied here is `crate::ddl`'s own emission.
        let db = rusqlite::Connection::open_in_memory().unwrap();
        for table in sqlite_fixture() {
            for statement in ddl::create_table(&table, Dialect::Sqlite) {
                db.execute_batch(&statement).unwrap();
            }
        }

        let columns = sqlite_columns(&db);
        let wide: Vec<_> = columns.iter().filter(|row| row.table == "wide").collect();
        assert_eq!(
            wide.iter()
                .map(|row| row.column.as_str())
                .collect::<Vec<_>>(),
            ["id", "label", "note"],
            "declaration order is `cid`, not the name"
        );
        assert_eq!(
            wide.iter().map(|row| row.ordinal).collect::<Vec<_>>(),
            [1, 2, 3]
        );
        // The rowid alias, read back as an identity: `INTEGER PRIMARY KEY` is
        // the whole of what SQLite records, `AUTOINCREMENT` being a
        // `sqlite_sequence` row and not a column property.
        assert_eq!((wide[0].ty.as_str(), wide[0].identity), ("INTEGER", true));
        assert!(!wide[1].identity && !wide[2].identity);
        // SQLite records no `notnull` for a rowid alias, so `id` would read
        // back nullable and no `Table` could be built from the row; the read
        // is what closes that, not the assembly above it.
        assert_eq!(
            (wide[0].nullable, wide[1].nullable, wide[2].nullable),
            (false, false, true),
            "only `note` was declared nullable"
        );

        let keys = sqlite_indexes(&db);
        // `wide`'s primary key has no index of its own, and `pair`'s has one
        // the second branch drops: both arrive through the first branch alone,
        // once each, in key order.
        let primary: Vec<_> = keys
            .iter()
            .filter(|row| row.primary)
            .map(|row| (row.table.as_str(), row.column.as_str(), row.ordinal))
            .collect();
        assert_eq!(
            primary,
            [
                ("pair", "left_id", 1),
                ("pair", "right_id", 2),
                ("wide", "id", 1)
            ]
        );
        assert!(
            !keys.iter().any(|row| row.table == "pair" && !row.primary),
            "`pair`'s `sqlite_autoindex_…` was reported beside its primary key"
        );
        // The unique constraint is an autoindex whose name SQLite minted, and
        // the plain index kept the name the schema gave it — gap 5 of
        // `crate::ddl`'s own doc, and § 5's to normalise rather than this
        // module's to hide.
        let unique: Vec<_> = keys
            .iter()
            .filter(|row| !row.primary && row.unique)
            .map(|row| row.column.as_str())
            .collect();
        assert_eq!(unique, ["label"]);
        assert!(keys.iter().any(|row| {
            (
                row.index.as_str(),
                row.column.as_str(),
                row.unique,
                row.primary,
            ) == ("wide_note", "note", false, false)
        }));
    }

    #[test]
    fn every_catalog_query_orders_the_rows_the_assembly_walks() {
        // The row shape is positional and the grouping above this module is by
        // table and then by index, so an unordered read would assemble a
        // different schema on a server that happened to return the rows in
        // another order — the failure a diff would then report as a change.
        for read in READS {
            assert_eq!(read.arity(), read.row().len());
            for dialect in DIALECTS {
                assert!(
                    query(read, dialect).contains("ORDER BY"),
                    "{read:?} on {dialect:?} leaves the row order to the server"
                );
            }
        }
    }

    /// The vocabulary's own family name for a type, by a match with no
    /// wildcard: a variant added to [`ScalarType`] stops this compiling, so
    /// the sweep below cannot quietly stop covering one.
    fn family(ty: &ScalarType) -> &'static str {
        match ty {
            ScalarType::Int(_) => "int",
            ScalarType::Uint(_) => "uint",
            ScalarType::Float(_) => "float",
            ScalarType::Decimal { .. } => "decimal",
            ScalarType::Text { .. } => "text",
            ScalarType::Bytes { .. } => "bytes",
            ScalarType::Bool => "bool",
            ScalarType::Date => "date",
            ScalarType::Time => "time",
            ScalarType::DateTime => "datetime",
            ScalarType::Instant => "instant",
            ScalarType::Uuid => "uuid",
            ScalarType::Json => "json",
        }
    }

    /// Every family in § 11's vocabulary, with both sides of every width and
    /// parameter the reverse map has to choose between.
    fn every_type() -> Vec<ScalarType> {
        vec![
            ScalarType::Int(IntWidth::Small),
            ScalarType::Int(IntWidth::Normal),
            ScalarType::Int(IntWidth::Big),
            ScalarType::Uint(IntWidth::Small),
            ScalarType::Uint(IntWidth::Normal),
            ScalarType::Uint(IntWidth::Big),
            ScalarType::Float(FloatWidth::Single),
            ScalarType::Float(FloatWidth::Double),
            ScalarType::Decimal {
                precision: 20,
                scale: 0,
            },
            ScalarType::Decimal {
                precision: 12,
                scale: 4,
            },
            ScalarType::Text { max: Some(200) },
            ScalarType::Text { max: None },
            ScalarType::Bytes { max: Some(64) },
            ScalarType::Bytes { max: None },
            ScalarType::Bool,
            ScalarType::Date,
            ScalarType::Time,
            ScalarType::DateTime,
            ScalarType::Instant,
            ScalarType::Uuid,
            ScalarType::Json,
        ]
    }

    /// `rule:core-classes/schema-introspection`: the reverse of [`ddl::column_type`] reads back every
    /// spelling that emitter writes.
    ///
    /// Asserted as a **round trip over the emitters** rather than as a table
    /// of expected names, because the map is deliberately not injective —
    /// `INTEGER` is both `int32` and `uint16` wherever there is no unsigned
    /// integer — and a table would have to state the choice twice, once here
    /// and once in the map. What matters is not which of the two comes back but
    /// that what comes back is the **same column**: its own emission must be
    /// the string that was read.
    #[test]
    fn every_catalog_spelling_the_emitter_wrote_reads_back_as_its_own_type() {
        let types = every_type();
        for wanted in [
            "int", "uint", "float", "decimal", "text", "bytes", "bool", "date", "time", "datetime",
            "instant", "uuid", "json",
        ] {
            assert!(
                types.iter().any(|ty| family(ty) == wanted),
                "the sweep covers no `{wanted}`"
            );
        }

        for dialect in DIALECTS {
            for ty in &types {
                let sql = ddl::column_type(ty, dialect);
                let read = scalar_type(&sql, dialect)
                    .unwrap_or_else(|| panic!("{dialect:?} cannot read `{sql}` back at all"));
                assert_eq!(
                    ddl::column_type(&read, dialect),
                    sql,
                    "{dialect:?} read `{sql}` as {read:?}, which is a different column"
                );
                // The one answer that would look plausible everywhere: the
                // canonical spelling is a different reader's input entirely.
                assert!(
                    ScalarType::from_spelling(&sql).is_err(),
                    "`{sql}` is also a canonical name, so the two readers overlap"
                );
            }
        }
    }

    /// The server's own words, which are not the emitter's — and the refusals.
    #[test]
    fn a_catalog_spelling_is_read_in_the_servers_words_and_not_the_emitters() {
        // PostgreSQL's `format_type` never says `VARCHAR` or `TIMESTAMPTZ`.
        assert_eq!(
            scalar_type("character varying(200)", Dialect::PostgreSql),
            Some(ScalarType::Text { max: Some(200) })
        );
        assert_eq!(
            scalar_type("timestamp with time zone", Dialect::PostgreSql),
            Some(ScalarType::Instant)
        );
        // A parameter in the middle of the name, and one the vocabulary has no
        // room for: gap 2 of this module's doc, in the read direction.
        assert_eq!(
            scalar_type("timestamp(3) without time zone", Dialect::PostgreSql),
            Some(ScalarType::DateTime)
        );
        assert_eq!(
            scalar_type("DATETIME2(7)", Dialect::SqlServer),
            Some(ScalarType::DateTime)
        );
        // Case and whitespace are the server's business, not the map's, and an
        // older MySQL still prints the display width it no longer needs.
        assert_eq!(
            scalar_type("  BIGINT   UNSIGNED ", Dialect::MySql),
            Some(ScalarType::Uint(IntWidth::Big))
        );
        assert_eq!(
            scalar_type("int(10) unsigned", Dialect::MySql),
            Some(ScalarType::Uint(IntWidth::Normal))
        );

        // A dialect reads its own vocabulary and not its neighbour's: MySQL's
        // boolean is a `BIT(1)` and PostgreSQL's is a `BOOLEAN`, and neither
        // spelling means anything on the other server.
        assert_eq!(scalar_type("bit(1)", Dialect::PostgreSql), None);
        assert_eq!(scalar_type("boolean", Dialect::MySql), None);

        // Refusals: a type outside § 11, a precision no vocabulary decimal can
        // carry, a zero width, and text that is not a type at all.
        for (spelling, dialect) in [
            ("hstore", Dialect::PostgreSql),
            ("numeric(1000, 500)", Dialect::PostgreSql),
            ("varchar(0)", Dialect::Sqlite),
            ("enum('a','b')", Dialect::MySql),
            ("", Dialect::SqlServer),
        ] {
            assert_eq!(
                scalar_type(spelling, dialect),
                None,
                "`{spelling}` read as a {dialect:?} type"
            );
        }
    }

    /// § 2's own name for a default, by a match with no wildcard: a case added
    /// to [`ColumnDefault`] stops this compiling.
    fn case(default: &ColumnDefault) -> &'static str {
        match default {
            ColumnDefault::Int(_) => "int",
            ColumnDefault::Uint(_) => "uint",
            ColumnDefault::Float(_) => "float",
            ColumnDefault::Decimal(_) => "decimal",
            ColumnDefault::Text(_) => "text",
            ColumnDefault::Bool(_) => "bool",
            ColumnDefault::Now => "now",
        }
    }

    /// Every case of § 2's closed set, on a type it fits, with the text values
    /// that carry an escape.
    fn every_default() -> Vec<(ColumnDefault, ScalarType)> {
        vec![
            (ColumnDefault::Int(-7), ScalarType::Int(IntWidth::Big)),
            (ColumnDefault::Uint(9), ScalarType::Uint(IntWidth::Normal)),
            (
                ColumnDefault::Float(1.5),
                ScalarType::Float(FloatWidth::Double),
            ),
            (
                ColumnDefault::Decimal("12.34".to_owned()),
                ScalarType::Decimal {
                    precision: 6,
                    scale: 2,
                },
            ),
            (
                ColumnDefault::Text("plain".to_owned()),
                ScalarType::Text { max: Some(20) },
            ),
            (
                ColumnDefault::Text("it's".to_owned()),
                ScalarType::Text { max: Some(20) },
            ),
            (
                ColumnDefault::Text("a\\b".to_owned()),
                ScalarType::Text { max: Some(20) },
            ),
            (ColumnDefault::Bool(true), ScalarType::Bool),
            (ColumnDefault::Bool(false), ScalarType::Bool),
            (ColumnDefault::Now, ScalarType::DateTime),
            (ColumnDefault::Now, ScalarType::Instant),
        ]
    }

    /// `rule:core-classes/schema-introspection`: the reverse of [`crate::ddl`]'s literal reads back every
    /// default that emitter writes, as the same case and the same value.
    ///
    /// A round trip again rather than a table of expected texts, and here the
    /// equality is exact — unlike a type, a default has no choice to make, so
    /// anything but the value that went in is a normalisation § 5 would have
    /// to invent to hide.
    #[test]
    fn a_default_the_emitter_wrote_reads_back_as_the_same_case() {
        let defaults = every_default();
        for wanted in ["int", "uint", "float", "decimal", "text", "bool", "now"] {
            assert!(
                defaults.iter().any(|(default, _)| case(default) == wanted),
                "the sweep covers no `{wanted}` default"
            );
        }

        for (default, ty) in &defaults {
            for dialect in DIALECTS {
                let sql = ddl::literal(default, ty, dialect);
                let read = column_default(&sql, ty, dialect).unwrap_or_else(|| {
                    panic!("{dialect:?} cannot read `{sql}` back as a {default:?}")
                });
                assert_eq!(&read, default, "{dialect:?} read `{sql}` as another value");
            }
        }
    }

    /// The wrappers a server adds that no emitter wrote, and the expressions
    /// that are refused rather than guessed at.
    #[test]
    fn a_default_is_read_in_the_servers_words_and_not_the_emitters() {
        let text20 = ScalarType::Text { max: Some(20) };
        // PostgreSQL labels every literal with the column's own type.
        assert_eq!(
            column_default("'hi'::character varying", &text20, Dialect::PostgreSql),
            Some(ColumnDefault::Text("hi".to_owned()))
        );
        assert_eq!(
            column_default(
                "'12.34'::numeric",
                &ScalarType::Decimal {
                    precision: 6,
                    scale: 2
                },
                Dialect::PostgreSql
            ),
            Some(ColumnDefault::Decimal("12.34".to_owned()))
        );
        // `CURRENT_TIMESTAMP` is stored as `now()` and printed as one.
        assert_eq!(
            column_default("now()", &ScalarType::DateTime, Dialect::PostgreSql),
            Some(ColumnDefault::Now)
        );
        assert_eq!(
            column_default("true", &ScalarType::Bool, Dialect::PostgreSql),
            Some(ColumnDefault::Bool(true))
        );
        // MySQL prints a literal with no quotes at all, which is why the
        // reader is type-directed: nothing about `hi` says it is text.
        assert_eq!(
            column_default("hi", &text20, Dialect::MySql),
            Some(ColumnDefault::Text("hi".to_owned()))
        );
        assert_eq!(
            column_default("current_timestamp()", &ScalarType::DateTime, Dialect::MySql),
            Some(ColumnDefault::Now)
        );
        // SQL Server parenthesises, sometimes twice, and has two clocks.
        assert_eq!(
            column_default(
                "((42))",
                &ScalarType::Int(IntWidth::Big),
                Dialect::SqlServer
            ),
            Some(ColumnDefault::Int(42))
        );
        assert_eq!(
            column_default("(N'hi')", &text20, Dialect::SqlServer),
            Some(ColumnDefault::Text("hi".to_owned()))
        );
        for clock in ["(getdate())", "(sysdatetimeoffset())"] {
            assert_eq!(
                column_default(clock, &ScalarType::Instant, Dialect::SqlServer),
                Some(ColumnDefault::Now),
                "{clock} is not this server's clock"
            );
        }

        // A `1` is whatever the column is, and `CURRENT_TIMESTAMP` is a
        // default only where a timestamp could be stored.
        assert_eq!(
            column_default("1", &ScalarType::Bool, Dialect::Sqlite),
            Some(ColumnDefault::Bool(true))
        );
        assert_eq!(
            column_default("1", &ScalarType::Int(IntWidth::Small), Dialect::Sqlite),
            Some(ColumnDefault::Int(1))
        );
        assert_eq!(
            column_default("CURRENT_TIMESTAMP", &text20, Dialect::Sqlite),
            Some(ColumnDefault::Text("CURRENT_TIMESTAMP".to_owned()))
        );

        // Refusals: an expression, a literal that ran on past its own quote, a
        // MySQL escape this vocabulary never writes, and a default on a type
        // `ColumnDefault::fits` refuses outright.
        for (spelling, ty, dialect) in [
            (
                "nextval('t_id_seq'::regclass)",
                ScalarType::Int(IntWidth::Big),
                Dialect::PostgreSql,
            ),
            ("uuid_generate_v4()", ScalarType::Uuid, Dialect::PostgreSql),
            ("'a' || 'b'", text20.clone(), Dialect::PostgreSql),
            ("'a\\nb'", text20.clone(), Dialect::MySql),
            ("'hi'", ScalarType::Text { max: None }, Dialect::PostgreSql),
        ] {
            assert_eq!(
                column_default(spelling, &ty, dialect),
                None,
                "`{spelling}` read as a {dialect:?} default on {ty:?}"
            );
        }
    }

    /// The `NULL` keyword is no default on every dialect, and MariaDB is the
    /// server that makes it matter.
    ///
    /// That server prints the keyword in `COLUMN_DEFAULT` for every nullable
    /// column nobody gave a default, so reading it as a value gave each one a
    /// four-character text default the emitter never wrote — a plan that
    /// rewrote the column on every apply and never converged. The quoted
    /// spelling beside it is the value, and it still reads as one: MariaDB
    /// quotes a text default, which is what leaves the keyword unambiguous
    /// there.
    #[test]
    fn the_null_keyword_is_no_default_and_a_quoted_null_is_the_text() {
        let text20 = ScalarType::Text { max: Some(20) };
        for dialect in DIALECTS {
            for spelling in ["NULL", "null", "(NULL)"] {
                assert_eq!(
                    column_default(spelling, &text20, dialect),
                    None,
                    "{dialect:?} read `{spelling}` as a value"
                );
            }
            assert_eq!(
                column_default("'NULL'", &text20, dialect),
                Some(ColumnDefault::Text("NULL".to_owned())),
                "{dialect:?} lost a quoted `NULL`"
            );
        }
    }

    /// `rule:core-classes/a-unique-key-reads-nulls-as-distinct`: the filtered index SQL Server's
    /// emitter writes reads back as the unique key that asked for it, and every
    /// other predicate is still dropped.
    ///
    /// This is the half of the round trip that closes it: a key emitted as an
    /// index and read back as an index would be a plan that proposes the same
    /// unique key on every deployment, for ever. `plan` is what the assertion
    /// goes through rather than the value alone, because *converging* is what
    /// the rule promises and the value is only how it gets there.
    ///
    /// The predicates that must stay unreadable are asserted beside it: one
    /// over a value, one over a column the key does not name, and one naming
    /// only part of what the key filters. None is a `Schema` this vocabulary
    /// can hold, so each has to leave the key out of the value entirely rather
    /// than arrive with its predicate discarded.
    #[test]
    fn a_filtered_unique_index_reads_back_as_the_same_key() {
        let fixture = assembled_fixture();
        let (columns, indexes) = rows_of(&fixture, Dialect::SqlServer);
        let filtered: Vec<&IndexRow> = indexes.iter().filter(|row| row.filter.is_some()).collect();
        assert_eq!(
            filtered
                .iter()
                .map(|row| (row.index.as_str(), row.filter.as_deref().unwrap()))
                .collect::<Vec<_>>(),
            [("wide_slug", "([slug] IS NOT NULL)")],
            "the fixture's nullable unique key is the only filtered index on it"
        );

        let read = assemble(&columns, &indexes, Dialect::SqlServer).unwrap();
        let wide = read.table(&Ident::new("wide").unwrap()).unwrap();
        assert_eq!(
            wide.unique_keys()
                .iter()
                .map(|key| key.name().to_string())
                .collect::<Vec<_>>(),
            ["wide_label", "wide_slug"],
            "the filtered index came back as something other than its key"
        );
        assert_eq!(
            wide.indexes()
                .iter()
                .map(|key| key.name().to_string())
                .collect::<Vec<_>>(),
            ["wide_weight"],
            "a unique key read back as a plain index"
        );
        let plan = crate::plan::diff(&fixture, &read, Dialect::SqlServer);
        assert!(
            plan.is_empty(),
            "a converged SQL Server database still has a plan:\n{plan}"
        );

        for predicate in [
            "([weight] = 7)",
            "([label] IS NOT NULL)",
            "([slug] IS NOT NULL AND [weight] > 0)",
            "([slug] IS NOT NULL AND [note] IS NOT NULL)",
        ] {
            let mut altered = indexes.clone();
            for row in altered.iter_mut().filter(|row| row.index == "wide_slug") {
                row.filter = Some(predicate.to_owned());
            }
            let read = assemble(&columns, &altered, Dialect::SqlServer).unwrap();
            let wide = read.table(&Ident::new("wide").unwrap()).unwrap();
            assert_eq!(
                wide.unique_keys().len(),
                1,
                "`{predicate}` is not a key this vocabulary holds"
            );
        }
    }

    /// A schema carrying every part the assembly has to put back together: an
    /// identity primary key, a composite one, a unique constraint, a plain
    /// index, a nullable column and both kinds of default.
    ///
    /// It carries **no `uint` and no bounded `bytes`**, the families where
    /// what comes back depends on which server answered —
    /// `scalar_type`'s own doc names every case, and § 5 owes the declared
    /// side the same normalisation. A fixture carrying one would be asserting
    /// that gap closed rather than that the assembly is one function.
    ///
    /// Two of its names are load-bearing, because
    /// [`introspects_back_to_an_empty_plan`] applies this value to four real
    /// servers and each refused an earlier spelling of it:
    ///
    /// - **The index is over `weight` and not over the unbounded `note`.** SQL
    ///   Server refuses an index whose key column is `NVARCHAR(MAX)` outright,
    ///   and MySQL takes one only as [`crate::ddl`]'s prefix key, so an index
    ///   over unbounded text is a construct the vocabulary holds and the five
    ///   backends do not agree on. That disagreement is its own question and
    ///   `crate::schema`'s gap list is where it is written down; a fixture
    ///   carrying it would report it as a round-trip failure on every run.
    /// - **The `int` column is `weight` and not `rank`.** `RANK` is reserved on
    ///   MySQL 8, and `rule:core-classes/schema-is-a-value`'s identifiers are
    ///   validated rather than delimited — so a name a backend reserves is a
    ///   `CREATE TABLE` that server will not parse, whatever this crate does.
    ///
    /// **`slug` is nullable and unique**, which is the one construct whose SQL
    /// Server spelling is not a constraint at all
    /// (`rule:core-classes/a-unique-key-reads-nulls-as-distinct`): it is the
    /// filtered index [`crate::ddl`] writes there, and this fixture is what
    /// asks every server whether it reads back as the key that asked for it.
    /// `label` beside it is unique and `not null`, so a reader that answered
    /// the same way for both would fail here.
    fn assembled_fixture() -> Schema {
        let wide = Table::new(
            "wide",
            vec![
                Column::new("id", ScalarType::Int(IntWidth::Big))
                    .unwrap()
                    .identity()
                    .unwrap(),
                Column::new("label", ScalarType::Text { max: Some(200) }).unwrap(),
                Column::new("note", ScalarType::Text { max: None })
                    .unwrap()
                    .null(),
                Column::new("slug", ScalarType::Text { max: Some(64) })
                    .unwrap()
                    .null(),
                Column::new("weight", ScalarType::Int(IntWidth::Normal))
                    .unwrap()
                    .default(ColumnDefault::Int(7))
                    .unwrap(),
                Column::new("seen_at", ScalarType::Instant)
                    .unwrap()
                    .default(ColumnDefault::Now)
                    .unwrap(),
            ],
        )
        .unwrap()
        .primary_key(&["id"])
        .unwrap()
        .unique("wide_label", &["label"])
        .unwrap()
        .unique("wide_slug", &["slug"])
        .unwrap()
        .index("wide_weight", &["weight"])
        .unwrap();
        let pair = Table::new(
            "pair",
            vec![
                Column::new("left_id", ScalarType::Int(IntWidth::Normal)).unwrap(),
                Column::new("right_id", ScalarType::Int(IntWidth::Normal)).unwrap(),
            ],
        )
        .unwrap()
        .primary_key(&["left_id", "right_id"])
        .unwrap();
        Schema::new(vec![wide, pair]).unwrap()
    }

    /// The rows a server of `dialect` answers for `schema`, in [`Read::row`]'s
    /// positions.
    ///
    /// The type and the default are [`ddl`]'s own spellings, which makes the
    /// assertion below a round trip over the emitters rather than a table of
    /// strings written out twice. The rows are deliberately hostile: the
    /// ordinals are **gapped**, because PostgreSQL leaves a hole in
    /// `attnum` where a column was dropped and an assembly indexing with one
    /// would lose a column on any altered table, and the rows of each group
    /// arrive **backwards**, because the ordinal is what orders them and not
    /// the order they were handed over in.
    fn rows_of(schema: &Schema, dialect: Dialect) -> (Vec<ColumnRow>, Vec<IndexRow>) {
        let mut columns = Vec::new();
        let mut indexes = Vec::new();
        for table in schema.tables() {
            let name = table.name().to_string();
            let mut block: Vec<ColumnRow> = table
                .columns()
                .iter()
                .enumerate()
                .map(|(at, column)| ColumnRow {
                    table: name.clone(),
                    column: column.name().to_string(),
                    ordinal: i64::try_from(at).unwrap() * 3 + 1,
                    ty: ddl::column_type(column.ty(), dialect),
                    nullable: column.is_nullable(),
                    default: column
                        .default_value()
                        .map(|value| ddl::literal(value, column.ty(), dialect)),
                    identity: column.is_identity(),
                })
                .collect();
            block.reverse();
            columns.append(&mut block);

            // The primary key's name is one the server made up — `PRIMARY` on
            // MySQL, `wide_pkey` on PostgreSQL — and the assembly reads the
            // `primary` flag rather than the name, so this one is made up too.
            if !table.primary_key_columns().is_empty() {
                indexes.append(&mut key_rows(
                    &name,
                    &format!("{name}_pkey"),
                    table.primary_key_columns(),
                    true,
                    true,
                ));
            }
            for key in table.unique_keys() {
                let key_name = key.name().to_string();
                let mut block = key_rows(&name, &key_name, key.columns(), true, false);
                if let Some(predicate) = stored_filter(table, key.columns(), dialect) {
                    for row in &mut block {
                        row.filter = Some(predicate.clone());
                    }
                }
                indexes.append(&mut block);
            }
            for key in table.indexes() {
                let key_name = key.name().to_string();
                indexes.append(&mut key_rows(&name, &key_name, key.columns(), false, false));
            }
        }
        (columns, indexes)
    }

    /// What SQL Server stores as the predicate of the filtered unique index
    /// [`ddl`] writes for a key over a nullable column, and `None` for every
    /// key that is a plain constraint.
    ///
    /// The spelling is the **server's** and not the emitter's: SQL Server
    /// normalises the `WHERE slug IS NOT NULL` it was handed into
    /// `([slug] IS NOT NULL)` before it stores it, delimiters and parentheses
    /// added. A reader that matched the emitter's own text byte for byte would
    /// pass every assertion here and fail on the first real server.
    fn stored_filter(table: &Table, columns: &[Ident], dialect: Dialect) -> Option<String> {
        if dialect != Dialect::SqlServer {
            return None;
        }
        let terms: Vec<String> = columns
            .iter()
            .filter(|name| table.column(name).is_some_and(Column::is_nullable))
            .map(|name| format!("[{name}] IS NOT NULL"))
            .collect();
        (!terms.is_empty()).then(|| format!("({})", terms.join(" AND ")))
    }

    /// One key's rows, backwards, for the reason [`rows_of`] states.
    fn key_rows(
        table: &str,
        index: &str,
        columns: &[Ident],
        unique: bool,
        primary: bool,
    ) -> Vec<IndexRow> {
        let mut rows: Vec<IndexRow> = columns
            .iter()
            .enumerate()
            .map(|(at, column)| IndexRow {
                table: table.to_owned(),
                index: index.to_owned(),
                column: column.to_string(),
                ordinal: i64::try_from(at).unwrap() + 1,
                unique,
                primary,
                filter: None,
            })
            .collect();
        rows.reverse();
        rows
    }

    /// `rule:core-classes/schema-introspection`: the two reads answer one [`Schema`], whichever driver
    /// answered them.
    ///
    /// The claim is an **agreement across the drivers**, not a value written
    /// out here: each server is handed the spelling [`ddl`] wrote for it, and
    /// every one must arrive back at the schema that produced it. A reader that
    /// grew a per-server special case still passes its own dialect's round
    /// trip and fails here.
    #[test]
    fn every_driver_introspects_into_the_same_schema_value_shape() {
        const DRIVERS: [Driver; 5] = [
            Driver::Postgres,
            Driver::MySql,
            Driver::MariaDb,
            Driver::Sqlite,
            Driver::SqlServer,
        ];
        let fixture = assembled_fixture();
        for driver in DRIVERS {
            let dialect = Dialect::of(driver);
            let (columns, indexes) = rows_of(&fixture, dialect);
            let read = assemble(&columns, &indexes, dialect)
                .unwrap_or_else(|why| panic!("{driver:?} did not assemble its own rows: {why}"));
            assert_eq!(read, fixture, "{driver:?} read back a different schema");
        }

        // The refusal and the drop are opposite directions on purpose, and
        // `assemble`'s own doc says which is which: a type outside the
        // vocabulary would under-report a column, so it fails the read, while
        // a default outside it over-reports and costs a step in the plan.
        let (mut columns, indexes) = rows_of(&fixture, Dialect::PostgreSql);
        let at = columns
            .iter()
            .position(|row| row.column == "label")
            .unwrap();
        let spelling = columns[at].ty.clone();
        columns[at].ty = "citext".to_owned();
        assert_eq!(
            assemble(&columns, &indexes, Dialect::PostgreSql),
            Err(SchemaError::UnknownType("citext".to_owned())),
            "a type outside the vocabulary assembled into something"
        );
        columns[at].ty = spelling;
        columns[at].default = Some("'a' || 'b'".to_owned());
        let read = assemble(&columns, &indexes, Dialect::PostgreSql).unwrap();
        let label = read
            .table(&Ident::new("wide").unwrap())
            .unwrap()
            .column(&Ident::new("label").unwrap())
            .unwrap();
        assert_eq!(
            label.default_value(),
            None,
            "an expression default assembled into a value"
        );

        // A key on a table the column read did not report: there is no such
        // table in the value, so there is nothing for the key to attach to.
        let mut orphaned = indexes.clone();
        orphaned.append(&mut key_rows(
            "gone",
            "gone_pkey",
            &[Ident::new("id").unwrap()],
            true,
            true,
        ));
        assert_eq!(
            assemble(&columns, &orphaned, Dialect::PostgreSql).unwrap(),
            read,
            "a key for a table out of view reached the value"
        );
    }

    /// [`Read::Columns`] against a real SQLite, as the rows [`assemble`] takes.
    fn sqlite_columns(db: &rusqlite::Connection) -> Vec<ColumnRow> {
        let mut read = db.prepare(query(Read::Columns, Dialect::Sqlite)).unwrap();
        let rows = read
            .query_map([], |row| {
                Ok(ColumnRow {
                    table: row.get(0)?,
                    column: row.get(1)?,
                    ordinal: row.get(2)?,
                    ty: row.get(3)?,
                    nullable: row.get(4)?,
                    default: row.get(5)?,
                    identity: row.get(6)?,
                })
            })
            .unwrap();
        rows.collect::<Result<_, _>>().unwrap()
    }

    /// [`Read::Indexes`] against a real SQLite, as the rows [`assemble`] takes.
    fn sqlite_indexes(db: &rusqlite::Connection) -> Vec<IndexRow> {
        let mut read = db.prepare(query(Read::Indexes, Dialect::Sqlite)).unwrap();
        let rows = read
            .query_map([], |row| {
                Ok(IndexRow {
                    table: row.get(0)?,
                    index: row.get(1)?,
                    column: row.get(2)?,
                    ordinal: row.get(3)?,
                    unique: row.get(4)?,
                    primary: row.get(5)?,
                    filter: row.get(6)?,
                })
            })
            .unwrap();
        rows.collect::<Result<_, _>>().unwrap()
    }

    /// `schema` applied to a fresh in-memory database, read back and
    /// assembled.
    fn applied_and_read(schema: &Schema) -> Schema {
        let db = rusqlite::Connection::open_in_memory().unwrap();
        for statement in ddl::create_schema(schema, Dialect::Sqlite) {
            db.execute_batch(&statement).unwrap();
        }
        assemble(&sqlite_columns(&db), &sqlite_indexes(&db), Dialect::Sqlite).unwrap()
    }

    /// `rule:core-classes/schema-introspection`: the assembly, over the rows a real server answers.
    ///
    /// SQLite is the one dialect whose server is a file, so it is the one that
    /// can be asked this with no container — and it is also the harshest ask,
    /// because it is the dialect that rewrites the most of what it was given.
    ///
    /// The property asserted is a **fixed point**: what a server answers,
    /// emitted and applied again, must be answered back unchanged. That is the
    /// half of § 5's acceptance criterion this crate can prove alone. The
    /// other half — that the value applied and the value read are the same
    /// schema — is § 5's normalisation, and the places it is owed here are
    /// pinned below rather than left for it to discover.
    #[test]
    fn a_schema_applied_to_sqlite_assembles_back_into_itself() {
        let applied = Schema::new(sqlite_fixture()).unwrap();
        let read = applied_and_read(&applied);
        assert_eq!(
            read,
            applied_and_read(&read),
            "a second round changed the value, so the read is not a fixed point"
        );

        let wide = read.table(&Ident::new("wide").unwrap()).unwrap();
        assert_eq!(
            wide.columns()
                .iter()
                .map(|column| column.name().to_string())
                .collect::<Vec<_>>(),
            ["id", "label", "note"],
            "declaration order did not survive the round trip"
        );
        assert!(wide.columns()[0].is_identity() && !wide.columns()[2].is_identity());
        assert!(!wide.columns()[1].is_nullable() && wide.columns()[2].is_nullable());
        assert_eq!(wide.primary_key_columns(), [Ident::new("id").unwrap()]);
        assert_eq!(
            wide.unique_keys()[0].columns(),
            [Ident::new("label").unwrap()]
        );
        assert_eq!(wide.indexes()[0].name(), &Ident::new("wide_note").unwrap());
        let pair = read.table(&Ident::new("pair").unwrap()).unwrap();
        assert_eq!(
            pair.primary_key_columns(),
            [
                Ident::new("left_id").unwrap(),
                Ident::new("right_id").unwrap()
            ],
            "the composite key lost its order"
        );

        // The differences from the value that was applied, none of them this
        // module's to hide. § 5 normalises each out of the *comparison*
        // and not out of the value — `crate::plan`'s `same_key` and
        // `same_column` are where, and
        // `an_applied_schema_introspects_back_to_an_empty_plan_on_sqlite` is
        // what that buys — so each is still true of what a read answers,
        // and a session that changes the read itself fails here.
        assert_eq!(
            read.tables()
                .iter()
                .map(|table| table.name().to_string())
                .collect::<Vec<_>>(),
            ["pair", "wide"],
            "a read is ordered by name and a built schema by declaration"
        );
        assert!(
            wide.unique_keys()[0]
                .name()
                .to_string()
                .starts_with("sqlite_autoindex_"),
            "SQLite mints the name of an inline UNIQUE — `crate::ddl`'s gap 5"
        );
        assert_eq!(
            wide.columns()[0].ty(),
            &ScalarType::Int(IntWidth::Normal),
            "`INTEGER PRIMARY KEY` is the rowid spelling whatever width was declared"
        );
        assert_eq!(
            applied
                .table(&Ident::new("wide").unwrap())
                .unwrap()
                .columns()[0]
                .ty(),
            &ScalarType::Int(IntWidth::Big),
            "the fixture stopped declaring the width that gets rewritten"
        );
    }

    /// `rule:core-classes/schema-is-a-value`'s acceptance criterion, on the one backend whose server is
    /// a file: apply a schema to it, introspect it back, and the plan between
    /// the two is **empty**.
    ///
    /// This is the property the whole goal reduces to, and the reason every
    /// normalisation in [`crate::plan::diff`] exists — the differences
    /// `a_schema_applied_to_sqlite_assembles_back_into_itself` pins in the
    /// *value* are what this asserts are not differences in the *plan*. The
    /// other backends ask the same question of a container, and
    /// `docs/agent/loop-goal.toml` runs those.
    #[test]
    fn an_applied_schema_introspects_back_to_an_empty_plan_on_sqlite() {
        let applied = Schema::new(sqlite_fixture()).unwrap();
        let read = applied_and_read(&applied);
        let plan = crate::plan::diff(&applied, &read, Dialect::Sqlite);
        assert!(
            plan.is_empty(),
            "the schema this server was given is not the schema it answers:\n{plan}"
        );
    }

    /// How long one matrix handshake below may take.
    ///
    /// `crates/nvs-db/tests/handshake.rs`'s bound, for its reason: the server
    /// is a container the harness may have started moments ago, and a leg that
    /// hangs reports nothing at all.
    const MATRIX_DEADLINE: Duration = Duration::from_secs(10);

    /// This process's server, when the matrix pointed it at `driver`'s.
    ///
    /// [`crate::matrix`]'s skip rule, written once over the driver rather than
    /// four times as `handshake.rs` writes it: the matrix runs one driver per
    /// process, so each case below asks for the one backend it can assert
    /// about and the other three return without asserting.
    fn matrix_server(driver: Driver) -> Option<Server> {
        let endpoint = crate::matrix::endpoint()?;
        if endpoint.driver != driver {
            return None;
        }
        let Location::Server(server) = endpoint.location else {
            unreachable!("SQLite is the only driver reached by path, and this is not it")
        };
        Some(server)
    }

    /// Where the harness published `server`, as the address a driver opens.
    ///
    /// Resolved here rather than inside a driver for the reason
    /// `handshake.rs`'s twin gives: the *name* stays on the target because that
    /// is what the certificate is checked against.
    fn matrix_address(server: &Server) -> SocketAddr {
        (server.host.as_str(), server.port)
            .to_socket_addrs()
            .expect("the matrix host is an address")
            .next()
            .expect("the matrix host resolves to somewhere")
    }

    /// `rule:core-classes/schema-is-a-value`'s acceptance criterion against a
    /// real server: apply [`assembled_fixture`] to `conn`, introspect it back
    /// through the same reader `nvs schema plan` uses, and the plan between the
    /// two is **empty**.
    ///
    /// The four cases below are this function and a handshake, because the
    /// property is one property: what differs between the backends is which
    /// spellings [`crate::ddl`] wrote and which words the catalog answered, and
    /// § 5 normalises every one of those out of the *comparison*. A case that
    /// asserted a per-backend list of tolerated differences would be asserting
    /// this module's current gaps rather than the criterion.
    ///
    /// **The fixture's own tables are what is compared, not the database.** A
    /// matrix server is shared — the queue's cases push rows into `nvs_jobs` on
    /// the same database — and every table a schema value does not declare is a
    /// `DropTable` report, which `rule:core-classes/schema-absence-never-destroys`
    /// is right to raise and which says nothing about whether what was applied
    /// came back. Narrowing the *read* keeps this case about the round trip;
    /// that a report is raised and never run is
    /// `a_table_the_schema_does_not_declare_is_reported_and_never_dropped`'s
    /// question.
    ///
    /// The tables are dropped first rather than created `if not exists`: the
    /// emitter writes no such guard ([`ddl::create_table`]), and a run against
    /// a table an *earlier* fixture created would otherwise assert about that
    /// one.
    fn introspects_back_to_an_empty_plan(conn: &mut Connection) {
        let dialect = Dialect::of(conn.driver());
        let applied = assembled_fixture();
        for table in applied.tables() {
            let drop = format!("DROP TABLE IF EXISTS {}", table.name());
            crate::direct::run(conn, &drop).expect("the server ran the drop");
        }
        for statement in ddl::create_schema(&applied, dialect) {
            crate::direct::run(conn, &statement)
                .unwrap_or_else(|err| panic!("the server refused `{statement}`: {err}"));
        }

        let read = crate::direct::schema_of(conn).expect("the catalog answered a schema");
        let mine: Vec<Table> = read
            .tables()
            .iter()
            .filter(|table| applied.table(table.name()).is_some())
            .cloned()
            .collect();
        let plan = crate::plan::diff(&applied, &Schema::new(mine).unwrap(), dialect);
        assert!(
            plan.is_empty(),
            "the schema this server was given is not the schema it answers:\n{plan}"
        );
    }

    /// [`introspects_back_to_an_empty_plan`] on PostgreSQL.
    #[test]
    fn an_applied_schema_introspects_back_to_an_empty_plan_on_postgres() {
        let Some(server) = matrix_server(Driver::Postgres) else {
            return;
        };
        let target = PgTarget {
            host: &server.host,
            user: &server.user,
            password: &server.password,
            database: &server.database,
            tls_ca_file: Some(server.ca.as_path()),
            time_zone: 0,
            statement_cache: 8,
        };
        let conn = PgConn::connect(
            matrix_address(&server),
            &target,
            Some(Instant::now() + MATRIX_DEADLINE),
        )
        .expect("the matrix server accepts a handshake verified against its own anchor");
        introspects_back_to_an_empty_plan(&mut Connection::Postgres(conn));
    }

    /// [`introspects_back_to_an_empty_plan`] on MySQL.
    #[test]
    fn an_applied_schema_introspects_back_to_an_empty_plan_on_mysql() {
        let Some(server) = matrix_server(Driver::MySql) else {
            return;
        };
        let target = MySqlTarget {
            host: &server.host,
            user: &server.user,
            password: &server.password,
            database: &server.database,
            tls_ca_file: Some(server.ca.as_path()),
            time_zone: 0,
            statement_cache: 8,
        };
        let conn = MySqlConn::connect(
            matrix_address(&server),
            &target,
            Some(Instant::now() + MATRIX_DEADLINE),
        )
        .expect("the matrix server accepts a handshake verified against its own anchor");
        introspects_back_to_an_empty_plan(&mut Connection::MySql(conn));
    }

    /// [`introspects_back_to_an_empty_plan`] on MariaDB, which shares MySQL's
    /// dialect and answers its own catalog.
    #[test]
    fn an_applied_schema_introspects_back_to_an_empty_plan_on_mariadb() {
        let Some(server) = matrix_server(Driver::MariaDb) else {
            return;
        };
        let target = MariaTarget {
            host: &server.host,
            user: &server.user,
            password: &server.password,
            database: &server.database,
            tls_ca_file: Some(server.ca.as_path()),
            time_zone: 0,
            statement_cache: 8,
        };
        let conn = MariaConn::connect(
            matrix_address(&server),
            &target,
            Some(Instant::now() + MATRIX_DEADLINE),
        )
        .expect("the matrix server accepts a handshake verified against its own anchor");
        introspects_back_to_an_empty_plan(&mut Connection::MariaDb(conn));
    }

    /// [`introspects_back_to_an_empty_plan`] on SQL Server.
    #[test]
    fn an_applied_schema_introspects_back_to_an_empty_plan_on_sqlserver() {
        let Some(server) = matrix_server(Driver::SqlServer) else {
            return;
        };
        let target = TdsTarget {
            host: &server.host,
            user: &server.user,
            password: &server.password,
            database: &server.database,
            tls_ca_file: Some(server.ca.as_path()),
            time_zone: 0,
            statement_cache: 8,
        };
        let conn = TdsConn::connect(
            matrix_address(&server),
            &target,
            Some(Instant::now() + MATRIX_DEADLINE),
        )
        .expect("the matrix server accepts a handshake verified against its own anchor");
        introspects_back_to_an_empty_plan(&mut Connection::SqlServer(conn));
    }
}
