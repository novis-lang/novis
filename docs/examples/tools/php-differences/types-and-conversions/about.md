Every parameter, property and variable in Novis declares a type, and a conversion is written with
`as`.

`function f($x)` and `public $x;` do not compile. Write `function f(int $x)` and
`public int $x = 0;`. A cast such as `(int)$s` is written `$s as int`. It throws an error when the
conversion would lose part of the value. `$s as ?int` returns `null` in that case.

Values of two different types do not compare. `1 == "1"` does not compile, so convert one side
first. A string is converted before arithmetic, as in `($s as int) * 2`. Only an anonymous
function or a method reference is callable, so a function name in a string such as `"strlen"` is
not. The `resource` type does not
exist, and `iterable` is written `array<T>` or `Iterable<T>`.

**Good to know:** the default value of a property is a value written directly in the code, such
as a number, a string, `null` or `[]`. It can also be an enum case or a constant. Compute any other
value in the constructor.
