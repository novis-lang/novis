//! The `nvs-stdlib` helper boundary — a registered member reached from compiled code, borrowing its argument.
//!
//! See `tests/common/mod.rs` for the shared fixtures and for why these go
//! through the real pipeline.

mod common;

use common::*;

#[test]
fn a_core_member_call_reaches_its_native_implementation() {
    // The whole Tier 0 path end to end: `nvs-stdlib`'s registry resolves the
    // signature, `nvs-ir` lowers a `core.call`, and this crate emits it
    // through the same helper shape every runtime helper uses, against the
    // symbol the JIT resolved from `nvs_stdlib::symbols`.
    assert_eq!(
        output_of("<?nvs\narray<int> $a = [1, 2, 3];\necho Core\\Arr::count($a);\n"),
        "3"
    );
    assert_eq!(
        output_of("<?nvs\narray<int> $a = [];\necho Core\\Arr::count($a);\n"),
        "0"
    );
}

#[test]
fn a_core_member_borrows_its_argument_rather_than_consuming_it() {
    // The helper convention `nvs_ir::ir::InstKind::CoreCall` states: no retain
    // goes in before the call and the callee releases nothing, so the local
    // still owns its one reference afterwards. A spurious release here would
    // free the array out from under the read that follows.
    let source = "<?nvs
array<int> $a = [7, 8];
var $n = Core\\Arr::count($a);
var $m = Core\\Arr::count($a);
echo $n . \"/\" . $m . \"/\" . $a[\"1\"];
";
    assert_eq!(output_of(source), "2/2/8");
}

#[test]
fn a_written_duration_is_a_constant_and_reads_back_its_count() {
    // `rule:types/duration`: the unit carries the finished instance, so its
    // class word and its one slot have to be the ones `Duration::nanoseconds`
    // builds. Every member below reads one or the other.
    let source = "<?nvs
class Job {
    public static function wait(): Core\\Time\\Duration {
        return 1h30m;
    }
}
Core\\Time\\Duration $d = Job::wait();
echo $d->toSeconds(), \" \", $d->plus(30s)->toString(), \" \";
echo $d->compareTo(Core\\Time\\Duration::minutes(90)), \" \";
echo ($d is Core\\Time\\Duration) ? \"duration\" : \"other\", \" \";
echo (Job::wait() == Job::wait()) ? \"same\" : \"distinct\", \" \", (90m == 1h30m) ? \"same\" : \"distinct\";
";
    assert_eq!(output_of(source), "5400 1h30m30s 0 duration same distinct");

    let program = lower(source);
    let kinds = || {
        program
            .functions
            .iter()
            .flat_map(|function| function.blocks.iter())
            .flat_map(|block| block.insts.iter())
            .map(|inst| &inst.kind)
    };
    let constants = kinds()
        .filter(|kind| matches!(kind, nvs_ir::ir::InstKind::ConstDuration(_)))
        .count();
    assert_eq!(constants, 4, "one per written duration");

    // A run allocates nothing for a written duration held in a local.
    let unit = compile("<?nvs\nCore\\Time\\Duration $d = 30s;\n").expect("the fixture compiles");
    let entry = unit.script().expect("the script frame was compiled");
    let mut ctx = Ctx::buffered();
    entry.call(&mut ctx).expect("the script ran to completion");
    let before = nvs_runtime::budget::allocations();
    entry.call(&mut ctx).expect("the script ran to completion");
    assert_eq!(nvs_runtime::budget::allocations() - before, 0);
}
