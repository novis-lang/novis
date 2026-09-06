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
