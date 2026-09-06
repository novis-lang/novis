//! Integer arithmetic whose edges diverge from a native instruction — `%`'s signs, a divisor that throws rather than traps, `rule:types/arithmetic`'s overflow throw, and the integer compare an enum pair reaches one representation down.
//!
//! Split out of the single `compile_and_run.rs`; every test keeps its own name
//! and body. See `tests/common/mod.rs` for the shared fixtures and for why
//! these go through the real pipeline.

mod common;

use common::*;

#[test]
fn integer_modulo_answers_php_s_value_for_every_sign() {
    // The sign rules are PHP's: the result takes the *dividend's* sign, which
    // is what `srem` already gives, so these are here to hold that rather than
    // to describe a conversion.
    assert_eq!(output_of("<?nvs\nint $q = 17 % 5;\necho $q;\n"), "2");
    assert_eq!(output_of("<?nvs\nint $q = -17 % 5;\necho $q;\n"), "-2");
    assert_eq!(output_of("<?nvs\nint $q = 17 % -5;\necho $q;\n"), "2");
    assert_eq!(
        output_of("<?nvs\nuint $a = 17;\nuint $b = 5;\necho $a % $b;\n"),
        "2"
    );
}

#[test]
fn a_modulo_by_minus_one_answers_zero_rather_than_trapping() {
    // `i64::MIN % -1` is the second input `srem` traps on, and a trap takes
    // the whole process down. It is *not* an overflow — `x % -1` is `0` for
    // every `x`, which is representable, and PHP 8 answers `0` — so
    // `Emitter::emit_int_mod` rewrites the divisor rather than throwing.
    // Reaching `i64::MIN` needs the negation, since the literal itself is out
    // of `int`'s range.
    assert_eq!(
        output_of("<?nvs\nint $a = -9223372036854775807 - 1;\nint $b = -1;\necho $a % $b;\n"),
        "0"
    );
}

#[test]
fn a_modulo_by_zero_throws_arithmetic_error_rather_than_trapping() {
    // The whole reason this operator waited for the throw path: `srem` on a
    // zero divisor traps, which is a request-isolation failure rather than a
    // wrong answer. The message is PHP's own, and the class is spec § 10's
    // `ArithmeticError` — raised by naming a descriptor from the inline code,
    // since there is no helper call here to carry a `Fault` out of.
    assert_eq!(
        output_of(
            "<?nvs
int $b = 0;
try {
  echo 10 % $b;
} catch (ArithmeticError $e) {
  echo \"caught: \" . $e->message;
}
"
        ),
        "caught: Modulo by zero"
    );
}

#[test]
fn an_integer_division_emits_both_of_its_representations() {
    // `rule:types/arithmetic` types `int / int` as `int|float`, PHP-exact, so which of the
    // two a given pair produces is a *runtime* question and
    // `Emitter::emit_int_div` answers it with a branch rather than with a type.
    // Both arms are asked, because either alone is satisfied by an
    // implementation that always tags its result the same way.
    assert_eq!(
        output_of(
            "<?nvs
var $q = 7 / 2;
echo $q;
"
        ),
        "3.5"
    );
    assert_eq!(
        output_of(
            "<?nvs
var $q = 6 / 3;
echo $q;
"
        ),
        "2"
    );
    assert_eq!(
        output_of(
            "<?nvs
uint $a = 7;
uint $b = 2;
var $q = $a / $b;
echo $q;
"
        ),
        "3.5"
    );
    // `i64::MIN / -1` is the signed overflow `sdiv` traps on, and it takes the
    // *float* arm rather than becoming a second throw: the quotient is not an
    // `int` at all, and PHP answers this value for it.
    assert_eq!(
        output_of(
            "<?nvs
int $a = -9223372036854775807 - 1;
int $b = -1;
var $q = $a / $b;
echo $q;
"
        ),
        "9.2233720368548E+18"
    );
}

#[test]
fn an_integer_addition_traps_on_overflow() {
    // `rule:types/arithmetic`'s overflow row, at the bound and one step inside it. The check is
    // the machine's own overflow flag (`Emitter::emit_checked_int_arith`), so
    // the not-taken side is a predicted branch and the taken one raises spec
    // § 10's `ArithmeticError` from a baked-in descriptor, exactly as the zero
    // divisor above does.
    assert_eq!(
        output_of(
            "<?nvs
int $a = 9223372036854775806;
int $b = 1;
echo $a + $b;
"
        ),
        "9223372036854775807"
    );
    assert_eq!(
        output_of(
            "<?nvs
int $a = 9223372036854775807;
int $b = 1;
try {
  echo $a + $b;
} catch (ArithmeticError $e) {
  echo \"caught: \" . $e->message;
}
"
        ),
        "caught: Integer addition overflowed"
    );
    // The unsigned bound is its own, and it is a carry out of bit 63 rather
    // than a sign flip: `uadd_overflow`, not `sadd_overflow` read differently.
    assert_eq!(
        output_of(
            "<?nvs
uint $a = 18446744073709551615;
uint $b = 1;
try {
  echo $a + $b;
} catch (ArithmeticError $e) {
  echo \"caught: \" . $e->message;
}
"
        ),
        "caught: Integer addition overflowed"
    );
}

#[test]
fn two_enum_values_compare_as_their_backing_integer() {
    // `Emitter::emit_binop`'s `integral` set is `Int | Uint | Bool` and there
    // is no `Ty::Enum` row anywhere in its table, so `rule:expressions/disjoint-comparison-refused`'s "an enum
    // is its own equality domain" is answered one representation down: the
    // lowering relabels each operand with the free `InstKind::Reinterpret`
    // `rule:types/conversion` already spends on `$m as int`, and what arrives here is an
    // ordinary integer compare. This guards the whole path rather than the
    // relabel, since a missing arm shows up as `output_of` failing to compile
    // at all rather than as a wrong answer.
    assert_eq!(
        output_of(
            "<?nvs
enum Rank { Bronze, Silver, Gold }
Rank $a = Rank::Silver;
Rank $b = Rank::Silver;
Rank $c = Rank::Gold;
echo ($a == $b) as string, \"|\", ($a == $c) as string, \"|\", ($a != $c) as string;
"
        ),
        "1||1"
    );
    // The zero-backed case on both sides: `rule:enums/truthiness` keeps an enum out of
    // the truthy table entirely, so nothing here may read `0` as "unset".
    assert_eq!(
        output_of(
            "<?nvs
enum Signal { Stop = -1, Idle = 0, Go = 10 }
Signal $s = Signal::Idle;
echo ($s == Signal::Idle) as string, \"|\", ($s == Signal::Stop) as string, \"|\";
echo ($s != Signal::Go) as string;
"
        ),
        "1||1"
    );
    // `rule:enums/one-backing-type`'s second backing type, at a value with no `int`: the other
    // `EnumRepr` arm, and the one that would silently fall through to a
    // `Ty::Enum` compare if only the signed row were relabelled.
    assert_eq!(
        output_of(
            "<?nvs
enum Mask: uint { None = 0, All = 18446744073709551615 }
Mask $m = Mask::All;
echo ($m == Mask::All) as string, \"|\", ($m == Mask::None) as string, \"|\";
echo ($m != Mask::None) as string;
"
        ),
        "1||1"
    );
}

#[test]
fn an_integer_subtraction_and_multiplication_trap_on_overflow() {
    // The other two binary rows and the unary one, each named at the value
    // that has no answer. `-i64::MIN` is the whole reason unary `-` joined the
    // checked set: it is the one `int` whose negation is not an `int`.
    assert_eq!(
        output_of(
            "<?nvs
int $a = -9223372036854775807 - 1;
int $b = 1;
try {
  echo $a - $b;
} catch (ArithmeticError $e) {
  echo \"caught: \" . $e->message;
}
"
        ),
        "caught: Integer subtraction overflowed"
    );
    assert_eq!(
        output_of(
            "<?nvs
int $a = 4611686018427387904;
int $b = 2;
try {
  echo $a * $b;
} catch (ArithmeticError $e) {
  echo \"caught: \" . $e->message;
}
"
        ),
        "caught: Integer multiplication overflowed"
    );
    assert_eq!(
        output_of(
            "<?nvs
int $a = -9223372036854775807 - 1;
try {
  echo -$a;
} catch (ArithmeticError $e) {
  echo \"caught: \" . $e->message;
}
"
        ),
        "caught: Integer negation overflowed"
    );
    // A `uint` has no negative side at all, so every non-zero one negates to
    // nothing while `0` still answers itself.
    assert_eq!(
        output_of(
            "<?nvs
uint $a = 0;
echo -$a;
"
        ),
        "0"
    );
    assert_eq!(
        output_of(
            "<?nvs
uint $a = 1;
try {
  echo -$a;
} catch (ArithmeticError $e) {
  echo \"caught: \" . $e->message;
}
"
        ),
        "caught: Integer negation overflowed"
    );
    // One step inside each bound still answers, so "refuse everything" is not
    // a way to pass the three assertions above.
    assert_eq!(
        output_of(
            "<?nvs
int $a = -9223372036854775807;
int $b = 1;
echo $a - $b;
"
        ),
        "-9223372036854775808"
    );
    assert_eq!(
        output_of(
            "<?nvs
int $a = 4611686018427387903;
int $b = 2;
echo $a * $b;
"
        ),
        "9223372036854775806"
    );
}

/// `rule:classes/ordering-lowers-to-compare-to`'s `<=>` over a *scalar*, which until now had only an object
/// row: `-1`, `0` or `1` as an `int`, never the operands' own type, over every
/// representation the relational operators already order.
///
/// Swept rather than spot-checked, because what is being pinned is that the
/// three routes into this operator — the inline `BinOp::Cmp` for a matched
/// pair, `Helper::NumericCmp` for a mixed numeric one and `Helper::DecimalCmp`
/// for a `decimal` — **agree**, rather than what any one of them answered.
///
/// The `NaN` row is the one that had a choice to make, and it is why the
/// emission is a three-way "less, else equal, else 1" and not the tidier
/// `(a > b) - (a < b)`: PHP answers `1` for an unordered pair, and the
/// arithmetic formula would answer `0` — that is, "equal" — for two values
/// that are not.
#[test]
fn a_spaceship_answers_minus_one_zero_or_one_for_a_scalar() {
    for (declared, less, more) in [
        ("int", "2", "3"),
        ("uint", "7", "9"),
        ("float", "1.5", "2.5"),
        ("decimal", "1.25", "2.50"),
    ] {
        let source = format!(
            "<?nvs\n{declared} $a = {less};\n{declared} $b = {more};\n\
             echo $a <=> $b, $b <=> $a, $a <=> $a;\n"
        );
        assert_eq!(output_of(&source), "-110", "{declared}");
    }
    // A `bool` orders `false < true`, so `true <=> false` is `1` — PHP's own
    // answer, and the row that reaches `emit_binop`'s unsigned comparison.
    assert_eq!(
        output_of(
            "<?nvs\nbool $t = true;\nbool $f = false;\necho $t <=> $f, $f <=> $t, $t <=> $t;\n"
        ),
        "1-10"
    );
    // A mixed numeric pair does not widen: `rule:types/conversion`'s implicit `int` into
    // `float` throws above 2^53, and a pair that far apart still orders.
    assert_eq!(output_of("<?nvs\necho 1 <=> 1.5, 2.5 <=> 2;\n"), "-11");
    // Unordered, in both directions and against itself. The `NaN` comes from
    // the constant and not from `0.0 / 0.0`, which `rule:types/arithmetic` now makes a
    // throw — the zero divisor is refused before the operand types are
    // consulted, so there is no float division left that answers one.
    assert_eq!(
        output_of(
            "<?nvs\nfloat $n = Core\\Math::NAN;\nfloat $x = 1.5;\n\
             echo $n <=> $x, $x <=> $n, $n <=> $n;\n"
        ),
        "111"
    );
}
