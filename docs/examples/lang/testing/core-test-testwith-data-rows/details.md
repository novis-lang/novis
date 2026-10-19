A row value is a literal of the parameter's type: an `int`, `uint`, `float`, `bool` or `string`
written in the code. An `int` literal also fills a `float` parameter. A class constant, an enum case,
an array and a negative number for a `uint` are `E0738`.

A field that names no parameter is `E0738`, and so is a value of the wrong type. A field given twice
in one row is `E0304`. A row that leaves out a parameter that another row fills is `E0738`.

`#[TestWith]` on a method without `#[Test]` is `E0738`. A parameter with no row and no fixture is
`E0736`.

When a row and a fixture can both fill a parameter, the row is used.

`skip:` on a test with rows skips every row, and each row is reported as skipped. With `retries:`,
each row gets its own attempts.

`--filter` selects a single row by its name, such as `--filter SlugTest::itSlugs#1`.
