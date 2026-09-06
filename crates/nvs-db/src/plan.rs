//! [ADR 0145](/docs/adr/0145-a-schema-is-a-value-core-db-schema-converges-a-closed.md)
//! §§ 6-8: a plan is a document, and a step in it is a change plus the reason
//! it costs what it costs.
//!
//! This module is the plan's *vocabulary* and holds no SQL. What a change is
//! spelled as, and what it is graded, are one dialect's questions and both are
//! [`crate::ddl::step`]'s — § 6 puts the grade in the emitter because the same
//! change is not the same risk on two backends, and a grade computed anywhere
//! else would have to be corrected there anyway.
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

use crate::schema::{Column, Ident, Key, Table};

/// § 6's three grades: what a step can cost, at worst.
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
/// The two are one variant of [`Change`] rather than two because they differ
/// only in a keyword on three of the four dialects — and on the fourth,
/// SQLite, a unique constraint added after the fact *is* a unique index, which
/// is a distinction the emitter makes and the vocabulary does not.
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
    /// [ADR 0084](/docs/adr/0084-durable-background-jobs.md) — the queue's two.
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
/// Built by [`crate::ddl::step`] and by nothing else: the three fields past
/// the change are all one dialect's answers, and a `Step` assembled anywhere
/// else would be a second emitter.
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
    /// then its SQL, in the one comment syntax all four dialects read.
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
