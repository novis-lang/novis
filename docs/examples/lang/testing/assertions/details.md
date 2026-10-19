`Core\Test::assertSame` compares numbers, strings and arrays by value. Two objects pass only when
they are the same object, so `assertSame(new P(1), new P(1))` fails.

`assertEquals` is the same as `assertSame`, but it compares two objects with their `compareTo`
method. It passes when `compareTo` returns `0`. For a class without `compareTo`, it throws a
`RuntimeError`, and the test fails.

`assertEqualsDeep` compares arrays entry by entry and objects property by property. The message
names the first place where the two differ, such as `$actual->rows["1"]->name`. A `secret`
property is compared, and the message shows `«redacted»` in place of its value.

Both sides of `assertSame`, `assertEquals` and `assertEqualsDeep` have one type. `assertSame(1, "1")`
and `assertSame(1, 1.0)` are `E0401`. `assertTrue` takes a `bool`, so `assertTrue(1)` is `E0401`.

`assertCount` counts every entry, with any keys.

`assertThrows` passes when the function throws the class you name or a subclass of it. Its message
names the class that was thrown instead, or says that the function returned. `assertDoesNotThrow`
fails with the message of the error the function threw.

An assertion that fails inside the function you give `assertThrows` still makes the test fail. To
test that an assertion fails, use `Core\Test::expectFailure`. It passes when an assertion inside the
function failed, and that failure does not count against the test. It fails when no assertion
inside the function failed.

A failing assertion throws, so the lines after it in the test do not run. A test that catches each
`Core\Test\Failure` goes on, and the report lists every failed assertion.

`Core\Test\Failure` is a `Throwable`. It is not a `RuntimeError` and not a `LogicError`.

Under `nvs run`, an assertion that fails throws the same `Core\Test\Failure`. If nothing catches it,
the program ends with exit status `1`.

`Core\Test::advance` is not an assertion. It moves the clock that `#[Test(at:)]` fixed.
