`Summary::class()` returns the name of the test class, with its namespace. For a class `CartTest` in
the namespace `Shop\Tests`, the result is `Shop\Tests\CartTest`.

You get a `Summary` as the parameter of a `public static` method with `#[AfterClass]`. The test runner
creates it after the last test of the class and passes it to that method. You cannot create a
`Summary` yourself, and your own class cannot extend it.

Use `class()` to label a report about the whole class, such as a heading or a file name.

**The examples below** show a heading for each test class, the class name without its namespace, and
one report file for each class.
