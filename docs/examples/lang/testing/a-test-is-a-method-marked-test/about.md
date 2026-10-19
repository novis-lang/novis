A test is a method with `#[Test]` written above it.

Any class in your program may have tests. There is no base class to extend, and no name a test
class or a test method has to have. You write `use Core\Test;` once, put `#[Test]` above a method,
and run `nvs test yourfile.nvs`. The compiler collects every `#[Test]` method while it checks your
program.

A test method is `public`, is not `static`, and returns `void`. A method that breaks one of those
three rules does not compile. A test that checks nothing fails, so every test calls at least one
assertion.

**Good to know:** `nvs run` on the same file runs your program and runs no test.
