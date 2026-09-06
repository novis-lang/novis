`var $name = expr;` declares a local without writing its type. The type stored on the binding is
`expr`'s own checked type, computed exactly as it is for any position with no expected type, and it is
then fixed on the binding like a written one — `rule:types/declaration`'s "no binding's type ever
changes" is untouched.

The initializer is mandatory: `var $n;` is a parse error naming `=`. The inferred type is exact and
honest — `var $n = 1;` gives `int`, and `var $id = Core\Request::query('id');` gives `mixed`, because
that is what the initializer is.

One initializer shape is refused: a **bare array literal**. `var $x = [1, 2];` is
`E_VAR_ARRAY_LITERAL_NEEDS_TYPE`, naming `array<T> $x = [1, 2];` as the fix, because an array literal
is checked against a target rather than inferring one (`rule:types/arrays`). The restriction is on
the initializer's own top level only — `var $x = f([1, 2]);` is fine, since `f`'s parameter is the
literal's target.

Every other rule that applies to a typed local declaration — declare-once, definite assignment,
redeclaration diagnostics — applies unchanged, because by the time those checks run `var` has already
resolved to a concrete type.
