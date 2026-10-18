//! `nvs_codegen::compile_unoptimised`: the switch the differential check of
//! `rule:testing/probes-on-is-a-tested-configuration` runs the conformance
//! suite under.
//!
//! A binary of its own because the switch is process-wide and one-way: in any
//! other test binary it would move every test after it to the other level.

mod common;

use common::*;

/// A loop, a call and a branch, so the optimiser has something to change.
const FIXTURE: &str = "<?nvs
class M {
  public static function twice(int $n): int { return $n * 2; }
}
int $sum = 0;
for (int $i = 0; $i < 10; $i++) {
  if ($i % 3 == 0) { $sum = $sum + M::twice($i); } else { $sum = $sum + $i; }
}
echo $sum;
";

/// The switch changes the machine code, and the program prints the same.
#[test]
fn the_switch_changes_the_code_and_not_the_output() {
    assert!(!nvs_codegen::is_unoptimised());
    let program = lower(FIXTURE);
    let optimised = nvs_codegen::disassemble(&program).expect("the fixture compiles");
    let printed = output_of(FIXTURE);

    nvs_codegen::compile_unoptimised();
    assert!(nvs_codegen::is_unoptimised());
    let unoptimised = nvs_codegen::disassemble(&program).expect("the fixture compiles");

    assert_ne!(
        optimised, unoptimised,
        "`opt_level` `none` emits different code"
    );
    assert_eq!(output_of(FIXTURE), printed);
    assert_eq!(printed, "63");
}
