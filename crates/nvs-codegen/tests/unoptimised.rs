//! `nvs_codegen::set_opt_level`: the switch the differential check of
//! `rule:testing/probes-on-is-a-tested-configuration` runs the conformance
//! suite under.
//!
//! A binary of its own because the switch is process-wide: in any other test
//! binary it would move every test after it to the other level.

mod common;

use common::*;
use nvs_codegen::OptLevel;

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
    assert_eq!(nvs_codegen::opt_level(), OptLevel::Speed);
    let program = lower(FIXTURE);
    let optimised = nvs_codegen::disassemble(&program).expect("the fixture compiles");
    let printed = output_of(FIXTURE);

    nvs_codegen::set_opt_level(OptLevel::None);
    assert_eq!(nvs_codegen::opt_level(), OptLevel::None);
    let unoptimised = nvs_codegen::disassemble(&program).expect("the fixture compiles");

    assert_ne!(
        optimised, unoptimised,
        "`opt_level` `none` emits different code"
    );
    assert_eq!(output_of(FIXTURE), printed);
    assert_eq!(printed, "63");
}

/// Each level's name reads back as that level, and no other text does.
#[test]
fn a_level_is_named_by_its_name() {
    for level in [OptLevel::None, OptLevel::Speed] {
        assert_eq!(OptLevel::named(level.name()), Some(level));
    }
    assert_eq!(OptLevel::named("size"), None);
}
