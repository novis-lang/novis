`nvs test main.nvs` compiles a program and runs every method that has the `#[Test]` attribute. It
prints one line for each test and a count at the end.

You can write tests in the same file as the code, or in a separate file that loads the code. The
statements outside a class do not run under `nvs test`.

`nvs test tests/` runs every `.nvs` file in a directory as one program, so you do not need a list
of files. Put one file in that directory that requires the bootstrap file of your application.
Every test file can then use the classes it loads.

`--filter <text>` runs only the tests whose name contains that text. The name of a test is
`Class::method`, so `--filter CartTest::` runs one class. `--list` prints the tests and where each
one is written, and runs none of them. `--format json` and `--format junit` print a report for a
build server. The exit status is `1` when a test failed and `0` when no test failed.

**The examples below** show a program with two tests, a program with two test classes and a filter,
and a program that a build server runs.
