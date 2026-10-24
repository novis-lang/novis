`nvs test <paths>` runs tests. It runs one of two kinds of test, and the path decides which kind.

A `.nvs` file is a program, and the command runs every method that has the `#[Test]` attribute. A
directory with `.nvs` files is run as one program. Any other path is a `.nvst` case file, or a
directory of such files. A case is one program with its expected output.

`--filter <text>` runs only the tests whose name contains the text. `--format json` and
`--format junit` print a report for a build server. `--list` prints the tests and runs none. The
exit status is not zero when a test failed. A skipped test is not a failure.

`--fail-fast` stops starting tests after the first test fails. The summary counts the tests that did
not start. `--rerun-failed` runs only the tests that failed in the last run of the program. `--shuffle`
starts the tests in a random order and prints the seed. `--shuffle=<seed>` repeats that order. The
report still lists the tests in the order they are written.

`--coverage-lcov <file>`, `--coverage-clover <file>` and `--coverage-cobertura <file>` write which
lines, functions and branches the tests ran. Most build servers and editors can read these files.

**Good to know:** one call runs one kind of test. `--update` writes the new text into each failed
`Core\Test::assertMatchesInline` snapshot in your source file. Those tests are still reported as
failed, so run the tests again.
