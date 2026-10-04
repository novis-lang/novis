//! `rule:types/anonymous-function`'s callables as compiled objects: the method table, captured values, arity, and a throw out of one.
//!
//! See `tests/common/mod.rs` for the shared fixtures and for why these go
//! through the real pipeline.

mod common;

use common::*;

#[test]
fn a_callable_is_reachable_through_the_method_table() {
    // `rule:types/anonymous-function` end to end: `Core\Arr::filter` is native Rust and reaches the
    // callable through `nvs_runtime::call_callable`, which resolves
    // `nvs_runtime::CALLABLE_INVOKE` against the receiver's descriptor — so
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
fn a_callable_object_carries_its_own_arity_in_slot_zero() {
    // The two-parameter predicate and the one-parameter one run over the same
    // array through the same `Core` member. `nvs_runtime::CALLABLE_ARITY_SLOT`
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
fn an_anon_fn_captures_an_outer_local_by_value() {
    // `rule:types/implicit-capture`: the snapshot is taken when the literal is evaluated, so
    // reassigning the captured local afterwards does not reach the callable.
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
fn an_anon_fn_capturing_a_string_in_a_loop_leaks_nothing() {
    // Ten thousand callable objects, each holding one retained capture. A
    // missing release in `lower_anon_fn`'s exit sweep leaks a buffer per
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
fn a_callable_that_throws_propagates_out_of_the_core_member_that_called_it() {
    // `nvs_runtime::Fault::Pending` is what keeps the exception the callable
    // raised intact: a `Fault::Thrown` built inside `call_callable` would
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

/// The subject and the predicate the guards below share: a `string`
/// parameter, and an `array<mixed>` whose third entry is an `int`.
///
/// The mismatch lands on the *third* entry on purpose, so `Core\Arr::filter`
/// is two kept entries into its walk when the check refuses — which is what
/// gives the leak guard a partial result to abandon.
///
/// `$forward` is what carries the bare `callable` to a member that declares
/// its callback's signature (`rule:types/callable-signature`): the position
/// hands a `mixed`, which is what the forwarder declares, and the argument
/// that gets refused is the one it passes on. Declared once and outside every
/// loop below, so the leak guard is still measuring one callable's walk.
const MISMATCH: &str = "<?nvs
callable $wantsString = fn (string $s): bool => $s != \"zzz\";
callable(mixed, string): bool $forward = fn (mixed $m): bool => $wantsString($m) as bool;
array<mixed> $mixed = [\"a\", \"b\", 3];
";

#[test]
fn a_mismatched_argument_throws_a_logic_error_out_of_the_core_member_that_called_it() {
    // `rule:types/anonymous-function`: a `callable` carries no parameter list, so nothing above
    // the call site saw what this callable requires and
    // `nvs_runtime::call_callable` is the only thing that can refuse the
    // argument. Caught as `LogicError` specifically, which is the half
    // `tests/conformance/lang/a-closure-argument-is-checked-against-its-parameter-type.nvst`
    // cannot pin: it catches `Throwable`, and the widening row beside this one
    // throws `ArithmeticError` past 2^53 — a `Throwable` catch cannot tell the
    // two apart.
    let caught = format!(
        "{MISMATCH}try {{
    var $kept = Core\\Arr::filter($mixed, $forward);
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
    // compiler bug (`nvs_runtime::callable`'s `check_param_tags`).
    let mut ctx = Ctx::buffered();
    let uncaught = format!("{MISMATCH}Core\\Arr::filter($mixed, $forward);\n");
    assert_eq!(run_with(&mut ctx, &uncaught).unwrap_err(), THROWN);
    assert_eq!(
        ctx.pending().as_deref(),
        Some("argument 1 to a `callable` must be of type string, int given")
    );
}

#[test]
fn an_int_argument_widens_into_a_float_parameter_and_is_refused_past_two_to_the_53() {
    // `rule:types/conversion`'s one implicit conversion, reached from the caller no
    // written `as float` ever passes through: `Core\Arr::map` hands a native
    // `int` to a callable whose parameter is declared `float`, and
    // `nvs_runtime::call_callable` converts it in place because no checker saw
    // this call site to insert it — a `callable` carries no parameter list
    // (`rule:types/anonymous-function`).
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
// `map` declares what it hands its callback (`rule:types/callable-signature`),
// which a bare `callable` does not promise, so `$half` is reached through a
// literal — and the conversion under test is still the one
// `nvs_runtime::call_callable` makes on the way into `$half`.
echo Core\\Json::encode(Core\\Arr::map($ints, fn (int $n) => $half($n))), \"\\n\";

// The first refused one, one past that bound. Caught as `ArithmeticError`
// specifically — `rule:types/arithmetic`'s class for a numeric overflow, and the
// distinction a `catch (Throwable)` in a `.nvst` case cannot make against the
// `LogicError` the mismatched-tag row throws.
array<int> $edge = [1, 9007199254740993];
try {
    // Bound before it is echoed: `echo` writes its arguments one at a time,
    // so evaluating the call inside the list prints the prefix before the
    // throw reaches the `catch`.
    var $past = Core\\Arr::map($edge, fn (int $n) => $half($n));
    echo \"did not throw \", Core\\Json::encode($past), \"\\n\";
} catch (ArithmeticError $e) {
    echo \"caught: \", $e->message, \"\\n\";
}

// `uint` is a tag of its own and takes the same row rather than a second one.
uint $nine = 9;
array<uint> $uints = [$nine];
echo Core\\Json::encode(Core\\Arr::map($uints, fn (uint $n) => $half($n))), \"\\n\";
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
/// and `nvs-runtime` registers one in every `not(test)` build.
///
/// **Only run in a debug build.** What the gate keeps out is an optimized
/// build's inlining: the number below is pinned against what an unoptimized
/// build allocates, and nothing has measured it under `--release`.
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
    // entries into its walk when `nvs_runtime::call_callable` refuses the
    // third, and `nvs_core_arr_filter`'s doc claims the early return itself
    // frees the partial result and the entry's key, since `NvsArray` and
    // `NvsStr` release on drop. A missed release leaks that array plus the two
    // string buffers it retained, every iteration.
    //
    // Measured as a *slope* rather than against a fixed bound: a run's own
    // fixed overhead is the same in both runs, so ten times the iterations
    // must not cost ten times anything. Both runs hold the same handful of
    // bytes — the buffer the echoed count lands in — and the slack below is
    // what tells that apart from a leak. Calibrated rather than guessed:
    // leaking the partial result with a `std::mem::forget` on
    // `nvs_core_arr_filter`'s own error path drives the pair orders of
    // magnitude apart, a slope far past that slack.
    let source = |iterations: u32| {
        format!(
            "{MISMATCH}var $i = 0;
var $caught = 0;
while ($i < {iterations}) {{
    string $tag = \"t\" . $i;
    array<mixed> $fresh = [$tag, $tag . \"!\", 3];
    try {{
        var $kept = Core\\Arr::filter($fresh, $forward);
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

/// Every runtime helper the whole lowered program calls, in no order.
///
/// The two tests below are about which of the two callable-call helpers a site
/// emits, and that is a property of the call site rather than of what the
/// program printed — so this reads the IR the rest of this file runs, and each
/// test asserts on the output *as well*, so a program that stopped working
/// cannot pass by emitting the right helper.
fn helpers_of(program: &nvs_ir::Program) -> Vec<nvs_ir::ir::Helper> {
    program
        .functions
        .iter()
        .flat_map(|function| &function.blocks)
        .flat_map(|block| &block.insts)
        .filter_map(|inst| match inst.kind {
            nvs_ir::ir::InstKind::HelperCall { helper, .. } => Some(helper),
            _ => None,
        })
        .collect()
}

#[test]
fn a_proven_callable_call_site_emits_no_param_tag_check() {
    // `rule:types/callable-signature`'s whole purpose: the callee's type names
    // its parameters, so `nvs_types` checked both arguments where they are
    // written and `nvs_runtime::callable::check_param_tags` has nothing left to
    // decide. `Helper::CallCallableProven` is the site that skips it — the
    // runtime reads the tags from `nvs_call_callable` alone.
    let source = "<?nvs
callable(int, string): string $f = fn(int $n, string $s): string => $s . $n;
echo $f(2, \"x\");
";
    let helpers = helpers_of(&lower(source));

    assert!(
        helpers.contains(&nvs_ir::ir::Helper::CallCallableProven),
        "a call through a written signature must reach the proven helper: {helpers:?}"
    );
    assert!(
        !helpers.contains(&nvs_ir::ir::Helper::CallCallable),
        "nothing may still pay the per-argument check here: {helpers:?}"
    );
    assert_eq!(output_of(source), "x2");
}

#[test]
fn a_callable_object_still_carries_both_metadata_slots() {
    // What a proven site removes is the work, not the metadata: a literal does
    // not know which kind of site will call it, and the *same* callable object
    // reaches both here. `nvs_runtime::CALLABLE_ARITY_SLOT` is what trims the
    // third argument at the dynamic site and `CALLABLE_PARAM_TAGS_SLOT` is what
    // refuses the `string` in the first position — so dropping either write
    // from `nvs_ir::lower::lower_anon_fn_expr` fails here while the proven
    // site above keeps printing.
    let source = "<?nvs
callable(int, string): string $join = fn (int $n, string $s): string => $s . $n;
echo $join(2, \"x\"), \"|\";
callable $bare = $join;
echo $bare(3, \"y\", \"extra\"), \"|\";
try {
    echo $bare(\"a\", \"b\");
} catch (LogicError $e) {
    echo $e->message;
}
";
    assert_eq!(
        output_of(source),
        "x2|y3|argument 1 to a `callable` must be of type int, string given"
    );
}

#[test]
fn a_bare_callable_call_site_still_emits_the_param_tag_check() {
    // The other half, and the reason the metadata stays on every callable
    // object: bare `callable` is the top of the lattice and names no
    // parameter, so nothing was proven and the tag check is the only thing
    // standing between a mismatched argument and the callee reading the
    // payload at the wrong representation.
    let source = "<?nvs
callable $f = fn(int $n, string $s): string => $s . $n;
echo $f(2, \"x\");
";
    let helpers = helpers_of(&lower(source));

    assert!(
        helpers.contains(&nvs_ir::ir::Helper::CallCallable),
        "a call through bare `callable` keeps the dynamic helper: {helpers:?}"
    );
    assert!(
        !helpers.contains(&nvs_ir::ir::Helper::CallCallableProven),
        "nothing proved these arguments: {helpers:?}"
    );
    assert_eq!(output_of(source), "x2");
}
