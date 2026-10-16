A `#[Test]` method may be declared in any file — beside the class it tests, or in a separate tree.
What makes a method a test is the attribute, never the path.

`nvs test` compiles and runs them. `nvs run` and `nvs build` do not lower them at all: a test
method, its fixtures, its data rows, its assertion messages and its doubles are absent from a built
artifact. Production pays nothing for them, and no test surface is reachable at run time.

`nvs check` does type-check test code. That is the one place tests and non-tests are treated alike,
and deliberately so: a test cannot rot silently while the code around it changes.

**Not on disk.** `nvs check` type-checks test methods (`crates/nvs-types/src/check.rs:413`). `nvs run`
and `nvs build` still lower them: nothing in the lowering leaves a `#[Test]` method out
(`crates/nvs-ir/src/lower/mod.rs:774`).
