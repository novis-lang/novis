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

/// How many `E0741` errors `diags` has.
fn not_the_called_class(diags: &nvs_diagnostics::Diagnostics) -> usize {
    diags
        .iter()
        .filter(|d| d.code == Some(code::E_STATIC_RETURN_NOT_CALLED_CLASS))
        .count()
}

/// `Model` with the bodies `methods`, a `readonly ?Model $parent`, and one
/// subclass, so the class is not `final`.
fn model(methods: &str) -> String {
    format!(
        "<?nvs
class Model {{
    public readonly ?Model $parent;
    function constructor(?Model $p) {{ $this->parent = $p; }}
{methods}
}}
class Order extends Model {{}}
"
    )
}

/// On the true edge of `is static` and `is ?static`, a variable, or a
/// `readonly` path, may be returned as `static` and `?static`, also inside a
/// loop that does not write it.
#[test]
fn a_value_narrowed_by_is_static_is_the_called_class() {
    let src = model(
        "    public static function first(array<Model> $all): ?static {
        foreach ($all as Model $o) {
            if ($o is static) {
                return $o;
            }
        }
        return null;
    }
    public static function pick(?Model $o): ?static {
        if ($o is ?static) {
            return $o;
        }
        return null;
    }
    public static function only(Model $o, Model $fallback): static {
        return $o is static ? $o : static::only($fallback, $fallback);
    }
    public function parentAsStatic(): ?static {
        if ($this->parent is static) {
            return $this->parent;
        }
        return null;
    }
    public static function both(Model $a, Model $b): array<static> {
        if ($a is static && $b is static) {
            return [$a, $b];
        }
        return [];
    }",
    );
    let diags = check_src(&src);
    assert!(!diags.has_errors(), "{diags:?}");
}

/// A write to the variable drops the proof with the narrowing, and so does a
/// loop that writes it, at a read written before the write too.
#[test]
fn a_write_after_is_static_drops_the_proof() {
    let src = model(
        "    public static function replaced(Model $o, Model $other): ?static {
        if ($o is static) {
            $o = $other;
            return $o;
        }
        return null;
    }
    public static function lastOf(array<Model> $all, Model $o): ?static {
        if ($o is static) {
            foreach ($all as Model $x) {
                $o = $x;
            }
            return $o;
        }
        return null;
    }
    public static function inLoop(array<Model> $all, Model $o): ?static {
        if ($o is static) {
            foreach ($all as Model $x) {
                if ($x is Order) {
                    return $o;
                }
                $o = $x;
            }
        }
        return null;
    }",
    );
    let diags = check_src(&src);
    assert_eq!(not_the_called_class(&diags), 3, "{diags:?}");
}

/// Only the edge where the test holds proves anything: the `else` block and
/// the code after an `if` that does not leave are not proven.
#[test]
fn the_else_edge_of_is_static_proves_nothing() {
    let src = model(
        "    public static function other(Model $o): ?static {
        if ($o is static) {
            return null;
        } else {
            return $o;
        }
    }
    public static function after(Model $o): ?static {
        if ($o is static) {
            echo \"called class\";
        }
        return $o;
    }",
    );
    let diags = check_src(&src);
    assert_eq!(not_the_called_class(&diags), 2, "{diags:?}");
}

/// `if (!($o is static)) { continue; }` proves the rest of the block, as every
/// other guard clause does.
#[test]
fn a_negated_is_static_guard_clause_proves_the_rest() {
    let src = model(
        "    public static function first(array<Model> $all): ?static {
        foreach ($all as Model $o) {
            if (!($o is static)) {
                continue;
            }
            return $o;
        }
        return null;
    }
    public static function own(Model $o): ?static {
        if (!($o is static)) {
            return null;
        }
        return $o;
    }",
    );
    let diags = check_src(&src);
    assert!(!diags.has_errors(), "{diags:?}");
}
