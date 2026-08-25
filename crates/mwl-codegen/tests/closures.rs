//! ADR 0031's closures as compiled objects: the method table, captured values, arity, and a throw out of one.
//!
//! Split out of the single `compile_and_run.rs`; every test keeps its own name
//! and body. See `tests/common/mod.rs` for the shared fixtures and for why
//! these go through the real pipeline.

mod common;

use common::*;

#[test]
fn a_closure_is_reachable_through_the_method_table() {
    // ADR 0031 end to end: `Core\Arr::filter` is native Rust and reaches the
    // closure through `mwl_runtime::call_closure`, which resolves
    // `mwl_runtime::CLOSURE_INVOKE` against the receiver's descriptor — so
    // this fails the moment `mwl-ir`'s label for that method and the
    // runtime's stop agreeing.
    let source = "<?mwl
array<int> $nums = [1, 2, 3, 4, 5, 6];
var $big = Core\\Arr::filter($nums, fn(int $n): bool => $n > 3);
echo Core\\Arr::count($big);
";
    assert_eq!(output_of(source), "3");
}

#[test]
fn a_closure_object_carries_its_own_arity_in_slot_zero() {
    // The two-parameter predicate and the one-parameter one run over the same
    // array through the same `Core` member. `mwl_runtime::CLOSURE_ARITY_SLOT`
    // is what tells them apart, so a disagreement with `mwl_ir::lower`'s
    // `FN_ARITY` field order shows up here as a wrong count or a fault, not as
    // a silent extra argument.
    let source = "<?mwl
array<string> $a = [\"keep\" => \"x\", \"drop\" => \"y\"];
var $byKey = Core\\Arr::filter($a, fn(string $v, string $k): bool => $k == \"keep\");
var $byValue = Core\\Arr::filter($a, fn(string $v): bool => $v == \"y\");
echo Core\\Arr::count($byKey), \"|\", Core\\Arr::count($byValue);
";
    assert_eq!(output_of(source), "1|1");
}

#[test]
fn a_closure_captures_an_outer_local_by_value() {
    // ADR 0031 § 2: the snapshot is taken when the literal is evaluated, so
    // reassigning the captured local afterwards does not reach the closure.
    let source = "<?mwl
int $floor = 3;
array<int> $nums = [1, 2, 3, 4, 5, 6];
var $f = fn(int $n): bool => $n > $floor;
$floor = 0;
echo Core\\Arr::count(Core\\Arr::filter($nums, $f));
";
    assert_eq!(output_of(source), "3");
}

#[test]
fn a_closure_capturing_a_string_in_a_loop_leaks_nothing() {
    // Ten thousand closure objects, each holding one retained capture. A
    // missing release in `lower_closure`'s exit sweep leaks a buffer per
    // iteration; a doubled one crashes here.
    let source = "<?mwl
array<int> $nums = [1, 2, 3];
var $i = 0;
var $seen = 0;
while ($i < 10000) {
    string $tag = \"t\" . $i;
    var $kept = Core\\Arr::filter($nums, fn(int $n): bool => $tag != \"zzz\");
    $seen = $seen + Core\\Arr::count($kept) as int;
    $i = $i + 1;
}
echo $seen;
";
    assert_eq!(output_of(source), "30000");
}

#[test]
fn a_closure_that_throws_propagates_out_of_the_core_member_that_called_it() {
    // `mwl_runtime::Fault::Pending` is what keeps the exception the closure
    // raised intact: a `Fault::Thrown` built inside `call_closure` would
    // replace it with a bare message from the helper.
    let source = "<?mwl
class Boom {
    public static function check(int $n): bool {
        throw new LogicError(\"nope\");
    }
}
array<int> $nums = [1, 2, 3];
try {
    var $kept = Core\\Arr::filter($nums, fn(int $n): bool => Boom::check($n));
    echo \"unreachable\";
} catch (LogicError $e) {
    echo \"caught: \" . $e->message;
}
";
    assert_eq!(output_of(source), "caught: nope");
}
