//! `rule:core-classes/schema-plan` and `rule:core-classes/schema-absence-never-destroys`: a plan is a document, and a step in it is a change plus the reason
//! it costs what it costs.
//!
//! This module is the plan's *vocabulary* and holds no SQL. What a change is
//! spelled as, and what it is graded, are one dialect's questions and both are
//! [`crate::ddl::step`]'s — § 6 puts the grade in the emitter because the same
//! change is not the same risk on two backends, and a grade computed anywhere
//! else would have to be corrected there anyway.
//!
//! It is also where [`diff`] lives, and by the same division: § 5's comparison
//! reads two [`crate::schema::Schema`] values and reaches no server, so
//! *finding* a difference is a question about values where spelling and grading
//! it are questions about a server. The diff answers the first and hands each
//! change it found to [`crate::ddl::step`] for the second.
//!
//! The one thing it asks a dialect is what that dialect can *hold*, because a
//! type the server cannot tell from another is not a difference: [`stored_as`]
//! is § 5's normalisation, the single function every lossy case lives in, and
//! the comparison either side of it stays a plain equality.
//!
//! # A step is never elided
//!
//! Every [`Step`] carries complete, terminated SQL, **including the ones the
//! applier refuses to run**. § 8's argument for that is the deployment where
//! the application's own credentials cannot issue DDL at all: the change is
//! made by a DBA, in a window, from a ticket, and a plan that summarised its
//! risky half as "3 unsafe changes" would be useless to that person. A step
//! Novis will not apply is still a step Novis shows in full.
//!
//! # Two kinds of step, and only one of them is ever run
//!
//! § 7's rule — absence never destroys — makes every drop a **report**: a
//! table, column, constraint or index that the database has and the schema
//! value does not is *shown*, with the SQL that would remove it, and is never
//! removed. [`Change::is_report`] is that classification, and it is total on
//! the vocabulary rather than a flag a caller sets.
//!
//! It is also why [`Plan::first_refused`] reads the grade of the steps it
//! would **run**. Every plan against a shared database holds reports, they are
//! [`Grade::Destructive`] by the paragraph above, and a rule that read every
//! step's grade would therefore refuse every plan ever computed against a real
//! server — which is § 9's `applySafe` never working once.

use std::fmt;

use crate::catalog;
use crate::ddl;
use crate::schema::{Column, Ident, Key, ScalarType, Schema, Table};
use crate::sql::Dialect;

/// § 6's grades: what a step can cost, at worst.
///
/// Ordered, and the order is the point: [`Grade::up_to`] is § 6's "an unknown
/// grade grades up" as an operation, and an emitter with no rule for a case
/// takes the higher of the two rather than guessing. An out-of-date grader
/// over-reports risk, which costs an operator a confirmation; an out-of-date
/// optimist costs them an outage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Grade {
    /// Cannot lose data, cannot fail on existing rows, cannot hold a long lock.
    Safe,
    /// Cannot lose data, but can fail on existing rows or block writes for a
    /// long time — a unique key over data that already collides, `NOT NULL` on
    /// a populated column, a type change that rewrites the table.
    Locking,
    /// Can lose data. Every drop is here, by § 7, and so is SQLite's rebuild.
    Destructive,
}

impl Grade {
    /// The higher of two grades — § 6's grade-up rule, as a function.
    #[must_use]
    pub fn up_to(self, other: Grade) -> Grade {
        if other > self { other } else { self }
    }

    /// The word a plan document prints for this grade.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Grade::Safe => "safe",
            Grade::Locking => "locking",
            Grade::Destructive => "destructive",
        }
    }
}

impl fmt::Display for Grade {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Whether a key constrains its columns or only indexes them.
///
/// The kinds are one variant of [`Change`] rather than several because they
/// differ only in a keyword everywhere but SQLite, where a unique constraint
/// added after the fact *is* a unique index — a distinction the emitter makes
/// and the vocabulary does not.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KeyKind {
    /// A `UNIQUE` constraint: the server refuses a duplicate.
    Unique,
    /// A plain index: nothing is refused.
    Index,
}

impl KeyKind {
    /// The word a plan document prints for this kind.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            KeyKind::Unique => "unique key",
            KeyKind::Index => "index",
        }
    }
}

/// One difference between a schema value and a database, in the closed set the
/// vocabulary of § 2 can express.
///
/// **Every `table` field is the table as it should be *after* the change**,
/// which is the one convention that makes SQLite's rebuild expressible: that
/// backend answers half of these by building the after-table under a temporary
/// name and copying into it, so the after-shape is what an emitter needs and
/// the before-shape is only ever a column list it does not select.
/// [`Change::ChangeColumn`] carries `from` as well, because the grade of a
/// type change is a question about both ends of it and nothing else here is.
///
/// There is no rename. A column is identified by its name in the canonical
/// form of § 1, so a rename is a drop and an add to any diff that compares
/// values — and by § 7 the drop half of that is reported rather than applied.
#[derive(Debug, Clone, PartialEq)]
pub enum Change {
    /// A table the schema names and the database does not have.
    CreateTable(Table),
    /// A table the database has and the schema does not name. A report.
    DropTable(Ident),
    /// A column added to a table both sides already have.
    AddColumn {
        /// The table, as it should be after the column is added.
        table: Table,
        /// The new column, which is one of `table`'s own.
        column: Column,
    },
    /// A column the database has and the schema does not name. A report.
    DropColumn {
        /// The table, as it should be after the column is gone.
        table: Table,
        /// The doomed column's name, which is *not* one of `table`'s.
        column: Ident,
    },
    /// A column whose type, nullability or default differs between the two.
    ChangeColumn {
        /// The table, as it should be after the change.
        table: Table,
        /// The column as the database has it — read for the grade alone.
        from: Column,
        /// The column as the schema wants it, which is one of `table`'s own.
        to: Column,
    },
    /// A unique constraint or an index the schema names and the database lacks.
    AddKey {
        /// The table, as it should be after the key exists.
        table: Table,
        /// The key itself, which is one of `table`'s own.
        key: Key,
        /// Whether it constrains or only indexes.
        kind: KeyKind,
    },
    /// A unique constraint or an index the database has and the schema does
    /// not name. A report.
    DropKey {
        /// The table, as it should be after the key is gone.
        table: Table,
        /// The doomed key's name, which is *not* one of `table`'s.
        key: Ident,
        /// Whether it constrains or only indexes.
        kind: KeyKind,
    },
}

impl Change {
    /// The table this change is about, whether or not that table survives it.
    #[must_use]
    pub fn table_name(&self) -> &Ident {
        match self {
            Change::CreateTable(table) => table.name(),
            Change::DropTable(name) => name,
            Change::AddColumn { table, .. }
            | Change::DropColumn { table, .. }
            | Change::ChangeColumn { table, .. }
            | Change::AddKey { table, .. }
            | Change::DropKey { table, .. } => table.name(),
        }
    }

    /// Whether this is § 7's *report*: a step the plan carries, prints in
    /// full, and never applies.
    ///
    /// Total on the vocabulary and not a flag: a change is a report exactly
    /// when it removes something, because a schema value describes what its
    /// author knows about and a database legitimately holds another
    /// application's tables, an operator's own, and — under
    /// `rule:concurrency/enqueue-commits-with-your-write` — the queue's two.
    #[must_use]
    pub fn is_report(&self) -> bool {
        matches!(
            self,
            Change::DropTable(_) | Change::DropColumn { .. } | Change::DropKey { .. }
        )
    }
}

impl fmt::Display for Change {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Change::CreateTable(table) => write!(f, "create table {}", table.name()),
            Change::DropTable(name) => write!(f, "drop table {name}"),
            Change::AddColumn { table, column } => {
                write!(f, "add column {}.{}", table.name(), column.name())
            }
            Change::DropColumn { table, column } => {
                write!(f, "drop column {}.{column}", table.name())
            }
            Change::ChangeColumn { table, to, .. } => {
                write!(f, "change column {}.{}", table.name(), to.name())
            }
            Change::AddKey { table, key, kind } => {
                write!(
                    f,
                    "add {} {} on {}",
                    kind.as_str(),
                    key.name(),
                    table.name()
                )
            }
            Change::DropKey { table, key, kind } => {
                write!(f, "drop {} {} on {}", kind.as_str(), key, table.name())
            }
        }
    }
}

/// One change, graded, with the SQL that makes it.
///
/// Built by [`crate::ddl::step`] and by nothing else: the fields past the
/// change are all one dialect's answers, and a `Step` assembled anywhere else
/// would be a second emitter.
#[derive(Debug, Clone)]
pub struct Step {
    change: Change,
    grade: Grade,
    reason: String,
    sql: Vec<String>,
}

impl Step {
    /// The step for `change` in a dialect that grades it `grade` for `reason`
    /// and spells it `sql`.
    pub(crate) fn new(change: Change, grade: Grade, reason: String, sql: Vec<String>) -> Step {
        Step {
            change,
            grade,
            reason,
            sql,
        }
    }

    /// What this step changes.
    #[must_use]
    pub fn change(&self) -> &Change {
        &self.change
    }

    /// What this step can cost, at worst.
    #[must_use]
    pub fn grade(&self) -> Grade {
        self.grade
    }

    /// Why it is graded that way, in one sentence an operator reads.
    #[must_use]
    pub fn reason(&self) -> &str {
        &self.reason
    }

    /// The complete, terminated, dialect-correct statements that make it.
    ///
    /// Never empty, and never elided — § 8's rule holds for a refused step
    /// exactly as it does for one the applier will run.
    #[must_use]
    pub fn sql(&self) -> &[String] {
        &self.sql
    }

    /// Whether this step is § 7's report, carried and never applied.
    #[must_use]
    pub fn is_report(&self) -> bool {
        self.change.is_report()
    }
}

/// A plan: every difference between a schema value and a database, in order.
#[derive(Debug, Clone, Default)]
pub struct Plan {
    steps: Vec<Step>,
}

impl Plan {
    /// The plan holding `steps`, in the order they must run.
    #[must_use]
    pub fn new(steps: Vec<Step>) -> Plan {
        Plan { steps }
    }

    /// Every step, reports included.
    #[must_use]
    pub fn steps(&self) -> &[Step] {
        &self.steps
    }

    /// How many steps there are, reports included.
    #[must_use]
    pub fn len(&self) -> usize {
        self.steps.len()
    }

    /// Whether the database already matches the schema value.
    ///
    /// This is § 5's acceptance criterion in one call: apply a schema to a
    /// server, introspect it back, and the plan against it is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.steps.is_empty()
    }

    /// How many steps carry `grade`, reports included.
    #[must_use]
    pub fn count(&self, grade: Grade) -> usize {
        self.steps.iter().filter(|step| step.grade == grade).count()
    }

    /// The steps that would actually run — everything that is not a report.
    pub fn runnable(&self) -> impl Iterator<Item = &Step> {
        self.steps.iter().filter(|step| !step.is_report())
    }

    /// The first step `applySafe` would refuse, or `None` if it would run.
    ///
    /// Reads the grades of the steps it would **run**, per § 7: a plan against
    /// any shared database holds `Destructive` reports it was never going to
    /// touch, and reading those too would refuse every real plan.
    #[must_use]
    pub fn first_refused(&self) -> Option<&Step> {
        self.runnable().find(|step| step.grade != Grade::Safe)
    }
}

impl fmt::Display for Plan {
    /// The plan as § 8's document: every step's grade and reason as a comment,
    /// then its SQL, in the one comment syntax every dialect reads.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (index, step) in self.steps.iter().enumerate() {
            if index > 0 {
                writeln!(f)?;
            }
            let carried = if step.is_report() {
                ", reported and never applied"
            } else {
                ""
            };
            writeln!(f, "-- [{}] {}{carried}", step.grade, step.change)?;
            writeln!(f, "-- {}", step.reason)?;
            for statement in &step.sql {
                writeln!(f, "{statement}")?;
            }
        }
        Ok(())
    }
}

/// Every difference between the schema a program declares and the schema a
/// database was introspected into, as the plan that closes it in `dialect`.
///
/// This is § 5's comparison and the whole of it: `want` and `have` are two
/// values of one vocabulary, and the diff reads their fields. No statement is
/// retrieved or parsed here and no server is reached — the SQL a step carries
/// is [`crate::ddl::step`]'s answer *after* the difference has already been
/// found, which is why one difference is the same [`Change`] on every dialect
/// and only its spelling and its grade move. The one statement fragment that
/// is written is [`stored_as`]'s, and it is never emitted: it is how the
/// comparison asks what `dialect` can hold.
///
/// The order is the order the steps must run in: `want`'s tables in
/// declaration order, and within a table its columns in declaration order —
/// each added or altered — then its new keys, then § 7's reports for what only
/// the database has. Every `DROP TABLE` report comes last. A report never
/// runs, so its position is a question about the document rather than about
/// correctness; [`Plan::runnable`] is what the applier reads.
///
/// **A difference the vocabulary cannot name produces no step.** § 2's set is
/// closed and [`Change`]'s is closed with it, so a primary key that differs
/// between two tables *both* sides already have is not reported: no change
/// expresses one, and § 6 grades nothing that does not exist. The case that
/// occurs is a table only one side has, which carries its key in its
/// `CREATE TABLE`. For the same reason a key whose columns or kind changed is
/// a drop and an add under one name, exactly as [`Change`]'s own doc says a
/// rename is — and by § 7 the drop half of that is reported rather than run.
///
/// § 5's normalisation is `dialect`'s other job. The ways a value a builder
/// wrote and the same value read back off the server it was applied to differ
/// without being different schemas, each normalised out of the comparison and
/// none of them out of the [`Change`] — what a step emits is always `want`'s
/// own spelling, because that is the name an operator wrote and expects to
/// read in the SQL:
///
/// 1. **Table order.** A read is ordered by whatever the catalog answers and a
///    built schema by declaration, so the diff matches tables by name and
///    never by position. The same holds of a table's columns.
/// 2. **A unique constraint's name** — [`same_key`], which is also § 5's
///    "implicitly created index".
/// 3. **Every type the dialect spells the same way as another** — [`stored_as`],
///    which folds both sides through the emitter's map and the catalog's
///    reader before they are compared.
/// 4. **The width of SQLite's rowid** — [`same_column`], and the one case
///    [`stored_as`] cannot reach, because that identity is written outside
///    [`crate::ddl::column_type`] altogether.
#[must_use]
pub fn diff(want: &Schema, have: &Schema, dialect: Dialect) -> Plan {
    let mut changes: Vec<Change> = Vec::new();
    for table in want.tables() {
        match have.table(table.name()) {
            Some(current) => diff_table(table, current, dialect, &mut changes),
            None => changes.push(Change::CreateTable(table.clone())),
        }
    }
    for table in have.tables() {
        if want.table(table.name()).is_none() {
            changes.push(Change::DropTable(table.name().clone()));
        }
    }
    Plan::new(
        changes
            .into_iter()
            .map(|change| ddl::step(change, dialect))
            .collect(),
    )
}

/// The differences between one table a program declares and the same table as
/// the database has it, appended to `out`.
///
/// Every change's `table` is `want`, because [`Change`]'s convention is the
/// table as it should be *after* the change — and that is true of a report
/// too, whose after-shape is the one the schema already describes.
fn diff_table(want: &Table, have: &Table, dialect: Dialect, out: &mut Vec<Change>) {
    for column in want.columns() {
        match have.column(column.name()) {
            None => out.push(Change::AddColumn {
                table: want.clone(),
                column: column.clone(),
            }),
            Some(current) if !same_column(want, current, column, dialect) => {
                out.push(Change::ChangeColumn {
                    table: want.clone(),
                    from: current.clone(),
                    to: column.clone(),
                });
            }
            Some(_) => {}
        }
    }
    for (kind, key) in keys(want) {
        if !keys(have).any(|(other_kind, other)| other_kind == kind && same_key(kind, key, other)) {
            out.push(Change::AddKey {
                table: want.clone(),
                key: key.clone(),
                kind,
            });
        }
    }
    for column in have.columns() {
        if want.column(column.name()).is_none() {
            out.push(Change::DropColumn {
                table: want.clone(),
                column: column.name().clone(),
            });
        }
    }
    for (kind, key) in keys(have) {
        if !keys(want).any(|(other_kind, other)| other_kind == kind && same_key(kind, key, other)) {
            out.push(Change::DropKey {
                table: want.clone(),
                key: key.name().clone(),
                kind,
            });
        }
    }
}

/// `ty` as `dialect` holds it — § 5's normalisation, and the one function
/// every lossy case lives in.
///
/// The map is not written out here, and deliberately: what the server is asked
/// for is whatever [`crate::ddl::column_type`] writes for `ty`, and what any
/// introspection of that column can then report is whatever
/// [`crate::catalog::scalar_type`] makes of that same string. Composing the two
/// is not a model of the round trip, it *is* the round trip with the server
/// left out — so a spelling the emitter grows is normalised the day it is
/// written rather than the day somebody remembers this function exists.
///
/// That is every case both maps document: a `bytes(64)` is unbounded `bytes`
/// on PostgreSQL and SQLite, which hold no declared length for binary; a
/// `uint32` is an `int64` on the three dialects with no unsigned integer and a
/// `uint64` a `decimal(20, 0)` on two of them; a `json` is unbounded `text` on
/// SQL Server, which writes `NVARCHAR(MAX)` for both. Each pair is two columns
/// one server cannot tell apart, so no DDL could turn one into the other and a
/// diff reporting one would propose the same step on every run for ever.
///
/// It is idempotent, which is what makes folding *both* sides sound rather than
/// only the declared one: the read-back side is already a normalised type, and
/// `every_catalog_spelling_the_emitter_wrote_reads_back_as_its_own_type` is
/// exactly the assertion that emitting it again yields the string it was read
/// from. A type this dialect cannot read back is left as it is rather than
/// dropped — that same test says there is none, and if one ever appears the
/// diff compares what it was given.
fn stored_as(ty: &ScalarType, dialect: Dialect) -> ScalarType {
    catalog::scalar_type(&ddl::column_type(ty, dialect), dialect).unwrap_or_else(|| ty.clone())
}

/// Whether the column the database has is the column the schema declares,
/// after § 5's normalisation.
///
/// Equality of the values first: a [`Column`] is its type, its nullability, its
/// identity and its default, and its name is equal already or this pair would
/// not have been looked up. A column that survived its dialect's map unchanged
/// — which is most of them, on most dialects — is answered there and allocates
/// nothing.
///
/// Otherwise the other three fields must match exactly and the two types are
/// compared as [`stored_as`] leaves them, because a difference the server
/// cannot represent is not a difference. The emitted step still carries
/// `want`'s own spelling: normalising is how the comparison is made, never what
/// it produces.
///
/// SQLite's rowid is the case that survives all of it. `INTEGER PRIMARY KEY
/// AUTOINCREMENT` is the only identity that dialect has
/// ([`crate::ddl::rowid_identity`]), it is an alias for the 64-bit rowid
/// whatever width was declared, and `pragma table_info` answers the declared
/// type back as `INTEGER` — so a table written from `Int(Big)` reads as
/// `Int(Normal)`, [`stored_as`] cannot see it because that spelling is written
/// by [`crate::ddl`]'s column clause rather than by its type map, and there is
/// nothing the server can be asked that would say otherwise. Normalising the
/// width out is the only alternative to a plan that rewrites the same table
/// forever.
fn same_column(table: &Table, have: &Column, want: &Column, dialect: Dialect) -> bool {
    if have == want {
        return true;
    }
    if have.is_nullable() != want.is_nullable()
        || have.is_identity() != want.is_identity()
        || have.default_value() != want.default_value()
    {
        return false;
    }
    if stored_as(have.ty(), dialect) == stored_as(want.ty(), dialect) {
        return true;
    }
    ddl::rowid_identity(table, dialect) == Some(want.name())
        && matches!(have.ty(), ScalarType::Int(_))
        && matches!(want.ty(), ScalarType::Int(_))
}

/// Whether two keys of one kind are the same key, after § 5's normalisation.
///
/// **A unique constraint is its columns.** § 5 names "a unique constraint's
/// implicitly created index" as a normalisation this system lives or dies on,
/// and the name is where it bites: the constraint is *implemented* as an index
/// on every backend, and what that index is called is the server's answer
/// rather than ours. SQLite mints `sqlite_autoindex_wide_1` for a
/// constraint [`crate::ddl`] spelled `CONSTRAINT wide_label UNIQUE (label)`,
/// and it takes no name for an inline one at all. Two unique constraints over
/// the same columns in the same order are one constraint whatever either end
/// calls it — and § 2's vocabulary cannot rename one anyway, so treating the
/// name as data would only ever produce a drop and an add that never converge.
///
/// **An index is its name and its columns.** Every backend takes the name an
/// operator wrote for a plain index, keeps it and answers it back, and two
/// indexes over the same columns are a thing a database legitimately has.
fn same_key(kind: KeyKind, left: &Key, right: &Key) -> bool {
    match kind {
        KeyKind::Unique => left.columns() == right.columns(),
        KeyKind::Index => left.name() == right.name() && left.columns() == right.columns(),
    }
}

/// A table's keys, unique constraints first, each with the kind it is.
fn keys(table: &Table) -> impl Iterator<Item = (KeyKind, &Key)> {
    table
        .unique_keys()
        .iter()
        .map(|key| (KeyKind::Unique, key))
        .chain(table.indexes().iter().map(|key| (KeyKind::Index, key)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::{IntWidth, ScalarType};

    /// Every dialect, in the order [`Dialect`] declares them.
    const DIALECTS: [Dialect; 4] = [
        Dialect::PostgreSql,
        Dialect::MySql,
        Dialect::Sqlite,
        Dialect::SqlServer,
    ];

    fn col(name: &str, ty: ScalarType) -> Column {
        Column::new(name, ty).unwrap()
    }

    /// One table a program declares and one it is about to create, carrying a
    /// difference of every kind the vocabulary has against [`introspected`].
    fn declared() -> Schema {
        Schema::new(vec![
            Table::new(
                "kept",
                vec![
                    col("id", ScalarType::Int(IntWidth::Big))
                        .identity()
                        .unwrap(),
                    col("name", ScalarType::Text { max: Some(200) }),
                    col("live", ScalarType::Bool).null(),
                ],
            )
            .unwrap()
            .primary_key(&["id"])
            .unwrap()
            .unique("kept_name", &["name"])
            .unwrap()
            .index("kept_live", &["live"])
            .unwrap(),
            Table::new("fresh", vec![col("id", ScalarType::Int(IntWidth::Big))])
                .unwrap()
                .primary_key(&["id"])
                .unwrap(),
        ])
        .unwrap()
    }

    /// The same database as it actually is: `name` is narrower, `live` is
    /// missing, `gone` and its index are not declared, and `stale` is a table
    /// the schema value has never heard of.
    fn introspected() -> Schema {
        Schema::new(vec![
            Table::new(
                "kept",
                vec![
                    col("id", ScalarType::Int(IntWidth::Big))
                        .identity()
                        .unwrap(),
                    col("name", ScalarType::Text { max: Some(40) }),
                    col("gone", ScalarType::Bool).null(),
                ],
            )
            .unwrap()
            .primary_key(&["id"])
            .unwrap()
            .index("kept_gone", &["gone"])
            .unwrap(),
            Table::new("stale", vec![col("id", ScalarType::Int(IntWidth::Big))])
                .unwrap()
                .primary_key(&["id"])
                .unwrap(),
        ])
        .unwrap()
    }

    /// § 5, as the property that the *differences* are dialect-free.
    ///
    /// Agreement rather than a reading of one dialect's answer: each spells
    /// this plan its own way, and a diff that had reached for any of that
    /// text — a type name, a quoted identifier, a rebuild — would disagree
    /// with itself here while still printing plausibly on its own.
    /// The `to_string` list is the same claim from the other side, since a
    /// [`Change`] prints only what the values said.
    #[test]
    fn the_diff_compares_normalized_values_and_never_sql_text() {
        let (want, have) = (declared(), introspected());
        let mut shapes: Vec<Vec<Change>> = Vec::new();
        let mut documents: Vec<String> = Vec::new();
        for dialect in DIALECTS {
            let plan = diff(&want, &have, dialect);
            shapes.push(
                plan.steps()
                    .iter()
                    .map(|step| step.change().clone())
                    .collect(),
            );
            documents.push(plan.to_string());
            assert!(
                diff(&want, &want, dialect).is_empty(),
                "{dialect:?} found a difference between a value and itself"
            );
            assert!(
                diff(&have, &have, dialect).is_empty(),
                "{dialect:?} found a difference between a read value and itself"
            );
        }
        for (dialect, shape) in DIALECTS.iter().zip(&shapes) {
            assert_eq!(
                shape, &shapes[0],
                "{dialect:?} found different differences from PostgreSQL"
            );
        }
        for (at, document) in documents.iter().enumerate() {
            for (other, against) in documents.iter().enumerate().skip(at + 1) {
                assert_ne!(
                    document, against,
                    "{:?} and {:?} spell this plan identically, so the agreement above is vacuous",
                    DIALECTS[at], DIALECTS[other]
                );
            }
        }
        let printed: Vec<String> = shapes[0].iter().map(Change::to_string).collect();
        assert_eq!(
            printed,
            [
                "change column kept.name",
                "add column kept.live",
                "add unique key kept_name on kept",
                "add index kept_live on kept",
                "drop column kept.gone",
                "drop index kept_gone on kept",
                "create table fresh",
                "drop table stale",
            ]
        );
    }

    /// One table, a primary key and the column under test.
    fn one_column(ty: ScalarType) -> Schema {
        Schema::new(vec![
            Table::new(
                "wide",
                vec![col("id", ScalarType::Int(IntWidth::Big)), col("value", ty)],
            )
            .unwrap()
            .primary_key(&["id"])
            .unwrap(),
        ])
        .unwrap()
    }

    /// § 5's normalisation, at the type: a column the server cannot tell from
    /// another is not a difference, and one it can still is.
    ///
    /// Both bounds, because [`stored_as`] folds *both* sides and a fold that
    /// went too far would pass the first half alone. The pairs are the lossy
    /// cases that function's doc names, and the second half is where each of
    /// them is a real change — MySQL keeps the unsigned integer and the
    /// bounded binary the other three lose, and no dialect confuses two types
    /// it has separate spellings for.
    #[test]
    fn a_type_the_dialect_cannot_tell_from_another_is_not_a_change() {
        for (dialect, declared, read_back) in [
            (
                Dialect::PostgreSql,
                ScalarType::Bytes { max: Some(64) },
                ScalarType::Bytes { max: None },
            ),
            (
                Dialect::Sqlite,
                ScalarType::Bytes { max: Some(64) },
                ScalarType::Bytes { max: None },
            ),
            (
                Dialect::PostgreSql,
                ScalarType::Uint(IntWidth::Normal),
                ScalarType::Int(IntWidth::Big),
            ),
            (
                Dialect::SqlServer,
                ScalarType::Uint(IntWidth::Small),
                ScalarType::Int(IntWidth::Normal),
            ),
            (
                Dialect::PostgreSql,
                ScalarType::Uint(IntWidth::Big),
                ScalarType::Decimal {
                    precision: 20,
                    scale: 0,
                },
            ),
            (
                Dialect::SqlServer,
                ScalarType::Json,
                ScalarType::Text { max: None },
            ),
        ] {
            let plan = diff(
                &one_column(declared.clone()),
                &one_column(read_back),
                dialect,
            );
            assert!(
                plan.is_empty(),
                "{dialect:?} proposes a step between two columns it spells `{}`:\n{plan}",
                ddl::column_type(&declared, dialect)
            );
        }

        for (dialect, declared, read_back) in [
            (
                Dialect::MySql,
                ScalarType::Uint(IntWidth::Normal),
                ScalarType::Int(IntWidth::Big),
            ),
            (
                Dialect::MySql,
                ScalarType::Bytes { max: Some(64) },
                ScalarType::Bytes { max: None },
            ),
            (
                Dialect::PostgreSql,
                ScalarType::Text { max: Some(200) },
                ScalarType::Text { max: Some(40) },
            ),
            (Dialect::SqlServer, ScalarType::Date, ScalarType::DateTime),
        ] {
            let plan = diff(
                &one_column(declared.clone()),
                &one_column(read_back),
                dialect,
            );
            assert!(
                !plan.is_empty(),
                "{dialect:?} normalised away a difference it spells `{}`",
                ddl::column_type(&declared, dialect)
            );
        }
    }

    /// § 5's normalisation, at the name a unique constraint's index is given.
    ///
    /// Both bounds, because the rule is narrow and a blanket "names do not
    /// count" would pass the first assert alone: a unique constraint over
    /// *other* columns is still a difference, and a plain index under another
    /// name is still a difference, since nothing mints one of those.
    #[test]
    fn an_implicit_index_a_unique_constraint_created_is_not_a_difference() {
        let table = |key: &str, on: &str, index: &str| {
            Schema::new(vec![
                Table::new(
                    "wide",
                    vec![
                        col("label", ScalarType::Text { max: Some(200) }),
                        col("note", ScalarType::Text { max: Some(200) }),
                    ],
                )
                .unwrap()
                .unique(key, &[on])
                .unwrap()
                .index(index, &["note"])
                .unwrap(),
            ])
            .unwrap()
        };
        let declared = table("wide_label", "label", "wide_note");
        for dialect in DIALECTS {
            assert!(
                diff(
                    &declared,
                    &table("sqlite_autoindex_wide_1", "label", "wide_note"),
                    dialect
                )
                .is_empty(),
                "{dialect:?} saw the server's own name for the constraint's index"
            );
            assert_eq!(
                diff(
                    &declared,
                    &table("wide_label", "note", "wide_note"),
                    dialect
                )
                .len(),
                2,
                "{dialect:?} lost a unique constraint that moved to another column"
            );
            assert_eq!(
                diff(
                    &declared,
                    &table("wide_label", "label", "wide_note_idx"),
                    dialect
                )
                .len(),
                2,
                "{dialect:?} normalised the name of a plain index, which nothing mints"
            );
        }
    }

    /// § 7, for a whole table: the operator's own table is shown with the SQL
    /// that would remove it, and the plan that holds it still runs clean.
    #[test]
    fn a_table_the_schema_does_not_declare_is_reported_and_never_dropped() {
        let app = Table::new("app", vec![col("id", ScalarType::Int(IntWidth::Big))]).unwrap();
        let want = Schema::new(vec![app.clone()]).unwrap();
        let have = Schema::new(vec![
            app,
            Table::new(
                "operators_own",
                vec![col("note", ScalarType::Text { max: Some(40) })],
            )
            .unwrap(),
        ])
        .unwrap();
        for dialect in DIALECTS {
            let plan = diff(&want, &have, dialect);
            assert_eq!(plan.len(), 1, "{dialect:?}");
            let step = &plan.steps()[0];
            assert_eq!(step.change().to_string(), "drop table operators_own");
            assert!(step.is_report(), "{dialect:?}");
            assert_eq!(step.grade(), Grade::Destructive, "{dialect:?}");
            assert!(
                step.sql()
                    .iter()
                    .any(|statement| statement.starts_with("DROP TABLE")
                        && statement.contains("operators_own")),
                "{dialect:?} elided the SQL of a report: {:?}",
                step.sql()
            );
            assert_eq!(plan.runnable().count(), 0, "{dialect:?}");
            assert!(plan.first_refused().is_none(), "{dialect:?}");
        }
    }

    /// § 7 again, one column down — and the case SQLite answers with a whole
    /// rebuild, which is still a report and still never applied.
    #[test]
    fn a_column_the_schema_does_not_declare_is_reported_and_never_dropped() {
        let declared = vec![col("id", ScalarType::Int(IntWidth::Big))];
        let mut carried = declared.clone();
        carried.push(col("added_by_hand", ScalarType::Text { max: Some(40) }).null());
        let want = Schema::new(vec![Table::new("app", declared).unwrap()]).unwrap();
        let have = Schema::new(vec![Table::new("app", carried).unwrap()]).unwrap();
        for dialect in DIALECTS {
            let plan = diff(&want, &have, dialect);
            assert_eq!(plan.len(), 1, "{dialect:?}");
            let step = &plan.steps()[0];
            assert_eq!(step.change().to_string(), "drop column app.added_by_hand");
            assert!(step.is_report(), "{dialect:?}");
            assert_eq!(step.grade(), Grade::Destructive, "{dialect:?}");
            assert!(!step.sql().is_empty(), "{dialect:?}");
            assert_eq!(plan.runnable().count(), 0, "{dialect:?}");
            assert!(plan.first_refused().is_none(), "{dialect:?}");
        }
    }

    /// The queue's jobs table, with `tag` declared or not and indexed or not.
    ///
    /// The columns are the ones the difference is about and not `nvs_stdlib::queue::schema`'s
    /// whole list — this crate cannot see that value, and a transcription of it here would be a
    /// second home for what the queue's columns are, which is the thing that value exists to
    /// prevent. What is being asked is the *shape* of the change: a nullable text column with no
    /// default, arriving on a table that already has rows and keys.
    fn jobs(tagged: bool, indexed: bool) -> Schema {
        let key = ScalarType::Text { max: Some(255) };
        let mut columns = vec![
            col("id", ScalarType::Int(IntWidth::Big))
                .identity()
                .unwrap(),
            col("queue", key.clone()),
            col("state", ScalarType::Int(IntWidth::Small)),
            col("run_at", ScalarType::Int(IntWidth::Big)),
            col("dedupe_pending", key.clone()).null(),
        ];
        if tagged {
            columns.push(col("tag", key).null());
        }
        let mut table = Table::new("nvs_jobs", columns)
            .unwrap()
            .primary_key(&["id"])
            .unwrap()
            .unique("nvs_jobs_dedupe", &["dedupe_pending"])
            .unwrap()
            .index("nvs_jobs_due", &["queue", "state", "run_at"])
            .unwrap();
        if indexed {
            table = table.index("nvs_jobs_tag", &["queue", "tag"]).unwrap();
        }
        Schema::new(vec![table]).unwrap()
    }

    /// `rule:concurrency/a-tag-groups-jobs-and-a-key-dedupes-them`, read as a converge: a database
    /// holding the queue's schema from before `tag` differs from it by **one** column.
    ///
    /// The whole plan is named rather than counted, because the count alone passes just as well
    /// when the diff found a column change it should not have — and because the second step is the
    /// index, which is a fact about this converge an operator has to know: the column arrives
    /// unasked and the index does not.
    #[test]
    fn converging_the_pre_tag_queue_schema_plans_exactly_one_add_column_step() {
        let (want, have) = (jobs(true, true), jobs(false, false));
        for dialect in DIALECTS {
            let plan = diff(&want, &have, dialect);
            let changes: Vec<String> = plan
                .steps()
                .iter()
                .map(|step| step.change().to_string())
                .collect();
            assert_eq!(
                changes,
                [
                    "add column nvs_jobs.tag",
                    "add index nvs_jobs_tag on nvs_jobs"
                ],
                "{dialect:?} converges the pre-`tag` queue by something other than one column and \
                 its index"
            );
        }
    }

    /// § 6's `Safe` at the case the queue's `tag` is declared for: `nvs queue migrate` runs the
    /// column without being asked twice, on all four.
    ///
    /// The column alone is the plan under test, because [`Plan::first_refused`] answers about the
    /// whole plan and the index that follows the column is `Locking` — built over every row that is
    /// already there, with no concurrent build in v1. So the two halves are asserted apart: the
    /// column needs no `--including-risky` and the index is the only step that does. A `tag`
    /// declared `NOT NULL`, or with a default, fails the first half here rather than on the first
    /// live queue that runs the migration.
    #[test]
    fn that_step_is_graded_safe_on_every_dialect_and_needs_no_including_risky() {
        let have = jobs(false, false);
        let (column, indexed) = (jobs(true, false), jobs(true, true));
        for dialect in DIALECTS {
            let plan = diff(&column, &have, dialect);
            assert_eq!(plan.len(), 1, "{dialect:?}");
            let step = &plan.steps()[0];
            assert_eq!(step.grade(), Grade::Safe, "{dialect:?}: {}", step.reason());
            assert!(
                plan.first_refused().is_none(),
                "{dialect:?} would not add the column without --including-risky"
            );
            // SQLite answers an add it cannot express as an `ALTER` with a whole rebuild, which is
            // `Destructive` — so the absence of one is the same claim as the grade, read off the
            // SQL an operator would paste.
            assert!(
                !step.sql().concat().contains("_nvs_rebuild"),
                "{dialect:?} rebuilt the jobs table for a column every row already has"
            );
            assert_eq!(
                diff(&indexed, &have, dialect)
                    .first_refused()
                    .map(|refused| refused.change().to_string()),
                Some("add index nvs_jobs_tag on nvs_jobs".to_owned()),
                "{dialect:?} refuses a step of this converge that is not the index build"
            );
        }
    }
}
