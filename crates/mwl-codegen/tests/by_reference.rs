//! `inout` parameters — writing back into a local, a property, and through a loop.
//!
//! Split out of the single `compile_and_run.rs`; every test keeps its own name
//! and body. See `tests/common/mod.rs` for the shared fixtures and for why
//! these go through the real pipeline.

mod common;

use common::*;

/// An `inout` parameter, end to end: the callee's write lands in the caller's
/// own local. `mwl_ir::Ty::Ref` owns the representation this proves —
/// a caller-staged one-cell slot, passed by address, copied back after the
/// call — and this is the fixture `examples/hooks.mwl`'s last line is.
#[test]
fn a_by_reference_parameter_writes_back_into_the_callers_local() {
    let source = "<?mwl
class Adder {
    public static function bump(inout int $slot): void {
        $slot = $slot + 5;
    }
}
int $n = 1;
Adder::bump(inout $n);
echo \"n=\", $n;
";
    assert_eq!(output_of(source), "n=6");
}

/// The callee reads through the reference as well as writing through it, and
/// several writes in one body compose — each one is a store into the same
/// staged cell, so only the last is what the caller sees.
#[test]
fn a_by_reference_parameter_reads_and_writes_the_same_cell() {
    let source = "<?mwl
class Steps {
    public static function walk(inout int $slot): void {
        $slot = $slot * 2;
        $slot = $slot + $slot;
    }
}
int $n = 3;
Steps::walk(inout $n);
echo $n;
";
    assert_eq!(output_of(source), "12");
}

/// A callee that never writes leaves the caller's value alone — the staged
/// cell is copied back either way, so the no-write path has to be a genuine
/// round trip rather than an accident of the write happening to land.
#[test]
fn a_by_reference_parameter_a_callee_never_writes_round_trips_unchanged() {
    let source = "<?mwl
class Peek {
    public static function look(inout int $slot): int {
        return $slot + 1;
    }
}
int $n = 7;
var $seen = Peek::look(inout $n);
echo $seen, \"/\", $n;
";
    assert_eq!(output_of(source), "8/7");
}

/// A refcounted pointee — the case `mwl_ir::Ty::Ref`'s refcounting section
/// exists for. The staging retain, the callee's release-old-store-new, and
/// the copy-back's release of the holder's previous value have to balance, or
/// this either double-frees or leaks a string.
#[test]
fn a_by_reference_string_parameter_replaces_the_callers_string() {
    let source = "<?mwl
class Shout {
    public static function upper(inout string $s): void {
        $s = $s . \"!\";
    }
}
string $msg = \"hi\";
Shout::upper(inout $msg);
Shout::upper(inout $msg);
echo $msg;
";
    assert_eq!(output_of(source), "hi!!");
}

/// A property passed by reference: the receiver is evaluated once at staging
/// time and the copy-back re-uses that same value, writing through
/// `InstKind::FieldSet` rather than re-lowering the receiver expression.
#[test]
fn a_by_reference_argument_writes_back_through_a_property() {
    let source = "<?mwl
class Bump {
    public static function up(inout int $slot): void { $slot = $slot + 4; }
}
class Counter {
    public int $hits;
    public function constructor(int $hits) { $this->hits = $hits; }
}
var $c = new Counter(2);
Bump::up(inout $c->hits);
echo $c->hits;
";
    assert_eq!(output_of(source), "6");
}

/// An instance method's `inout` parameter works exactly like a static one's:
/// the staging is per-argument and knows nothing about the receiver slot.
#[test]
fn a_by_reference_parameter_on_an_instance_method_writes_back_too() {
    let source = "<?mwl
class Adder {
    public int $step;
    public function constructor(int $step) { $this->step = $step; }
    public function apply(inout int $slot): void { $slot = $slot + $this->step; }
}
var $a = new Adder(3);
int $n = 10;
$a->apply(inout $n);
$a->apply(inout $n);
echo $n;
";
    assert_eq!(output_of(source), "16");
}

/// A by-reference call inside a loop restages into the same frame-scoped
/// stack slot every iteration, so the write-back has to be observed on each
/// one rather than only on the last.
#[test]
fn a_by_reference_call_in_a_loop_writes_back_every_iteration() {
    let source = "<?mwl
class Adder {
    public static function bump(inout int $slot): void { $slot = $slot + 1; }
}
int $n = 0;
int $i = 0;
while ($i < 5) {
    Adder::bump(inout $n);
    $i = $i + 1;
}
echo $n;
";
    assert_eq!(output_of(source), "5");
}
