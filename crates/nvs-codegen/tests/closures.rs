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
    // closure through `nvs_runtime::call_closure`, which resolves
    // `nvs_runtime::CLOSURE_INVOKE` against the receiver's descriptor — so
    // this fails the moment `nvs-ir`'s label for that method and the
    // runtime's stop agreeing.
    let source = "<?nvs
array<int> $nums = [1, 2, 3, 4, 5, 6];
var $big = Core\\Arr::filter($nums, fn(int $n): bool => $n > 3);
echo Core\\Arr::count($big);
";
    assert_eq!(output_of(source), "3");
}

#[test]
fn a_closure_object_carries_its_own_arity_in_slot_zero() {
    // The two-parameter predicate and the one-parameter one run over the same
    // array through the same `Core` member. `nvs_runtime::CLOSURE_ARITY_SLOT`
    // is what tells them apart, so a disagreement with `nvs_ir::lower`'s
    // `FN_ARITY` field order shows up here as a wrong count or a fault, not as
    // a silent extra argument.
    let source = "<?nvs
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
    let source = "<?nvs
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
    let source = "<?nvs
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
    // `nvs_runtime::Fault::Pending` is what keeps the exception the closure
    // raised intact: a `Fault::Thrown` built inside `call_closure` would
    // replace it with a bare message from the helper.
    let source = "<?nvs
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

/// The subject and the predicate the two guards below share: a `string`
/// parameter, and an `array<mixed>` whose third entry is an `int`.
///
/// The mismatch lands on the *third* entry on purpose, so `Core\Arr::filter`
/// is two kept entries into its walk when the check refuses — which is what
/// gives the leak guard a partial result to abandon.
const MISMATCH: &str = "<?nvs
callable $wantsString = fn (string $s): bool => $s != \"zzz\";
array<mixed> $mixed = [\"a\", \"b\", 3];
";

#[test]
fn a_mismatched_argument_throws_a_logic_error_out_of_the_core_member_that_called_it() {
    // ADR 0031 § 1: a `callable` carries no parameter list, so nothing above
    // the call site saw what this closure requires and
    // `nvs_runtime::call_closure` is the only thing that can refuse the
    // argument. Caught as `LogicError` specifically, which is the half
    // `tests/conformance/lang/a-closure-argument-is-checked-against-its-parameter-type.nvst`
    // cannot pin: it catches `Throwable`, and the widening row beside this one
    // throws `ArithmeticError` past 2^53 — a `Throwable` catch cannot tell the
    // two apart.
    let caught = format!(
        "{MISMATCH}try {{
    var $kept = Core\\Arr::filter($mixed, $wantsString);
    echo \"did not throw \", Core\\Arr::count($kept) as string;
}} catch (LogicError $e) {{
    echo \"caught: \", $e->message;
}}
"
    );
    assert_eq!(
        output_of(&caught),
        "caught: argument 1 to a `callable` must be of type string, int given"
    );

    // Uncaught, the throw is an ordinary `THROWN` status carrying the same
    // message — never a `Fault::Fatal`, which is what the check reserves for a
    // compiler bug (`nvs_runtime::closure`'s `check_param_tags`).
    let mut ctx = Ctx::buffered();
    let uncaught = format!("{MISMATCH}Core\\Arr::filter($mixed, $wantsString);\n");
    assert_eq!(run_with(&mut ctx, &uncaught).unwrap_err(), THROWN);
    assert_eq!(
        ctx.pending().as_deref(),
        Some("argument 1 to a `callable` must be of type string, int given")
    );
}

#[test]
fn an_int_argument_widens_into_a_float_parameter_and_is_refused_past_two_to_the_53() {
    // ADR 0007 § 2's one implicit conversion, reached from the caller no
    // written `as float` ever passes through: `Core\Arr::map` hands a native
    // `int` to a closure whose parameter is declared `float`, and
    // `nvs_runtime::call_closure` converts it in place because no checker saw
    // this call site to insert it — a `callable` carries no parameter list
    // (ADR 0031 § 1).
    //
    // `Core\Json::encode` is the assertion rather than an `echo` of the
    // number: it renders a `float` with a trailing `.0`, so an `int` that
    // arrived unconverted would print `4503599627370496` and fail here. That
    // is the half the `.nvst` case of the same name reads off `$half(7)`'s own
    // rendering instead.
    let source = "<?nvs
callable $half = fn (float $f): float => $f / 2.0;

// 2^53 is the last integer an `f64` holds exactly, so it is the last one this
// row accepts — asserted beside 7, which no reading of the value could
// mistake for anything else.
array<int> $ints = [7, 9007199254740992];
echo Core\\Json::encode(Core\\Arr::map($ints, $half)), \"\\n\";

// The first refused one, one past that bound. Caught as `ArithmeticError`
// specifically — ADR 0007 § 4's class for a numeric overflow, and the
// distinction a `catch (Throwable)` in a `.nvst` case cannot make against the
// `LogicError` the mismatched-tag row throws.
array<int> $edge = [1, 9007199254740993];
try {
    // Bound before it is echoed: `echo` writes its arguments one at a time,
    // so evaluating the call inside the list prints the prefix before the
    // throw reaches the `catch`.
    var $past = Core\\Arr::map($edge, $half);
    echo \"did not throw \", Core\\Json::encode($past), \"\\n\";
} catch (ArithmeticError $e) {
    echo \"caught: \", $e->message, \"\\n\";
}

// `uint` is a tag of its own and takes the same row rather than a second one.
uint $nine = 9;
array<uint> $uints = [$nine];
echo Core\\Json::encode(Core\\Arr::map($uints, $half)), \"\\n\";
";
    assert_eq!(
        output_of(source),
        "[3.5,4503599627370496.0]\n\
         caught: cannot convert argument 1 to a `callable` from `int` to `float`\n\
         [4.5]\n"
    );
}

/// Compiles `source`, runs it, and answers how many bytes the **run**
/// allocated and did not free — compilation and `install_in` happen outside
/// the window on purpose, since what is under test is what compiled code
/// spends.
///
/// The balance is `nvs_runtime::budget`'s, which every build maintains because
/// the memory limit is read off it; this binary installs no allocator of its
/// own, and could not, since a `#[global_allocator]` is chosen once per binary
/// and `nvs-runtime` now registers one in every `not(test)` build.
///
/// **Only run in a debug build.** What the gate keeps out is no longer a
/// missing counter but an optimized one's inlining: the number below is pinned
/// against what an unoptimized build allocates, and nothing has measured it
/// under `--release`.
#[cfg(debug_assertions)]
fn live_bytes_of_run(source: &str) -> isize {
    let unit = compile(source).expect("the fixture compiles");
    let mut ctx = Ctx::buffered();
    unit.install_in(&mut ctx);
    let entry = unit.script().expect("the script frame was compiled");
    let before = nvs_runtime::budget::live_bytes();
    entry.call(&mut ctx).expect("the script ran to completion");
    nvs_runtime::budget::live_bytes() - before
}

#[test]
#[cfg(debug_assertions)]
fn the_partial_result_a_mismatched_argument_abandons_leaks_nothing() {
    // The half a `.nvst` case cannot see. `Core\Arr::filter` is two kept
    // entries into its walk when `nvs_runtime::call_closure` refuses the
    // third, and `nvs_core_arr_filter`'s doc claims the early return itself
    // frees the partial result and the entry's key, since `NvsArray` and
    // `NvsStr` release on drop. A missed release leaks that array plus the two
    // string buffers it retained, every iteration.
    //
    // Measured as a *slope* rather than against a fixed bound: a run's own
    // fixed overhead is the same in both runs, so ten times the iterations
    // must not cost ten times anything. Both runs hold 8 bytes today — the
    // buffer the echoed count lands in — and the slack below is what tells
    // that apart from a leak. Calibrated rather than guessed: leaking the
    // partial result with a `std::mem::forget` on `nvs_core_arr_filter`'s own
    // error path moves the pair to 46,388 and 467,788 bytes, a slope of
    // 421 KiB over the extra 1,800 iterations.
    let source = |iterations: u32| {
        format!(
            "{MISMATCH}var $i = 0;
var $caught = 0;
while ($i < {iterations}) {{
    string $tag = \"t\" . $i;
    array<mixed> $fresh = [$tag, $tag . \"!\", 3];
    try {{
        var $kept = Core\\Arr::filter($fresh, $wantsString);
        $caught = $caught + Core\\Arr::count($kept) as int;
    }} catch (LogicError $e) {{
        $caught = $caught + 1;
    }}
    $i = $i + 1;
}}
echo $caught;
"
        )
    };

    // The subject is rebuilt from a fresh concatenation each iteration, so a
    // retained-but-never-released entry is a buffer nothing else holds.
    assert_eq!(output_of(&source(200)), "200");

    let short = live_bytes_of_run(&source(200));
    let long = live_bytes_of_run(&source(2000));
    assert!(
        long - short < 4096,
        "the abandoned partial result leaks: 200 iterations held {short} bytes, 2000 held {long}"
    );
}
