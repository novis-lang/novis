//! `Core\\Str` end to end, and ADR 0063 R2's options bag: omission, name-matching, and a rejected option.
//!
//! Split out of the single `compile_and_run.rs`; every test keeps its own name
//! and body. See `tests/common/mod.rs` for the shared fixtures and for why
//! these go through the real pipeline.

mod common;

use common::*;

#[test]
fn a_core_str_member_runs_and_hands_its_result_back_as_a_string() {
    let source = "<?mwl
echo Core\\Str::upper(\"mwl\"), \"|\", Core\\Str::upperFirst(\"runs\"), \"|\", Core\\Str::repeat(\"ab\", 2);
";
    assert_eq!(output_of(source), "MWL|Runs|abab");
}

/// The `Core` half of the parameter-default mechanism: `join`'s separator and
/// `padStart`'s padding come from `mwl_stdlib::registry::CoreMethod::defaults`
/// through the same call-site materialization a written `= expr` uses, so the
/// helper still receives its declared arity.
#[test]
fn a_core_member_may_have_its_trailing_argument_omitted() {
    let source = "<?mwl
array<string> $parts = [\"a\", \"b\", \"c\"];
echo Core\\Str::join($parts), \"|\", Core\\Str::join($parts, \"-\");
echo \"|\", Core\\Str::padStart(\"7\", 3, \"0\"), \"|\", Core\\Str::padStart(\"7\", 3);
";
    assert_eq!(output_of(source), "abc|a-b-c|007|  7");
}

/// ADR 0063 R2's options bag end to end. The bag has no runtime
/// representation at all — `mwl_ir::lower::lower_call_args` flattens it into
/// one ordinary argument per declared option — so this is the check that the
/// flattened arity and the helper's own `args: [3]` agree, in both the
/// written and the omitted case.
#[test]
fn a_core_member_takes_an_options_shape_that_may_be_omitted() {
    let source = "<?mwl
echo Core\\Arr::count(Core\\Arr::range(1, 10));
echo \"|\", Core\\Arr::count(Core\\Arr::range(1, 10, {step: 3}));
echo \"|\", Core\\Arr::count(Core\\Arr::range(10, 1, {step: 2}));
";
    assert_eq!(output_of(source), "10|4|5");
}

/// A `Core` parameter declared `int|string` takes either, at the argument's
/// own representation: `mwl-codegen` writes each helper argument's tag from
/// the value it is passing, so the union never needs an IR type of its own.
/// `mwl_ir::lower::ArgSig::helper` owns why that is the helper convention's
/// property and not this member's.
#[test]
fn a_core_union_parameter_arrives_at_the_helper_tagged_as_what_was_passed() {
    let source = "<?mwl
array<int> $a = [\"name\" => 1, 5 => 2];
if (Core\\Arr::hasKey($a, \"name\")) { echo \"y\"; } else { echo \"n\"; }
if (Core\\Arr::hasKey($a, 5)) { echo \"y\"; } else { echo \"n\"; }
if (Core\\Arr::hasKey($a, \"5\")) { echo \"y\"; } else { echo \"n\"; }
if (Core\\Arr::hasKey($a, 6)) { echo \"y\"; } else { echo \"n\"; }
";
    assert_eq!(output_of(source), "yyyn");
}

/// A bag with *two* options, written in every combination: neither, one, the
/// other, both, and the second one written first. Options are named rather
/// than positional, so the written order is not the ABI order — this is what
/// holds `lower_options_arg`'s "walk the declared options, look each one up"
/// against the easier and wrong "walk the written fields".
#[test]
fn an_option_is_matched_by_name_not_by_the_order_it_was_written() {
    let source = "<?mwl
var $s = \"a-b-A-b\";
echo Core\\Str::replace($s, \"-\", \"+\");
echo \"|\", Core\\Str::replace($s, \"-\", \"+\", {limit: 2});
echo \"|\", Core\\Str::replace($s, \"a\", \"z\", {caseInsensitive: true});
echo \"|\", Core\\Str::replace($s, \"a\", \"z\", {limit: 1, caseInsensitive: true});
";
    assert_eq!(output_of(source), "a+b+A+b|a+b+A-b|z-b-z-b|z-b-A-b");
}

/// A `Core` member may throw over an option it was given: `range`'s `step`
/// must be positive, PHP 8.5's own rule. Proves the option reached the helper
/// as a real argument rather than being dropped on the way — an option that
/// never arrived would have taken its default of `1` and succeeded.
///
/// Asserted as a status rather than through a `catch`, which is the narrower
/// claim: what is under test is that the option reached the helper at all, and
/// a status says so without also depending on the promotion `catch` needs. The
/// `mwl_stdlib` unit test beside the member covers the message itself.
#[test]
fn an_option_a_core_member_rejects_throws_through_the_helper_boundary() {
    let source = "<?mwl
echo Core\\Arr::count(Core\\Arr::range(1, 5, {step: 0}));
";
    let mut ctx = Ctx::buffered();
    let status = run_with(&mut ctx, source).expect_err("a step of 0 is refused");
    assert_eq!(status, mwl_runtime::THROWN);
}
