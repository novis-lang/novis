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

A **bare array literal** is `array<T>` when every element has the same type `T`, and is refused
otherwise. An element's type is what `var` gives that element alone, so `var $ids = [1, 2];` is
`array<int>`, `["a" => 1]` is `array<int>` because keys take no part, a nested literal is inferred the
same way first, and a spread gives its source's element type. Nothing is widened to a common type:
`[1, 2.5]`, `[$user, $admin]` with `Admin` a child of `User`, and `[$tainted, "plain"]` are each two
types. Those, an empty literal at any depth, and a literal with an element that could not be typed
are `E_VAR_ARRAY_LITERAL_NEEDS_TYPE`, whose help names the declaration to write: the union of the
element types, `?T` where the other one is `null`, or `array<T>` for an empty literal. A bare literal
as the subject of a `var` value binding follows the same rule, and the help there is a typed local,
iterated. The rule is on the expression's own top level only — in `var $x = f([1, 2]);` and
`foreach (f([1, 2]) as var $x)`, `f`'s parameter is the literal's target.

Every other rule that applies to a typed binding — declare-once, definite assignment, redeclaration
diagnostics, the cursor's missing key — applies unchanged, because by the time those checks run `var`
has already resolved to a concrete type. A later write that does not fit that type is the mismatch a
written type gets, with one addition, because the type it misses is not on the line: a label on the
`var` line, and a help naming the declaration that would accept the value — `var $prices = [10, 20];`
then `$prices[] = 12.5;` names `array<int|float> $prices = [10, 20];`, the element type widened at
the depth the write went through.
