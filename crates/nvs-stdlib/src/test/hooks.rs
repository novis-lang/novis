//! `Core\Test\Outcome` and `Core\Test\Summary`: what the runner passes to the
//! test hooks of `rule:testing/after-each-test` and `rule:testing/class-hooks`.
//!
//! Both are `Core`-owned instances that only the runner builds. A program
//! receives one as a hook's parameter and reads it through its members, since
//! a `Core` instance has no property a program can reach
//! ([`CoreTy::Instance`]). [`outcome`] and [`summary`] are the two builders,
//! re-exported for `nvs_cli`'s runner, which is their one caller.
//!
//! **What it spends:** one `Outcome` per attempt and one `Summary` per class,
//! each a few strings and one list, released when the hook returns.

use nvs_runtime::{NvsArray, Value};

use crate::registry::{ClassDoc, CoreClass, CoreMethod, CoreTy, MethodDoc};

/// `Core\Test\Outcome`'s fully-qualified name.
pub(crate) const OUTCOME_NAME: &str = r"Core\Test\Outcome";

/// `Core\Test\Summary`'s fully-qualified name.
pub(crate) const SUMMARY_NAME: &str = r"Core\Test\Summary";

/// One reader: a member with no parameter that returns one slot.
const fn reader(
    name: &'static str,
    return_ty: CoreTy,
    symbol: &'static str,
    doc: &'static MethodDoc,
) -> CoreMethod {
    CoreMethod {
        name,
        names: &[],
        params: &[],
        defaults: &[],
        return_ty,
        symbol,
        doc: Some(doc),
    }
}

/// How one attempt at one test came out — `rule:testing/after-each-test`'s
/// table, one reader per row.
///
/// `rule:testing/after-each-test` and `rule:testing/class-hooks`: what the
/// runner passes to a test hook, beside `Core\Test` for the reason
/// `Core\Test\Response` is.
// rank: 490
pub(crate) const OUTCOME: CoreClass = CoreClass {
    name: OUTCOME_NAME,
    doc: Some(&OUTCOME_CARD),
    methods: &[],
    instance: &[
        reader(
            "class",
            CoreTy::Str,
            "nvs_core_test_outcome_class",
            &OUTCOME_CLASS_DOC,
        ),
        reader(
            "name",
            CoreTy::Str,
            "nvs_core_test_outcome_name",
            &OUTCOME_NAME_DOC,
        ),
        reader(
            "attempt",
            CoreTy::Int,
            "nvs_core_test_outcome_attempt",
            &OUTCOME_ATTEMPT_DOC,
        ),
        reader(
            "passed",
            CoreTy::Bool,
            "nvs_core_test_outcome_passed",
            &OUTCOME_PASSED_DOC,
        ),
        reader(
            "failures",
            CoreTy::Array(&CoreTy::Str),
            "nvs_core_test_outcome_failures",
            &OUTCOME_FAILURES_DOC,
        ),
    ],
    slots: &["class", "name", "attempt", "passed", "failures"],
    constants: &[],
};

/// How one test class came out — `rule:testing/class-hooks`'s fields, one
/// reader each.
// rank: 500
pub(crate) const SUMMARY: CoreClass = CoreClass {
    name: SUMMARY_NAME,
    doc: Some(&SUMMARY_CARD),
    methods: &[],
    instance: &[
        reader(
            "class",
            CoreTy::Str,
            "nvs_core_test_summary_class",
            &SUMMARY_CLASS_DOC,
        ),
        reader(
            "passed",
            CoreTy::Int,
            "nvs_core_test_summary_passed",
            &SUMMARY_PASSED_DOC,
        ),
        reader(
            "failed",
            CoreTy::Int,
            "nvs_core_test_summary_failed",
            &SUMMARY_FAILED_DOC,
        ),
        reader(
            "flaky",
            CoreTy::Int,
            "nvs_core_test_summary_flaky",
            &SUMMARY_FLAKY_DOC,
        ),
        reader(
            "skipped",
            CoreTy::Int,
            "nvs_core_test_summary_skipped",
            &SUMMARY_SKIPPED_DOC,
        ),
        reader(
            "failedTests",
            CoreTy::Array(&CoreTy::Str),
            "nvs_core_test_summary_failed_tests",
            &SUMMARY_FAILED_TESTS_DOC,
        ),
    ],
    slots: &[
        "class",
        "passed",
        "failed",
        "flaky",
        "skipped",
        "failedTests",
    ],
    constants: &[],
};

const OUTCOME_CARD: ClassDoc = ClassDoc {
    short: "How one run of one test ended. A method with `#[AfterEachTest]` can take one as its \
            parameter. The test runner creates it. You cannot create one yourself.",
};

const OUTCOME_CLASS_DOC: MethodDoc = MethodDoc {
    short: "Returns the name of the test class.",
    params: &[],
    ret: "The class name, with its namespace.",
    errors: &[],
};

const OUTCOME_NAME_DOC: MethodDoc = MethodDoc {
    short: "Returns the name of the test method that ran.",
    params: &[],
    ret: "The method name. A test with `#[TestWith]` rows has `#0`, `#1` and so on after the \
          name, as the report prints it.",
    errors: &[],
};

const OUTCOME_ATTEMPT_DOC: MethodDoc = MethodDoc {
    short: "Returns which run of the test this was.",
    params: &[],
    ret: "`1` for the first run. Each retry from `retries:` adds one.",
    errors: &[],
};

const OUTCOME_PASSED_DOC: MethodDoc = MethodDoc {
    short: "Checks whether this run of the test passed.",
    params: &[],
    ret: "`true` when the run passed, and `false` when it failed.",
    errors: &[],
};

const OUTCOME_FAILURES_DOC: MethodDoc = MethodDoc {
    short: "Returns the failure messages of this run.",
    params: &[],
    ret: "One message per failure, as the report prints it. The list is empty when the run \
          passed.",
    errors: &[],
};

const SUMMARY_CARD: ClassDoc = ClassDoc {
    short: "How all tests of one test class ended. A method with `#[AfterClass]` can take one as \
            its parameter. The test runner creates it. You cannot create one yourself.",
};

const SUMMARY_CLASS_DOC: MethodDoc = MethodDoc {
    short: "Returns the name of the test class.",
    params: &[],
    ret: "The class name, with its namespace.",
    errors: &[],
};

const SUMMARY_PASSED_DOC: MethodDoc = MethodDoc {
    short: "Returns how many tests of the class passed.",
    params: &[],
    ret: "The number of passed tests. A flaky test is not counted here.",
    errors: &[],
};

const SUMMARY_FAILED_DOC: MethodDoc = MethodDoc {
    short: "Returns how many tests of the class failed.",
    params: &[],
    ret: "The number of failed tests.",
    errors: &[],
};

const SUMMARY_FLAKY_DOC: MethodDoc = MethodDoc {
    short: "Returns how many tests of the class failed first and then passed on a retry.",
    params: &[],
    ret: "The number of flaky tests.",
    errors: &[],
};

const SUMMARY_SKIPPED_DOC: MethodDoc = MethodDoc {
    short: "Returns how many tests of the class were skipped.",
    params: &[],
    ret: "The number of skipped tests.",
    errors: &[],
};

const SUMMARY_FAILED_TESTS_DOC: MethodDoc = MethodDoc {
    short: "Returns the names of the tests that failed.",
    params: &[],
    ret: "One name per failed test, as the report prints it. The list is empty when no test \
          failed.",
    errors: &[],
};

/// Where each member's helper lives.
pub(crate) fn address(symbol: &str) -> Option<*const u8> {
    Some(match symbol {
        "nvs_core_test_outcome_class" => (nvs_core_test_outcome_class as *const ()).cast(),
        "nvs_core_test_outcome_name" => (nvs_core_test_outcome_name as *const ()).cast(),
        "nvs_core_test_outcome_attempt" => (nvs_core_test_outcome_attempt as *const ()).cast(),
        "nvs_core_test_outcome_passed" => (nvs_core_test_outcome_passed as *const ()).cast(),
        "nvs_core_test_outcome_failures" => (nvs_core_test_outcome_failures as *const ()).cast(),
        "nvs_core_test_summary_class" => (nvs_core_test_summary_class as *const ()).cast(),
        "nvs_core_test_summary_passed" => (nvs_core_test_summary_passed as *const ()).cast(),
        "nvs_core_test_summary_failed" => (nvs_core_test_summary_failed as *const ()).cast(),
        "nvs_core_test_summary_flaky" => (nvs_core_test_summary_flaky as *const ()).cast(),
        "nvs_core_test_summary_skipped" => (nvs_core_test_summary_skipped as *const ()).cast(),
        "nvs_core_test_summary_failed_tests" => {
            (nvs_core_test_summary_failed_tests as *const ()).cast()
        }
        _ => return None,
    })
}

fn text(value: &str) -> Value {
    Value::text(value)
}

fn texts(values: &[String]) -> Value {
    let mut list = NvsArray::new();
    for value in values {
        list.append(text(value));
    }
    Value::array(list)
}

fn count(value: usize) -> Value {
    Value::int(i64::try_from(value).unwrap_or(i64::MAX))
}

/// A new `Core\Test\Outcome` for one attempt, owned by the caller.
#[must_use]
pub fn outcome(class: &str, name: &str, attempt: usize, failures: &[String]) -> Value {
    crate::instance::build(
        &OUTCOME,
        [
            text(class),
            text(name),
            count(attempt),
            Value::bool(failures.is_empty()),
            texts(failures),
        ],
    )
}

/// The counts of one test class, in the order `Core\Test\Summary` reads them.
#[derive(Clone, Copy, Debug, Default)]
pub struct Counts {
    /// Tests that passed on their first attempt.
    pub passed: usize,
    /// Tests that failed, or ended the run with `exit()`.
    pub failed: usize,
    /// Tests that failed and then passed on a retry.
    pub flaky: usize,
    /// Tests that were skipped.
    pub skipped: usize,
}

/// A new `Core\Test\Summary` for one class, owned by the caller.
#[must_use]
pub fn summary(class: &str, counts: Counts, failed_tests: &[String]) -> Value {
    crate::instance::build(
        &SUMMARY,
        [
            text(class),
            count(counts.passed),
            count(counts.failed),
            count(counts.flaky),
            count(counts.skipped),
            texts(failed_tests),
        ],
    )
}

macro_rules! readers {
    ($class:ident, $($helper:ident => $slot:literal, $member:literal;)*) => {
        $(
            nvs_runtime::nvs_helper! {
                /// One slot of the receiver, retained for the caller.
                fn $helper(_ctx, args: [1]) {
                    crate::instance::read_slot(args, &$class, $slot, $member)
                }
            }
        )*
    };
}

readers! {
    OUTCOME,
    nvs_core_test_outcome_class => 0, "class";
    nvs_core_test_outcome_name => 1, "name";
    nvs_core_test_outcome_attempt => 2, "attempt";
    nvs_core_test_outcome_passed => 3, "passed";
    nvs_core_test_outcome_failures => 4, "failures";
}

readers! {
    SUMMARY,
    nvs_core_test_summary_class => 0, "class";
    nvs_core_test_summary_passed => 1, "passed";
    nvs_core_test_summary_failed => 2, "failed";
    nvs_core_test_summary_flaky => 3, "flaky";
    nvs_core_test_summary_skipped => 4, "skipped";
    nvs_core_test_summary_failed_tests => 5, "failedTests";
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_reader_names_a_symbol_this_module_claims() {
        for method in OUTCOME.instance.iter().chain(SUMMARY.instance) {
            assert!(
                address(method.symbol).is_some(),
                "`{}` is not claimed",
                method.symbol
            );
        }
    }

    #[test]
    fn every_reader_reads_the_slot_of_its_own_name() {
        for class in [&OUTCOME, &SUMMARY] {
            for (index, method) in class.instance.iter().enumerate() {
                assert_eq!(class.slots[index], method.name, "{}", class.name);
            }
        }
    }

    fn dropped(value: Value) {
        #[expect(unsafe_code, reason = "the reader transferred what it returned")]
        // SAFETY: a reader's answer carries a reference of its own, and this
        // test is the caller that would otherwise hold it.
        unsafe {
            value.release();
        }
    }

    /// What `member` returns for `receiver`, owned by the caller.
    fn read(ctx: &mut nvs_runtime::Ctx, member: nvs_runtime::NvsFn, receiver: Value) -> Value {
        nvs_runtime::call(member, ctx, &[receiver]).expect("a reader of its own class cannot fail")
    }

    fn text_of(ctx: &mut nvs_runtime::Ctx, member: nvs_runtime::NvsFn, receiver: Value) -> String {
        let value = read(ctx, member, receiver);
        let text = value
            .as_text()
            .expect("the reader returns a string")
            .to_owned();
        dropped(value);
        text
    }

    fn int_of(ctx: &mut nvs_runtime::Ctx, member: nvs_runtime::NvsFn, receiver: Value) -> i64 {
        let value = read(ctx, member, receiver);
        value.as_int().expect("the reader returns an integer")
    }

    fn texts_of(
        ctx: &mut nvs_runtime::Ctx,
        member: nvs_runtime::NvsFn,
        receiver: Value,
    ) -> Vec<String> {
        let value = read(ctx, member, receiver);
        let texts = {
            let list = value.as_array().expect("the reader returns a list");
            (0..list.count())
                .map(|index| {
                    let item = list.get_index(i64::try_from(index).expect("a short list"));
                    let item = item.expect("a list has every index below its count");
                    item.as_text().expect("each item is a string").to_owned()
                })
                .collect()
        };
        dropped(value);
        texts
    }

    /// Each reader returns the slot its builder wrote, and reading twice
    /// returns the same value. A receiver of the other class, or one that is
    /// not an object, throws.
    // covers: Core\Test\Outcome::class, Core\Test\Outcome::name, Core\Test\Outcome::attempt, Core\Test\Outcome::passed, Core\Test\Outcome::failures
    // covers: Core\Test\Summary::class, Core\Test\Summary::passed, Core\Test\Summary::failed, Core\Test\Summary::flaky, Core\Test\Summary::skipped, Core\Test\Summary::failedTests
    #[test]
    fn each_reader_returns_its_slot_and_a_receiver_of_another_class_throws() {
        let mut ctx = nvs_runtime::Ctx::buffered();
        let failures = ["the queue is empty".to_owned()];
        let failed = outcome(r"Shop\CartTest", "itAdds#1", 2, &failures);
        assert_eq!(
            text_of(&mut ctx, nvs_core_test_outcome_class, failed),
            r"Shop\CartTest"
        );
        assert_eq!(
            text_of(&mut ctx, nvs_core_test_outcome_name, failed),
            "itAdds#1"
        );
        assert_eq!(int_of(&mut ctx, nvs_core_test_outcome_attempt, failed), 2);
        let passed = read(&mut ctx, nvs_core_test_outcome_passed, failed);
        assert_eq!(
            passed.as_bool(),
            Some(false),
            "a run with a failure did not pass"
        );
        assert_eq!(
            texts_of(&mut ctx, nvs_core_test_outcome_failures, failed),
            failures
        );
        assert_eq!(
            texts_of(&mut ctx, nvs_core_test_outcome_failures, failed),
            failures,
            "releasing a reading leaves the outcome whole"
        );

        let clean = outcome(r"Shop\CartTest", "itAdds", 1, &[]);
        let passed = read(&mut ctx, nvs_core_test_outcome_passed, clean);
        assert_eq!(passed.as_bool(), Some(true), "a run with no failure passed");
        assert!(texts_of(&mut ctx, nvs_core_test_outcome_failures, clean).is_empty());

        let counts = Counts {
            passed: 3,
            failed: 1,
            flaky: 2,
            skipped: 4,
        };
        let names = ["itCharges".to_owned()];
        let class = summary(r"Shop\CartTest", counts, &names);
        assert_eq!(
            text_of(&mut ctx, nvs_core_test_summary_class, class),
            r"Shop\CartTest"
        );
        assert_eq!(int_of(&mut ctx, nvs_core_test_summary_passed, class), 3);
        assert_eq!(int_of(&mut ctx, nvs_core_test_summary_failed, class), 1);
        assert_eq!(int_of(&mut ctx, nvs_core_test_summary_flaky, class), 2);
        assert_eq!(int_of(&mut ctx, nvs_core_test_summary_skipped, class), 4);
        assert_eq!(
            texts_of(&mut ctx, nvs_core_test_summary_failed_tests, class),
            names
        );

        assert!(
            nvs_runtime::call(nvs_core_test_outcome_passed, &mut ctx, &[class]).is_err(),
            "a summary is not an outcome"
        );
        let _ = ctx.take_pending();
        assert!(
            nvs_runtime::call(nvs_core_test_summary_failed, &mut ctx, &[failed]).is_err(),
            "an outcome is not a summary"
        );
        let _ = ctx.take_pending();
        assert!(
            nvs_runtime::call(nvs_core_test_outcome_name, &mut ctx, &[Value::int(7)]).is_err(),
            "an integer is not an outcome"
        );
        let _ = ctx.take_pending();

        dropped(failed);
        dropped(clean);
        dropped(class);
    }
}
