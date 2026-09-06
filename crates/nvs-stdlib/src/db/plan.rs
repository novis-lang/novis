//! `Core\Db\Plan` — `rule:core-classes/schema-plan` and `rule:core-classes/schema-absence-never-destroys`'s plan, as a value a program walks.
//!
//! [`nvs_db::plan`] computes the difference, grades every step and emits the
//! SQL; that crate is sans-io and holds no Novis value, so this module is the
//! one place a [`nvs_db::Plan`] becomes objects — the same seam
//! [`mod@super::schema`] is for a schema, and it decides nothing about DDL
//! either.
//!
//! # Decision: a step is copied out, and the plan is not a live query
//!
//! A [`nvs_db::Step`] is built out of a [`nvs_db::Change`] the diff already
//! finished with, so there is nothing left for it to be lazy about: the grade
//! is decided, the reason is written and the SQL is emitted before the plan
//! exists at all. The four slots below are therefore the whole of a step, and
//! walking a plan twice reads the same four values rather than asking a server
//! anything — which is what makes `examples/schema.nvs` able to plan once and
//! then run the reported drop out of the plan it already holds.
//!
//! The one field that is flattened is the SQL. [`nvs_db::Step::sql`] is a
//! *list* of statements, because SQLite's rebuild is four of them; `sql()`
//! answers them joined by a newline, which is § 8's pasteable document and the
//! only spelling `Core\Db::execute` — one statement per call, `rule:core-classes/db-statement-members` —
//! can be handed for the steps that are one statement long. A rebuild's four
//! are for an operator, and the member's reference card says so.
//!
//! **What it spends:** one object per step plus one string per step's reason
//! and SQL, charged to the request that planned and released with the plan.
//! A plan against a converged database is one empty array.

use super::*;

/// [`nvs_db::Plan`] as the object a program holds — `rule:core-classes/schema-plan`'s document.
///
/// Nothing here can fail: every field is a `String`, a `bool` or one of three
/// grades, and the array is built the way [`crate::arr`] builds any packed one.
pub(super) fn plan_value(plan: &nvs_db::Plan) -> Value {
    let mut steps = NvsArray::new();
    for step in plan.steps() {
        steps.append(step_value(step));
    }
    crate::instance::build(&PLAN, [Value::array(steps)])
}

/// One [`nvs_db::Step`] as the object [`plan_value`] fills its array with.
///
/// The grade is written out as its [`GRADE`] ordinal rather than cast from the
/// Rust enum's discriminant, so the two rosters are held together by
/// `the_grade_slot_agrees_with_the_registered_enum` rather than by a layout
/// nothing checks.
fn step_value(step: &nvs_db::Step) -> Value {
    crate::instance::build(
        &STEP,
        [
            Value::int(grade_case(step.grade())),
            Value::str(NvsStr::new(step.reason().as_bytes())),
            Value::str(NvsStr::new(step.sql().join("\n").as_bytes())),
            Value::bool(step.is_report()),
        ],
    )
}

/// The [`GRADE`] case value for one [`nvs_db::Grade`].
///
/// `pub(super)` because the test that holds the two rosters together is the
/// only other reader, and it has to ask this rather than repeat it.
pub(super) fn grade_case(grade: nvs_db::Grade) -> i64 {
    match grade {
        nvs_db::Grade::Safe => 0,
        nvs_db::Grade::Locking => 1,
        nvs_db::Grade::Destructive => 2,
    }
}

nvs_runtime::nvs_helper! {
    /// `$plan->steps(): array<Db\Plan\Step>` — every step, § 7's reports
    /// included.
    ///
    /// The slot already holds them, built when the plan was computed, so this
    /// is one retained reference rather than a second array — [`mod@super::schema`]'s
    /// `toArray` answers the identical way and for the identical reason.
    fn nvs_core_db_plan_steps(_ctx, args: [1]) {
        let receiver = crate::instance::receiver(args[0], &PLAN, "steps")?;
        Ok(owned(crate::instance::slot(receiver, PLAN_STEPS_AT)))
    }
}

nvs_runtime::nvs_helper! {
    /// `$step->grade(): Db\Plan\Grade` — `rule:core-classes/schema-plan`'s worst cost of this
    /// step.
    fn nvs_core_db_plan_step_grade(_ctx, args: [1]) {
        let receiver = crate::instance::receiver(args[0], &STEP, "grade")?;
        Ok(owned(crate::instance::slot(receiver, STEP_GRADE_AT)))
    }
}

nvs_runtime::nvs_helper! {
    /// `$step->reason(): string` — why it is graded that way, in the one
    /// sentence § 6 asks an emitter for.
    fn nvs_core_db_plan_step_reason(_ctx, args: [1]) {
        let receiver = crate::instance::receiver(args[0], &STEP, "reason")?;
        Ok(owned(crate::instance::slot(receiver, STEP_REASON_AT)))
    }
}

nvs_runtime::nvs_helper! {
    /// `$step->sql(): string` — the complete, terminated statements this step
    /// is, newline-joined.
    ///
    /// Never empty and never elided, § 8's rule holding for a refused step
    /// exactly as it does for one the applier runs: a deployment whose
    /// application credentials cannot issue DDL reads the plan and hands it to
    /// a DBA, and a step summarised rather than spelled would be useless there.
    fn nvs_core_db_plan_step_sql(_ctx, args: [1]) {
        let receiver = crate::instance::receiver(args[0], &STEP, "sql")?;
        Ok(owned(crate::instance::slot(receiver, STEP_SQL_AT)))
    }
}

nvs_runtime::nvs_helper! {
    /// `$step->isRefused(): bool` — whether this is § 7's report, carried and
    /// never applied.
    ///
    /// Named for what a caller does about it rather than for what it is: a
    /// program that wants the drop writes the `Core\Db::execute` itself, out of
    /// the SQL this step already carries, which is the only place absence
    /// destroys anything.
    fn nvs_core_db_plan_step_is_refused(_ctx, args: [1]) {
        let receiver = crate::instance::receiver(args[0], &STEP, "isRefused")?;
        Ok(owned(crate::instance::slot(receiver, STEP_REPORT_AT)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// [`grade_case`] and [`GRADE`] are one roster, and this is what says so.
    ///
    /// The slot holds an integer because `rule:enums/closed-integer-type` makes an enum one; a program
    /// matching `Grade::Destructive` is comparing against [`GRADE`]'s own case
    /// value, so a Rust arm that drifted from the table would answer a grade
    /// no branch of a correct program takes.
    #[test]
    fn the_grade_slot_agrees_with_the_registered_enum() {
        for (grade, name) in [
            (nvs_db::Grade::Safe, "Safe"),
            (nvs_db::Grade::Locking, "Locking"),
            (nvs_db::Grade::Destructive, "Destructive"),
        ] {
            let (_, value) = GRADE
                .cases
                .iter()
                .find(|(case, _)| *case == name)
                .unwrap_or_else(|| panic!("`{}` has no `{name}` case", GRADE.name));
            assert_eq!(
                grade_case(grade),
                *value,
                "the slot written for {grade:?} is not `{}::{name}`",
                GRADE.name
            );
        }
    }
}
