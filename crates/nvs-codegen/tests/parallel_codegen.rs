//! Code generation on many threads: the bytes are the ones one thread writes,
//! the unit runs the same, and the threads run below normal priority.
//!
//! See `tests/common/mod.rs` for the shared fixtures, and `src/pool.rs` for
//! the pool these tests reach.

mod common;

use common::*;

/// The threads every many-thread compile below uses. It is more than one on
/// every machine, because [`nvs_codegen::with_compile_threads`] fixes it.
const MANY: usize = 8;

/// A program with many functions, each with its own string literal, each
/// calling the one before it, so the order of definition and the literals the
/// emit writes both reach the bytes.
fn many_functions() -> String {
    let mut source = String::from("<?nvs\nclass Chain {\n");
    source.push_str("    public static function f0(int $n): int {\n        return $n;\n    }\n");
    for i in 1..40 {
        let previous = i - 1;
        source.push_str(&format!(
            "    public static function f{i}(int $n): int {{\n        echo \"step {i}\\n\";\n        return Chain::f{previous}($n) + {i};\n    }}\n"
        ));
    }
    source.push_str("}\necho Chain::f39(1);\n");
    source
}

#[test]
fn many_threads_write_the_object_one_thread_writes() {
    let program = lower(&many_functions());
    let one = nvs_codegen::with_compile_threads(1, || nvs_codegen::compile_object(&program))
        .expect("the fixture compiles on one thread");
    for _ in 0..3 {
        let many =
            nvs_codegen::with_compile_threads(MANY, || nvs_codegen::compile_object(&program))
                .expect("the fixture compiles on many threads");
        assert!(
            one == many,
            "the objects differ: {} and {} bytes",
            one.len(),
            many.len()
        );
    }
}

#[test]
fn a_unit_compiled_on_many_threads_runs_as_one_compiled_on_one() {
    let program = lower(&many_functions());
    let one = nvs_codegen::with_compile_threads(1, || output_of_program(&program));
    let many = nvs_codegen::with_compile_threads(MANY, || output_of_program(&program));
    assert_eq!(one, many);
    assert!(one.ends_with("step 1\n781"), "{one}");

    // The disassembly names every function in the same order, with the same
    // instructions.
    let text_one = nvs_codegen::with_compile_threads(1, || nvs_codegen::disassemble(&program))
        .expect("the fixture compiles on one thread");
    let text_many = nvs_codegen::with_compile_threads(MANY, || nvs_codegen::disassemble(&program))
        .expect("the fixture compiles on many threads");
    assert_eq!(text_one, text_many);
}

#[test]
fn compile_threads_run_below_normal_priority() {
    let caller = nvs_codegen::current_thread_priority();
    let threads = nvs_codegen::compile_thread_priorities(4);
    assert_eq!(threads.len(), 4);
    for priority in &threads {
        assert!(
            *priority < caller,
            "a compile thread at {priority}, the caller at {caller}"
        );
    }
    // The thread that started the compile keeps its own priority.
    assert_eq!(nvs_codegen::current_thread_priority(), caller);
}
