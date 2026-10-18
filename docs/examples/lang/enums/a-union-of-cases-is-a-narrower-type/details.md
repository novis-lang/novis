When the parameter is `Mode::Read|Mode::Write`, `Open::file(Mode::Exec)` and `Open::file($m)` with
a `Mode` variable give the same error: `E0401`, ``expected `Mode::Read|Mode::Write`, found `Mode` ``. A method that declares the smaller type and
returns another case is the error `E0403`. An array literal of the type `array<Mode::Read|Mode::Write>`
with another case in it is `E0401`.

A type may name one case alone, such as `Mode::Read`. It may also name cases of two enums, or
name the same case twice.

`if ($m == Mode::Read)` narrows `$m` to the type `Mode::Read` inside the block. This works on a
variable of the whole enum and on a variable of a smaller type. Nothing else narrows to a smaller
type:

- `$m == Mode::Read || $m == Mode::Write` does not narrow `$m` to `Mode::Read|Mode::Write`.
- The `else` block of `$m == Mode::Read`, and the block of `$m != Mode::Read`, keep the whole type.
- An arm of `match ($m)` keeps the whole type.

Write `$m as Mode::Read|Mode::Write` for a set of two or more cases.

`$m is Mode::Read|Mode::Write` is `true` when `$m` is one of the two cases.

`as` also converts a number. `1 as Mode::Read|Mode::Write` gives `Mode::Write`. A number written in
the code that is not one of the cases is the error `E0470`. A number in a `mixed` variable is checked
while the program runs. When it is not one of the cases, the `RuntimeError` message names the cases
the type accepts.

`?Mode::Read|Mode::Write` and `Mode::Read|Mode::Write|null` both accept `null` as well.

A `type` alias may name a set of cases: `type Rw = Mode::Read|Mode::Write;`.

`$m == Mode::Exec` compiles when `$m` has the type `Mode::Read|Mode::Write`. The result is always
`false`.
