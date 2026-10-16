Novis has the work of PHP's common development tools built in, so you install none of them.

Tests replace PHPUnit. A test is a method with the `#[Test]` attribute, `Core\Test` has the
assertions, and `nvs test` runs the tests. Data rows, a skip with a reason, retries, and JUnit and
JSON reports are included.

`nvs check` replaces PHPStan and Psalm. It is the compiler. It checks that every variable has a
type, every method exists, every path returns a value and every property is initialized. It also
checks where untrusted input (`tainted`) and secret values (`secret`) are used. It has no levels
and no baseline file.

The style rules that find bugs are compile errors, and this replaces PHP CS Fixer and
PHP_CodeSniffer. The compiler checks the case of every name and the visibility of every method,
property and constant. You configure nothing.
