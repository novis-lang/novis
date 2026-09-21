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
    // covers: Core\Db\Plan\Step::grade
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

    /// `rule:core-classes/schema-plan`'s document, read where a program reads it: `steps()` answers
    /// the array [`plan_value`] filled, in the differ's own order, and retains
    /// it rather than building a second one.
    ///
    /// **Asserted on the array's identity and its refcount**, not on what it
    /// holds: a member that copied the steps would answer an array equal to
    /// this one on every line and still charge a request one object per step
    /// per read, which is the cost this module's doc says a walk does not pay.
    /// The plan is a real [`nvs_db::diff`] rather than an instance built here,
    /// so the order asserted is the order the differ wrote and the empty one
    /// below is real convergence.
    // covers: Core\Db\Plan::steps
    #[test]
    fn steps_answers_the_plans_own_array_in_the_order_the_differ_wrote() {
        use nvs_db::schema::{Column, Schema, Table};
        use nvs_db::{Dialect, IntWidth, ScalarType};

        let table = |name: &str| {
            Table::new(
                name,
                vec![
                    Column::new("id", ScalarType::Int(IntWidth::Big))
                        .expect("`id` is an identifier"),
                ],
            )
            .expect("one column is a table")
        };
        let want =
            Schema::new(vec![table("notes"), table("tags")]).expect("two tables are a schema");
        let have = Schema::new(Vec::new()).expect("an empty database is a schema");

        let plan = nvs_db::diff(&want, &have, Dialect::Sqlite);
        assert_eq!(
            plan.len(),
            2,
            "the fixture stopped producing one step per missing table:\n{plan}"
        );

        let value = plan_value(&plan);
        let receiver = value.obj_ptr().expect("`build` answers an object");
        let held = crate::instance::slot(receiver, PLAN_STEPS_AT)
            .array_ptr()
            .expect("the plan's one slot holds its steps");

        #[expect(
            unsafe_code,
            reason = "the plan this frame built owns a reference to the array, so \
                      it is live for the whole test"
        )]
        let before = unsafe { nvs_runtime::NvsArray::refcount_of(held) };

        let mut ctx = nvs_runtime::Ctx::buffered();
        let answered = nvs_runtime::call(nvs_core_db_plan_steps, &mut ctx, &[value])
            .expect("reading a filled slot cannot fail");
        assert_eq!(
            answered.array_ptr(),
            Some(held),
            "a walk was handed a copy of the steps rather than the plan's own array"
        );

        #[expect(
            unsafe_code,
            reason = "the plan is still this frame's, and the answer holds a second \
                      reference to the same array"
        )]
        let held_now = unsafe { nvs_runtime::NvsArray::refcount_of(held) };
        assert_eq!(
            held_now,
            before + 1,
            "the answer outlives the call, so the member owes it a reference"
        );

        let steps = crate::arr::borrowed(held);
        assert_eq!(
            steps.count(),
            plan.len(),
            "the array is not as long as the plan it was built from"
        );
        for (at, step) in plan.steps().iter().enumerate() {
            let at = i64::try_from(at).expect("the fixture's plan is two steps long");
            let object = steps
                .get_index(at)
                .expect("every slot of a packed array is filled")
                .obj_ptr()
                .expect("a step is an object");
            let reason = crate::instance::slot(object, STEP_REASON_AT);
            assert_eq!(
                reason.as_text(),
                Some(step.reason()),
                "step {at} is not the one the differ wrote there"
            );
        }

        // § 6's other half: a database that already matches the schema plans
        // nothing, and a program reads that as an empty array rather than as an
        // absence it has to test for.
        let converged = plan_value(&nvs_db::diff(&want, &want, Dialect::Sqlite));
        let empty = crate::instance::slot(
            converged.obj_ptr().expect("`build` answers an object"),
            PLAN_STEPS_AT,
        )
        .array_ptr()
        .expect("the plan's one slot holds its steps");
        assert_eq!(
            crate::arr::borrowed(empty).count(),
            0,
            "a database that matches the schema planned a step anyway"
        );

        #[expect(
            unsafe_code,
            reason = "this frame owns the two plans it built and the one answer the \
                      member handed back, and releases each exactly once"
        )]
        unsafe {
            answered.release();
            value.release();
            converged.release();
        }
    }

    /// The report flag is [`nvs_db::Step::is_report`] for every step, so the
    /// two names are one decision made in the crate that computes the plan.
    ///
    /// A member reading the slot cannot be wrong on its own; what this holds
    /// together is the *mapping*, since a fourth slot filled from the wrong
    /// field would answer plausibly for a plan whose steps happen to agree.
    /// The fixture is a created table beside a dropped one, which disagree.
    // covers: Core\Db\Plan\Step::isRefused
    #[test]
    fn the_report_flag_is_what_the_differ_said_about_that_step() {
        use nvs_db::schema::{Column, Schema, Table};
        use nvs_db::{Dialect, IntWidth, ScalarType};

        let table = |name: &str| {
            Table::new(
                name,
                vec![
                    Column::new("id", ScalarType::Int(IntWidth::Big))
                        .expect("`id` is an identifier"),
                ],
            )
            .expect("one column is a table")
        };
        let want = Schema::new(vec![table("notes")]).expect("one table is a schema");
        let have = Schema::new(vec![table("leftovers")]).expect("one table is a schema");

        let plan = nvs_db::diff(&want, &have, Dialect::Sqlite);
        let reports = plan.steps().iter().filter(|step| step.is_report()).count();
        assert_eq!(
            (plan.len(), reports),
            (2, 1),
            "the fixture stopped being one change beside one report:\n{plan}"
        );

        let value = plan_value(&plan);
        let steps = crate::arr::borrowed(
            crate::instance::slot(
                value.obj_ptr().expect("`build` answers an object"),
                PLAN_STEPS_AT,
            )
            .array_ptr()
            .expect("the plan's one slot holds its steps"),
        );

        let mut ctx = nvs_runtime::Ctx::buffered();
        for (at, step) in plan.steps().iter().enumerate() {
            let index = i64::try_from(at).expect("the fixture's plan is two steps long");
            let object = steps
                .get_index(index)
                .expect("every slot of a packed array is filled");
            let read = nvs_runtime::call(nvs_core_db_plan_step_is_refused, &mut ctx, &[object])
                .expect("reading a filled slot cannot fail");
            assert_eq!(
                read.as_bool(),
                Some(step.is_report()),
                "the flag on step {at} is not what the differ said about it, \
                 which is {}",
                step.is_report()
            );
        }

        #[expect(
            unsafe_code,
            reason = "this frame owns the plan it built and releases it exactly once; \
                      a `bool` answer holds no reference"
        )]
        unsafe {
            value.release();
        }
    }

    /// Every step's reason slot is the sentence the emitter wrote for **that**
    /// change, and a read hands that string over rather than writing it again.
    ///
    /// The fixture is a created table beside a dropped one, because those are
    /// the two sentences a member reading one slot for the whole plan would
    /// answer identically — a fixture of one step cannot tell the two apart.
    /// The identity of the text is asserted as well as its bytes: a program
    /// printing a plan reads this member once per step, and
    /// `benches/members/core/Db-Plan-Step/reason.nvs` declares no allocation
    /// for a read.
    // covers: Core\Db\Plan\Step::reason
    #[test]
    fn a_steps_reason_is_the_sentence_the_emitter_wrote_for_that_change() {
        use nvs_db::schema::{Column, Schema, Table};
        use nvs_db::{Dialect, IntWidth, ScalarType};

        let table = |name: &str| {
            Table::new(
                name,
                vec![
                    Column::new("id", ScalarType::Int(IntWidth::Big))
                        .expect("`id` is an identifier"),
                ],
            )
            .expect("one column is a table")
        };
        let want = Schema::new(vec![table("notes")]).expect("one table is a schema");
        let have = Schema::new(vec![table("leftovers")]).expect("one table is a schema");

        let plan = nvs_db::diff(&want, &have, Dialect::Sqlite);
        assert_eq!(
            plan.len(),
            2,
            "the fixture stopped being a created table beside a reported drop:\n{plan}"
        );
        assert_ne!(
            plan.steps()[0].reason(),
            plan.steps()[1].reason(),
            "the fixture's two steps share a sentence, so a member answering one \
             of them for every step would pass this"
        );

        let value = plan_value(&plan);
        let steps = crate::arr::borrowed(
            crate::instance::slot(
                value.obj_ptr().expect("`build` answers an object"),
                PLAN_STEPS_AT,
            )
            .array_ptr()
            .expect("the plan's one slot holds its steps"),
        );

        let mut ctx = nvs_runtime::Ctx::buffered();
        let mut answers = Vec::new();
        for (at, step) in plan.steps().iter().enumerate() {
            let index = i64::try_from(at).expect("the fixture's plan is two steps long");
            let object = steps
                .get_index(index)
                .expect("every slot of a packed array is filled");
            let slot = crate::instance::slot(
                object.obj_ptr().expect("a step is an object"),
                STEP_REASON_AT,
            );
            assert!(
                !step.reason().is_empty(),
                "the emitter wrote no reason for step {at}"
            );
            assert_eq!(
                slot.as_text(),
                Some(step.reason()),
                "step {at} carries a sentence the emitter wrote for another change"
            );

            let read = nvs_runtime::call(nvs_core_db_plan_step_reason, &mut ctx, &[object])
                .expect("reading a filled slot cannot fail");
            assert_eq!(
                read.as_text().map(str::as_ptr),
                slot.as_text().map(str::as_ptr),
                "step {at}'s reason was written again for the reader rather than \
                 handed over"
            );
            answers.push(read);
        }

        #[expect(
            unsafe_code,
            reason = "this frame owns the plan it built and one reference per answer \
                      the member handed back, and releases each exactly once"
        )]
        unsafe {
            for read in answers {
                read.release();
            }
            value.release();
        }
    }

    /// The one field this module flattens: a step's SQL slot is **every**
    /// statement the emitter wrote for it, joined by a newline.
    ///
    /// The fixture is a widened integer column, because on SQLite that is a
    /// create-copy-drop-rename rebuild and therefore the one ordinary step
    /// whose SQL is more than one statement — the case the join exists for,
    /// and the one a fixture of a single `CREATE TABLE` cannot tell apart from
    /// a member that answered the first statement alone.
    // covers: Core\Db\Plan\Step::sql
    #[test]
    fn a_steps_sql_is_every_statement_the_emitter_wrote_joined_by_a_newline() {
        use nvs_db::schema::{Column, Schema, Table};
        use nvs_db::{Dialect, IntWidth, ScalarType};

        let of = |width| {
            Schema::new(vec![
                Table::new(
                    "notes",
                    vec![Column::new("id", ScalarType::Int(width)).expect("`id` is an identifier")],
                )
                .expect("one column is a table"),
            ])
            .expect("one table is a schema")
        };

        let plan = nvs_db::diff(&of(IntWidth::Big), &of(IntWidth::Normal), Dialect::Sqlite);
        let step = plan.steps().first().expect("a widened column is one step");
        assert!(
            step.sql().len() > 1,
            "the fixture stopped being SQLite's rebuild, so the join is not \
             exercised: {:?}",
            step.sql()
        );

        let value = plan_value(&plan);
        let steps = crate::arr::borrowed(
            crate::instance::slot(
                value.obj_ptr().expect("`build` answers an object"),
                PLAN_STEPS_AT,
            )
            .array_ptr()
            .expect("the plan's one slot holds its steps"),
        );
        let object = steps
            .get_index(0)
            .expect("the plan has a first step")
            .obj_ptr()
            .expect("a step is an object");
        let sql = crate::instance::slot(object, STEP_SQL_AT);
        assert_eq!(
            sql.as_text(),
            Some(step.sql().join("\n").as_str()),
            "the slot is not the whole of what the emitter wrote for this step"
        );

        #[expect(
            unsafe_code,
            reason = "this frame owns the plan it built and releases it exactly once"
        )]
        unsafe {
            value.release();
        }
    }
}
