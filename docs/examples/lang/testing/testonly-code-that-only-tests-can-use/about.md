`#[TestOnly]` says that a class, an interface, an enum or a method is only for tests. You write it
in your normal code, next to the code it marks. A fake service for your tests or a method that
resets a cache between tests are typical uses.

- On a method, only test code can call the method.
- On a class, interface or enum, only test code can use the type. This includes every method and
  the constructor.

Test code is a method with a testing attribute, such as `#[Test]` or `#[Fixture]`, and any
`#[TestOnly]` code. Any other use of test-only code does not compile.

`nvs run` and `nvs build` leave all test-only code out of your program. `nvs test` compiles it.

**Good to know:** `Core\Program::implementing` never lists a test-only class, also under `nvs test`.
So your tests see the same classes as your program.
