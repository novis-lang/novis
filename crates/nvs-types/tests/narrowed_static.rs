//! `rule:statements/static-is-a-member-modifier`'s `$o is static`: a test
//! against the class the call names, which only the running frame knows.
//!
//! In a class that is not `final`, the checker records
//! `ExprInfo::CalledClassTest` and never folds the test to `true`, because a
//! subject of the declaring class may be a sibling of the called class. In a
//! `final` class `static` is the class itself, so the ordinary fold stays.
//! Inside an anonymous function no frame holds the called class, so the test
//! is `E0834`. `tests/conformance/lang/is-static-tests-the-class-the-call-names.nvst`
//! pins the run-time answer.

mod common;

use common::{check_src, check_src_declared};
use nvs_diagnostics::code;
use nvs_types::expr_table::ExprInfo;

/// A base class with a static and an instance method that test `$o` against
/// `static`, and one subclass, so the class is not `final`.
const MODEL: &str = "<?nvs
class Model {
    public static function owns(Model $o): bool {
        return $o is static;
    }
    public function sameAs(?Model $o): bool {
        return $o is ?static;
    }
}
class Order extends Model {}
";

/// Both spellings record a test against the called class. `Model $o` is
/// `static` exactly when the call names `Model`, so neither may fold to
/// `true`.
#[test]
fn is_static_is_not_folded_in_a_class_that_is_not_final() {
    let (diags, declared) = check_src_declared(MODEL);
    assert!(!diags.has_errors(), "{diags:?}");
    assert!(
        matches!(
            declared.folded_at("$o is static"),
            Some(ExprInfo::CalledClassTest {
                nullable: false,
                ..
            })
        ),
        "{:?}",
        declared.folded_at("$o is static")
    );
    assert!(
        matches!(
            declared.folded_at("$o is ?static"),
            Some(ExprInfo::CalledClassTest { nullable: true, .. })
        ),
        "{:?}",
        declared.folded_at("$o is ?static")
    );
}

/// An anonymous function runs in a frame of its own, which has no called
/// class, so `is static` there is `E0834` as `static::` is.
#[test]
fn is_static_inside_an_anonymous_function_does_not_compile() {
    let src = "<?nvs
class Model {
    public function check(Model $o): bool {
        callable $f = fn(): bool => $o is static;
        return $f();
    }
}
";
    let diags = check_src(src);
    assert!(
        diags
            .iter()
            .any(|d| d.code == Some(code::E_STATIC_IN_ANON_FN)),
        "{diags:?}"
    );
}

/// In a `final` class no subclass can be the called class, so `static` is the
/// class itself, and `Order $o is static` folds to `true` as `is Order` does.
#[test]
fn is_static_in_a_final_class_still_folds() {
    let src = "<?nvs
final class Order {
    public static function owns(Order $o): bool {
        return $o is static;
    }
}
";
    let (diags, declared) = check_src_declared(src);
    assert!(!diags.has_errors(), "{diags:?}");
    assert!(
        matches!(
            declared.folded_at("$o is static"),
            Some(ExprInfo::SettledTypeTest { answer: true })
        ),
        "{:?}",
        declared.folded_at("$o is static")
    );
}
