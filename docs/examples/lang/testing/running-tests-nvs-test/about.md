`nvs test main.nvs` compiles a program and runs every method marked `#[Test]` in it. It prints one
line per test and a count at the end.

Tests live wherever a class lives. You can write them beside the code in one file, or put them in a
file of their own that loads the code under test. The lines outside a class do not run under
`nvs test`.

`--filter <text>` runs only the tests whose name contains that text. A test's name is
`Class::method`, so `--filter CartTest::` runs one class. `--list` prints which tests the program
declares and where each one is written, without running any of them. `--format json` and
`--format junit` print a machine-readable report instead, which is what a build server reads. The
exit status is `1` when a test failed and `0` when none did.

**The examples below** show a program with two tests, a program with two test classes to filter
between, and a program a build server runs.
