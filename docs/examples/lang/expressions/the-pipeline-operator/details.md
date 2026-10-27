The compiler replaces `$_` with the value on the left. So a pipeline has the same type, the same
errors and the same result as the nested calls it replaces. A `tainted string` stays tainted, and a
`?string` value on the left is the error `E0401` at a step that needs a `string`. The value on the
left runs once.

A step is a call, an array index, a property, a method call or an expression in brackets.
`$xs |> $_[1]`, `$p |> $_->name`, `4 |> $p->add($_)`, `2 |> self::f($_)` and `3 |> new Point($_)`
all work. For any other expression, write brackets around it: `$n |> ($_ * 2)`.

The step ends where the call, index or brackets end. In `$n |> $_ * 2`, the step is only `$_`, and
then the result is multiplied by 2. The result is the same here, but `$n |> 10 - $_` does not
compile: the step is `10`, which has no `$_`.

`$_` may be deep inside the step. `$s |> Core\Str::lower(Core\Str::trim($_))` works. So does a
second pipeline in an argument, or an anonymous function in an argument that uses `$_`.

These are the three errors:

- A step without `$_` is the error `E0129`. A method reference such as `Core\Str::upper` and an
  anonymous function are steps without `$_` too.
- A step with `$_` twice is the error `E0130`. Store the value in a variable first.
- `$_` anywhere else is the error `E0131`. This includes `$_` as a parameter name, and the second
  `$_` in `$n |> $_ * $_`.

`|>` runs after a prefix `-` and before every operator with two sides. `"a" . "b" |> Core\Str::upper($_)`
prints `aB`, and `$s ?? "d" |> Core\Str::upper($_)` is `$s ?? Core\Str::upper("d")`. `!` runs after
`|>`, so `!$a |> f($_)` is `!f($a)`.
