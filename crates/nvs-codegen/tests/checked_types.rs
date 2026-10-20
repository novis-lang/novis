//! `nvs_codegen::compile_checked_types`: the checked build, which tests each
//! typed read and stops at a value that is not its declared type.
//!
//! A binary of its own because the switch is process-wide and one-way: in any
//! other test binary it would add the tests to every unit lowered after it.
//!
//! No program the checker accepts reads a value as another type once this
//! goal's holes are closed, so the confused read here is made by hand: the
//! lowered `new Animal()` is rewritten to `new Rock()`, which is the shape of
//! a miscompile the checked build exists to find.

mod common;

use common::*;
use nvs_ir::ir::{Helper, InstKind};

const FIXTURE: &str = "<?nvs
class Animal { public function name(): string { return \"animal\"; } }
class Rock { public function name(): string { return \"rock\"; } }
class K {
  public static function make(): Animal { return new Animal(); }
}
Animal $a = K::make();
echo $a->name();
";

/// How many instructions `program` has, and how many of them are the checked
/// build's stop.
fn counts(program: &nvs_ir::Program) -> (usize, usize) {
    let insts = || {
        program
            .functions
            .iter()
            .flat_map(|function| &function.blocks)
            .flat_map(|block| &block.insts)
    };
    let stops = insts()
        .filter(|inst| {
            matches!(
                inst.kind,
                InstKind::HelperCall {
                    helper: Helper::TypeCheckFailed,
                    ..
                }
            )
        })
        .count();
    (insts().count(), stops)
}

/// Runs `program`'s script frame and returns its status and the `FATAL`
/// message it left, if any.
fn run(program: &nvs_ir::Program) -> (Result<String, i32>, Option<String>) {
    let unit = nvs_codegen::compile(program).expect("the fixture compiles");
    let mut ctx = Ctx::buffered();
    unit.install_in(&mut ctx);
    let entry = unit.script().expect("the script frame was compiled");
    let status = entry.call(&mut ctx).map(|_| {
        String::from_utf8(ctx.take_buffered_output().expect("a buffered context"))
            .expect("the script echoed UTF-8")
    });
    (status, ctx.take_pending().map(std::borrow::Cow::into_owned))
}

/// With the switch off a unit has no test, and with it on a correct program
/// prints the same. A read whose value is another class stops with a `FATAL`
/// that names the line, the declared type and the class the value is.
#[test]
fn the_switch_stops_a_confused_read_and_adds_nothing_when_off() {
    assert!(!nvs_codegen::is_checking_types());
    let unchecked = lower(FIXTURE);
    let (plain, plain_stops) = counts(&unchecked);
    assert_eq!(plain_stops, 0, "an unchecked unit has no stop");
    assert_eq!(run(&unchecked).0, Ok("animal".to_owned()));

    nvs_codegen::compile_checked_types();
    assert!(nvs_codegen::is_checking_types());
    let mut checked = lower(FIXTURE);
    let (total, stops) = counts(&checked);
    assert!(
        stops > 0 && total > plain,
        "the checked unit tests its reads"
    );
    assert_eq!(run(&checked).0, Ok("animal".to_owned()));

    let make = checked
        .functions
        .iter_mut()
        .find(|function| function.name == "K::make")
        .expect("`K::make` was lowered");
    let mut rewritten = 0;
    for inst in make.blocks.iter_mut().flat_map(|block| &mut block.insts) {
        if let InstKind::New { class, .. } = &mut inst.kind
            && class == "Animal"
        {
            "Rock".clone_into(class);
            rewritten += 1;
        }
    }
    assert_eq!(rewritten, 1, "`K::make` constructs one `Animal`");

    let (status, fatal) = run(&checked);
    assert_eq!(status, Err(FATAL));
    let fatal = fatal.expect("the stop leaves a `FATAL` message");
    assert!(
        fatal.contains("test.nvs:7")
            && fatal.contains("declared `Animal`")
            && fatal.contains("an object of class `Rock`"),
        "the message names the line and both types: {fatal}"
    );
}
