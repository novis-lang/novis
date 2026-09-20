A test is a method with `#[Test]` written above it.

Any class in your program may hold tests. There is no base class to extend, no name a test class or
a test method has to have, and no directory the runner looks in. You write `use Core\Test;` once,
put `#[Test]` above a method, and run `nvs test yourfile.nvs`. The compiler collects every marked
method while it checks your program, so the runner has the whole list before the first test starts.

A test method is `public`, is not `static`, and returns `void`. A method that breaks one of those
three rules does not compile. A test that checks nothing fails, so every test calls at least one
assertion.

**Good to know:** `nvs run` on the same file runs your program and leaves the tests alone.
