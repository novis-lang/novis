//! `Core\Test` — [ADR 0079](../../../../docs/adr/0079-testing-is-a-language-feature.md)
//! § 4's assertion surface, and the other half of the `QName` `#[Test]` already
//! names.
//!
//! The attribute and this class are deliberately **one name**: `#[Test]` marks
//! the method and `Test::assertEquals` is what its body calls, so a single
//! `use Core\Test;` places both. `nvs_types::derive::ATTRIBUTES` owns the
//! attribute half; this module is the class one, and until it existed the
//! assertion in § 1's own worked example resolved against nothing at all.
//!
//! # Subject first, and generic
//!
//! Every member takes `$actual` then `$expected`, which is the **opposite** of
//! PHPUnit's order and falls out of
//! [ADR 0063](../../../../docs/adr/0063-core-api-conventions.md) rather than
//! out of a preference. Because reversing the two is the single commonest
//! mistake in the ecosystem this language is migrated from, every failure
//! message below labels the sides `$actual` and `$expected` by **name**, so a
//! reversed call still reads correctly.
//!
//! Both parameters are one [`CoreTy::Var`], which is what makes § 4's
//! "a type mismatch is a compile error" true with no rule of its own:
//! `nvs_types::generics` binds `T` from the first argument and substitutes it
//! through the signature, so the second is checked against the first's type by
//! the ordinary assignability check every `Core` call already goes through.
//!
//! # The three rows, and where they differ
//!
//! § 4's table is one comparison with two substitutions at the object row:
//!
//! * `assertSame` is [`nvs_runtime::identity`] — ADR 0090 § 3's table exactly,
//!   so two objects are the same object and nothing else is.
//! * `assertEquals` is that comparison with the object row replaced by
//!   [ADR 0013](../../../../docs/adr/0013-comparable-interface.md)'s
//!   `compareTo`.
//! * `assertEqualsDeep` replaces it with the structural walk below.
//!
//! Writing them as one comparison rather than three is what keeps the scalar,
//! `string` and `array<T>` rows from drifting: those are ADR 0090's, not this
//! ADR's, and a second reading of them here would be a second set of
//! PHP-divergence decisions nothing keeps in step.
//!
//! # The ledger, and the one member that discharges from it
//!
//! § 5 gives an assertion two effects, not one: it throws `Core\Test\Failure`
//! *and* it records its outcome into a per-test ledger the test's own code
//! cannot reach — which is what abolishes the silently-passing test, since the
//! runner reads the ledger rather than the exception state. The ledger is
//! `nvs_runtime::Ctx`'s (its field's own docs own why it lives there), the two
//! writers are [`held`] and [`failed`], and every member here goes through one
//! of them on every edge. `Core\Test::expectFailure(callable)` is the only way
//! an entry ever leaves it.
//!
//! # Known gaps
//!
//! 1. **§ 4's two compile errors are runtime throws for now.** A non-
//!    `Comparable` object under `assertEquals` is refused where it is *written*
//!    by the ADR, and is a catchable throw naming `assertEqualsDeep` here; that
//!    refusal is `nvs_types`' to make and wants the class graph this crate does
//!    not hold. The mismatch error is already made, by the `T` above.
//! 2. **Nothing reads the ledger yet.** Every assertion records its outcome
//!    into `nvs_runtime::Ctx`'s ledger and `expectFailure` discharges an entry
//!    from it, but the runner that reports one at the end of a test is later in
//!    Stage 7 — so § 20's zero-assertion rule, which is a question about the
//!    entry count, is owed rather than broken.
//! 3. **The roster is three members**, the equality ones § 4 tabulates. The
//!    `assertTrue`/`assertNull`/`assertCount`/`assertThrows` that section's
//!    example also writes are the same shape and are owed.

use nvs_runtime::{Ctx, Fault, Tag, ThrownClass, Value, identity};

use crate::registry::{Const, CoreClass, CoreMethod, CoreOption, CoreTy};

// ============================================================================
// Registration — this class's rows, and where its symbols live
// ============================================================================

/// `Core\Test`'s fully-qualified name, written once so the registry row, the
/// failure messages and `nvs_types::derive`'s attribute roster cannot drift
/// apart.
pub(crate) const NAME: &str = r"Core\Test";

/// The one type variable every assertion's two subjects share — see this
/// module's docs for what it buys.
const T: CoreTy = CoreTy::Var("T");

/// `{message?: string}` — ADR 0063 R2's trailing bag, and § 4's own
/// `{message: "a fresh user is active"}`.
///
/// [`Const::Null`] rather than an empty `string` because "not given" and
/// `{message: ""}` are one thing to a reader and the helper takes the same path
/// for both; the option's declared type stays `string`, which is what a call
/// site may write.
const MESSAGE: &[CoreOption] = &[CoreOption {
    name: "message",
    ty: CoreTy::Str,
    default: Const::Null,
}];

/// `Core\Test`'s registry rows — § 4's three equality members, plus § 5's
/// `expectFailure`. See
/// [`crate::registry::CLASSES`].
pub(crate) const CLASS: CoreClass = CoreClass {
    name: NAME,
    methods: &[
        CoreMethod {
            name: "assertSame",
            params: &[T, T, CoreTy::Options(MESSAGE)],
            defaults: &[],
            return_ty: CoreTy::Void,
            symbol: "nvs_core_test_assert_same",
        },
        CoreMethod {
            name: "assertEquals",
            params: &[T, T, CoreTy::Options(MESSAGE)],
            defaults: &[],
            return_ty: CoreTy::Void,
            symbol: "nvs_core_test_assert_equals",
        },
        CoreMethod {
            name: "assertEqualsDeep",
            params: &[T, T, CoreTy::Options(MESSAGE)],
            defaults: &[],
            return_ty: CoreTy::Void,
            symbol: "nvs_core_test_assert_equals_deep",
        },
        CoreMethod {
            name: "expectFailure",
            params: &[CoreTy::Callable],
            defaults: &[],
            return_ty: CoreTy::Void,
            symbol: "nvs_core_test_expect_failure",
        },
    ],
    instance: &[],
    slots: &[],
    constants: &[],
};

/// The address of one of *this* module's symbols, or `None` for a symbol that
/// belongs to another domain. See [`crate::symbols`].
pub(crate) fn address(symbol: &str) -> Option<*const u8> {
    Some(match symbol {
        "nvs_core_test_assert_same" => (nvs_core_test_assert_same as *const ()).cast(),
        "nvs_core_test_assert_equals" => (nvs_core_test_assert_equals as *const ()).cast(),
        "nvs_core_test_assert_equals_deep" => {
            (nvs_core_test_assert_equals_deep as *const ()).cast()
        }
        "nvs_core_test_expect_failure" => (nvs_core_test_expect_failure as *const ()).cast(),
        _ => return None,
    })
}

// ============================================================================
// The three members
// ============================================================================

nvs_runtime::nvs_helper! {
    /// `Core\Test::assertSame(T $actual, T $expected, {message?: string}): void`
    /// — § 4's identity row, which is [`identity::value_identical`] and nothing
    /// added to it.
    fn nvs_core_test_assert_same(ctx, args: [3]) {
        if identity::value_identical(args[0], args[1]) {
            return Ok(held(ctx, "assertSame"));
        }
        Err(failed(
            ctx,
            "assertSame",
            &format!(
                "`$actual` is {}, `$expected` is {}",
                shown(args[0]),
                shown(args[1])
            ),
            args[2],
        ))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Test::assertEquals(T $actual, T $expected, {message?: string}): void`
    /// — § 4's value row: [`identity::value_identical`] everywhere except two
    /// objects, which are compared through ADR 0013's `Comparable::compareTo`.
    fn nvs_core_test_assert_equals(ctx, args: [3]) {
        if equals(ctx, args[0], args[1])? {
            return Ok(held(ctx, "assertEquals"));
        }
        Err(failed(
            ctx,
            "assertEquals",
            &format!(
                "`$actual` is {}, `$expected` is {}",
                shown(args[0]),
                shown(args[1])
            ),
            args[2],
        ))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Test::assertEqualsDeep(T $actual, T $expected, {message?: string}): void`
    /// — § 4's structural walk, which reports **where** the two differ rather
    /// than only that they do.
    fn nvs_core_test_assert_equals_deep(ctx, args: [3]) {
        let Some(diff) = difference(args[0], args[1], 0, "")? else {
            return Ok(held(ctx, "assertEqualsDeep"));
        };
        let at = if diff.path.is_empty() {
            "the value itself".to_owned()
        } else {
            format!("`$actual{}`", diff.path)
        };
        Err(failed(
            ctx,
            "assertEqualsDeep",
            &format!(
                "the two differ at {at}: `$actual` is {}, `$expected` is {}",
                diff.actual, diff.expected
            ),
            args[2],
        ))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Test::expectFailure(callable $body): void` — ADR 0079 § 5's one
    /// greppable spelling for "this failure was on purpose".
    ///
    /// It runs `$body`, requires that an assertion inside it **failed**, and
    /// removes that entry from the ledger — which is the only way an entry ever
    /// leaves it. Everything about the shape follows from the ledger being the
    /// record and the throw being control flow:
    ///
    /// * The question asked is *what the ledger says*, not what class was
    ///   thrown. A body that caught its own failure and returned normally still
    ///   failed, and § 5's silently-passing test is precisely the program that
    ///   would otherwise sneak through here.
    /// * A body in which nothing failed is itself a failure, recorded and
    ///   raised through [`failed`] like any other — so wrapping *this* call in
    ///   a `catch` cannot erase it either.
    /// * Only the failures are discharged. A passing assertion inside `$body`
    ///   really ran, and § 20 counts it.
    ///
    /// The pending throw is taken where a failure was discharged, because that
    /// throw *is* the failure this member consumed. **Known gap:** a body that
    /// swallowed a failed assertion and then raised something else has its
    /// second throw consumed here too, this member having no way to tell the
    /// two apart from the ledger alone; naming the class would need the
    /// pending exception's descriptor, which is `nvs-runtime`'s and not a
    /// question a `Fault` answers.
    fn nvs_core_test_expect_failure(ctx, args: [1]) {
        let mark = ctx.assertion_count();
        let outcome = nvs_runtime::call_closure(ctx, args[0], &[]);
        let discharged = ctx.discharge_failures_from(mark);
        if let Ok(result) = outcome {
            #[expect(
                unsafe_code,
                reason = "`call_closure` hands back a value the caller owns, and \
                          this one is never handed on"
            )]
            unsafe {
                result.release();
            }
        } else if discharged == 0 {
            return outcome;
        } else {
            // The body's throw was the failure just discharged, so it is this
            // member's to consume rather than to propagate.
            drop(ctx.take_pending());
        }
        if discharged == 0 {
            return Err(failed(
                ctx,
                "expectFailure",
                "the callable ran without a failed assertion",
                Value::null(),
            ));
        }
        Ok(held(ctx, "expectFailure"))
    }
}

/// The [`Fault`] every failed assertion raises, with the `{message?: string}`
/// option in front of it where one was given.
///
/// A **catchable** throw rather than a [`Fault::fatal`]: ADR 0079 § 5 makes the
/// ledger the record and the throw the control flow, and a test that means to
/// assert its subject throws has to be able to run one inside a `try`. That the
/// ledger does not exist yet is this module's known gap 2.
///
/// The class is `Core\Test\Failure` — [`ThrownClass::TestFailure`], one row of
/// `nvs_hir::errors::TREE` like any other — rather than the bare
/// [`Fault::thrown`]'s `RuntimeError`, because § 5's own worked example catches
/// it **by name**: a composite assertion that meant to intercept a failed
/// assertion would otherwise have to catch every "the world said no" beside it.
/// This is the one throw site the whole surface funnels through, so naming the
/// class here is what names it for all three members.
///
/// It is also the one place a **failed** entry reaches § 5's ledger, and the
/// order matters: the entry is recorded before the [`Fault`] is handed back, so
/// there is no edge on which the throw exists and the record does not. That is
/// the whole of what makes the ledger something a `catch` cannot erase.
fn failed(ctx: &mut Ctx, member: &'static str, detail: &str, message: Value) -> Fault {
    let text = match message.as_text() {
        Some(given) if !given.is_empty() => {
            format!("{given}: Core\\Test::{member} failed: {detail}")
        }
        _ => format!("Core\\Test::{member} failed: {detail}"),
    };
    ctx.record_assertion(member, Some(text.clone()));
    Fault::thrown_as(ThrownClass::TestFailure, text)
}

/// [`failed`]'s other half: the entry an assertion that **held** leaves, and
/// the `void` it answers.
///
/// A passing assertion records too, because § 20's "a test that asserts nothing
/// fails" is a question about how many entries a test produced — a ledger of
/// failures alone cannot answer it. It allocates nothing: `member` is the
/// literal the member names itself with.
fn held(ctx: &mut Ctx, member: &'static str) -> Value {
    ctx.record_assertion(member, None);
    Value::null()
}

// ============================================================================
// The object rows — what `assertEquals` and `assertEqualsDeep` substitute
// ============================================================================

/// § 4's value comparison: ADR 0090 § 3's table, with the object row answered
/// by [ADR 0013](../../../../docs/adr/0013-comparable-interface.md)'s
/// `compareTo` instead of by pointer identity.
///
/// # Errors
///
/// A catchable [`Fault::thrown`] naming `assertEqualsDeep` where the receiver's
/// class declares no `compareTo` — this module's known gap 1, since the ADR
/// refuses that program where it is *written*. Or the callee's own failure,
/// propagated as [`Fault::Pending`] by `nvs_runtime::call_method`.
fn equals(ctx: &mut Ctx, actual: Value, expected: Value) -> Result<bool, Fault> {
    if actual.tag() != Some(Tag::Object) || expected.tag() != Some(Tag::Object) {
        return Ok(identity::value_identical(actual, expected));
    }
    let what = "Core\\Test::assertEquals";
    let Some(answer) = nvs_runtime::call_method(ctx, actual, "compareTo", &[expected], what)?
    else {
        return Err(Fault::thrown(format!(
            "Core\\Test::assertEquals compares an object only through `Comparable`, and \
             {} declares no `compareTo` — write `Core\\Test::assertEqualsDeep` for a \
             structural comparison, or `Core\\Test::assertSame` for identity",
            shown(actual)
        )));
    };
    answer
        .as_int()
        .map(|ordering| ordering == 0)
        .ok_or_else(|| {
            Fault::fatal(format!(
                "Core\\Test::assertEquals expected an `int` from `compareTo`, got tag {}",
                answer.tag_byte()
            ))
        })
}

/// Where `actual` and `expected` first differ under § 4's structural walk, or
/// `None` when they agree everywhere.
///
/// The scalar, `string` and `bytes` rows are [`identity::value_identical`]'s,
/// so nothing about ADR 0090 § 3 is read a second time here; what this adds is
/// the two container rows and the object one, each of which the deep member
/// exists to descend rather than to answer by identity.
///
/// # Errors
///
/// A catchable [`Fault::thrown`] past [`MAX_DEPTH`] levels of containment.
/// An Novis array cannot contain itself (ADR 0090 § 3), so the only structure
/// that can recur is an object graph, and a depth cap is what bounds it —
/// refusing to answer is the honest outcome, an assertion that quietly compared
/// half a graph being worse than one that says it could not.
fn difference(
    actual: Value,
    expected: Value,
    depth: usize,
    path: &str,
) -> Result<Option<Diff>, Fault> {
    if depth >= MAX_DEPTH {
        return Err(Fault::thrown(format!(
            "Core\\Test::assertEqualsDeep walked {MAX_DEPTH} levels of containment at \
             `$actual{path}` without reaching a scalar — compare a narrower subject, or \
             `Core\\Test::assertSame` if identity is what was meant"
        )));
    }
    match (actual.tag(), expected.tag()) {
        (Some(Tag::Array), Some(Tag::Array)) => array_difference(actual, expected, depth, path),
        (Some(Tag::Object), Some(Tag::Object)) => object_difference(actual, expected, depth, path),
        _ => Ok(leaf(actual, expected, path)),
    }
}

/// How deep [`difference`] descends before it refuses to answer.
const MAX_DEPTH: usize = 64;

/// One difference: where it is, and what each side holds there.
struct Diff {
    /// The path from the subject, written as a program would subscript it —
    /// empty for the subject itself.
    path: String,
    /// What `$actual` holds there, rendered by [`shown`].
    actual: String,
    /// What `$expected` holds there.
    expected: String,
}

/// [`difference`]'s non-container rows, answered by identity.
fn leaf(actual: Value, expected: Value, path: &str) -> Option<Diff> {
    (!identity::value_identical(actual, expected)).then(|| Diff {
        path: path.to_owned(),
        actual: shown(actual),
        expected: shown(expected),
    })
}

/// ADR 0090 § 3's array row, descended rather than answered: the same length,
/// the same keys in the same order, and every value equal by this walk.
fn array_difference(
    actual: Value,
    expected: Value,
    depth: usize,
    path: &str,
) -> Result<Option<Diff>, Fault> {
    let (Some(left), Some(right)) = (entries(actual), entries(expected)) else {
        return Ok(leaf(actual, expected, path));
    };
    if left.len() != right.len() {
        return Ok(Some(Diff {
            path: path.to_owned(),
            actual: format!("an array of {}", left.len()),
            expected: format!("an array of {}", right.len()),
        }));
    }
    for ((key, mine), (theirs_key, theirs)) in left.into_iter().zip(right) {
        let at = format!("{path}[\"{key}\"]");
        if key != theirs_key {
            return Ok(Some(Diff {
                path: path.to_owned(),
                actual: format!("a key `{key}`"),
                expected: format!("a key `{theirs_key}`"),
            }));
        }
        if let Some(diff) = difference(mine, theirs, depth + 1, &at)? {
            return Ok(Some(diff));
        }
    }
    Ok(None)
}

/// Every live `(key, value)` of an array, in iteration order — which ADR 0007
/// § 5 makes part of the value, so this walk preserves it rather than keying a
/// map with it.
fn entries(value: Value) -> Option<Vec<(String, Value)>> {
    let array = crate::arr::borrowed(value.array_ptr()?);
    let mut out = Vec::new();
    let mut from = 0usize;
    while let Some(slot) = array.next_slot(from) {
        from = slot + 1;
        let Some(key) = array.key_at(slot) else {
            continue;
        };
        out.push((
            String::from_utf8_lossy(key.as_bytes()).into_owned(),
            array.value_at(slot).unwrap_or_else(Value::null),
        ));
    }
    Some(out)
}

/// § 4's walk of an object: the same class, then every declared property
/// compared by this walk.
///
/// The class is compared first and by **name**, because two objects of two
/// classes that happen to share a field set are not the same value and saying
/// so at the first differing property would name the wrong thing.
fn object_difference(
    actual: Value,
    expected: Value,
    depth: usize,
    path: &str,
) -> Result<Option<Diff>, Fault> {
    let (Some(left), Some(right)) = (actual.obj_ptr(), expected.obj_ptr()) else {
        return Ok(leaf(actual, expected, path));
    };
    // The one shortcut, and the reason a shared sub-object in both graphs does
    // not spend the depth budget twice: one allocation is equal to itself under
    // every row of this walk.
    if std::ptr::eq(left, right) {
        return Ok(None);
    }
    let left = handle(left);
    let right = handle(right);
    if left.class_name() != right.class_name() {
        return Ok(Some(Diff {
            path: path.to_owned(),
            actual: format!("a `{}`", left.class_name()),
            expected: format!("a `{}`", right.class_name()),
        }));
    }
    #[expect(
        unsafe_code,
        reason = "the descriptor is owned by the unit's class table, which \
                  outlives every instance of the class it describes"
    )]
    let desc = unsafe { &*left.class() };
    for slot in 0..left.field_count().min(right.field_count()) {
        let name = desc
            .field_name(slot)
            .map_or_else(|| slot.to_string(), ToOwned::to_owned);
        let at = format!("{path}->{name}");
        let Some(diff) = difference(left.field(slot), right.field(slot), depth + 1, &at)? else {
            continue;
        };
        // ADR 0092 § 5's redaction row, at the one place this module renders a
        // value a program declared `secret`: the comparison still descends —
        // whether two secrets agree is not itself a secret — but neither side is
        // quoted back into a message a build log keeps.
        return Ok(Some(if desc.field_is_secret(slot) {
            Diff {
                path: at,
                actual: REDACTED.to_owned(),
                expected: REDACTED.to_owned(),
            }
        } else {
            diff
        }));
    }
    Ok(None)
}

/// What a `secret`-typed property renders as wherever this module would
/// otherwise quote its value.
const REDACTED: &str = "«redacted»";

/// A borrowed handle on a live object allocation, for reading its class and its
/// slots without taking a reference this walk would then have to release.
fn handle(ptr: *mut nvs_runtime::ObjHeader) -> std::mem::ManuallyDrop<nvs_runtime::NvsObj> {
    #[expect(
        unsafe_code,
        reason = "the caller's value owns a reference to a live allocation, so it \
                  is live for this borrow; the handle is never dropped, so the \
                  reference is not released twice"
    )]
    std::mem::ManuallyDrop::new(unsafe { nvs_runtime::NvsObj::from_raw(ptr) })
}

// ============================================================================
// Rendering a side of a failure
// ============================================================================

/// `value` as a failure message quotes it — short, and never the whole of a
/// container.
///
/// A message is written to a build log, so a subject the test happened to build
/// out of a large input would otherwise choose how many bytes that log gains —
/// the same bound [`crate::uuid`] puts on a rejected operand, for the same
/// reason. A container names its size rather than its contents, because
/// `assertEqualsDeep` has already said *where* the two differ and the leaf it
/// names is what a reader wants quoted.
fn shown(value: Value) -> String {
    match value.tag() {
        None | Some(Tag::Null) => "null".to_owned(),
        Some(Tag::Unset) => "never written".to_owned(),
        Some(Tag::Bool) => (value.as_bool() == Some(true)).to_string(),
        Some(Tag::Int) => value.as_int().unwrap_or(0).to_string(),
        Some(Tag::Uint) => value.as_uint().unwrap_or(0).to_string(),
        Some(Tag::Float) => value.as_float().unwrap_or(f64::NAN).to_string(),
        Some(Tag::Decimal) => value
            .as_decimal()
            .map_or_else(|| "0".to_owned(), |decimal| decimal.to_string()),
        Some(Tag::Str) => format!("\"{}\"", quoted(value.as_str_bytes().unwrap_or_default())),
        Some(Tag::Bytes) => format!("{} bytes", value.as_bytes().unwrap_or_default().len()),
        Some(Tag::Array) => entries(value).map_or_else(
            || "an array".to_owned(),
            |entries| format!("an array of {}", entries.len()),
        ),
        Some(Tag::Object) => value.obj_ptr().map_or_else(
            || "an object".to_owned(),
            |ptr| format!("a `{}`", handle(ptr).class_name()),
        ),
        Some(Tag::Closure) => "a closure".to_owned(),
        Some(Tag::Resource) => "a resource".to_owned(),
    }
}

/// A `string`'s first [`SHOWN_CHARS`] characters, with an ellipsis where
/// anything was dropped. Decoded lossily for [`crate::debug`]'s reason: a
/// `string` is UTF-8 by ADR 0009's promise, so a run that is not is exactly
/// what a failing assertion is there to show.
fn quoted(bytes: &[u8]) -> String {
    let text = String::from_utf8_lossy(bytes);
    let mut out: String = text.chars().take(SHOWN_CHARS).collect();
    if text.chars().nth(SHOWN_CHARS).is_some() {
        out.push('…');
    }
    out
}

/// How much of a quoted `string` [`quoted`] keeps.
const SHOWN_CHARS: usize = 64;

#[cfg(test)]
mod tests {
    use super::*;

    /// § 4's three equality members — every row but § 5's `expectFailure`,
    /// which takes a body rather than a pair of subjects and has its own test
    /// below.
    fn equality_members() -> impl Iterator<Item = &'static CoreMethod> {
        CLASS
            .methods
            .iter()
            .filter(|method| method.name != "expectFailure")
    }

    /// § 4's order, which is the opposite of the one every migrated test suite
    /// was written in — so it is asserted rather than left to a reading of the
    /// rows above.
    #[test]
    fn every_assertion_is_subject_first_and_generic() {
        for method in equality_members() {
            assert_eq!(
                method.params.len(),
                3,
                "`{}` should take `$actual`, `$expected` and one options bag",
                method.name
            );
            assert!(
                matches!(method.params[0], CoreTy::Var("T"))
                    && matches!(method.params[1], CoreTy::Var("T")),
                "`{}` should bind both subjects to one type variable",
                method.name
            );
            assert!(
                matches!(method.return_ty, CoreTy::Void),
                "`{}` should answer nothing — a failure is a throw",
                method.name
            );
        }
    }

    /// Every row's symbol has an address here, which is what
    /// [`crate::symbols`]' own sweep asserts across the whole registry — held
    /// once more locally so a member added above fails here rather than in a
    /// crate-wide test that names no module.
    #[test]
    fn every_row_names_a_symbol_this_module_claims() {
        for method in CLASS.methods {
            assert!(
                address(method.symbol).is_some(),
                "`{}` names `{}`, which this module does not claim",
                method.name,
                method.symbol
            );
        }
    }

    /// The one option § 4 writes, and the reason it defaults to `null` rather
    /// than to an empty `string`.
    #[test]
    fn the_only_option_is_a_message_that_defaults_to_absent() {
        for method in equality_members() {
            let CoreTy::Options(bag) = method.params[2] else {
                panic!(
                    "`{}`'s third parameter should be an options bag",
                    method.name
                );
            };
            assert_eq!(bag.len(), 1);
            assert_eq!(bag[0].name, "message");
            assert!(matches!(bag[0].ty, CoreTy::Str));
            assert!(matches!(bag[0].default, Const::Null));
        }
    }

    /// § 5's own row, which is deliberately not shaped like § 4's: it takes the
    /// body whose failure is expected and nothing else — no `{message?:}`,
    /// because what it reports on is the ledger rather than a comparison, and
    /// no subject, because there is nothing being compared.
    #[test]
    fn expect_failure_takes_one_callable_and_answers_nothing() {
        let member = CLASS
            .methods
            .iter()
            .find(|method| method.name == "expectFailure")
            .expect("§ 5's member is registered");
        assert!(matches!(member.params, [CoreTy::Callable]));
        assert!(matches!(member.return_ty, CoreTy::Void));
        // And it is the only row that is not one of § 4's three.
        assert_eq!(equality_members().count(), CLASS.methods.len() - 1);
    }
}
