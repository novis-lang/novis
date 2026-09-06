//! [ADR 0145](/docs/adr/0145-a-schema-is-a-value-core-db-schema-converges-a-closed.md)
//! § 4's catalog readers: the SQL that reads a database back, per dialect.
//!
//! § 4 refuses a DDL parser at every tier, so the reverse direction of
//! [`crate::ddl`] is **live introspection** — `pg_catalog` for PostgreSQL,
//! `information_schema` for MySQL, MariaDB and SQL Server, and
//! `sqlite_master` with the `pragma_*` table-valued functions for SQLite. This
//! module is the text half of that and holds no rows: [`query`] answers one
//! statement, `nvs-stdlib` issues it over a connection the program already has
//! (§ 9 makes planning an ordinary read under `db.connect`), and the assembly
//! back into a [`Schema`](crate::Schema) is the other half.
//!
//! # One row shape per read, and every dialect answers it
//!
//! The point of keying on [`Dialect`] rather than on
//! [`Driver`](crate::Driver) is not that the SQL is shared — none of it is —
//! but that the **rows are**. [`Read`] names the two reads an introspection
//! makes and [`Read::row`] names the columns each answers, in order, so the
//! code that turns rows into a schema is written once over positions rather
//! than five times over one server's column names. Every value that could
//! differ in spelling is normalised in the statement instead: a boolean is
//! `0`/`1` on all four, an ordinal is one-based on all four, and the type is
//! one string in the server's own declared spelling rather than the three or
//! four `information_schema` columns it is scattered across.
//!
//! Two consequences of that are worth stating because they look like
//! omissions:
//!
//! - **The primary key arrives through [`Read::Indexes`], not through
//!   [`Read::Columns`]** — with `primary` set — because three of the four
//!   report it as an index and the fourth can be made to. SQLite is the one
//!   that cannot: an `INTEGER PRIMARY KEY` is the rowid and has no entry in
//!   `pragma_index_list` at all, so its statement is a `UNION ALL` whose first
//!   branch synthesises the key from `pragma_table_info`'s `pk` and whose
//!   second branch drops the `origin = 'pk'` index that would otherwise report
//!   it twice.
//! - **SQL Server's index read is the one statement that reaches `sys`.** Its
//!   `INFORMATION_SCHEMA` has views for constraints and none for indexes, so a
//!   plain non-unique index is invisible there. Its *column* read is
//!   `INFORMATION_SCHEMA.COLUMNS` like MySQL's, which is what
//!   `postgres_reads_pg_catalog_and_the_others_read_information_schema`
//!   asserts.
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
//! 1. **An index outside § 11's vocabulary is not reported at all.** A partial
//!    or expression index is filtered out of every dialect's index read —
//!    `indexprs`/`indpred` on PostgreSQL, `has_filter` on SQL Server,
//!    `partial` and a null `pragma_index_info` name on SQLite, and a null
//!    `column_name` on MySQL. A [`Schema`](crate::Schema) cannot hold one, so
//!    reporting it with its predicate dropped would put a *false* index in the
//!    value and the diff would then agree with a server it does not match. The
//!    cost is the other way round: a plan that creates an index whose name is
//!    already taken by a partial one fails on the server rather than in the
//!    plan.
//! 2. **SQL Server's type spelling is assembled from three columns and covers
//!    lengths and `decimal` only.** `DATETIME_PRECISION` is not folded in, so a
//!    `datetime2(7)` reads back as `datetime2`. That is § 5's normalisation to
//!    own rather than this module's, since the write direction in
//!    [`crate::ddl`] emits one precision for every instant column.
//! 3. **No spelling here is a [`ScalarType`](crate::ScalarType) yet.**
//!    `ScalarType::from_spelling` reads *this vocabulary's* canonical names —
//!    `int64`, `text(200)` — and a catalog answers the server's — `bigint`,
//!    `character varying(200)`. The map from one to the other is per dialect
//!    and is the reverse of [`crate::ddl::column_type`]; it belongs with the
//!    row assembly, which is the next slice.
//! 4. **The read is one schema deep.** A PostgreSQL search path with two
//!    schemas on it, or a SQL Server object under a schema other than the
//!    login's default, is out of view. § 11 has no cross-schema construct, so
//!    there is nothing in the vocabulary to lose yet.

use crate::sql::Dialect;

/// One of the two reads an introspection makes.
///
/// Each answers a fixed row shape — [`Read::row`] — that is the same on all
/// five drivers, which is what lets the assembly above this module be written
/// once.
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
    /// The names are this module's rather than any server's, and they are the
    /// documentation for a positional read: nothing above depends on a server
    /// having called a column `ORDINAL_POSITION` or `cid`.
    ///
    /// - [`Read::Columns`] — `table`, `column`, `ordinal` (one-based
    ///   declaration order, an ordering key and not an index — PostgreSQL
    ///   leaves gaps where a column was dropped), `type` (the server's own
    ///   declared spelling, parameters included), `nullable` (`1` when the
    ///   column accepts null), `default` (the server's spelling, or null),
    ///   `identity` (`1` when the server assigns the value).
    /// - [`Read::Indexes`] — `table`, `index` (the constraint or index name),
    ///   `column`, `ordinal` (one-based position within the key), `unique`
    ///   (`1` when a duplicate is refused), `primary` (`1` when this is the
    ///   table's primary key).
    #[must_use]
    pub fn row(self) -> &'static [&'static str] {
        match self {
            Read::Columns => &[
                "table", "column", "ordinal", "type", "nullable", "default", "identity",
            ],
            Read::Indexes => &["table", "index", "column", "ordinal", "unique", "primary"],
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
/// that as an agreement over all five drivers.
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
const PG_COLUMNS: &str = r"SELECT c.relname,
       a.attname,
       a.attnum,
       pg_catalog.format_type(a.atttypid, a.atttypmod),
       CASE WHEN a.attnotnull THEN 0 ELSE 1 END,
       pg_catalog.pg_get_expr(d.adbin, d.adrelid),
       CASE WHEN a.attidentity <> '' THEN 1 ELSE 0 END
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
/// index it belonged to.
const PG_INDEXES: &str = r"SELECT c.relname,
       i.relname,
       a.attname,
       k.ord,
       CASE WHEN ix.indisunique THEN 1 ELSE 0 END,
       CASE WHEN ix.indisprimary THEN 1 ELSE 0 END
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
const MYSQL_COLUMNS: &str = r"SELECT c.table_name,
       c.column_name,
       c.ordinal_position,
       c.column_type,
       CASE WHEN c.is_nullable = 'YES' THEN 1 ELSE 0 END,
       c.column_default,
       CASE WHEN c.extra LIKE '%auto_increment%' THEN 1 ELSE 0 END
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
const MYSQL_INDEXES: &str = r"SELECT s.table_name,
       s.index_name,
       s.column_name,
       s.seq_in_index,
       CASE WHEN s.non_unique = 0 THEN 1 ELSE 0 END,
       CASE WHEN s.index_name = 'PRIMARY' THEN 1 ELSE 0 END
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
const SQLSERVER_COLUMNS: &str = r"SELECT c.TABLE_NAME,
       c.COLUMN_NAME,
       c.ORDINAL_POSITION,
       CASE
         WHEN c.CHARACTER_MAXIMUM_LENGTH = -1 THEN c.DATA_TYPE + '(max)'
         WHEN c.CHARACTER_MAXIMUM_LENGTH IS NOT NULL
           THEN c.DATA_TYPE + '(' + CAST(c.CHARACTER_MAXIMUM_LENGTH AS varchar(11)) + ')'
         WHEN c.DATA_TYPE IN ('decimal', 'numeric')
           THEN c.DATA_TYPE + '(' + CAST(c.NUMERIC_PRECISION AS varchar(11))
                + ',' + CAST(c.NUMERIC_SCALE AS varchar(11)) + ')'
         ELSE c.DATA_TYPE
       END,
       CASE WHEN c.IS_NULLABLE = 'YES' THEN 1 ELSE 0 END,
       c.COLUMN_DEFAULT,
       COLUMNPROPERTY(
         OBJECT_ID(QUOTENAME(c.TABLE_SCHEMA) + '.' + QUOTENAME(c.TABLE_NAME)),
         c.COLUMN_NAME,
         'IsIdentity')
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
/// b-trees and drops the heap, the XML, spatial and columnstore forms; an
/// included column is not a key column and `has_filter` is the filtered index
/// the vocabulary cannot hold.
const SQLSERVER_INDEXES: &str = r"SELECT t.name,
       i.name,
       c.name,
       ic.key_ordinal,
       CASE WHEN i.is_unique = 1 THEN 1 ELSE 0 END,
       CASE WHEN i.is_primary_key = 1 THEN 1 ELSE 0 END
FROM sys.indexes i
JOIN sys.tables t ON t.object_id = i.object_id
JOIN sys.schemas s ON s.schema_id = t.schema_id
JOIN sys.index_columns ic
  ON ic.object_id = i.object_id AND ic.index_id = i.index_id
JOIN sys.columns c ON c.object_id = ic.object_id AND c.column_id = ic.column_id
WHERE s.name = SCHEMA_NAME()
  AND i.type IN (1, 2)
  AND i.has_filter = 0
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
const SQLITE_COLUMNS: &str = r#"SELECT m.name,
       p.name,
       p.cid + 1,
       p.type,
       CASE WHEN p."notnull" = 0 AND p.pk = 0 THEN 1 ELSE 0 END,
       p.dflt_value,
       CASE WHEN p.pk = 1 AND UPPER(p.type) = 'INTEGER'
                 AND (SELECT COUNT(*) FROM pragma_table_info(m.name) q WHERE q.pk > 0) = 1
            THEN 1 ELSE 0 END
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
const SQLITE_INDEXES: &str = r#"SELECT m.name, 'PRIMARY', p.name, p.pk, 1, 1
FROM sqlite_master m
JOIN pragma_table_info(m.name) p
WHERE m.type = 'table'
  AND m.name NOT LIKE 'sqlite_%'
  AND p.pk > 0
UNION ALL
SELECT m.name, il.name, ii.name, ii.seqno + 1,
       CASE WHEN il."unique" = 1 THEN 1 ELSE 0 END,
       0
FROM sqlite_master m
JOIN pragma_index_list(m.name) il
JOIN pragma_index_info(il.name) ii
WHERE m.type = 'table'
  AND m.name NOT LIKE 'sqlite_%'
  AND il.origin <> 'pk'
  AND il.partial = 0
  AND ii.name IS NOT NULL
ORDER BY 1, 2, 4"#;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::conn::Driver;
    use crate::ddl;
    use crate::schema::{Column, IntWidth, ScalarType, Table};

    /// The two reads, in the order [`Read`] declares them.
    const READS: [Read; 2] = [Read::Columns, Read::Indexes];

    /// The four dialects, in the order [`Dialect`] declares them.
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
        // An agreement rather than eight expected texts: MariaDB and MySQL are
        // two drivers for their authentication plugins and error tables, and
        // neither of those reaches a catalog query. The other four pairings
        // must differ, or a dialect would be reading a catalog it has not got.
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

    /// One row of [`Read::Columns`], in the order [`Read::row`] names.
    type ColumnRow = (String, String, i64, String, i64, Option<String>, i64);

    /// One row of [`Read::Indexes`], in the order [`Read::row`] names.
    type KeyRow = (String, String, String, i64, i64, i64);

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

        let mut columns = db.prepare(query(Read::Columns, Dialect::Sqlite)).unwrap();
        let columns: Vec<ColumnRow> = columns
            .query_map([], |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                    row.get(6)?,
                ))
            })
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        let wide: Vec<_> = columns.iter().filter(|row| row.0 == "wide").collect();
        assert_eq!(
            wide.iter().map(|row| row.1.as_str()).collect::<Vec<_>>(),
            ["id", "label", "note"],
            "declaration order is `cid`, not the name"
        );
        assert_eq!(wide.iter().map(|row| row.2).collect::<Vec<_>>(), [1, 2, 3]);
        // The rowid alias, read back as an identity: `INTEGER PRIMARY KEY` is
        // the whole of what SQLite records, `AUTOINCREMENT` being a
        // `sqlite_sequence` row and not a column property.
        assert_eq!((wide[0].3.as_str(), wide[0].6), ("INTEGER", 1));
        assert_eq!((wide[1].6, wide[2].6), (0, 0));
        // SQLite records no `notnull` for a rowid alias, so `id` would read
        // back nullable and no `Table` could be built from the row; the read
        // is what closes that, not the assembly above it.
        assert_eq!(
            (wide[0].4, wide[1].4, wide[2].4),
            (0, 0, 1),
            "only `note` was declared nullable"
        );

        let mut keys = db.prepare(query(Read::Indexes, Dialect::Sqlite)).unwrap();
        let keys: Vec<KeyRow> = keys
            .query_map([], |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                ))
            })
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        // `wide`'s primary key has no index of its own, and `pair`'s has one
        // the second branch drops: both arrive through the first branch alone,
        // once each, in key order.
        let primary: Vec<_> = keys
            .iter()
            .filter(|row| row.5 == 1)
            .map(|row| (row.0.as_str(), row.2.as_str(), row.3))
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
            !keys.iter().any(|row| row.0 == "pair" && row.5 == 0),
            "`pair`'s `sqlite_autoindex_…` was reported beside its primary key"
        );
        // The unique constraint is an autoindex whose name SQLite minted, and
        // the plain index kept the name the schema gave it — gap 5 of
        // `crate::ddl`'s own doc, and § 5's to normalise rather than this
        // module's to hide.
        let unique: Vec<_> = keys
            .iter()
            .filter(|row| row.5 == 0 && row.4 == 1)
            .map(|row| row.2.as_str())
            .collect();
        assert_eq!(unique, ["label"]);
        assert!(keys.iter().any(
            |row| (row.1.as_str(), row.2.as_str(), row.4, row.5) == ("wide_note", "note", 0, 0)
        ));
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
}
