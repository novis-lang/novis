`var` stands in for a written type in two places — a local declaration and a `foreach` binding — and
the binding takes its type from the expression it is filled from, then keeps it for good. In
`var $name = expr;` that expression is `expr`. In `foreach ($subject as var $k => var $v)` it is the
subject: `$v` takes the subject's element type, and `$k` takes `string`, the one type an array's key
binding may have (`rule:types/arrays`).

The type is computed exactly as it is for any position with no expected type, and it is then fixed on
the binding like a written one — `rule:types/declaration`'s "no binding's type ever changes" is
untouched. It is exact and honest: `var $n = 1;` gives `int`, `var $id = Core\Request::query('id');`
gives `mixed`, and a `var` value binding over a `mixed` or `iterable` subject gives `mixed`, because
that is what the source expression is. A qualifier on the element type, such as `tainted`, arrives on
the binding with it. `inout var $v` binds at the element type itself, so its two sides declare the same
type by construction.

A local's initializer is mandatory: `var $n;` is a parse error naming `=`. A `foreach` binding with
neither a type nor `var` is the parse error it always was.

One source shape is refused: a **bare array literal**. `var $x = [1, 2];` is
`E_VAR_ARRAY_LITERAL_NEEDS_TYPE`, naming `array<T> $x = [1, 2];` as the fix, because an array literal
is checked against a target rather than inferring one (`rule:types/arrays`). A bare literal as the
subject of a `var` value binding is the same code, and the fix is the same typed local, iterated. The
restriction is on the expression's own top level only — `var $x = f([1, 2]);` and
`foreach (f([1, 2]) as var $x)` are fine, since `f`'s parameter is the literal's target.

Every other rule that applies to a typed binding — declare-once, definite assignment, redeclaration
diagnostics, the cursor's missing key — applies unchanged, because by the time those checks run `var`
has already resolved to a concrete type.
