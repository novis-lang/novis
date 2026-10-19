`Outcome::class()` returns the name of the test class that ran, with its namespace. For a class
`CartTest` in the namespace `Shop\Tests`, the result is `Shop\Tests\CartTest`.

You get an `Outcome` as the parameter of a method with `#[AfterEachTest]`. The test runner creates it
after each run of each test and passes it to that method. You cannot create an `Outcome` yourself,
and your own class cannot extend it.

Use `class()` with `name()` to build a label for one test, such as a log file name or a line in a
report.

**The examples below** show a label for each test, the class name without its namespace, and one log
file for each test class.
