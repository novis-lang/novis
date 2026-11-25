Reads an array as the type you write at the call site. Form and query values arrive as text.

The type goes between angle brackets. It can be a shape, such as `{ name: string, age: int }`, or a
class with `#[Core\Json\Derive]`. A class inside a shape or an array needs the attribute too. Each
field the type names is converted with `as`, and other keys are left out. Write `array<T>` to read a
whole list in one call.

A field can also be a time value or a `Core\Uuid`, read from text. A date or a time is its ISO text,
such as `2026-05-14`. A `Duration` is text such as `1h30m`, and a `Zone` is a name such as
`Europe/Vienna`.

**Good to know:** a missing field, or a value its type cannot take, throws a `ParseError`. The error
lists every field that failed at its own path, so a form can show all its problems at once.

**The examples below** read a form, order lines, a search query and a date with a time.
