//! A compiled unit's code pages are unmapped once nothing can reach them.
//!
//! [`nvs_codegen::mapped_jit_units`] counts for the whole process, so this
//! file is its own binary with one test: no other test's compile moves the
//! count while it runs.

mod common;

use common::*;

const SOURCE: &str = "<?nvs\nclass Box {\n    public function size(): int {\n        return 3;\n    }\n}\necho (new Box())->size();\n";

#[test]
fn a_recompile_loop_holds_the_mapped_code_steady_and_an_error_class_keeps_its_unit_mapped() {
    let before = nvs_codegen::mapped_jit_units();
    for _ in 0..50 {
        let unit = compile(SOURCE).expect("the fixture compiles");
        assert_eq!(nvs_codegen::mapped_jit_units(), before + 1);
        drop(unit);
        assert_eq!(nvs_codegen::mapped_jit_units(), before);
    }

    // A context keeps the unit's `RuntimeError` after the unit is gone, and
    // that class's methods are code on the same pages, so they stay mapped
    // until the class is dropped too.
    let unit = compile(SOURCE).expect("the fixture compiles");
    let class = unit
        .runtime_error_class()
        .expect("every checked program declares `RuntimeError`");
    drop(unit);
    assert_eq!(nvs_codegen::mapped_jit_units(), before + 1);
    drop(class);
    assert_eq!(nvs_codegen::mapped_jit_units(), before);
}
