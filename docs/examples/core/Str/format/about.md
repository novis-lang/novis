Fills the placeholders in a template with values, and returns the text.

`Core\Str::format` takes a template and a list of values. Each placeholder in the template starts
with `%` and is replaced by one value. `%s` writes a string, `%d` a whole number and `%f` a number
with decimals. `%x` writes a number in hexadecimal, and `%%` writes one `%`.

A placeholder can also set a width, such as `%8s`, which pads the value with spaces. A precision,
such as `%.2f`, sets the number of decimals. A width counts `"é"` as one character. `%2$s` uses the second value, so a template can use the values in
another order.

When the template is written in the code, Novis checks it when the program compiles. Every
placeholder needs a value, and every value must be used. A template built while the program runs
throws a `LogicError` if it does not fit its values. The template must be trusted text, so a string
from a user is not allowed.

`%f` always writes a `.` before the decimals. For a separator between thousands, use
`Core\Math::format`.

related: Core\Math::format, Core\Str::padStart
