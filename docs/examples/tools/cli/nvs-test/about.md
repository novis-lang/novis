`nvs test <paths>` runs tests. It runs one of two kinds of test, and the path decides which kind.

A `.nvs` file is a program, and the command runs every method that has the `#[Test]` attribute. A
directory with `.nvs` files is run as one program. Any other path is a `.nvst` case file, or a
directory of such files. A case is one program with its expected output.

`--filter <text>` runs only the tests whose name contains the text. `--list` prints the tests and
runs none. `--format json` and `--format junit` print a report for a build server. The exit status
is not zero when a test failed. A skipped test is not a failure.

`--fail-fast` stops after the first failure. `--rerun-failed` runs only the tests that failed in the
last run. `--watch` runs the tests again each time you save a file they use. `nvs test --help` lists
every flag.

`--tag` and `--skip-tag` select tests by their tags. `--bench` runs the benches and no test.

**Good to know:** `--update` writes the new text into each failed `Core\Test::assertMatchesInline`
snapshot. Those tests are still reported as failed, so run the tests again.
