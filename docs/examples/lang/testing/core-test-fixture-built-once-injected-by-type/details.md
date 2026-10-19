A fixture method that is not `static` or not `public`, or that returns `void`, is `E0735`. A
`#[Fixture]` with a field, such as `#[Fixture(name: "x")]`, is `E0454`.

Two fixtures of one class that return the same type are `E0304`. Fixtures that need each other, such
as `s` needing `r` and `r` needing `s`, are `E0737`, and the message shows the cycle.

The type must be the same. A fixture that returns a subclass `S2` does not fill a parameter of type
`S`, and that parameter is `E0736`. A fixture may return any type, such as `int`, `?int` or
`array<int>`.

A fixture is built only when a test that runs needs it. A fixture that only skipped tests need is
not built.

A fixture that throws an error makes each test that needs it fail, with the error's message. The
other tests of the class still run.

A variadic or `inout` parameter on a test method is `E0733`.

A fixture method is an ordinary `static` method. Other code may call it, and `nvs run` runs it like
any other method.
